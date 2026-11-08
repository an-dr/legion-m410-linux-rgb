use crate::color::{self, Rgb, Rgba};
use crate::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// One draw command, in the order it must be drawn: later ones paint over earlier ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cmd {
    Clear(Rgb),
    Rect {
        rect: Rect,
        color: Rgba,
    },
    Circle {
        x: i32,
        y: i32,
        radius: u32,
        color: Rgba,
        filled: bool,
    },
    Line {
        from: (i32, i32),
        to: (i32, i32),
        color: Rgba,
    },
    Text {
        text: String,
        x: i32,
        y: i32,
        size: u16,
        color: Rgba,
        align: Align,
    },
}

/// Collects commands, with the shapes the renderer lacks built from the ones it has.
#[derive(Default)]
pub struct Canvas {
    pub cmds: Vec<Cmd>,
}

impl Canvas {
    pub fn clear(&mut self, color: Rgb) {
        self.cmds.push(Cmd::Clear(color));
    }

    pub fn rect(&mut self, rect: Rect, color: Rgba) {
        self.cmds.push(Cmd::Rect { rect, color });
    }

    pub fn circle(&mut self, x: i32, y: i32, radius: u32, color: Rgba) {
        self.cmds.push(Cmd::Circle {
            x,
            y,
            radius,
            color,
            filled: true,
        });
    }

    pub fn ring(&mut self, x: i32, y: i32, radius: u32, color: Rgba) {
        self.cmds.push(Cmd::Circle {
            x,
            y,
            radius,
            color,
            filled: false,
        });
    }

    pub fn line(&mut self, from: (i32, i32), to: (i32, i32), color: Rgba) {
        self.cmds.push(Cmd::Line { from, to, color });
    }

    pub fn text(&mut self, text: impl Into<String>, x: i32, y: i32, size: u16, color: Rgba, align: Align) {
        self.cmds.push(Cmd::Text {
            text: text.into(),
            x,
            y,
            size,
            color,
            align,
        });
    }

    /// A rectangle with rounded corners: two overlapping rectangles and a circle in each corner.
    pub fn rounded(&mut self, rect: Rect, radius: u32, color: Rgba) {
        let r = radius.min(rect.w / 2).min(rect.h / 2);
        let ri = r as i32;
        if rect.w > 2 * r {
            self.rect(Rect::new(rect.x + ri, rect.y, rect.w - 2 * r, rect.h), color);
        }
        if rect.h > 2 * r {
            self.rect(Rect::new(rect.x, rect.y + ri, rect.w, rect.h - 2 * r), color);
        }
        // The renderer's circles reach one pixel past their radius on every side,
        // while its rectangles stop at their last pixel; centering the right and
        // bottom circles one pixel in makes the two meet cleanly.
        let (far_x, far_y) = (rect.right() - 1 - ri, rect.bottom() - 1 - ri);
        for (cx, cy) in [
            (rect.x + ri, rect.y + ri),
            (far_x, rect.y + ri),
            (rect.x + ri, far_y),
            (far_x, far_y),
        ] {
            self.circle(cx, cy, r, color);
        }
    }

    /// A rounded rectangle with a border of `width`: the border color, then the fill inset by it.
    pub fn bordered(&mut self, rect: Rect, radius: u32, width: u32, border: Rgba, fill: Rgba) {
        self.rounded(rect, radius, border);
        let w = width as i32;
        let inner = Rect::new(
            rect.x + w,
            rect.y + w,
            rect.w.saturating_sub(2 * width),
            rect.h.saturating_sub(2 * width),
        );
        self.rounded(inner, radius.saturating_sub(width), fill);
    }

    /// A soft glow of `color` over a solid `background`, brightest in the middle.
    ///
    /// The renderer draws shapes without blending, so transparency is not an option:
    /// each ring is the background already mixed toward the color by the right amount.
    pub fn glow(&mut self, x: i32, y: i32, radius: u32, background: Rgb, color: Rgb, strength: f32) {
        const LAYERS: u32 = 28;
        for layer in 0..LAYERS {
            let depth = (layer + 1) as f32 / LAYERS as f32;
            let r = radius - radius * layer / LAYERS;
            let amount = strength.clamp(0.0, 1.0) * 0.85 * depth.powf(1.3);
            self.circle(x, y, r, color::with_alpha(color::mix(background, color, amount), 255));
        }
    }
}
