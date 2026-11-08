use m410::{Connection, Lighting};

/// How the last attempt to talk to the mouse went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The mouse answered; the lighting is what it holds.
    Ready,
    /// The mouse did not answer, usually because it is asleep. A pending change
    /// is applied as soon as it wakes.
    Waiting,
    /// No M410 is connected.
    NotFound,
    /// The mouse is there but this user may not open it (the udev rule is missing).
    NoAccess,
    Failed,
}

/// A request from the interface to the worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Read the mouse and report, even if nothing changed.
    Refresh,
    /// Store these settings. Held until the mouse answers, so a sleeping mouse
    /// gets them when it wakes.
    Apply(Lighting),
}

/// What the worker knows, kept as owned data so the controller can compare one with the next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// `None` when neither the cable nor the receiver is plugged in.
    pub connection: Option<Connection>,
    pub status: Status,
    /// The lighting last read from or written to the mouse, if any.
    pub lighting: Option<Lighting>,
    /// A sentence for the user about what went wrong or what to do; empty when fine.
    pub detail: String,
}

impl Snapshot {
    /// Before anything has been learned about the mouse.
    pub fn unknown() -> Self {
        Self {
            connection: None,
            status: Status::NotFound,
            lighting: None,
            detail: String::new(),
        }
    }
}
