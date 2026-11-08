//! What the M410 native module and the UI extension say to each other.
//!
//! - `m410/command` goes to the module: apply these settings, or report what the
//!   mouse is doing now.
//! - `m410/state` comes back from the module whenever something changed.
//!
//! Both are published, not sent: talking to the mouse can take seconds when it
//! is asleep, and a direct `send` would hold the frame loop for that long.

mod command;
mod effect;
mod lighting;
mod state;

pub use command::Command;
pub use effect::Effect;
pub use lighting::{Lighting, BRIGHTNESS_MAX, SPEED_MAX, SPEED_MIN};
pub use state::{Connection, State, Status};

#[cfg(test)]
mod tests;
