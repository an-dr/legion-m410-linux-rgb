use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use crate::{Command, Controller, Mouse, Snapshot};

/// How long the worker waits for a command before checking the mouse is still there.
const POLL_INTERVAL: Duration = Duration::from_millis(1500);

/// The mouse, seen from the interface: commands go in and snapshots come out,
/// and nothing on the interface's side ever waits for the device.
///
/// A sleeping wireless mouse takes seconds to give up on a request, so all
/// device I/O runs on a worker thread. The thread ends when the `Backend` is dropped.
pub struct Backend {
    commands: Sender<Command>,
    snapshots: Receiver<Snapshot>,
}

impl Backend {
    pub fn spawn<M: Mouse>(mouse: M) -> Self {
        Self::spawn_polling(mouse, POLL_INTERVAL)
    }

    /// As [`Backend::spawn`], checking in more often; for tests.
    pub fn spawn_polling<M: Mouse>(mouse: M, poll: Duration) -> Self {
        let (commands, inbox) = mpsc::channel();
        let (outbox, snapshots) = mpsc::channel();
        let controller = Controller::new(mouse);
        thread::Builder::new()
            .name("m410-worker".into())
            .spawn(move || work(controller, inbox, outbox, poll))
            .expect("the operating system refused a thread");
        Self { commands, snapshots }
    }

    /// A backend with no worker behind it, for tests that watch the commands go out
    /// and feed in the snapshots themselves.
    #[cfg(test)]
    pub(crate) fn from_channels(commands: Sender<Command>, snapshots: Receiver<Snapshot>) -> Self {
        Self { commands, snapshots }
    }

    pub fn send(&self, command: Command) {
        // The worker only ends once this value is dropped, so a failed send has nobody to tell.
        let _ = self.commands.send(command);
    }

    /// The next snapshot the worker has published, if any, without waiting.
    pub fn try_recv(&self) -> Option<Snapshot> {
        self.snapshots.try_recv().ok()
    }

    /// The next snapshot, waiting up to `timeout`; for tests.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Snapshot> {
        self.snapshots.recv_timeout(timeout).ok()
    }
}

fn work<M: Mouse>(mut controller: Controller<M>, inbox: Receiver<Command>, outbox: Sender<Snapshot>, poll: Duration) {
    loop {
        let snapshots = match inbox.recv_timeout(poll) {
            Ok(first) => {
                let mut batch = vec![first];
                batch.extend(inbox.try_iter());
                controller.handle(batch)
            }
            Err(RecvTimeoutError::Timeout) => controller.poll().into_iter().collect(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        for snapshot in snapshots {
            if outbox.send(snapshot).is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests;
