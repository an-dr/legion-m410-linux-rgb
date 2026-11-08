//! The extension and the native module together, against a fake mouse. The panel is
//! read from the text it draws and driven by clicks at the coordinates it lays out.

mod support;

use bones_test::messages::gfx::{Clear, SetDisplay};
use m410_messages::{Command, Connection, Effect, Lighting, Status};
use m410_panel::color::SWATCHES;
use m410_panel::layout;
use support::{factory_lighting, Rig};

fn shows(texts: &[String], wanted: &str) -> bool {
    texts.iter().any(|text| text == wanted)
}

/// The point on a slider where it reads `fraction` of the way along.
fn along(track: m410_panel::Rect, fraction: f32) -> (i32, i32) {
    (track.x + (fraction * track.w as f32) as i32, track.center().1)
}

#[test]
fn it_asks_for_the_state_when_it_starts_and_shows_it() {
    let mut rig = Rig::start();
    rig.ready();

    assert_eq!(rig.commands().first(), Some(&Command::Refresh));
    let state = rig.state().unwrap();
    assert_eq!(state.connection, Connection::Wireless);
    assert_eq!(state.lighting, Some(factory_lighting()));

    let texts = rig.texts();
    for expected in ["Legion M410", "Wireless", "Breathing", "4 / 4", "3 / 5", "Random"] {
        assert!(shows(&texts, expected), "missing {expected:?} in {texts:?}");
    }
}

#[test]
fn choosing_an_effect_stores_it_on_the_mouse() {
    let mut rig = Rig::start();
    rig.ready();

    rig.click_at(layout::effect_button(Effect::Static).center());
    rig.wait_for_stored(Lighting {
        effect: Effect::Static,
        ..factory_lighting()
    });
    assert!(!shows(&rig.texts(), "3 / 5"), "a static light has no speed to show");
}

#[test]
fn only_effects_that_use_a_color_offer_one() {
    let mut rig = Rig::start();
    rig.ready();

    rig.click_at(layout::effect_button(Effect::RainbowWave).center());
    rig.wait_for_stored(Lighting {
        effect: Effect::RainbowWave,
        ..factory_lighting()
    });
    let texts = rig.texts();
    assert!(shows(&texts, "Chosen by the effect"), "{texts:?}");

    let before = rig.writes();
    rig.click_at(layout::swatch(0, SWATCHES.len()).center());
    assert_eq!(rig.writes(), before, "the wave ignores a swatch");
}

#[test]
fn dragging_a_slider_writes_once_when_it_is_released() {
    let mut rig = Rig::start_with(|mouse| {
        mouse.stored = Lighting {
            brightness: 4,
            ..factory_lighting()
        }
    });
    rig.ready();

    // Press at the top, pass through the middle, let go at two thirds: brightness 3.
    rig.drag(along(layout::BRIGHTNESS, 1.0), along(layout::BRIGHTNESS, 2.0 / 3.0));
    rig.wait_for_stored(Lighting {
        brightness: 3,
        ..factory_lighting()
    });
    assert_eq!(rig.writes(), 1, "one drag must cost one write");
}

#[test]
fn the_speed_slider_sets_a_speed() {
    let mut rig = Rig::start();
    rig.ready();

    rig.drag(along(layout::SPEED, 0.5), along(layout::SPEED, 1.0));
    rig.wait_for_stored(Lighting {
        speed: 5,
        ..factory_lighting()
    });
}

#[test]
fn a_swatch_chooses_its_color_and_ends_random_mode() {
    let mut rig = Rig::start();
    rig.ready();

    rig.click_at(layout::swatch(4, SWATCHES.len()).center());
    let (r, g, b) = SWATCHES[4];
    rig.wait_for_stored(Lighting {
        random: false,
        color: SWATCHES[4],
        ..factory_lighting()
    });
    assert!(shows(&rig.texts(), &format!("#{r:02x}{g:02x}{b:02x}")));
}

#[test]
fn the_hue_bar_picks_a_color_of_any_hue() {
    let mut rig = Rig::start_with(|mouse| {
        mouse.stored = Lighting {
            random: false,
            ..factory_lighting()
        }
    });
    rig.ready();

    rig.click_at(along(layout::HUE, 1.0 / 3.0));
    rig.settle("a green", |state| {
        state
            .lighting
            .is_some_and(|l| l.color.1 == 255 && l.color.2 == 0 && l.color.0 < 8)
    });
}

#[test]
fn random_colors_can_be_switched_on_and_off() {
    let mut rig = Rig::start_with(|mouse| {
        mouse.stored = Lighting {
            random: false,
            ..factory_lighting()
        }
    });
    rig.ready();

    rig.click_at(layout::RANDOM.center());
    rig.wait_for_stored(Lighting {
        random: true,
        ..factory_lighting()
    });
    rig.click_at(layout::RANDOM.center());
    rig.wait_for_stored(Lighting {
        random: false,
        ..factory_lighting()
    });
}

