use m410_messages::{Connection, Effect, Status, BRIGHTNESS_MAX, SPEED_MAX, SPEED_MIN};

use crate::color::{self, Rgb, Rgba, SWATCHES};
use crate::draw::Canvas;
use crate::{layout, preview, Align, Cmd, Control, Model, Rect};

const BG: Rgb = (15, 15, 21);
const CARD: Rgb = (26, 26, 36);
const RAISED: Rgb = (40, 40, 55);
const RAISED_HOVER: Rgb = (54, 54, 74);
const TRACK: Rgb = (46, 46, 62);
const TEXT: Rgb = (234, 234, 242);
const MUTED: Rgb = (128, 128, 152);
const FAINT: Rgb = (72, 72, 92);
const ACCENT: Rgb = (92, 150, 255);
const ON_ACCENT: Rgb = (10, 16, 34);
const OK: Rgb = (72, 214, 124);
const WARN: Rgb = (255, 190, 64);
const BAD: Rgb = (255, 92, 92);

const HUE_COLUMNS: u32 = 84;
/// Hack, the renderer's font, is monospaced at about this fraction of its size per glyph.
const GLYPH_WIDTH: f32 = 0.6;

fn solid(color: Rgb) -> Rgba {
    color::with_alpha(color, 255)
}

/// The frame for the model, as draw commands in painting order.
pub fn draw(model: &Model) -> Vec<Cmd> {
    let mut canvas = Canvas::default();
    canvas.clear(BG);
    header(&mut canvas, model);
    preview_card(&mut canvas, model);
    effects(&mut canvas, model);
    brightness(&mut canvas, model);
    speed(&mut canvas, model);
    colors(&mut canvas, model);
    buttons(&mut canvas, model);
    note(&mut canvas, model);
    canvas.cmds
}

fn header(c: &mut Canvas, model: &Model) {
    c.text(
        "Legion M410",
        layout::TITLE.0,
        layout::TITLE.1,
        22,
        solid(TEXT),
        Align::Left,
    );

    let (dot, label) = match model.held() {
        None => (MUTED, "Looking...".to_string()),
        Some(held) => match (held.status, held.connection) {
            (Status::Ready, Connection::Wired) => (OK, "USB cable".into()),
            (Status::Ready, _) => (OK, "Wireless".into()),
            (Status::Waiting, _) => (WARN, "Asleep".into()),
            (Status::NotFound, _) => (MUTED, "Not connected".into()),
            (Status::NoAccess, _) => (BAD, "No access".into()),
            (Status::Failed, _) => (BAD, "Problem".into()),
        },
    };
    let size = 13;
    let (right, y) = layout::STATUS_RIGHT;
    let width = (label.chars().count() as f32 * f32::from(size) * GLYPH_WIDTH) as i32;
    c.text(label, right, y, size, solid(MUTED), Align::Right);
    c.circle(right - width - 14, y + 8, 5, solid(dot));
}

fn preview_card(c: &mut Canvas, model: &Model) {
    c.rounded(layout::PREVIEW, 18, solid(CARD));
    let lit = model.lighting().filter(|_| model.controllable());
    let (cx, _) = layout::PREVIEW.center();
    let body = Rect::new(cx - 58, layout::PREVIEW.y + 22, 116, 184);
    let radius = 50u32;

    if let Some(lighting) = lit {
        let glow = preview::glow(&lighting, model.clock());
        let strength = f32::from(glow.0.max(glow.1).max(glow.2)) / 255.0;
        // Inside the card all the way round, because it is opaque.
        let (_, cy) = layout::PREVIEW.center();
        c.glow(cx, cy, 108, CARD, glow, strength);
    }

    c.bordered(body, radius, 2, solid((64, 64, 86)), solid((32, 32, 44)));

    // The lit shell: the top cap and the part below it, inset from the body's edge.
    let inset = 7i32;
    let cap = radius as i32 - inset;
    let shell_height = 92;
    let shell_top = body.y + inset;
    match lit {
        Some(lighting) if lighting.brightness > 0 => {
            let bands = 12;
            let strip = (shell_height - cap) / bands as i32 + 1;
            let color_of = |i: usize| preview::band(&lighting, model.clock(), i, bands);
            c.circle(cx, shell_top + cap, cap as u32, solid(color_of(0)));
            for i in 0..bands {
                let y = shell_top + cap + i as i32 * (shell_height - cap) / bands as i32;
                c.rect(Rect::new(cx - cap, y, 2 * cap as u32, strip as u32), solid(color_of(i)));
            }
        }
        _ => {
            c.circle(cx, shell_top + cap, cap as u32, solid((40, 40, 54)));
            c.rect(
                Rect::new(cx - cap, shell_top + cap, 2 * cap as u32, (shell_height - cap) as u32),
                solid((40, 40, 54)),
            );
        }
    }

    // Details over the shell: the button split and the wheel.
    let seam = solid((18, 18, 26));
    c.rect(Rect::new(cx - 1, shell_top + 8, 2, (shell_height - 8) as u32), seam);
    c.rect(
        Rect::new(cx - cap, shell_top + shell_height - 2, 2 * cap as u32, 3),
        seam,
    );
    c.rounded(Rect::new(cx - 6, body.y + 38, 12, 32), 6, solid((22, 22, 30)));
    c.rounded(Rect::new(cx - 3, body.y + 42, 6, 24), 3, solid((70, 70, 90)));

    power_button(c, model);
}

