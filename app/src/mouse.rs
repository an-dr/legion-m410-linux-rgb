use m410::{Connection, Lighting};

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
        m410::Device::open().map(|device| device.connection()).map_err(error)
    }

    fn read(&mut self) -> Result<Lighting, MouseError> {
        m410::Device::open()
            .and_then(|mut device| device.lighting())
            .map_err(error)
    }

    fn write(&mut self, lighting: &Lighting) -> Result<(), MouseError> {
        m410::Device::open()
            .and_then(|mut device| device.set_lighting(lighting))
            .map_err(error)
    }
}

fn error(error: m410::Error) -> MouseError {
    match error {
        m410::Error::NotFound => MouseError::NotFound,
        m410::Error::PermissionDenied(path) => MouseError::NoAccess(path.display().to_string()),
        m410::Error::NoReply(_) => MouseError::Asleep,
        other => MouseError::Failed(other.to_string()),
    }
}

#[cfg(test)]
mod tests;
