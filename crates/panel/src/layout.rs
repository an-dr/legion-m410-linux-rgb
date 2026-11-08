//! Where everything is on the 460 x 720 canvas. One place, so drawing, hit-testing
//! and the checks that click the panel cannot disagree.

use m410_messages::Effect;

use crate::Rect;

pub const WIDTH: u32 = 460;
pub const HEIGHT: u32 = 720;

const MARGIN: i32 = 24;
const INNER: u32 = WIDTH - 2 * MARGIN as u32;

pub const TITLE: (i32, i32) = (MARGIN, 22);
pub const STATUS_RIGHT: (i32, i32) = (WIDTH as i32 - MARGIN, 30);

pub const PREVIEW: Rect = Rect::new(MARGIN, 66, INNER, 228);
pub const POWER: Rect = Rect::new(WIDTH as i32 - MARGIN - 52, 78, 40, 40);

const EFFECT_GAP: u32 = 8;
const EFFECT_WIDTH: u32 = (INNER - 3 * EFFECT_GAP) / 4;
pub const EFFECT_LABEL_Y: i32 = 316;

pub fn effect(index: usize) -> Rect {
    Rect::new(
        MARGIN + (index as u32 * (EFFECT_WIDTH + EFFECT_GAP)) as i32,
        338,
        EFFECT_WIDTH,
        40,
    )
}

/// The effect button for `effect`, which is where a click chooses it.
pub fn effect_button(effect: Effect) -> Rect {
    self::effect(
        Effect::ALL
            .iter()
            .position(|candidate| *candidate == effect)
            .unwrap_or(0),
    )
}

pub const BRIGHTNESS_LABEL_Y: i32 = 396;
pub const BRIGHTNESS: Rect = Rect::new(MARGIN, 424, INNER, 28);

pub const SPEED_LABEL_Y: i32 = 462;
pub const SPEED: Rect = Rect::new(MARGIN, 490, INNER, 28);

pub const COLOR_LABEL_Y: i32 = 528;
pub const SWATCH_RADIUS: u32 = 15;

/// Centers are spread evenly, the first and last touching the margins.
pub fn swatch_center(index: usize, count: usize) -> (i32, i32) {
    let span = INNER as i32 - 2 * SWATCH_RADIUS as i32;
    let step = if count > 1 { span / (count as i32 - 1) } else { 0 };
    (MARGIN + SWATCH_RADIUS as i32 + step * index as i32, 566)
}

pub fn swatch(index: usize, count: usize) -> Rect {
    let (x, y) = swatch_center(index, count);
    let r = SWATCH_RADIUS as i32;
    Rect::new(x - r, y - r, 2 * r as u32, 2 * r as u32)
}

pub const HUE: Rect = Rect::new(MARGIN, 600, INNER, 26);

const PILL_WIDTH: u32 = (INNER - 12) / 2;
pub const RANDOM: Rect = Rect::new(MARGIN, 644, PILL_WIDTH, 36);
pub const REFRESH: Rect = Rect::new(MARGIN + PILL_WIDTH as i32 + 12, 644, PILL_WIDTH, 36);

pub const NOTE_Y: i32 = 692;
