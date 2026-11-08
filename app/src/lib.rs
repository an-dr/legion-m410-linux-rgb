//! A control panel for the Lenovo Legion M410's lighting, as a bones app.
//!
//! - [`Backend`] runs the mouse on a worker thread behind two channels.
//! - [`Controller`] is what that thread runs: a small state machine over the
//!   [`Mouse`] trait, so everything interesting is testable without hardware.
//! - [`gui`] is the interface: an egui module of this app's own, drawn through the
//!   engine's `draw-target` service, with a theme that follows the desktop.

mod backend;
mod controller;
pub mod gui;
mod mouse;
mod snapshot;

pub use backend::Backend;
pub use controller::Controller;
pub use mouse::{Mouse, MouseError, UsbMouse};
pub use snapshot::{Command, Snapshot, Status};
