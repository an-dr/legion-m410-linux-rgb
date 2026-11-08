use std::collections::HashMap;
use std::ops::RangeInclusive;

use egui::{Align, Button, Color32, CornerRadius, Layout, Margin, RichText, Slider, Ui};
use egui_phosphor::regular as icon;
use m410::{Connection, Effect, Lighting, Rgb, BRIGHTNESS_MAX, SPEED_MAX, SPEED_MIN};

use super::egui_module::App;
use super::look::Look;
use super::preview;
use super::widgets::{card, dot, header, swatch, switch};
use crate::{Backend, Command, Snapshot, Status};

/// The widest the column grows; a wider window gets margins instead.
const MAX_WIDTH: f32 = 520.0;
/// How long a picked color must hold still before it is written.
const SETTLE_SECONDS: f64 = 0.4;

/// The swatches, in the order they are drawn.
pub(crate) const SWATCHES: [Rgb; 8] = [
    Rgb(0xff, 0x30, 0x30),
    Rgb(0xff, 0x90, 0x20),
    Rgb(0xff, 0xe0, 0x30),
    Rgb(0x30, 0xe0, 0x50),
    Rgb(0x20, 0xe0, 0xe0),
    Rgb(0x30, 0x80, 0xff),
    Rgb(0xa0, 0x50, 0xff),
    Rgb(0xff, 0xff, 0xff),
];

/// What the controls are drawn from when there is no mouse to ask.
fn resting() -> Lighting {
    Lighting {
        effect: Effect::Static,
        brightness: BRIGHTNESS_MAX,
        speed: 3,
        random: false,
        color: Rgb(110, 110, 130),
    }
}

/// The control panel: shows what the mouse is doing and asks the worker to change it.
pub struct Panel {
    backend: Backend,
    look: Look,
    /// The newest word from the worker.
    held: Snapshot,
    /// What the user has asked for and the worker has not confirmed yet, drawn in
    /// place of the held lighting so the panel answers at once.
    draft: Option<Lighting>,
    /// The last brightness above off, which the switch returns to.
    resume_brightness: u8,
    /// A color being picked is written once it has stopped changing.
    settle_at: Option<f64>,
    /// Where the controls were last drawn, for the tests that press them.
    pub(crate) hits: HashMap<String, egui::Rect>,
}

/// The changes the controls make in one frame.
struct Edit {
    lighting: Lighting,
    /// The user is dragging: show the change, write nothing yet.
    live: bool,
    /// The user finished a change: write it.
    commit: bool,
    /// A color is being picked: write it when it stops moving.
    settle: bool,
    resume_brightness: u8,
    hits: HashMap<String, egui::Rect>,
}

impl Panel {
    pub fn new(backend: Backend) -> Self {
        Self::with_look(backend, Look::new())
    }

    pub(crate) fn with_look(backend: Backend, look: Look) -> Self {
        backend.send(Command::Refresh);
        Self {
            backend,
            look,
            held: Snapshot::unknown(),
            draft: None,
            resume_brightness: BRIGHTNESS_MAX,
            settle_at: None,
            hits: HashMap::new(),
        }
    }

    /// Takes in whatever the worker has published since the last frame.
    fn drain(&mut self, pointer_down: bool) {
        while let Some(snapshot) = self.backend.try_recv() {
            if let Some(lighting) = snapshot.lighting.filter(|l| l.brightness > 0) {
                self.resume_brightness = lighting.brightness;
            }
            self.held = snapshot;
            if !pointer_down && self.settle_at.is_none() {
                self.draft = None;
            }
        }
    }

    /// The lighting to show: the user's pending change, else what the mouse holds.
    pub(crate) fn lighting(&self) -> Option<Lighting> {
        self.draft.or(self.held.lighting)
    }

    /// Whether there is a mouse to control: it answered, or it is only asleep.
    pub(crate) fn controllable(&self) -> bool {
        matches!(self.held.status, Status::Ready | Status::Waiting | Status::Failed) && self.lighting().is_some()
    }

