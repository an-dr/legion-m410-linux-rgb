//! A rig that runs the real extension and the real native module together,
//! with a fake mouse where the USB device would be.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use bones_messages::{DecodeMessage, Message};
use bones_test::kernel::logging::Logger;
use bones_test::kernel::Kernel;
use bones_test::messages::gfx::DrawText;
use bones_test::messages::input::{MouseDown, MouseMove, MouseUp};
use bones_test::App;
use m410_messages::{Command, Connection, Effect, Lighting, State, Status};
use m410_rgb_app::{M410Module, Mouse, MouseError};

/// What the fake mouse holds, shared so a test can change it while the app runs.
pub struct Hardware {
    pub connection: Result<Connection, MouseError>,
    pub asleep: bool,
    pub stored: Lighting,
    pub writes: Vec<Lighting>,
}

#[derive(Clone)]
pub struct FakeMouse(pub Arc<Mutex<Hardware>>);

impl Mouse for FakeMouse {
    fn connection(&mut self) -> Result<Connection, MouseError> {
        self.0.lock().unwrap().connection.clone()
    }

    fn read(&mut self) -> Result<Lighting, MouseError> {
        let hardware = self.0.lock().unwrap();
        if hardware.asleep {
            return Err(MouseError::Asleep);
        }
        Ok(hardware.stored)
    }

    fn write(&mut self, lighting: &Lighting) -> Result<(), MouseError> {
        let mut hardware = self.0.lock().unwrap();
        if hardware.asleep {
            return Err(MouseError::Asleep);
        }
        hardware.stored = *lighting;
        hardware.writes.push(*lighting);
        Ok(())
    }
}

pub fn factory_lighting() -> Lighting {
    Lighting {
        effect: Effect::Breathing,
        brightness: 4,
        speed: 3,
        random: true,
        color: (255, 0, 255),
    }
}

pub struct Rig {
    pub app: App,
    pub hardware: Arc<Mutex<Hardware>>,
    saves: PathBuf,
}

impl Rig {
    pub fn start() -> Self {
        Self::start_with(|_| {})
    }

    /// Starts the app after `setup` has adjusted the fake mouse.
    pub fn start_with(setup: impl FnOnce(&mut Hardware)) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let mut hardware = Hardware {
            connection: Ok(Connection::Wireless),
            asleep: false,
            stored: factory_lighting(),
            writes: Vec::new(),
        };
        setup(&mut hardware);
        let hardware = Arc::new(Mutex::new(hardware));

        let saves = std::env::temp_dir().join(format!(
            "m410-checks-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let kernel = Kernel::new()
            .saves_dir(&saves)
            .module(M410Module::with_mouse(FakeMouse(hardware.clone()), Logger::default()));
        let wasm = bones_test::build(concat!(env!("CARGO_MANIFEST_DIR"), "/../extension")).unwrap();
        let app = App::with_kernel(kernel, wasm).unwrap();
        Self { app, hardware, saves }
    }

    /// Steps frames until the newest `m410/state` satisfies `done`, or fails.
    /// The module answers from a worker thread, so frames and wall time both pass.
    pub fn settle(&mut self, what: &str, done: impl Fn(&State<'_>) -> bool) {
        for _ in 0..300 {
            self.app.step(1);
            sleep(Duration::from_millis(20));
            if self.state().is_some_and(|state| done(&state)) {
                return;
            }
        }
        let logs: Vec<String> = self
            .app
            .logs()
            .into_iter()
            .filter(|line| !line.starts_with("DEBUG"))
            .collect();
        panic!("never reached: {what}\nlast state: {:?}\nlogs: {logs:?}", self.state());
    }

    pub fn state(&self) -> Option<State<'_>> {
        self.app
            .messages()
            .iter()
            .rev()
            .find(|envelope| envelope.topic == State::TOPIC)
            .and_then(|envelope| State::decode(&envelope.payload).ok())
    }

    /// Every command the extension published, oldest first.
    pub fn commands(&self) -> Vec<Command> {
        self.app
            .messages()
            .iter()
            .filter(|envelope| envelope.topic == Command::TOPIC)
            .filter_map(|envelope| Command::decode(&envelope.payload).ok())
            .collect()
    }

    /// Every piece of text the panel drew in the last frame.
    pub fn texts(&mut self) -> Vec<String> {
        self.app.clear().step(1);
        self.app
            .published::<DrawText>()
            .iter()
            .map(|draw| draw.text.to_string())
            .collect()
    }

    /// A press and release of the left button at a point of the panel.
    pub fn click_at(&mut self, (x, y): (i32, i32)) {
        self.app.send(&MouseDown {
            button: 1,
            x: x as f32,
            y: y as f32,
        });
        self.app.step(1);
        self.app.send(&MouseUp {
            button: 1,
            x: x as f32,
            y: y as f32,
        });
        self.app.step(2);
    }

    /// Presses at `from`, drags to `to`, and releases there: how a slider is used.
    pub fn drag(&mut self, from: (i32, i32), to: (i32, i32)) {
        self.app.send(&MouseDown {
            button: 1,
            x: from.0 as f32,
            y: from.1 as f32,
        });
        self.app.step(1);
        self.app.send(&MouseMove {
            x: to.0 as f32,
            y: to.1 as f32,
            dx: 0.0,
            dy: 0.0,
        });
        self.app.step(1);
        self.app.send(&MouseUp {
            button: 1,
            x: to.0 as f32,
            y: to.1 as f32,
        });
        self.app.step(2);
    }

    /// How many `gfx/*` messages one frame costs.
    pub fn frame_commands(&mut self) -> usize {
        self.app.clear().step(1);
        self.app
            .messages()
            .iter()
            .filter(|envelope| envelope.topic.starts_with("gfx/"))
            .count()
    }

    pub fn writes(&self) -> usize {
        self.hardware.lock().unwrap().writes.len()
    }

    pub fn stored(&self) -> Lighting {
        self.hardware.lock().unwrap().stored
    }

    pub fn ready(&mut self) {
        self.settle("the first state", |state| {
            state.status == Status::Ready && state.lighting.is_some()
        });
    }

    /// Waits for the mouse to hold `expected`, which is how a click shows it landed.
    pub fn wait_for_stored(&mut self, expected: Lighting) {
        self.settle("the mouse to hold the change", |state| {
            state.status == Status::Ready && state.lighting == Some(expected)
        });
        assert_eq!(self.stored(), expected);
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.saves);
    }
}
