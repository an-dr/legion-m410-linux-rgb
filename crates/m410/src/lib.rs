//! Lighting control for the Lenovo Legion M410 mouse on Linux, over its USB cable
//! (`17ef:6197`) or its wireless receiver (`17ef:6198`).
//!
//! The mouse keeps its settings in one flash page. Changing anything means
//! reading that page, editing it, and rewriting it whole; the firmware applies
//! the new lighting at once and keeps it across power cycles.

mod device;
mod lighting;

use std::fmt;
use std::path::PathBuf;

pub use device::{Connection, Device, Page, PAGE_LEN, RECEIVER_PRODUCT_ID, VENDOR_ID, WIRED_PRODUCT_ID};
pub use lighting::{Effect, Lighting, Rgb, BRIGHTNESS_MAX, SPEED_MAX, SPEED_MIN};

#[derive(Debug)]
pub enum Error {
    NotFound,
    PermissionDenied(PathBuf),
    NoReply(Connection),
    Io(std::io::Error),
    Protocol(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound => write!(
                f,
                "Legion M410 not found (neither the mouse {VENDOR_ID:04x}:{WIRED_PRODUCT_ID:04x} \
                 nor its receiver {VENDOR_ID:04x}:{RECEIVER_PRODUCT_ID:04x} is connected)"
            ),
            Error::PermissionDenied(path) => write!(
                f,
                "no permission to open {}; install the udev rule from res/ (see the README) and replug the mouse",
                path.display()
            ),
            Error::NoReply(Connection::Wired) => write!(f, "the mouse did not answer"),
            Error::NoReply(Connection::Wireless) => write!(
                f,
                "the mouse did not answer through its receiver; switch it on and move it to wake it up"
            ),
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Protocol(msg) => write!(f, "unexpected reply from the mouse: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl Device {
    pub fn lighting(&mut self) -> Result<Lighting, Error> {
        Ok(Lighting::from_page(&self.read_page()?))
    }

    /// Stores new lighting. Skips the flash write when nothing changed.
    pub fn set_lighting(&mut self, lighting: &Lighting) -> Result<(), Error> {
        let mut page = self.read_page()?;
        let before = page;
        lighting.apply_to(&mut page);
        if page == before {
            return Ok(());
        }
        self.write_page(&page)
    }
}
