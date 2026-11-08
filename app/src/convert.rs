//! Translation between the `m410` protocol types and the bus vocabulary.
//!
//! Two copies of "lighting" exist on purpose: the vocabulary crate must compile
//! for wasm, so it cannot borrow the protocol crate's type.

use m410_messages::{Connection, Effect, Lighting};

use crate::MouseError;

pub(crate) fn effect_to_wire(effect: m410::Effect) -> Effect {
    match effect {
        m410::Effect::Static => Effect::Static,
        m410::Effect::Breathing => Effect::Breathing,
        m410::Effect::ColorCycle => Effect::ColorCycle,
        m410::Effect::RainbowWave => Effect::RainbowWave,
    }
}

pub(crate) fn effect_from_wire(effect: Effect) -> m410::Effect {
    match effect {
        Effect::Static => m410::Effect::Static,
        Effect::Breathing => m410::Effect::Breathing,
        Effect::ColorCycle => m410::Effect::ColorCycle,
        Effect::RainbowWave => m410::Effect::RainbowWave,
    }
}

pub(crate) fn lighting_to_wire(lighting: &m410::Lighting) -> Lighting {
    Lighting {
        effect: effect_to_wire(lighting.effect),
        brightness: lighting.brightness,
        speed: lighting.speed,
        random: lighting.random,
        color: (lighting.color.0, lighting.color.1, lighting.color.2),
    }
}

pub(crate) fn lighting_from_wire(lighting: &Lighting) -> m410::Lighting {
    m410::Lighting {
        effect: effect_from_wire(lighting.effect),
        brightness: lighting.brightness,
        speed: lighting.speed,
        random: lighting.random,
        color: m410::Rgb(lighting.color.0, lighting.color.1, lighting.color.2),
    }
}

pub(crate) fn connection(connection: m410::Connection) -> Connection {
    match connection {
        m410::Connection::Wired => Connection::Wired,
        m410::Connection::Wireless => Connection::Wireless,
    }
}

pub(crate) fn mouse_error(error: m410::Error) -> MouseError {
    match error {
        m410::Error::NotFound => MouseError::NotFound,
        m410::Error::PermissionDenied(path) => MouseError::NoAccess(path.display().to_string()),
        m410::Error::NoReply(_) => MouseError::Asleep,
        other => MouseError::Failed(other.to_string()),
    }
}

#[cfg(test)]
mod tests;
