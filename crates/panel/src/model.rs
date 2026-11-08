use m410_messages::{Connection, Effect, Lighting, State, Status, BRIGHTNESS_MAX, SPEED_MAX, SPEED_MIN};

use crate::color::{self, SWATCHES};
use crate::layout;
use crate::Rect;

/// The last state the native module published, owned so it outlives the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Held {
    pub connection: Connection,
    pub status: Status,
    pub lighting: Option<Lighting>,
    pub detail: String,
}

/// A thing on the panel that reacts to the mouse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Power,
    Effect(Effect),
    Brightness,
    Speed,
    Swatch(usize),
    Hue,
    Random,
    Refresh,
}

/// What the panel asks of the native module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Apply(Lighting),
    Refresh,
}

/// Everything the panel is drawn from, and what a press or a drag does to it.
///
/// A slider is written when the button is released, not while it moves, so a
/// drag costs one flash write on the mouse instead of one per step.
pub struct Model {
    canvas: (u32, u32),
    window: (u32, u32),
    held: Option<Held>,
    /// What the user has asked for and the module has not confirmed yet; drawn in
    /// place of the held lighting so the panel answers at once.
    draft: Option<Lighting>,
    drag: Option<Control>,
    hover: Option<Control>,
    clock: f32,
    /// The last brightness above off, which the power button returns to.
    resume_brightness: u8,
}

impl Default for Model {
    fn default() -> Self {
        let size = (layout::WIDTH, layout::HEIGHT);
        Self {
            canvas: size,
            window: size,
            held: None,
            draft: None,
            drag: None,
            hover: None,
            clock: 0.0,
            resume_brightness: BRIGHTNESS_MAX,
        }
    }
}

impl Model {
    /// The renderer's fixed coordinate space, which all layout is written in.
    pub fn set_canvas(&mut self, width: u32, height: u32) {
        self.canvas = (width, height);
    }

    /// The window's real size, which mouse positions are measured in.
    pub fn set_window(&mut self, width: u32, height: u32) {
        self.window = (width, height);
    }

    pub fn set_state(&mut self, state: &State<'_>) {
        self.held = Some(Held {
            connection: state.connection,
            status: state.status,
            lighting: state.lighting,
            detail: state.detail.to_string(),
        });
        if let Some(lighting) = state.lighting.filter(|l| l.brightness > 0) {
            self.resume_brightness = lighting.brightness;
        }
        if self.drag.is_none() {
            self.draft = None;
        }
    }

    pub fn tick(&mut self, seconds: f32) {
        // Wrapped, so the preview's phase keeps its precision over a long session.
        self.clock = (self.clock + seconds) % 3600.0;
    }

    pub fn clock(&self) -> f32 {
        self.clock
    }

    pub fn held(&self) -> Option<&Held> {
        self.held.as_ref()
    }

    /// The lighting to show: the user's pending change, else what the mouse holds.
    pub fn lighting(&self) -> Option<Lighting> {
        self.draft.or_else(|| self.held.as_ref().and_then(|held| held.lighting))
    }

    pub fn hovered(&self) -> Option<Control> {
        self.hover
    }

    pub fn dragging(&self) -> Option<Control> {
        self.drag
    }

    /// Whether there is a mouse to control: it answered, or it is only asleep.
    pub fn controllable(&self) -> bool {
        let usable = self
            .held
            .as_ref()
            .is_some_and(|held| matches!(held.status, Status::Ready | Status::Waiting | Status::Failed));
        usable && self.lighting().is_some()
    }

    /// Whether a control does anything right now; the rest are drawn dimmed.
    pub fn enabled(&self, control: Control) -> bool {
        if control == Control::Refresh {
            return true;
        }
        let Some(lighting) = self.lighting().filter(|_| self.controllable()) else {
            return false;
        };
        match control {
            Control::Power => true,
            _ if lighting.brightness == 0 => false,
            Control::Speed => lighting.effect.animated(),
            Control::Swatch(_) | Control::Hue | Control::Random => lighting.effect.uses_color(),
            _ => true,
        }
    }

