//! What the mouse's light is doing, so the panel can show it moving.

use m410_messages::{Effect, Lighting, BRIGHTNESS_MAX};

use crate::color::{self, Rgb};

/// Seconds for one breath, from the slowest speed to the fastest.
fn breath_seconds(speed: u8) -> f32 {
    7.0 - 1.1 * f32::from(speed)
}

/// Seconds for the color to go once around the wheel.
fn cycle_seconds(speed: u8) -> f32 {
    14.0 - 2.5 * f32::from(speed)
}

/// 0 when off, 1 at full brightness.
pub fn level(lighting: &Lighting) -> f32 {
    f32::from(lighting.brightness) / f32::from(BRIGHTNESS_MAX)
}

fn hue_at(lighting: &Lighting, clock: f32, offset: f32) -> Rgb {
    color::hue(360.0 * clock / cycle_seconds(lighting.speed) + offset)
}

/// The color the whole light shows at `clock` seconds, already dimmed by brightness
/// and, for breathing, by the breath.
pub fn glow(lighting: &Lighting, clock: f32) -> Rgb {
    let base = match lighting.effect {
        Effect::ColorCycle | Effect::RainbowWave => hue_at(lighting, clock, 0.0),
        _ if lighting.random => hue_at(lighting, clock, 0.0),
        _ => lighting.color,
    };
    let mut strength = level(lighting);
    if lighting.effect == Effect::Breathing {
        let phase = std::f32::consts::TAU * clock / breath_seconds(lighting.speed);
        strength *= 0.2 + 0.8 * (0.5 + 0.5 * phase.sin());
    }
    color::scale(base, strength)
}

/// The color of one of `bands` slices of the light. Only the rainbow wave differs from
/// slice to slice; everything else shows [`glow`] across the whole light.
pub fn band(lighting: &Lighting, clock: f32, index: usize, bands: usize) -> Rgb {
    if lighting.effect != Effect::RainbowWave {
        return glow(lighting, clock);
    }
    let offset = 360.0 * index as f32 / bands as f32;
    color::scale(hue_at(lighting, clock, -offset), level(lighting))
}
