use bones_wasm_sdk::bus;
use bones_wasm_sdk::messages::gfx::{Clear, DrawCircle, DrawLine, DrawRect, DrawText, TextAlign};
use m410_panel::{Align, Cmd};

/// Publishes one panel draw command as the engine's gfx message.
///
/// Everything is on layer 0, where the renderer draws in publish order, which is
/// the order the panel produced them in.
pub fn publish(cmd: &Cmd) {
    match cmd {
        Cmd::Clear((r, g, b)) => bus::publish(&Clear {
            r: *r,
            g: *g,
            b: *b,
            a: 255,
        }),
        Cmd::Rect { rect, color } => bus::publish(&DrawRect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
            filled: true,
            color: *color,
            layer: 0,
            screen_space: true,
        }),
        Cmd::Circle {
            x,
            y,
            radius,
            color,
            filled,
        } => bus::publish(&DrawCircle {
            x: *x,
            y: *y,
            radius: *radius,
            filled: *filled,
            color: *color,
            layer: 0,
        }),
        Cmd::Line { from, to, color } => bus::publish(&DrawLine {
            x1: from.0,
            y1: from.1,
            x2: to.0,
            y2: to.1,
            color: *color,
            layer: 0,
        }),
        Cmd::Text {
            text,
            x,
            y,
            size,
            color,
            align,
        } => bus::publish(&DrawText {
            text,
            x: *x,
            y: *y,
            size: *size,
            color: *color,
            layer: 0,
            screen_space: true,
            align: match align {
                Align::Left => TextAlign::Left,
                Align::Center => TextAlign::Center,
                Align::Right => TextAlign::Right,
            },
        }),
    }
}
