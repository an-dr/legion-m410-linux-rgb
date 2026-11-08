use m410_messages::{Connection, Lighting};

use crate::convert;

/// Why the mouse could not be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MouseError {
    /// Neither the cable nor the wireless receiver is plugged in.
    NotFound,
    /// The device is there but this user may not open it; carries the device path.
    NoAccess(String),
    /// The mouse did not answer, which is what a sleeping wireless mouse does.
    Asleep,
    Failed(String),
}

/// The mouse as the controller sees it. Reading and writing may block for seconds.
pub trait Mouse: Send + 'static {
    /// Which connection is present. Does not talk to the mouse, so it is cheap
    /// enough to ask every second.
    fn connection(&mut self) -> Result<Connection, MouseError>;
    fn read(&mut self) -> Result<Lighting, MouseError>;
    /// Stores the lighting on the mouse, verified by reading it back.
    fn write(&mut self, lighting: &Lighting) -> Result<(), MouseError>;
}

/// The real mouse, found fresh on every call so replugging needs no restart.
pub struct UsbMouse;

impl Mouse for UsbMouse {
    fn connection(&mut self) -> Result<Connection, MouseError> {
        let device = m410::Device::open().map_err(convert::mouse_error)?;
        Ok(convert::connection(device.connection()))
    }

    fn read(&mut self) -> Result<Lighting, MouseError> {
        let mut device = m410::Device::open().map_err(convert::mouse_error)?;
        device
            .lighting()
            .map(|l| convert::lighting_to_wire(&l))
            .map_err(convert::mouse_error)
    }

    fn write(&mut self, lighting: &Lighting) -> Result<(), MouseError> {
        let mut device = m410::Device::open().map_err(convert::mouse_error)?;
        device
            .set_lighting(&convert::lighting_from_wire(lighting))
            .map_err(convert::mouse_error)
    }
}
