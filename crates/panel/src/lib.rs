//! The M410 control panel, as pure data.
//!
//! The extension draws it with the engine's `gfx/*` primitives, so everything
//! here is geometry and decisions: where each control is ([`layout`]), what a
//! mouse press or drag means ([`Model`]), how the live preview moves
//! ([`preview`]), and the ordered list of draw commands for a frame ([`draw`]).
//! Nothing here touches the bus, which is what lets it run in plain unit tests.

pub mod color;
mod draw;
mod geometry;
pub mod layout;
mod model;
pub mod preview;
mod view;

pub use draw::{Align, Cmd};
pub use geometry::Rect;
pub use model::{Action, Control, Held, Model};
pub use view::draw;

#[cfg(test)]
mod tests;