#[test]
fn turning_the_light_off_and_on_keeps_the_rest_of_the_settings() {
    let mut rig = Rig::start();
    rig.ready();

    rig.click_at(layout::POWER.center());
    rig.wait_for_stored(Lighting {
        brightness: 0,
        ..factory_lighting()
    });
    assert!(shows(&rig.texts(), "Off"));

    rig.click_at(layout::POWER.center());
    rig.wait_for_stored(factory_lighting());
}

#[test]
fn a_change_made_while_the_mouse_sleeps_lands_when_it_wakes() {
    let mut rig = Rig::start();
    rig.ready();
    rig.hardware.lock().unwrap().asleep = true;

    rig.click_at(layout::effect_button(Effect::Static).center());
    let wanted = Lighting {
        effect: Effect::Static,
        ..factory_lighting()
    };
    rig.settle("the waiting notice", |state| state.status == Status::Waiting);
    let texts = rig.texts();
    assert!(shows(&texts, "Asleep"), "{texts:?}");
    // The note wraps onto two lines, so each half is checked on its own.
    assert!(
        texts.iter().any(|text| text.contains("The mouse is asleep")),
        "{texts:?}"
    );
    assert!(shows(&texts, "applies then."), "{texts:?}");
    assert_eq!(rig.writes(), 0);

    rig.hardware.lock().unwrap().asleep = false;
    rig.wait_for_stored(wanted);
}

#[test]
fn it_says_so_when_no_mouse_is_connected_and_ignores_the_controls() {
    let mut rig = Rig::start_with(|mouse| mouse.connection = Err(m410_rgb_app::MouseError::NotFound));
    rig.settle("the not-found notice", |state| state.status == Status::NotFound);

    let texts = rig.texts();
    assert!(shows(&texts, "Not connected"), "{texts:?}");
    assert!(
        shows(&texts, "Plug in the mouse or its wireless receiver."),
        "{texts:?}"
    );

    let commands = rig.commands().len();
    rig.click_at(layout::effect_button(Effect::Static).center());
    rig.click_at(layout::POWER.center());
    assert_eq!(rig.commands().len(), commands, "controls do nothing without a mouse");
    rig.click_at(layout::REFRESH.center());
    assert_eq!(rig.commands().len(), commands + 1, "Refresh still works");
}

#[test]
fn plugging_the_mouse_in_brings_the_controls_up_by_themselves() {
    let mut rig = Rig::start_with(|mouse| mouse.connection = Err(m410_rgb_app::MouseError::NotFound));
    rig.settle("the not-found notice", |state| state.status == Status::NotFound);

    rig.hardware.lock().unwrap().connection = Ok(Connection::Wired);
    rig.ready();
    assert!(shows(&rig.texts(), "USB cable"));
}

#[test]
fn every_frame_is_cleared_and_fits_the_command_budget() {
    let mut rig = Rig::start();
    rig.ready();
    rig.app.step(5);
    assert!(
        rig.app.published::<Clear>().len() >= 5,
        "a frame that is not cleared leaves ghosts of the last one"
    );

    // An extension may publish 1024 messages a frame before it is quarantined.
    let commands = rig.frame_commands();
    assert!((100..1024).contains(&commands), "{commands} gfx commands in one frame");
}

#[test]
fn it_keeps_running_without_warnings() {
    let mut rig = Rig::start();
    rig.ready();
    rig.click_at(layout::effect_button(Effect::Static).center());
    rig.click_at(layout::swatch(2, SWATCHES.len()).center());
    rig.drag(along(layout::BRIGHTNESS, 0.0), along(layout::BRIGHTNESS, 0.5));
    rig.app.step(120);

    assert!(rig.app.is_running(), "quarantined: {:?}", rig.app.logs());
    let problems: Vec<String> = rig
        .app
        .logs()
        .into_iter()
        .filter(|line| line.starts_with("WARN") || line.starts_with("ERROR"))
        .collect();
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn it_keeps_the_window_the_size_the_layout_assumes() {
    let mut rig = Rig::start();
    rig.app.step(200);
    let asks = rig.app.published::<SetDisplay>();
    assert!(asks.len() >= 2, "asked {} times in 200 frames", asks.len());
    assert!(asks
        .iter()
        .all(|ask| (ask.width, ask.height, ask.fullscreen) == (layout::WIDTH, layout::HEIGHT, false)));
    assert!(
        asks.len() < 10,
        "{} asks is more than a periodic check should make",
        asks.len()
    );
}