    /// A mouse press, in window pixels.
    pub fn mouse_down(&mut self, x: f32, y: f32) -> Option<Action> {
        let (x, y) = self.logical(x, y)?;
        let control = self.control_at(x, y)?;
        match control {
            Control::Refresh => Some(Action::Refresh),
            Control::Brightness | Control::Speed | Control::Hue => {
                self.drag = Some(control);
                self.drag_to(x);
                None
            }
            _ => self.press(control),
        }
    }

    pub fn mouse_move(&mut self, x: f32, y: f32) {
        let Some((x, y)) = self.logical(x, y) else { return };
        if self.drag.is_some() {
            self.drag_to(x);
        } else {
            self.hover = self.control_at(x, y);
        }
    }

    /// A mouse release; ends a drag and asks for the value it reached.
    pub fn mouse_up(&mut self, x: f32, y: f32) -> Option<Action> {
        self.drag.as_ref()?;
        if let Some((x, _)) = self.logical(x, y) {
            self.drag_to(x);
        }
        self.drag = None;
        let draft = self.draft?;
        let held = self.held.as_ref().and_then(|held| held.lighting);
        (Some(draft) != held).then_some(Action::Apply(draft))
    }

    fn logical(&self, x: f32, y: f32) -> Option<(f32, f32)> {
        let ((cw, ch), (ww, wh)) = (self.canvas, self.window);
        if ww == 0 || wh == 0 {
            return None;
        }
        Some((x * cw as f32 / ww as f32, y * ch as f32 / wh as f32))
    }

    fn control_at(&self, x: f32, y: f32) -> Option<Control> {
        let hit = |rect: Rect, control: Control| (rect.contains(x, y) && self.enabled(control)).then_some(control);
        let slider = |rect: Rect| Rect::new(rect.x - 8, rect.y - 8, rect.w + 16, rect.h + 16);

        hit(layout::POWER, Control::Power)
            .or_else(|| {
                Effect::ALL
                    .iter()
                    .enumerate()
                    .find_map(|(i, effect)| hit(layout::effect(i), Control::Effect(*effect)))
            })
            .or_else(|| hit(slider(layout::BRIGHTNESS), Control::Brightness))
            .or_else(|| hit(slider(layout::SPEED), Control::Speed))
            .or_else(|| (0..SWATCHES.len()).find_map(|i| hit(layout::swatch(i, SWATCHES.len()), Control::Swatch(i))))
            .or_else(|| hit(slider(layout::HUE), Control::Hue))
            .or_else(|| hit(layout::RANDOM, Control::Random))
            .or_else(|| hit(layout::REFRESH, Control::Refresh))
    }

    /// A click on a control that acts at once.
    fn press(&mut self, control: Control) -> Option<Action> {
        let current = self.lighting()?;
        let mut next = current;
        match control {
            Control::Power => {
                next.brightness = if current.brightness > 0 {
                    0
                } else {
                    self.resume_brightness
                }
            }
            Control::Effect(effect) => next.effect = effect,
            Control::Swatch(index) => {
                next.color = SWATCHES[index];
                next.random = false;
            }
            Control::Random => next.random = !current.random,
            _ => return None,
        }
        if next == current {
            return None;
        }
        self.draft = Some(next);
        Some(Action::Apply(next))
    }

    /// Moves the dragged slider to where the pointer is, without writing anything yet.
    fn drag_to(&mut self, logical_x: f32) {
        let Some(control) = self.drag else { return };
        let Some(mut next) = self.lighting() else { return };
        match control {
            Control::Brightness => {
                let fraction = layout::BRIGHTNESS.fraction_x(logical_x);
                next.brightness = 1 + (fraction * f32::from(BRIGHTNESS_MAX - 1)).round() as u8;
                self.resume_brightness = next.brightness;
            }
            Control::Speed => {
                let fraction = layout::SPEED.fraction_x(logical_x);
                next.speed = SPEED_MIN + (fraction * f32::from(SPEED_MAX - SPEED_MIN)).round() as u8;
            }
            Control::Hue => {
                next.color = color::hue(360.0 * layout::HUE.fraction_x(logical_x));
                next.random = false;
            }
            _ => return,
        }
        self.draft = Some(next);
    }
}
