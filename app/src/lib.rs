//! The native half of the app: the one thing the sandboxed extension cannot do,
//! which is talking to the mouse.
//!
//! - [`M410Module`] is the bus endpoint. It receives `m410/command`, answers
//!   with `m410/state`, and does all device I/O on a worker thread.
//! - [`Controller`] is what the worker runs: a small state machine over the
//!   [`Mouse`] trait, so everything interesting is testable without hardware.
//! - [`UsbMouse`] is the real [`Mouse`], backed by the `m410` protocol crate.

mod controller;
mod convert;
mod module;
mod mouse;
mod snapshot;

pub use controller::Controller;
pub use module::{M410Module, ENDPOINT};
pub use mouse::{Mouse, MouseError, UsbMouse};
pub use snapshot::Snapshot;
