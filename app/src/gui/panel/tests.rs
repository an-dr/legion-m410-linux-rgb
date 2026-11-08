use std::sync::mpsc::{channel, Receiver, Sender};

use egui::{Event, PointerButton, Pos2, Rect, Vec2};
use m410::{Connection, Effect, Lighting, Rgb};

use super::*;
use crate::gui::desktop::Desktop;
use crate::gui::theme::{self, System};

/// The panel run headlessly: frames of synthetic input, with the worker's side of
/// the channels in the test's hands.
struct Harness {
    ctx: egui::Context,
    panel: Panel,
    commands: Receiver<Command>,
    snapshots: Sender<Snapshot>,
    size: Vec2,
    time: f64,
    pointer: Pos2,
}

fn lighting(effect: Effect, brightness: u8) -> Lighting {
    Lighting {
        effect,
        brightness,
        speed: 3,
        random: false,
        color: Rgb(0x30, 0x80, 0xff),
    }
}

fn ready(lighting: Lighting) -> Snapshot {
    Snapshot {
        connection: Some(Connection::Wired),
        status: Status::Ready,
        lighting: Some(lighting),
        detail: String::new(),
    }
}

impl Harness {
    fn new() -> Self {
        Self::sized(480.0, 700.0)
    }

    fn sized(width: f32, height: f32) -> Self {
        let (command_tx, commands) = channel();
        let (snapshots, snapshot_rx) = channel();
        let backend = Backend::from_channels(command_tx, snapshot_rx);
        let mut panel = Panel::with_look(backend, Look::fixed(Desktop::Cosmic));
        let ctx = egui::Context::default();
        theme::apply(
            &ctx,
            Desktop::Cosmic,
            System {
                dark: true,
                accent: None,
            },
        );
        panel.setup(&ctx);
        let mut harness = Self {
            ctx,
            panel,
            commands,
            snapshots,
            size: egui::vec2(width, height),
            time: 0.0,
            pointer: Pos2::ZERO,
        };
        harness.frames(2);
        harness
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.time += 1.0 / 60.0;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let panel = &mut self.panel;
        let _ = self.ctx.run(raw, |ctx| panel.ui(ctx));
    }

    fn frames(&mut self, count: usize) {
        for _ in 0..count {
            self.frame(Vec::new());
        }
    }

    fn show(&mut self, snapshot: Snapshot) {
        self.snapshots.send(snapshot).unwrap();
        self.frames(3);
    }

    fn rect(&self, key: &str) -> Rect {
        *self
            .panel
            .hits
            .get(key)
            .unwrap_or_else(|| panic!("no control '{key}' in {:?}", self.panel.hits.keys()))
    }

    fn button(&mut self, pos: Pos2, pressed: bool) {
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        }]);
    }

    fn move_to(&mut self, pos: Pos2) {
        self.pointer = pos;
        self.frame(vec![Event::PointerMoved(pos)]);
    }

    fn click(&mut self, key: &str) {
        let pos = self.rect(key).center();
        self.move_to(pos);
        self.button(pos, true);
        self.button(pos, false);
        self.frames(2);
    }

    /// Press at `from`, drag to `to`, release there.
    fn drag(&mut self, key: &str, from: f32, to: f32) {
        let rect = self.rect(key);
        let at = |fraction: f32| Pos2::new(rect.left() + fraction * rect.width(), rect.center().y);
        self.move_to(at(from));
        self.button(at(from), true);
        self.move_to(at((from + to) / 2.0));
        self.move_to(at(to));
        self.button(at(to), false);
        self.frames(2);
    }

    fn sent(&self) -> Vec<Command> {
        self.commands.try_iter().collect()
    }
}

#[test]
fn it_asks_for_the_state_when_it_starts() {
    let harness = Harness::new();
    assert_eq!(harness.sent(), vec![Command::Refresh]);
}