    /// Asks the worker to store `next`, unless the mouse already holds it.
    fn change(&mut self, next: Lighting) {
        if next.brightness > 0 {
            self.resume_brightness = next.brightness;
        }
        if self.held.lighting == Some(next) {
            self.draft = None;
            return;
        }
        self.draft = Some(next);
        self.backend.send(Command::Apply(next));
    }

    fn settle(&mut self, ctx: &egui::Context) {
        let Some(due) = self.settle_at else { return };
        let (time, down) = ctx.input(|i| (i.time, i.pointer.any_down()));
        if time >= due && !down {
            self.settle_at = None;
            if let Some(next) = self.draft {
                self.change(next);
            }
        }
    }

    fn content(&mut self, ui: &mut Ui) {
        let clock = (ui.input(|i| i.time) % 3600.0) as f32;
        let enabled = self.controllable();
        let current = self.lighting().unwrap_or_else(resting);
        let mut edit = Edit {
            lighting: current,
            live: false,
            commit: false,
            settle: false,
            resume_brightness: self.resume_brightness,
            hits: HashMap::new(),
        };

        ui.horizontal(|ui| {
            ui.heading("Legion M410");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| status_chip(ui, &self.held));
        });
        ui.add_space(2.0);
        paint_preview(ui, enabled.then_some(&current), clock);

        ui.add_enabled_ui(enabled, |ui| {
            effects(ui, &mut edit);
            brightness(ui, &mut edit);
            speed(ui, &mut edit);
            colours(ui, &mut edit);
        });
        self.footer(ui, &mut edit);

        self.hits.extend(edit.hits);
        if edit.commit {
            self.change(edit.lighting);
        } else if edit.live {
            self.draft = Some(edit.lighting);
            if edit.settle {
                self.settle_at = Some(ui.input(|i| i.time) + SETTLE_SECONDS);
            }
        }
    }

    /// What went wrong or what to do, and the way to try again.
    fn footer(&mut self, ui: &mut Ui, edit: &mut Edit) {
        ui.horizontal(|ui| {
            let note = &self.held.detail;
            if !note.is_empty() {
                let colour = if self.held.status == Status::Waiting {
                    ui.visuals().warn_fg_color
                } else {
                    ui.visuals().error_fg_color
                };
                ui.add(egui::Label::new(RichText::new(note).color(colour)).wrap());
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let refresh = ui.add(Button::new(format!("{}  Refresh", icon::ARROW_CLOCKWISE)));
                edit.hits.insert("refresh".into(), refresh.rect);
                if refresh.clicked() {
                    self.backend.send(Command::Refresh);
                }
            });
        });
    }
}

impl App for Panel {
    fn setup(&mut self, ctx: &egui::Context) {
        // Icons must exist on the first frame; the desktop's font arrives later.
        super::theme::install_font(ctx, None);
    }

    fn ui(&mut self, ctx: &egui::Context) {
        self.look.follow(ctx);
        self.drain(ctx.input(|i| i.pointer.any_down()));
        let fill = ctx.style().visuals.panel_fill;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(fill).inner_margin(Margin::same(16)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    let width = ui.available_width().min(MAX_WIDTH);
                    let left = (ui.available_width() - width) / 2.0;
                    ui.horizontal_top(|ui| {
                        ui.add_space(left);
                        ui.vertical(|ui| {
                            ui.set_width(width);
                            self.content(ui);
                        });
                    });
                });
            });
        self.settle(ctx);
    }

    fn scale(&self) -> f32 {
        self.look.scale()
    }
}

fn effect_icon(effect: Effect) -> &'static str {
    match effect {
        Effect::Static => icon::CIRCLE,
        Effect::Breathing => icon::PULSE,
        Effect::ColorCycle => icon::ARROWS_CLOCKWISE,
        Effect::RainbowWave => icon::WAVE_SINE,
    }
}

/// Short enough for four buttons across the narrowest window.
fn effect_name(effect: Effect) -> &'static str {
    match effect {
        Effect::Static => "Static",
        Effect::Breathing => "Breathe",
        Effect::ColorCycle => "Cycle",
        Effect::RainbowWave => "Wave",
    }
}

