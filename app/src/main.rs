//! The executable: the bones kernel with a window, a renderer, and the app's own
//! egui interface, in one native process.

use bones_kernel::logging::Logger;
use m410_rgb_app::gui::{EguiModule, Panel};
use m410_rgb_app::{Backend, UsbMouse};

const TITLE: &str = "Legion M410 RGB";

fn main() {
    let logger = Logger::default();
    let panel = Panel::new(Backend::spawn(UsbMouse));
    let result = bones_kernel::Kernel::new()
        .logger(logger.clone())
        .window(TITLE, 480, 700)
        .renderer()
        .module(EguiModule::new(panel, logger))
        .devtools_from_env("BONES_DEVTOOLS_PORT")
        .offscreen_from_env("BONES_OFFSCREEN")
        .run();
    if let Err(error) = result {
        eprintln!("fatal: {error}");
        std::process::exit(1);
    }
}