#[test]
fn the_state_it_is_given_is_what_it_shows() {
    let mut harness = Harness::new();
    assert!(!harness.panel.controllable(), "nothing is known yet");
    harness.show(ready(lighting(Effect::Breathing, 3)));
    assert!(harness.panel.controllable());
    assert_eq!(harness.panel.lighting(), Some(lighting(Effect::Breathing, 3)));
}

#[test]
fn choosing_an_effect_asks_for_it() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Breathing, 3)));
    harness.sent();
    harness.click("effect-Static");
    assert_eq!(harness.sent(), vec![Command::Apply(lighting(Effect::Static, 3))]);
}

#[test]
fn choosing_the_current_effect_asks_for_nothing() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Breathing, 3)));
    harness.sent();
    harness.click("effect-Breathing");
    assert_eq!(harness.sent(), Vec::new());
}

#[test]
fn a_swatch_chooses_its_color_and_ends_random_mode() {
    let mut harness = Harness::new();
    harness.show(ready(Lighting {
        random: true,
        ..lighting(Effect::Static, 4)
    }));
    harness.sent();
    harness.click("swatch-3");
    let expected = Lighting {
        random: false,
        color: SWATCHES[3],
        ..lighting(Effect::Static, 4)
    };
    assert_eq!(harness.sent(), vec![Command::Apply(expected)]);
}

#[test]
fn the_random_checkbox_switches_random_colors_on() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Static, 4)));
    harness.sent();
    harness.click("random");
    assert_eq!(
        harness.sent(),
        vec![Command::Apply(Lighting {
            random: true,
            ..lighting(Effect::Static, 4)
        })]
    );
}

#[test]
fn the_switch_turns_the_light_off_and_back_on_at_the_earlier_brightness() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Static, 2)));
    harness.sent();

    harness.click("power");
    assert_eq!(harness.sent(), vec![Command::Apply(lighting(Effect::Static, 0))]);

    harness.show(ready(lighting(Effect::Static, 0)));
    harness.click("power");
    assert_eq!(harness.sent(), vec![Command::Apply(lighting(Effect::Static, 2))]);
}

#[test]
fn dragging_a_slider_writes_once_when_it_is_released() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Static, 4)));
    harness.sent();
    // From the right end back to two thirds of the way, which is step 3 of 4.
    harness.drag("brightness", 1.0, 2.0 / 3.0);
    let sent = harness.sent();
    assert_eq!(sent.len(), 1, "one drag must cost one write: {sent:?}");
    let Command::Apply(applied) = sent[0] else {
        panic!("{sent:?}")
    };
    assert_eq!(applied.brightness, 3);
}

#[test]
fn a_slider_is_never_driven_down_to_off() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Static, 4)));
    harness.sent();
    harness.drag("brightness", 1.0, 0.0);
    let Command::Apply(applied) = harness.sent()[0] else {
        panic!("no write")
    };
    assert_eq!(applied.brightness, 1, "off is the switch's job");
}

#[test]
fn the_speed_slider_works_for_an_animated_light_and_not_for_a_static_one() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Breathing, 4)));
    harness.sent();
    harness.drag("speed", 0.5, 1.0);
    let Command::Apply(applied) = harness.sent()[0] else {
        panic!("no write")
    };
    assert_eq!(applied.speed, 5);

    harness.show(ready(lighting(Effect::Static, 4)));
    harness.sent();
    harness.drag("speed", 0.0, 1.0);
    assert_eq!(harness.sent(), Vec::new(), "a static light has no speed");
}

#[test]
fn the_color_controls_are_off_for_effects_that_choose_their_own() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::RainbowWave, 4)));
    harness.sent();
    harness.click("swatch-0");
    harness.click("random");
    assert_eq!(harness.sent(), Vec::new());
}