fn effects(ui: &mut Ui, edit: &mut Edit) {
    card(ui, |ui| {
        header(ui, "Effect", |_| {});
        ui.columns(Effect::ALL.len(), |columns| {
            for (column, effect) in columns.iter_mut().zip(Effect::ALL) {
                let selected = edit.lighting.effect == effect;
                let text = format!("{}  {}", effect_icon(effect), effect_name(effect));
                let width = column.available_width();
                let button = Button::new(text)
                    .selected(selected)
                    .wrap_mode(egui::TextWrapMode::Truncate);
                let response = column.add_sized([width, 34.0], button);
                edit.hits.insert(format!("effect-{}", effect.label()), response.rect);
                if response.clicked() && !selected {
                    edit.lighting.effect = effect;
                    edit.commit = true;
                }
            }
        });
    });
}

fn brightness(ui: &mut Ui, edit: &mut Edit) {
    card(ui, |ui| {
        let mut level = edit.lighting.brightness.max(1);
        let text = if edit.lighting.brightness == 0 {
            "Off".to_string()
        } else {
            format!("{level} / {BRIGHTNESS_MAX}")
        };
        header(ui, "Brightness", |ui| {
            let mut on = edit.lighting.brightness > 0;
            let toggle = switch(ui, &mut on);
            edit.hits.insert("power".into(), toggle.rect);
            if toggle.changed() {
                edit.lighting.brightness = if on { edit.resume_brightness } else { 0 };
                edit.commit = true;
            }
            ui.weak(text);
        });
        ui.add_enabled_ui(edit.lighting.brightness > 0, |ui| {
            let (rect, moved, released) = slider(ui, &mut level, 1..=BRIGHTNESS_MAX);
            edit.hits.insert("brightness".into(), rect);
            if moved {
                edit.lighting.brightness = level;
                edit.resume_brightness = level;
                edit.live = true;
            }
            edit.commit |= released;
        });
    });
}

fn speed(ui: &mut Ui, edit: &mut Edit) {
    card(ui, |ui| {
        let animated = edit.lighting.effect.animated();
        let text = if animated {
            format!("{} / {SPEED_MAX}", edit.lighting.speed)
        } else {
            "Static light".to_string()
        };
        header(ui, "Speed", |ui| {
            ui.weak(text);
        });
        ui.add_enabled_ui(animated && edit.lighting.brightness > 0, |ui| {
            let mut value = edit.lighting.speed;
            let (rect, moved, released) = slider(ui, &mut value, SPEED_MIN..=SPEED_MAX);
            edit.hits.insert("speed".into(), rect);
            if moved {
                edit.lighting.speed = value;
                edit.live = true;
            }
            edit.commit |= released;
        });
    });
}

/// A full-width slider. Says where it is, whether it moved, and whether the user let go.
fn slider(ui: &mut Ui, value: &mut u8, range: RangeInclusive<u8>) -> (egui::Rect, bool, bool) {
    ui.spacing_mut().slider_width = (ui.available_width() - 6.0).max(80.0);
    let response = ui.add(Slider::new(value, range).show_value(false));
    let released = response.drag_stopped() || (response.changed() && !response.dragged());
    (response.rect, response.changed(), released)
}

fn colours(ui: &mut Ui, edit: &mut Edit) {
    card(ui, |ui| {
        let usable = edit.lighting.effect.uses_color();
        let text = match (usable, edit.lighting.random) {
            (false, _) => "Chosen by the effect".to_string(),
            (true, true) => "Random".to_string(),
            (true, false) => format!("#{}", edit.lighting.color),
        };
        header(ui, "Color", |ui| {
            ui.weak(text);
        });
        ui.add_enabled_ui(usable && edit.lighting.brightness > 0, |ui| {
            ui.horizontal(|ui| {
                for (index, colour) in SWATCHES.into_iter().enumerate() {
                    let selected = !edit.lighting.random && edit.lighting.color == colour;
                    let response = swatch(ui, Color32::from_rgb(colour.0, colour.1, colour.2), selected);
                    edit.hits.insert(format!("swatch-{index}"), response.rect);
                    if response.clicked() {
                        edit.lighting.color = colour;
                        edit.lighting.random = false;
                        edit.commit = true;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut rgb = [edit.lighting.color.0, edit.lighting.color.1, edit.lighting.color.2];
                    let picker = ui.color_edit_button_srgb(&mut rgb);
                    edit.hits.insert("picker".into(), picker.rect);
                    if picker.changed() {
                        edit.lighting.color = Rgb(rgb[0], rgb[1], rgb[2]);
                        edit.lighting.random = false;
                        edit.live = true;
                        edit.settle = true;
                    }
                });
            });
            let mut random = edit.lighting.random;
            let response = ui.checkbox(&mut random, "Change color every cycle");
            edit.hits.insert("random".into(), response.rect);
            if response.changed() {
                edit.lighting.random = random;
                edit.commit = true;
            }
        });
    });
}