fn power_button(c: &mut Canvas, model: &Model) {
    let enabled = model.enabled(Control::Power);
    let on = model.lighting().is_some_and(|l| l.brightness > 0);
    let hover = model.hovered() == Some(Control::Power);
    let (x, y) = layout::POWER.center();
    let fill = if hover && enabled { RAISED_HOVER } else { RAISED };
    c.circle(x, y, 20, solid(fill));
    let icon = if !enabled {
        FAINT
    } else if on {
        ACCENT
    } else {
        MUTED
    };
    c.ring(x, y, 9, solid(icon));
    c.ring(x, y, 8, solid(icon));
    c.rect(Rect::new(x - 5, y - 13, 10, 7), solid(fill));
    for dx in -1..=1 {
        c.line((x + dx, y - 12), (x + dx, y), solid(icon));
    }
}

fn label(c: &mut Canvas, text: &str, y: i32) {
    c.text(text, layout::PREVIEW.x, y, 12, solid(MUTED), Align::Left);
}

fn value(c: &mut Canvas, text: &str, y: i32, enabled: bool) {
    let color = if enabled { TEXT } else { FAINT };
    c.text(text, layout::PREVIEW.right(), y, 12, solid(color), Align::Right);
}

fn effects(c: &mut Canvas, model: &Model) {
    label(c, "EFFECT", layout::EFFECT_LABEL_Y);
    let current = model.lighting().map(|l| l.effect);
    for (i, effect) in Effect::ALL.into_iter().enumerate() {
        let rect = layout::effect(i);
        let control = Control::Effect(effect);
        let enabled = model.enabled(control);
        let selected = current == Some(effect);
        let (fill, ink) = match (enabled, selected) {
            (false, true) => (FAINT, BG),
            (false, false) => (CARD, FAINT),
            (true, true) => (ACCENT, ON_ACCENT),
            (true, false) if model.hovered() == Some(control) => (RAISED_HOVER, TEXT),
            (true, false) => (RAISED, TEXT),
        };
        c.rounded(rect, 10, solid(fill));
        let name = match effect {
            Effect::ColorCycle => "Cycle",
            Effect::RainbowWave => "Wave",
            other => other.label(),
        };
        let (x, y) = rect.center();
        c.text(name, x, y - 8, 14, solid(ink), Align::Center);
    }
}

fn brightness(c: &mut Canvas, model: &Model) {
    label(c, "BRIGHTNESS", layout::BRIGHTNESS_LABEL_Y);
    let enabled = model.enabled(Control::Brightness);
    let lighting = model.lighting().filter(|_| model.controllable());
    let level = lighting.map_or(0, |l| l.brightness);
    let text = match lighting {
        None => String::new(),
        Some(_) if level == 0 => "Off".into(),
        Some(_) => format!("{level} / {BRIGHTNESS_MAX}"),
    };
    value(c, &text, layout::BRIGHTNESS_LABEL_Y, enabled);
    let fraction = f32::from(level.max(1) - 1) / f32::from(BRIGHTNESS_MAX - 1);
    slider(
        c,
        model,
        Control::Brightness,
        layout::BRIGHTNESS,
        fraction,
        usize::from(BRIGHTNESS_MAX),
    );
}

fn speed(c: &mut Canvas, model: &Model) {
    label(c, "SPEED", layout::SPEED_LABEL_Y);
    let enabled = model.enabled(Control::Speed);
    let lighting = model.lighting().filter(|_| model.controllable());
    let text = match lighting {
        Some(l) if l.effect.animated() => format!("{} / {SPEED_MAX}", l.speed),
        _ => String::new(),
    };
    value(c, &text, layout::SPEED_LABEL_Y, enabled);
    let speed = lighting.map_or(SPEED_MIN, |l| l.speed);
    let fraction = f32::from(speed - SPEED_MIN) / f32::from(SPEED_MAX - SPEED_MIN);
    slider(
        c,
        model,
        Control::Speed,
        layout::SPEED,
        fraction,
        usize::from(SPEED_MAX),
    );
}

/// A track with a notch per step and a knob; dimmed and knob-less in spirit when unusable.
fn slider(c: &mut Canvas, model: &Model, control: Control, rect: Rect, fraction: f32, steps: usize) {
    let enabled = model.enabled(control);
    let active = model.dragging() == Some(control);
    let y = rect.center().1;
    let track = Rect::new(rect.x, y - 3, rect.w, 6);
    let knob_x = rect.x + (fraction * rect.w as f32) as i32;

    c.rounded(track, 3, solid(TRACK));
    if enabled {
        c.rounded(
            Rect::new(rect.x, y - 3, (knob_x - rect.x).max(6) as u32, 6),
            3,
            solid(ACCENT),
        );
    }
    for step in 0..steps {
        let x = rect.x + (step as f32 / (steps - 1) as f32 * rect.w as f32) as i32;
        c.circle(x, y, 3, solid(if enabled && x <= knob_x { ON_ACCENT } else { FAINT }));
    }
    if !model.controllable() {
        return;
    }
    let hover = model.hovered() == Some(control);
    let radius = if active {
        13
    } else if hover {
        12
    } else {
        11
    };
    let knob = if enabled { TEXT } else { FAINT };
    c.circle(knob_x, y, radius, solid(knob));
    if enabled {
        c.circle(knob_x, y, radius - 4, solid(ACCENT));
    }
}

