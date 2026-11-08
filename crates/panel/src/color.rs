pub type Rgb = (u8, u8, u8);

/// A color with transparency, as the renderer takes it.
pub type Rgba = (u8, u8, u8, u8);

/// The swatches, in the order they are drawn.
pub const SWATCHES: [Rgb; 8] = [
    (0xff, 0x30, 0x30),
    (0xff, 0x90, 0x20),
    (0xff, 0xe0, 0x30),
    (0x30, 0xe0, 0x50),
    (0x20, 0xe0, 0xe0),
    (0x30, 0x80, 0xff),
    (0xa0, 0x50, 0xff),
    (0xff, 0xff, 0xff),
];

/// A fully saturated, full-value color for a hue in degrees.
pub fn hue(degrees: f32) -> Rgb {
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
    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

/// The hue of a color that is fully saturated at full value, as [`hue`] makes them;
/// `None` for anything else (white, a pastel, a dim color).
pub fn pure_hue((r, g, b): Rgb) -> Option<f32> {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max < 250 || min > 5 {
        return None;
    }
    let range = f32::from(max - min);
    let (is_red, is_green) = (max == r, max == g);
    let (r, g, b) = (f32::from(r), f32::from(g), f32::from(b));
    let degrees = if is_red {
        60.0 * ((g - b) / range).rem_euclid(6.0)
    } else if is_green {
        60.0 * ((b - r) / range + 2.0)
    } else {
        60.0 * ((r - g) / range + 4.0)
    };
    Some(degrees.rem_euclid(360.0))
}

/// `color` darkened toward black: 1 leaves it, 0 is black.
pub fn scale((r, g, b): Rgb, amount: f32) -> Rgb {
    let k = amount.clamp(0.0, 1.0);
    (
        (f32::from(r) * k) as u8,
        (f32::from(g) * k) as u8,
        (f32::from(b) * k) as u8,
    )
}

/// `from` blended toward `to` by `amount`.
pub fn mix(from: Rgb, to: Rgb, amount: f32) -> Rgb {
    let k = amount.clamp(0.0, 1.0);
    let channel = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * k).round() as u8;
    (channel(from.0, to.0), channel(from.1, to.1), channel(from.2, to.2))
}

pub fn with_alpha((r, g, b): Rgb, alpha: u8) -> Rgba {
    (r, g, b, alpha)
}

pub fn hex((r, g, b): Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}
