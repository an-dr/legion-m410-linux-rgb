use m410_messages::{Connection, Effect, Lighting, State, Status};

use crate::color::{self, SWATCHES};
use crate::{draw, layout, preview, Action, Cmd, Control, Model};

fn lighting() -> Lighting {
    Lighting {
        effect: Effect::Breathing,
        brightness: 4,
        speed: 3,
        random: false,
        color: SWATCHES[5],
    }
}

fn model(lighting: Lighting) -> Model {
    let mut model = Model::default();
    model.set_state(&State {
        connection: Connection::Wired,
        status: Status::Ready,
        lighting: Some(lighting),
        detail: "",
    });
    model
}

fn tap(model: &mut Model, (x, y): (i32, i32)) -> Option<Action> {
    let down = model.mouse_down(x as f32, y as f32);
    down.or_else(|| model.mouse_up(x as f32, y as f32))
}

fn texts(cmds: &[Cmd]) -> Vec<&str> {
    cmds.iter()
        .filter_map(|cmd| {
            if let Cmd::Text { text, .. } = cmd {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn clicking_an_effect_asks_for_it() {
    let mut model = model(lighting());
    let action = tap(&mut model, layout::effect_button(Effect::Static).center());
    assert_eq!(
        action,
        Some(Action::Apply(Lighting {
            effect: Effect::Static,
            ..lighting()
        }))
    );
}

#[test]
fn clicking_the_current_effect_asks_for_nothing() {
    let mut model = model(lighting());
    assert_eq!(tap(&mut model, layout::effect_button(Effect::Breathing).center()), None);
}

#[test]
fn a_swatch_sets_its_color_and_ends_random() {
    let mut model = model(Lighting {
        random: true,
        ..lighting()
    });
    let action = tap(&mut model, layout::swatch(2, SWATCHES.len()).center());
    assert_eq!(
        action,
        Some(Action::Apply(Lighting {
            color: SWATCHES[2],
            random: false,
            ..lighting()
        }))
    );
}

#[test]
fn power_switches_off_and_back_on_keeping_the_rest() {
    let mut model = model(lighting());
    let off = tap(&mut model, layout::POWER.center());
    assert_eq!(
        off,
        Some(Action::Apply(Lighting {
            brightness: 0,
            ..lighting()
        }))
    );

    let mut model = self::model(Lighting {
        brightness: 0,
        ..lighting()
    });
    let on = tap(&mut model, layout::POWER.center());
    assert_eq!(on, Some(Action::Apply(lighting())));
}

#[test]
fn power_on_returns_to_the_brightness_it_was_switched_off_at() {
    let mut model = model(Lighting {
        brightness: 2,
        ..lighting()
    });
    tap(&mut model, layout::POWER.center());
    model.set_state(&State {
        connection: Connection::Wired,
        status: Status::Ready,
        lighting: Some(Lighting {
            brightness: 0,
            ..lighting()
        }),
        detail: "",
    });
    let on = tap(&mut model, layout::POWER.center());
    assert_eq!(
        on,
        Some(Action::Apply(Lighting {
            brightness: 2,
            ..lighting()
        }))
    );
}

#[test]
fn power_on_after_a_slider_drag_returns_to_the_dragged_level() {
    let mut model = model(lighting());
    let track = layout::BRIGHTNESS;
    let y = track.center().1 as f32;
    model.mouse_down(track.x as f32, y);
    model.mouse_up(track.x as f32, y);
    model.set_state(&State {
        connection: Connection::Wired,
        status: Status::Ready,
        lighting: Some(Lighting {
            brightness: 0,
            ..lighting()
        }),
        detail: "",
    });
    let on = tap(&mut model, layout::POWER.center());
    assert_eq!(
        on,
        Some(Action::Apply(Lighting {
            brightness: 1,
            ..lighting()
        }))
    );
}

#[test]
fn nothing_but_power_and_refresh_works_while_the_light_is_off() {
    let mut model = model(Lighting {
        brightness: 0,
        ..lighting()
    });
    assert_eq!(tap(&mut model, layout::effect_button(Effect::Static).center()), None);
    assert_eq!(tap(&mut model, layout::swatch(0, SWATCHES.len()).center()), None);
    assert_eq!(tap(&mut model, layout::REFRESH.center()), Some(Action::Refresh));
}

#[test]
fn speed_and_color_controls_follow_the_effect() {
    let mut model = model(Lighting {
        effect: Effect::Static,
        ..lighting()
    });
    assert!(!model.enabled(Control::Speed), "a static light has no speed");
    assert!(model.enabled(Control::Hue));

    model = self::model(Lighting {
        effect: Effect::RainbowWave,
        ..lighting()
    });
    assert!(model.enabled(Control::Speed));
    assert!(!model.enabled(Control::Hue), "the wave chooses its own colors");
    assert_eq!(tap(&mut model, layout::swatch(0, SWATCHES.len()).center()), None);
}

#[test]
fn dragging_a_slider_writes_once_when_released() {
    let mut model = model(Lighting {
        brightness: 4,
        ..lighting()
    });
    let track = layout::BRIGHTNESS;
    let y = track.center().1 as f32;

    assert_eq!(
        model.mouse_down(track.right() as f32, y),
        None,
        "pressing a slider writes nothing"
    );
    model.mouse_move((track.x + track.w as i32 / 2) as f32, y);
    model.mouse_move(track.x as f32, y);
    assert_eq!(
        model.lighting().unwrap().brightness,
        1,
        "the panel follows the pointer at once"
    );

    let action = model.mouse_up(track.x as f32, y);
    assert_eq!(
        action,
        Some(Action::Apply(Lighting {
            brightness: 1,
            ..lighting()
        }))
    );
}

#[test]
fn a_slider_snaps_to_whole_steps_and_never_reaches_off() {
    let mut model = model(lighting());
    let track = layout::BRIGHTNESS;
    let y = track.center().1 as f32;
    for x in (track.x - 20..track.right() + 20).step_by(7) {
        model.mouse_down(x as f32, y);
        let brightness = model.lighting().unwrap().brightness;
        model.mouse_up(x as f32, y);
        assert!((1..=4).contains(&brightness), "brightness {brightness} at x={x}");
    }
}

#[test]
fn releasing_a_drag_where_it_started_asks_for_nothing() {
    let mut model = model(lighting());
    let track = layout::BRIGHTNESS;
    let y = track.center().1 as f32;
    model.mouse_down(track.right() as f32, y);
    assert_eq!(model.mouse_up(track.right() as f32, y), None);
}

#[test]
fn the_hue_bar_picks_any_color() {
    let mut model = model(lighting());
    let bar = layout::HUE;
    let y = bar.center().1 as f32;
    let third = (bar.x + bar.w as i32 / 3) as f32;

    model.mouse_down(third, y);
    let Some(Action::Apply(next)) = model.mouse_up(third, y) else {
        panic!("no action")
    };
    // A third of the way along is green, give or take the pixel the click landed on.
    let (r, g, b) = next.color;
    assert!(g == 255 && r < 8 && b == 0, "{:?}", next.color);
    assert!(!next.random);
}

#[test]
fn clicks_are_scaled_when_the_window_is_not_the_canvas() {
    let mut model = model(lighting());
    model.set_window(layout::WIDTH * 2, layout::HEIGHT * 2);
    let (x, y) = layout::effect_button(Effect::Static).center();
    let action = model.mouse_down(2.0 * x as f32, 2.0 * y as f32);
    assert!(matches!(action, Some(Action::Apply(l)) if l.effect == Effect::Static));
}

#[test]
fn a_new_state_replaces_the_pending_change() {
    let mut model = model(lighting());
    tap(&mut model, layout::effect_button(Effect::Static).center());
    assert_eq!(
        model.lighting().unwrap().effect,
        Effect::Static,
        "shown before the module confirms"
    );

    model.set_state(&State {
        connection: Connection::Wired,
        status: Status::Ready,
        lighting: Some(Lighting {
            effect: Effect::Static,
            ..lighting()
        }),
        detail: "",
    });
    assert_eq!(model.lighting().unwrap().effect, Effect::Static);
}

#[test]
fn without_a_mouse_nothing_but_refresh_reacts() {
    let mut model = Model::default();
    model.set_state(&State {
        connection: Connection::None,
        status: Status::NotFound,
        lighting: None,
        detail: "Plug in the mouse.",
    });
    assert!(!model.controllable());
    assert_eq!(tap(&mut model, layout::POWER.center()), None);
    assert_eq!(tap(&mut model, layout::REFRESH.center()), Some(Action::Refresh));
    let frame = draw(&model);
    assert!(texts(&frame).contains(&"Not connected"));
    assert!(texts(&frame).contains(&"Plug in the mouse."));
}

#[test]
fn the_frame_says_what_the_mouse_is_doing() {
    let frame = draw(&model(lighting()));
    let text = texts(&frame);
    for expected in [
        "Legion M410",
        "USB cable",
        "Breathing",
        "BRIGHTNESS",
        "4 / 4",
        "SPEED",
        "3 / 5",
        "COLOR",
        "#3080ff",
        "Refresh",
    ] {
        assert!(text.contains(&expected), "missing {expected:?} in {text:?}");
    }
    assert!(
        matches!(frame[0], Cmd::Clear(_)),
        "every frame starts by clearing the last"
    );
}

#[test]
fn the_frame_stays_inside_the_command_budget() {
    // An extension may publish 1024 messages a frame before it is quarantined.
    for lighting in [
        lighting(),
        Lighting {
            effect: Effect::RainbowWave,
            ..lighting()
        },
    ] {
        let count = draw(&model(lighting)).len();
        assert!(count < 500, "{count} draw commands in one frame");
    }
}

#[test]
fn breathing_rises_and_falls_and_static_holds() {
    let breathing = lighting();
    let samples: Vec<u8> = (0..60).map(|i| preview::glow(&breathing, i as f32 * 0.1).2).collect();
    assert!(samples.iter().max() > samples.iter().min(), "breathing must vary");

    let fixed = Lighting {
        effect: Effect::Static,
        ..lighting()
    };
    assert_eq!(preview::glow(&fixed, 0.0), preview::glow(&fixed, 3.7));
}

#[test]
fn brightness_scales_the_preview_and_off_is_black() {
    let fixed = |brightness| Lighting {
        effect: Effect::Static,
        brightness,
        color: (200, 100, 50),
        ..lighting()
    };
    assert_eq!(preview::glow(&fixed(0), 0.0), (0, 0, 0));
    let dim = preview::glow(&fixed(1), 0.0);
    let full = preview::glow(&fixed(4), 0.0);
    assert!(dim.0 < full.0 && full == (200, 100, 50));
}

#[test]
fn the_wave_shows_different_colors_along_the_mouse() {
    let wave = Lighting {
        effect: Effect::RainbowWave,
        ..lighting()
    };
    assert_ne!(preview::band(&wave, 0.0, 0, 12), preview::band(&wave, 0.0, 6, 12));
    let fixed = Lighting {
        effect: Effect::Static,
        ..lighting()
    };
    assert_eq!(preview::band(&fixed, 0.0, 0, 12), preview::band(&fixed, 0.0, 6, 12));
}

#[test]
fn hue_and_pure_hue_are_inverses() {
    for degrees in [0.0, 30.0, 60.0, 120.0, 200.0, 240.0, 300.0, 330.0] {
        let found = color::pure_hue(color::hue(degrees)).expect("a pure color");
        assert!((found - degrees).abs() < 1.5, "{degrees} came back as {found}");
    }
    assert_eq!(color::pure_hue((255, 255, 255)), None);
    assert_eq!(color::pure_hue((128, 0, 0)), None);
}

#[test]
fn text_wraps_at_spaces() {
    assert_eq!(crate::view::wrap("one two three", 7), vec!["one two", "three"]);
    assert_eq!(crate::view::wrap("unbreakable", 4), vec!["unbreakable"]);
    assert!(crate::view::wrap("", 10).is_empty());
}