fn colors(c: &mut Canvas, model: &Model) {
    label(c, "COLOR", layout::COLOR_LABEL_Y);
    let enabled = model.enabled(Control::Hue);
    let lighting = model.lighting().filter(|_| model.controllable());
    let text = match lighting {
        Some(l) if l.effect.uses_color() && l.random => "Random".to_string(),
        Some(l) if l.effect.uses_color() => color::hex(l.color),
        Some(_) => "Chosen by the effect".to_string(),
        None => String::new(),
    };
    value(c, &text, layout::COLOR_LABEL_Y, enabled);

    let chosen = lighting.filter(|l| !l.random).map(|l| l.color);
    for (i, swatch) in SWATCHES.into_iter().enumerate() {
        let (x, y) = layout::swatch_center(i, SWATCHES.len());
        let enabled = model.enabled(Control::Swatch(i));
        let hover = model.hovered() == Some(Control::Swatch(i));
        let selected = enabled && chosen == Some(swatch);
        let radius = layout::SWATCH_RADIUS;
        if selected {
            c.circle(x, y, radius + 5, solid(TEXT));
            c.circle(x, y, radius + 3, solid(BG));
        } else if hover {
            c.circle(x, y, radius + 3, solid(MUTED));
            c.circle(x, y, radius + 1, solid(BG));
        }
        c.circle(
            x,
            y,
            radius,
            solid(if enabled { swatch } else { color::scale(swatch, 0.22) }),
        );
    }

    hue_bar(c, model, chosen, enabled);
}

fn hue_bar(c: &mut Canvas, model: &Model, chosen: Option<Rgb>, enabled: bool) {
    let bar = layout::HUE;
    let (top, height) = (bar.y + 6, bar.h - 12);
    let dim = |color: Rgb| if enabled { color } else { color::scale(color, 0.22) };
    let width = bar.w as i32;
    for column in 0..HUE_COLUMNS as i32 {
        let x0 = bar.x + column * width / HUE_COLUMNS as i32;
        let x1 = bar.x + (column + 1) * width / HUE_COLUMNS as i32;
        let hue = 360.0 * (column as f32 + 0.5) / HUE_COLUMNS as f32;
        c.rect(
            Rect::new(x0, top, (x1 - x0) as u32, height),
            solid(dim(color::hue(hue))),
        );
    }
    // Round the ends.
    let cap = height / 2;
    c.circle(bar.x, top + cap as i32, cap, solid(dim(color::hue(0.0))));
    c.circle(bar.right(), top + cap as i32, cap, solid(dim(color::hue(359.0))));

    if let Some(hue) = enabled.then(|| chosen.and_then(color::pure_hue)).flatten() {
        let x = bar.x + (hue / 360.0 * bar.w as f32) as i32;
        let y = top + cap as i32;
        let radius = if model.dragging() == Some(Control::Hue) { 13 } else { 11 };
        c.circle(x, y, radius, solid(TEXT));
        c.circle(x, y, radius - 3, solid(color::hue(hue)));
    }
}

fn buttons(c: &mut Canvas, model: &Model) {
    let random_on = model.lighting().is_some_and(|l| l.random && l.effect.uses_color()) && model.controllable();
    for (rect, control, name) in [
        (layout::RANDOM, Control::Random, "Random colors"),
        (layout::REFRESH, Control::Refresh, "Refresh"),
    ] {
        let enabled = model.enabled(control);
        let selected = control == Control::Random && random_on;
        let (fill, ink) = match (enabled, selected) {
            (false, _) => (CARD, FAINT),
            (true, true) => (ACCENT, ON_ACCENT),
            (true, false) if model.hovered() == Some(control) => (RAISED_HOVER, TEXT),
            (true, false) => (RAISED, TEXT),
        };
        c.rounded(rect, 18, solid(fill));
        let (x, y) = rect.center();
        c.text(name, x, y - 8, 14, solid(ink), Align::Center);
    }
}

/// What went wrong or what to do, under everything else.
fn note(c: &mut Canvas, model: &Model) {
    let Some(held) = model.held().filter(|held| !held.detail.is_empty()) else {
        return;
    };
    let color = if held.status == Status::Waiting { WARN } else { BAD };
    let (center, _) = layout::PREVIEW.center();
    for (i, line) in wrap(&held.detail, 54).into_iter().enumerate() {
        c.text(
            line,
            center,
            layout::NOTE_Y + 15 * i as i32,
            12,
            solid(color),
            Align::Center,
        );
    }
}

/// Breaks `text` at spaces into lines of at most `width` characters.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}