fn status_chip(ui: &mut Ui, held: &Snapshot) {
    let (colour, glyph, label) = match (held.status, held.connection) {
        (Status::Ready, Some(Connection::Wired)) => (Color32::from_rgb(72, 214, 124), icon::USB, "USB cable"),
        (Status::Ready, _) => (Color32::from_rgb(72, 214, 124), icon::WIFI_HIGH, "Wireless"),
        (Status::Waiting, _) => (ui.visuals().warn_fg_color, icon::MOON, "Asleep"),
        (Status::NotFound, _) => (ui.visuals().weak_text_color(), icon::PLUGS, "Not connected"),
        (Status::NoAccess, _) => (ui.visuals().error_fg_color, icon::LOCK, "No access"),
        (Status::Failed, _) => (ui.visuals().error_fg_color, icon::WARNING, "Problem"),
    };
    ui.weak(format!("{glyph}  {label}"));
    dot(ui, colour);
}

/// A strip of lights that shows what the mouse's light is doing, with a soft glow behind it.
fn paint_preview(ui: &mut Ui, lighting: Option<&Lighting>, clock: f32) {
    let radius = ui.visuals().widgets.inactive.corner_radius;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 84.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    painter.rect_filled(rect, radius, ui.visuals().faint_bg_color);
    let strip = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width() - 56.0, 16.0));
    let lit = lighting.filter(|l| l.brightness > 0);
    if let Some(lighting) = lit {
        // The wave glows in each segment's own color; everything else in one.
        let segments = if lighting.effect == Effect::RainbowWave { 8 } else { 1 };
        let width = strip.width() / segments as f32;
        for segment in 0..segments {
            let colour = preview::band(lighting, clock, segment, segments);
            let strength = f32::from(colour.r().max(colour.g()).max(colour.b())) / 255.0;
            let left = strip.left() + segment as f32 * width;
            for layer in 1..=9 {
                let grow = layer as f32 * 3.5;
                let alpha = (strength * 0.045 * (10 - layer) as f32).clamp(0.0, 1.0);
                // Only the strip's two ends grow sideways, so segments meet without a gap.
                let before = if segment == 0 { grow } else { 0.0 };
                let after = if segment + 1 == segments { grow } else { 0.0 };
                let rect = egui::Rect::from_min_max(
                    egui::pos2(left - before, strip.top() - grow),
                    egui::pos2(left + width + after, strip.bottom() + grow),
                );
                let corners = CornerRadius::same((8.0 + grow) as u8);
                painter.rect_filled(rect, corners, colour.gamma_multiply(alpha));
            }
        }
    }
    // The strip's housing: it is there when the light is off, so the row reads as a light.
    painter.rect_filled(
        strip.expand2(egui::vec2(10.0, 7.0)),
        CornerRadius::same(15),
        ui.visuals().extreme_bg_color,
    );
    let dots = ((strip.width() / 20.0) as usize).clamp(6, 40);
    let off = ui.visuals().widgets.inactive.bg_fill;
    for index in 0..dots {
        let t = (index as f32 + 0.5) / dots as f32;
        let x = strip.left() + t * strip.width();
        let colour = lit.map_or(off, |l| preview::band(l, clock, index, dots));
        painter.circle_filled(egui::pos2(x, strip.center().y), 6.5, colour);
    }
}

#[cfg(test)]
mod tests;
