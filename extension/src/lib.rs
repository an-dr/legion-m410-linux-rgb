//! The control panel for the Legion M410's lighting, drawn with the engine's gfx primitives.
//!
//! It has no way to reach the mouse. The panel's logic is the `m410-panel` crate; this
//! extension feeds it the mouse and the module's state, draws what it says each frame, and
//! passes on the changes it asks for to the native `m410` module over the bus.

mod output;

use std::cell::{Cell, RefCell};

use bones_wasm_sdk::bindings::bones::extension::host_api::{log, Level};
use bones_wasm_sdk::messages::gfx::SetDisplay;
use bones_wasm_sdk::messages::input::{MouseDown, MouseMove, MouseUp};
use bones_wasm_sdk::messages::renderer::{DisplayChanged, LogicalCanvas};
use bones_wasm_sdk::messages::tick::Tick;
use bones_wasm_sdk::messages::{DecodeMessage, Message};
use bones_wasm_sdk::{bus, Guest, Inbound};
use m410_messages::{Command, State};
use m410_panel::{Action, Model};

/// The platform's index for the primary button.
const LEFT_BUTTON: u8 = 1;

/// Seconds between asks for the window's size.
const SIZE_SYNC_SECONDS: f32 = 1.5;

thread_local! {
    static MODEL: RefCell<Model> = RefCell::new(Model::default());
    /// Counts up to the next size sync; starts due, so the first frame asks.
    static SINCE_SYNC: Cell<f32> = const { Cell::new(SIZE_SYNC_SECONDS) };
}

/// Asks for the window to be the panel's size, which makes the renderer answer
/// with the size it really is.
///
/// The renderer announces the window's size only after such a request, not when
/// the window is resized, and a mouse position is only meaningful against it. The
/// panel is a fixed layout anyway, so keeping the window its size costs nothing,
/// and the request changes nothing when the window already is.
fn sync_window_size(dt: f32) {
    let elapsed = SINCE_SYNC.get() + dt;
    if elapsed < SIZE_SYNC_SECONDS {
        SINCE_SYNC.set(elapsed);
        return;
    }
    SINCE_SYNC.set(0.0);
    bus::publish(&SetDisplay {
        width: m410_panel::layout::WIDTH,
        height: m410_panel::layout::HEIGHT,
        fullscreen: false,
    });
}

struct Component;

impl Guest for Component {
    fn init() {
        bus::subscribe::<Tick>();
        bus::subscribe::<MouseDown>();
        bus::subscribe::<MouseUp>();
        bus::subscribe::<MouseMove>();
        bus::subscribe::<LogicalCanvas>();
        bus::subscribe::<DisplayChanged>();
        bus::subscribe::<State<'static>>();
        // Ask for the state: the module may have published it before this
        // extension was listening.
        bus::publish(&Command::Refresh);
        log(Level::Info, "ready");
    }

    fn shutdown() {}

    fn on_tick(dt: f32) {
        sync_window_size(dt);
        MODEL.with(|model| {
            model.borrow_mut().tick(dt);
            // The whole frame, every tick: the renderer keeps what it was last given, so
            // anything not republished would stay on screen, and the clear is what stops
            // each frame landing on the one before.
            for cmd in m410_panel::draw(&model.borrow()) {
                output::publish(&cmd);
            }
        });
    }

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        if topic == State::TOPIC {
            match State::decode(&payload) {
                Ok(state) => MODEL.with(|model| model.borrow_mut().set_state(&state)),
                Err(error) => log(Level::Warn, &format!("bad {topic}: {error}")),
            }
            return None;
        }
        let action = MODEL.with(|model| {
            let mut model = model.borrow_mut();
            match Inbound::decode(&topic, &payload) {
                Ok(Inbound::MouseDown(press)) if press.button == LEFT_BUTTON => model.mouse_down(press.x, press.y),
                Ok(Inbound::MouseUp(release)) if release.button == LEFT_BUTTON => model.mouse_up(release.x, release.y),
                Ok(Inbound::MouseMove(motion)) => {
                    model.mouse_move(motion.x, motion.y);
                    None
                }
                Ok(Inbound::LogicalCanvas(canvas)) => {
                    log(Level::Debug, &format!("canvas {}x{}", canvas.width, canvas.height));
                    model.set_canvas(canvas.width, canvas.height);
                    None
                }
                Ok(Inbound::DisplayChanged(display)) => {
                    log(Level::Debug, &format!("window {}x{}", display.width, display.height));
                    model.set_window(display.width, display.height);
                    None
                }
                Ok(_) => None,
                Err(error) => {
                    log(Level::Warn, &format!("bad {topic}: {error}"));
                    None
                }
            }
        });
        match action {
            Some(Action::Apply(lighting)) => bus::publish(&Command::Apply(lighting)),
            Some(Action::Refresh) => bus::publish(&Command::Refresh),
            None => {}
        }
        None
    }
}

bones_wasm_sdk::export!(Component);
