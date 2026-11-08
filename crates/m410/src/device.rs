//! Transport: vendor HID reports on the mouse's second interface.
//!
//! Every request is a 64-byte output report with ID 4:
//! `04 00 00 <cmd> <len> <offset lo> <offset hi> 00 <data...>`.
//! The mouse answers with the same header, a status byte at index 7
//! (0 = accepted) and, for reads, the data from index 8.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::Error;

pub const VENDOR_ID: u16 = 0x17ef;
/// The mouse itself, connected by its USB cable.
pub const WIRED_PRODUCT_ID: u16 = 0x6197;
/// The wireless receiver, which relays the same reports to the mouse.
pub const RECEIVER_PRODUCT_ID: u16 = 0x6198;
const VENDOR_INTERFACE: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    Wired,
    Wireless,
}

impl Connection {
    /// The cable is tried first: it works even when the mouse is asleep.
    const SEARCH_ORDER: [(Connection, u16); 2] = [
        (Connection::Wired, WIRED_PRODUCT_ID),
        (Connection::Wireless, RECEIVER_PRODUCT_ID),
    ];
}

const REPORT_ID: u8 = 0x04;
const REPORT_LEN: usize = 64;
const HEADER_LEN: usize = 8;
const STATUS_INDEX: usize = 7;

const CMD_READ: u8 = 0x05;
const CMD_WRITE: u8 = 0x06;
const CMD_FACTORY_RESET: u8 = 0x0d;

/// Size of the configuration page that holds lighting, DPI stages and profile data.
pub const PAGE_LEN: usize = 0x80;
/// Largest payload that survives the wireless link (the cable allows 0x38).
const CHUNK_LEN: usize = 0x18;

/// The page is moved in these pieces. A write that starts at offset 0 makes the
/// firmware erase the whole page first; writes anywhere else can only clear bits,
/// so the page is always rewritten in full, starting at offset 0.
fn chunks() -> impl Iterator<Item = (usize, usize)> {
    (0..PAGE_LEN)
        .step_by(CHUNK_LEN)
        .map(|offset| (offset, CHUNK_LEN.min(PAGE_LEN - offset)))
}

/// Long enough for a sleeping wireless mouse to wake and answer.
const REPLY_TIMEOUT: Duration = Duration::from_millis(1500);
const ATTEMPTS: usize = 3;

pub type Page = [u8; PAGE_LEN];

pub struct Device {
    file: File,
    path: PathBuf,
    connection: Connection,
}

impl Device {
    /// Opens the first connected M410.
    pub fn open() -> Result<Self, Error> {
        let (path, connection) = find_hidraw()?.ok_or(Error::NotFound)?;
        let file = OpenOptions::new().read(true).write(true).open(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::PermissionDenied(path.clone())
            } else {
                Error::Io(e)
            }
        })?;
        Ok(Self { file, path, connection })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connection(&self) -> Connection {
        self.connection
    }

    pub fn read_page(&mut self) -> Result<Page, Error> {
        let mut page = [0u8; PAGE_LEN];
        for (offset, len) in chunks() {
            let reply = self.transact(CMD_READ, offset, len, &[])?;
            page[offset..offset + len].copy_from_slice(&reply[HEADER_LEN..HEADER_LEN + len]);
        }
        Ok(page)
    }

    /// Rewrites the whole page and reads it back to confirm the mouse stored it.
    pub fn write_page(&mut self, page: &Page) -> Result<(), Error> {
        for (offset, len) in chunks() {
            self.transact(CMD_WRITE, offset, len, &page[offset..offset + len])?;
        }
        if self.read_page()? != *page {
            return Err(Error::Protocol("page read back differs from what was written".into()));
        }
        Ok(())
    }

    /// Restores factory settings, including DPI stages. The LEDs blink to confirm.
    pub fn factory_reset(&mut self) -> Result<(), Error> {
        self.transact(CMD_FACTORY_RESET, 0, 0, &[])?;
        // The mouse ignores requests for a moment while it rewrites its flash.
        std::thread::sleep(Duration::from_millis(1500));
        Ok(())
    }

    /// Sends one request and waits for its reply. Every request is safe to repeat,
    /// so it is resent when the reply does not come, as happens while a wireless
    /// mouse wakes up.
    fn transact(&mut self, cmd: u8, offset: usize, len: usize, data: &[u8]) -> Result<[u8; REPORT_LEN], Error> {
        let mut request = [0u8; REPORT_LEN];
        request[0] = REPORT_ID;
        request[3] = cmd;
        request[4] = len as u8;
        request[5] = (offset & 0xff) as u8;
        request[6] = (offset >> 8) as u8;
        request[HEADER_LEN..HEADER_LEN + data.len()].copy_from_slice(data);

        for _ in 0..ATTEMPTS {
            self.drain()?;
            self.file.write_all(&request)?;
            if let Some(reply) = self.await_reply(&request)? {
                if reply[STATUS_INDEX] != 0 {
                    return Err(Error::Protocol(format!(
                        "command {cmd:#04x} rejected with status {:#04x}",
                        reply[STATUS_INDEX]
                    )));
                }
                return Ok(reply);
            }
        }
        Err(Error::NoReply(self.connection))
    }

    fn await_reply(&mut self, request: &[u8; REPORT_LEN]) -> Result<Option<[u8; REPORT_LEN]>, Error> {
        let deadline = Instant::now() + REPLY_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let Some(reply) = self.read_report(remaining)? else {
                return Ok(None);
            };
            // Skip mouse and keyboard input reports, which share this interface,
            // and late replies to an earlier attempt.
            if reply[..HEADER_LEN - 1] == request[..HEADER_LEN - 1] {
                return Ok(Some(reply));
            }
        }
    }

    /// Discards replies left over from earlier requests.
    fn drain(&mut self) -> Result<(), Error> {
        while self.read_report(Duration::ZERO)?.is_some() {}
        Ok(())
    }

    fn read_report(&mut self, timeout: Duration) -> Result<Option<[u8; REPORT_LEN]>, Error> {
        let mut pfd = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        // SAFETY: pfd is a valid pollfd for the duration of the call.
        let ready = unsafe { libc::poll(&mut pfd, 1, ms) };
        if ready < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if ready == 0 {
            return Ok(None);
        }
        // hidraw returns exactly one report per read. Mouse and keyboard reports
        // are shorter than 64 bytes; the rest stays zero.
        let mut report = [0u8; REPORT_LEN];
        let _len = self.file.read(&mut report)?;
        Ok(Some(report))
    }
}

/// Finds the hidraw node of the vendor interface through sysfs, cable first.
fn find_hidraw() -> Result<Option<(PathBuf, Connection)>, Error> {
    let interface = format!(":1.{VENDOR_INTERFACE}");
    let mut nodes = Vec::new();
    for entry in fs::read_dir("/sys/class/hidraw")?.flatten() {
        let Ok(device) = fs::canonicalize(entry.path().join("device")) else {
            continue;
        };
        let hid = device
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        let usb_if = device
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if usb_if.ends_with(&interface) {
            nodes.push((entry.file_name(), hid));
        }
    }
    nodes.sort();
    for (connection, product_id) in Connection::SEARCH_ORDER {
        let hid_id = format!(":{VENDOR_ID:04X}:{product_id:04X}.");
        if let Some((name, _)) = nodes.iter().find(|(_, hid)| hid.contains(&hid_id)) {
            return Ok(Some((Path::new("/dev").join(name), connection)));
        }
    }
    Ok(None)
}
