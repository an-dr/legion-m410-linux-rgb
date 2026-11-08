use m410_messages::{Connection, Lighting, State, Status};

/// What the module last told the extension, kept as owned data so the
/// controller can compare it with the next one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub connection: Connection,
    pub status: Status,
    pub lighting: Option<Lighting>,
    pub detail: String,
}

impl Snapshot {
    /// Before anything has been learned about the mouse.
    pub fn unknown() -> Self {
        Self {
            connection: Connection::None,
            status: Status::NotFound,
            lighting: None,
            detail: String::new(),
        }
    }

    /// The wire form, borrowing this snapshot.
    pub fn state(&self) -> State<'_> {
        State {
            connection: self.connection,
            status: self.status,
            lighting: self.lighting,
            detail: &self.detail,
        }
    }
}
