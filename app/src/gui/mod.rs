//! The interface: egui, owned by this app, drawn through the engine's `draw-target`
//! service and themed after the desktop. The same arrangement as github.com/an-dr/cross-wall.

mod colour;
mod desktop;
mod egui_module;
mod look;
mod panel;
mod preview;
mod theme;
mod widgets;
mod window;

pub use egui_module::{App, EguiModule};
pub use panel::Panel;
