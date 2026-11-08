use m410::Rgb;

use super::*;

fn lighting(effect: Effect) -> Lighting {
    Lighting {
        effect,
        brightness: 4,
        speed: 3,
        random: false,
        color: Rgb(200, 100, 50),
    }
}

#[test]
fn breathing_rises_and_falls_and_static_holds() {
    let samples: Vec<u8> = (0..60)
        .map(|i| glow(&lighting(Effect::Breathing), i as f32 * 0.1).r())
        .collect();
    assert!(samples.iter().max() > samples.iter().min(), "breathing must vary");

    let fixed = lighting(Effect::Static);
    assert_eq!(glow(&fixed, 0.0), glow(&fixed, 3.7));
}

#[test]
fn brightness_scales_the_preview_and_off_is_black() {
    let at = |brightness| Lighting {
        brightness,
        ..lighting(Effect::Static)
    };
    assert_eq!(glow(&at(0), 0.0), Color32::BLACK);
    assert!(glow(&at(1), 0.0).r() < glow(&at(4), 0.0).r());
    assert_eq!(glow(&at(4), 0.0), Color32::from_rgb(200, 100, 50));
}

#[test]
fn the_cycle_moves_through_hues_and_faster_speeds_move_sooner() {
    let at = |speed, clock| {
        glow(
            &Lighting {
                speed,
                ..lighting(Effect::ColorCycle)
            },
            clock,
        )
    };
    assert_ne!(at(3, 0.0), at(3, 1.0));
    assert_ne!(at(5, 0.5), at(1, 0.5));
}

#[test]
fn the_wave_shows_different_colors_along_the_light() {
    let wave = lighting(Effect::RainbowWave);
    assert_ne!(band(&wave, 0.0, 0, 12), band(&wave, 0.0, 6, 12));
    let fixed = lighting(Effect::Static);
    assert_eq!(band(&fixed, 0.0, 0, 12), band(&fixed, 0.0, 6, 12));
}

#[test]
fn random_colors_ignore_the_chosen_color_and_move() {
    let random = Lighting {
        random: true,
        ..lighting(Effect::Static)
    };
    assert_ne!(glow(&random, 0.0), glow(&random, 2.0));
}

#[test]
fn hue_hits_the_primaries() {
    assert_eq!(hue(0.0), Color32::from_rgb(255, 0, 0));
    assert_eq!(hue(120.0), Color32::from_rgb(0, 255, 0));
    assert_eq!(hue(240.0), Color32::from_rgb(0, 0, 255));
    assert_eq!(hue(360.0), hue(0.0));
}