#[test]
fn nothing_but_refresh_works_without_a_mouse() {
    let mut harness = Harness::new();
    harness.show(Snapshot {
        connection: None,
        status: Status::NotFound,
        lighting: None,
        detail: "Plug in the mouse or its wireless receiver.".into(),
    });
    harness.sent();
    harness.click("effect-Static");
    harness.click("power");
    assert_eq!(harness.sent(), Vec::new());
    harness.click("refresh");
    assert_eq!(harness.sent(), vec![Command::Refresh]);
}

#[test]
fn a_sleeping_mouse_keeps_its_controls_and_shows_the_change_asked_for() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Static, 4)));
    harness.sent();
    harness.click("effect-Color cycle");
    harness.show(Snapshot {
        connection: Some(Connection::Wireless),
        status: Status::Waiting,
        lighting: Some(lighting(Effect::ColorCycle, 4)),
        detail: "The mouse is asleep. Move it to wake it; your change applies then.".into(),
    });
    assert!(harness.panel.controllable(), "asleep is not gone");
    assert_eq!(harness.panel.lighting().map(|l| l.effect), Some(Effect::ColorCycle));
}

#[test]
fn a_picked_color_is_written_once_it_has_stopped_moving() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Static, 4)));
    harness.sent();
    let picked = Lighting {
        color: Rgb(10, 200, 90),
        ..lighting(Effect::Static, 4)
    };
    harness.panel.draft = Some(picked);
    harness.panel.settle_at = Some(harness.time + SETTLE_SECONDS);

    harness.frames(10);
    assert_eq!(harness.sent(), Vec::new(), "still settling");
    harness.frames(40);
    assert_eq!(harness.sent(), vec![Command::Apply(picked)]);
    assert_eq!(harness.panel.settle_at, None);
}

#[test]
fn the_layout_holds_at_every_width_the_window_can_have() {
    for width in [400.0, 480.0, 520.0, 700.0, 1100.0, 1600.0] {
        let mut harness = Harness::sized(width, 700.0);
        harness.show(ready(lighting(Effect::Breathing, 3)));
        let window = Rect::from_min_size(Pos2::ZERO, egui::vec2(width, 700.0));
        for (key, rect) in &harness.panel.hits {
            assert!(
                window.contains_rect(*rect),
                "{key} {rect:?} falls outside a {width}-wide window"
            );
        }
        let widths: Vec<f32> = Effect::ALL
            .iter()
            .map(|e| harness.rect(&format!("effect-{}", e.label())).width())
            .collect();
        assert!(widths.iter().all(|w| *w >= 60.0), "{width}: effect buttons {widths:?}");
        let spread = widths.iter().cloned().fold(f32::MIN, f32::max) - widths.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            spread < 1.5,
            "{width}: the four effect buttons differ in width: {widths:?}"
        );
    }
}

#[test]
fn a_wide_window_gets_margins_not_a_stretched_column() {
    let mut harness = Harness::sized(1600.0, 700.0);
    harness.show(ready(lighting(Effect::Static, 3)));
    let slider = harness.rect("brightness");
    assert!(slider.width() < MAX_WIDTH, "{slider:?}");
    let centre = slider.center().x;
    assert!((centre - 800.0).abs() < 20.0, "the column is centred, not at {centre}");
}

#[test]
fn a_newer_snapshot_does_not_pull_a_slider_out_from_under_the_pointer() {
    let mut harness = Harness::new();
    harness.show(ready(lighting(Effect::Static, 4)));
    harness.sent();
    let rect = harness.rect("brightness");
    let start = Pos2::new(rect.left() + 0.5 * rect.width(), rect.center().y);
    harness.move_to(start);
    harness.button(start, true);
    harness.move_to(Pos2::new(rect.left() + 0.1 * rect.width(), rect.center().y));
    // The worker answers an earlier request while the button is still down.
    harness.show(ready(lighting(Effect::Static, 4)));
    assert_eq!(
        harness.panel.lighting().map(|l| l.brightness),
        Some(1),
        "the drag is kept"
    );
}
