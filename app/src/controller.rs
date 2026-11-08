use m410::{Connection, Lighting};

use crate::{Command, Mouse, MouseError, Snapshot, Status};

/// How many polls a change waits for a sleeping mouse before it is dropped.
/// A poll is 1.5 s of waiting plus an attempt that takes up to 4.5 s to fail
/// against a sleeping mouse, so ten of them are about a minute.
const MAX_WAITS: u32 = 10;

/// Decides what to do with commands and with the passage of time.
///
/// - Runs on the worker thread; every method may block while it talks to the mouse.
/// - A change is held as `pending` until the mouse accepts it, so a sleeping
///   wireless mouse gets it when it wakes, without the user clicking again.
/// - Methods return the [`Snapshot`] to publish, so the caller owns the bus and
///   this type needs none, which is what lets the tests run without one.
pub struct Controller<M: Mouse> {
    mouse: M,
    pending: Option<Lighting>,
    waits_left: u32,
    shown: Snapshot,
}

impl<M: Mouse> Controller<M> {
    pub fn new(mouse: M) -> Self {
        Self {
            mouse,
            pending: None,
            waits_left: 0,
            shown: Snapshot::unknown(),
        }
    }

    /// Handles everything that arrived while the worker was busy. Only the newest
    /// change is applied, so a burst of clicks costs one flash write, not many.
    pub fn handle(&mut self, commands: Vec<Command>) -> Vec<Snapshot> {
        let newest = commands.iter().rev().find_map(|command| match command {
            Command::Apply(lighting) => Some(*lighting),
            Command::Refresh => None,
        });
        match newest {
            // The optimistic snapshot first, so the panel responds at once
            // instead of after the write.
            Some(lighting) => vec![self.queue(lighting), self.flush()],
            None => vec![self.refresh()],
        }
    }

    /// Notes the change and shows it, without touching the mouse.
    fn queue(&mut self, lighting: Lighting) -> Snapshot {
        self.pending = Some(lighting);
        self.waits_left = MAX_WAITS;
        self.shown.lighting = Some(lighting);
        self.shown.clone()
    }

    /// Reads the mouse, publishing even if nothing changed.
    fn refresh(&mut self) -> Snapshot {
        self.waits_left = MAX_WAITS;
        self.attempt();
        self.shown.clone()
    }

    fn flush(&mut self) -> Snapshot {
        self.attempt();
        self.shown.clone()
    }

    /// Called when no command arrived for a while. Returns a snapshot only when
    /// something changed: a plug or unplug, or a sleeping mouse that woke.
    pub fn poll(&mut self) -> Option<Snapshot> {
        let before = self.shown.clone();
        if self.shown.status == Status::Waiting {
            if self.waits_left == 0 {
                self.pending = None;
                self.give_up();
            } else {
                self.waits_left -= 1;
                self.attempt();
            }
        } else {
            let present = self.mouse.connection().ok();
            if present != self.shown.connection {
                self.waits_left = MAX_WAITS;
                self.attempt();
            }
        }
        (self.shown != before).then(|| self.shown.clone())
    }

    /// One try at the mouse: writes the pending change, or reads if there is none.
    fn attempt(&mut self) {
        let connection = match self.mouse.connection() {
            Ok(connection) => connection,
            Err(error) => return self.fail(None, error),
        };
        let result = match self.pending {
            Some(lighting) => self.mouse.write(&lighting).map(|()| lighting),
            None => self.mouse.read(),
        };
        match result {
            Ok(lighting) => {
                self.pending = None;
                self.shown = Snapshot {
                    connection: Some(connection),
                    status: Status::Ready,
                    lighting: Some(lighting),
                    detail: String::new(),
                };
            }
            Err(error) => self.fail(Some(connection), error),
        }
    }

    fn fail(&mut self, connection: Option<Connection>, error: MouseError) {
        let lighting = self.shown.lighting;
        let (status, detail) = match error {
            MouseError::Asleep => {
                let detail = match self.pending {
                    Some(_) => "The mouse is asleep. Move it to wake it; your change applies then.",
                    None => "The mouse is asleep. Move it to wake it.",
                };
                (Status::Waiting, detail.to_string())
            }
            MouseError::NotFound => {
                self.pending = None;
                (
                    Status::NotFound,
                    "Plug in the mouse or its wireless receiver.".to_string(),
                )
            }
            MouseError::NoAccess(path) => {
                self.pending = None;
                (
                    Status::NoAccess,
                    format!("No permission to open {path}. Install the udev rule from res/ and replug the mouse."),
                )
            }
            MouseError::Failed(message) => {
                self.pending = None;
                (Status::Failed, message)
            }
        };
        self.shown = Snapshot {
            connection,
            status,
            lighting,
            detail,
        };
    }

    fn give_up(&mut self) {
        self.shown.status = Status::Failed;
        self.shown.detail = "The mouse did not wake up. Move it and press Refresh.".to_string();
    }
}

#[cfg(test)]
mod tests;
