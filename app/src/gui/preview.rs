//! What the mouse's light is doing, so the panel can show it moving.

use egui::Color32;
use m410::{Effect, Lighting, BRIGHTNESS_MAX};

/// A fully saturated, full-value color for a hue in degrees.
pub fn hue(degrees: f32) -> Color32 {
    let h = degrees.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Color32::from_rgb(
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

/// `color` darkened toward black: 1 leaves it, 0 is black.
pub fn scale(color: Color32, amount: f32) -> Color32 {
    let k = amount.clamp(0.0, 1.0);
    let channel = |c: u8| (f32::from(c) * k) as u8;
    Color32::from_rgb(channel(color.r()), channel(color.g()), channel(color.b()))
}

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

pub fn chosen(lighting: &Lighting) -> Color32 {
    Color32::from_rgb(lighting.color.0, lighting.color.1, lighting.color.2)
}

fn hue_at(lighting: &Lighting, clock: f32, offset: f32) -> Color32 {
    hue(360.0 * clock / cycle_seconds(lighting.speed) + offset)
}

/// The color the whole light shows at `clock` seconds, already dimmed by brightness
/// and, for breathing, by the breath.
pub fn glow(lighting: &Lighting, clock: f32) -> Color32 {
    let base = match lighting.effect {
        Effect::ColorCycle | Effect::RainbowWave => hue_at(lighting, clock, 0.0),
        _ if lighting.random => hue_at(lighting, clock, 0.0),
        _ => chosen(lighting),
    };
    let mut strength = level(lighting);
    if lighting.effect == Effect::Breathing {
        let phase = std::f32::consts::TAU * clock / breath_seconds(lighting.speed);
        strength *= 0.2 + 0.8 * (0.5 + 0.5 * phase.sin());
    }
    scale(base, strength)
}

/// The color of one of `bands` slices of the light. Only the rainbow wave differs from
/// slice to slice; everything else shows [`glow`] across the whole light.
pub fn band(lighting: &Lighting, clock: f32, index: usize, bands: usize) -> Color32 {
    if lighting.effect != Effect::RainbowWave {
        return glow(lighting, clock);
    }
    let offset = 360.0 * index as f32 / bands as f32;
    scale(hue_at(lighting, clock, -offset), level(lighting))
}

#[cfg(test)]
mod tests;
