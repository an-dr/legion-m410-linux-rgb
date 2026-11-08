//! The lighting record at the start of the configuration page.
//!
//! | Offset | Meaning                                         |
//! |--------|-------------------------------------------------|
//! | `0x01` | effect, see [`Effect`]                          |
//! | `0x02` | brightness, 0 (off) to 4                        |
//! | `0x03` | animation period, 0 (fastest) to 4              |
//! | `0x05` | `0xff`: a new random color each cycle, `0x00`: use the color below |
//! | `0x06` | red, green, blue                                |

use std::fmt;
use std::str::FromStr;

use crate::device::Page;

const OFF_EFFECT: usize = 0x01;
const OFF_BRIGHTNESS: usize = 0x02;
const OFF_SPEED: usize = 0x03;
const OFF_RANDOM: usize = 0x05;
const OFF_COLOR: usize = 0x06;

pub const BRIGHTNESS_MAX: u8 = 4;
pub const SPEED_MIN: u8 = 1;
pub const SPEED_MAX: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Static,
    Breathing,
    ColorCycle,
    RainbowWave,
}

impl Effect {
    pub const ALL: [Effect; 4] = [
        Effect::Static,
        Effect::Breathing,
        Effect::ColorCycle,
        Effect::RainbowWave,
    ];

    fn raw(self) -> u8 {
        match self {
            Effect::Static => 1,
            Effect::Breathing => 2,
            Effect::ColorCycle => 3,
            Effect::RainbowWave => 4,
        }
    }

    fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Effect::Static,
            3 => Effect::ColorCycle,
            4 => Effect::RainbowWave,
            // 0 is the factory value and behaves as breathing.
            _ => Effect::Breathing,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Effect::Static => "static",
            Effect::Breathing => "breathing",
            Effect::ColorCycle => "cycle",
            Effect::RainbowWave => "wave",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Effect::Static => "Static",
            Effect::Breathing => "Breathing",
            Effect::ColorCycle => "Color cycle",
            Effect::RainbowWave => "Rainbow wave",
        }
    }

    /// Whether the effect shows the configured color (cycle and wave pick their own).
    pub fn uses_color(self) -> bool {
        matches!(self, Effect::Static | Effect::Breathing)
    }

    pub fn animated(self) -> bool {
        self != Effect::Static
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Effect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Effect::ALL
            .into_iter()
            .find(|e| e.name().eq_ignore_ascii_case(s))
            .ok_or_else(|| format!("unknown effect '{s}', expected one of: static, breathing, cycle, wave"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

impl FromStr for Rgb {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.trim_start_matches('#');
        let bad = || format!("invalid color '{s}', expected RRGGBB");
        if hex.len() != 6 {
            return Err(bad());
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| bad());
        Ok(Rgb(byte(0)?, byte(2)?, byte(4)?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lighting {
    pub effect: Effect,
    /// 0 (off) to [`BRIGHTNESS_MAX`].
    pub brightness: u8,
    /// [`SPEED_MIN`] (slowest) to [`SPEED_MAX`] (fastest).
    pub speed: u8,
    /// A new random color each cycle instead of `color`.
    pub random: bool,
    pub color: Rgb,
}

impl Lighting {
    pub fn from_page(page: &Page) -> Self {
        let raw_speed = page[OFF_SPEED].min(SPEED_MAX - SPEED_MIN);
        Self {
            effect: Effect::from_raw(page[OFF_EFFECT]),
            brightness: page[OFF_BRIGHTNESS].min(BRIGHTNESS_MAX),
            speed: SPEED_MAX - raw_speed,
            random: page[OFF_RANDOM] != 0,
            color: Rgb(page[OFF_COLOR], page[OFF_COLOR + 1], page[OFF_COLOR + 2]),
        }
    }

    pub fn apply_to(&self, page: &mut Page) {
        page[OFF_EFFECT] = self.effect.raw();
        page[OFF_BRIGHTNESS] = self.brightness.min(BRIGHTNESS_MAX);
        page[OFF_SPEED] = SPEED_MAX - self.speed.clamp(SPEED_MIN, SPEED_MAX);
        page[OFF_RANDOM] = if self.random { 0xff } else { 0x00 };
        page[OFF_COLOR..OFF_COLOR + 3].copy_from_slice(&[self.color.0, self.color.1, self.color.2]);
    }

    pub fn is_off(&self) -> bool {
        self.brightness == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::PAGE_LEN;

    /// Lighting bytes of the factory page dumped from a real mouse.
    fn factory_page() -> Page {
        let mut page = [0u8; PAGE_LEN];
        page[..9].copy_from_slice(&[0x00, 0x00, 0x04, 0x02, 0x00, 0xff, 0xff, 0x00, 0xff]);
        page
    }

    #[test]
    fn decodes_factory_settings() {
        let l = Lighting::from_page(&factory_page());
        assert_eq!(l.effect, Effect::Breathing);
        assert_eq!(l.brightness, 4);
        assert_eq!(l.speed, 3);
        assert!(l.random);
        assert_eq!(l.color, Rgb(0xff, 0x00, 0xff));
    }

    #[test]
    fn encodes_only_lighting_bytes() {
        let mut page = factory_page();
        page[0x10] = 0x42;
        let l = Lighting {
            effect: Effect::Static,
            brightness: 2,
            speed: 5,
            random: false,
            color: Rgb(1, 2, 3),
        };
        l.apply_to(&mut page);
        assert_eq!(&page[..9], &[0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 1, 2, 3]);
        assert_eq!(page[0x10], 0x42);
        assert_eq!(Lighting::from_page(&page), l);
    }

    #[test]
    fn clamps_out_of_range_values() {
        let mut page = factory_page();
        let l = Lighting {
            effect: Effect::Static,
            brightness: 9,
            speed: 0,
            random: false,
            color: Rgb(0, 0, 0),
        };
        l.apply_to(&mut page);
        assert_eq!(page[OFF_BRIGHTNESS], BRIGHTNESS_MAX);
        assert_eq!(page[OFF_SPEED], SPEED_MAX - SPEED_MIN);
    }

    #[test]
    fn parses_colors_and_effects() {
        assert_eq!("#ff8000".parse::<Rgb>(), Ok(Rgb(0xff, 0x80, 0x00)));
        assert!("ff80".parse::<Rgb>().is_err());
        assert!("gg0000".parse::<Rgb>().is_err());
        assert_eq!("Wave".parse::<Effect>(), Ok(Effect::RainbowWave));
        assert!("disco".parse::<Effect>().is_err());
    }
}
