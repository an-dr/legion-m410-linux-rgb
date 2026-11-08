use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use bones_kernel::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_kernel::logging::Logger;
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use m410_messages::{Command, State};

use crate::{Controller, Mouse, Snapshot, UsbMouse};

/// The bus endpoint this module answers as.
pub const ENDPOINT: &str = "m410";

/// How long the worker waits for a command before checking the mouse is still there.
const POLL_INTERVAL: Duration = Duration::from_millis(1500);

/// The native half of the app: owns the mouse so the sandboxed extension does not have to.
///
/// - Receives `m410/command` on the bus and publishes `m410/state`.
/// - All device I/O runs on one worker thread. A sleeping wireless mouse takes
///   seconds to give up, and a module that waited would stall every frame.
pub struct M410Module<M: Mouse = UsbMouse> {
    mouse: Option<M>,
    commands: Option<Sender<Command>>,
    logger: Logger,
}

impl M410Module<UsbMouse> {
    pub fn new(logger: Logger) -> Self {
        Self::with_mouse(UsbMouse, logger)
    }
}

impl<M: Mouse> M410Module<M> {
    /// For tests, which stand in a fake mouse for the real one.
    pub fn with_mouse(mouse: M, logger: Logger) -> Self {
        Self {
            mouse: Some(mouse),
            commands: None,
            logger,
        }
    }
}

impl<M: Mouse> Handler for M410Module<M> {
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic != Command::TOPIC {
            return;
        }
        match Command::decode(&envelope.payload) {
            Ok(command) => {
                if let Some(commands) = &self.commands {
                    // The worker only goes away at shutdown, so a failed send has nobody to tell.
                    let _ = commands.send(command);
                }
            }
            Err(error) => self
                .logger
                .warn(ENDPOINT, &format!("{} sent a bad command: {error}", envelope.sender)),
        }
    }
}

impl<M: Mouse> Module for M410Module<M> {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        let bus = context
            .get_service::<Bus>()
            .cloned()
            .ok_or("no Bus service available")?;
        let mouse = self.mouse.take().ok_or("the M410 module was initialised twice")?;
        context.subscribe(Command::TOPIC);

        let (commands, inbox) = mpsc::channel();
        self.commands = Some(commands);
        let controller = Controller::new(mouse);
        thread::Builder::new()
            .name("m410-worker".into())
            .spawn(move || work(controller, inbox, bus))
            .map_err(|error| format!("could not start the worker thread: {error}"))?;
        self.logger.info(ENDPOINT, "ready: listening for m410/command");
        Ok(())
    }

    fn begin_shutdown(&mut self) {
        // Closing the channel ends the worker once its current attempt finishes.
        self.commands = None;
    }
}

fn work<M: Mouse>(mut controller: Controller<M>, inbox: mpsc::Receiver<Command>, bus: Bus) {
    loop {
        match inbox.recv_timeout(POLL_INTERVAL) {
            Ok(first) => {
                let mut batch = vec![first];
                batch.extend(inbox.try_iter());
                for snapshot in controller.handle(batch) {
                    publish(&bus, &snapshot);
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Some(snapshot) = controller.poll() {
                    publish(&bus, &snapshot);
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn publish(bus: &Bus, snapshot: &Snapshot) {
    bus.publish(Envelope {
        topic: State::TOPIC.into(),
        sender: ENDPOINT.into(),
        correlation: None,
        payload: snapshot.state().encode(),
    });
}
