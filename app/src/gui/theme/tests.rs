use super::*;

const DESKTOPS: [Desktop; 3] = [Desktop::Kde, Desktop::Gnome, Desktop::Cosmic];

#[test]
fn parses_system_values() {
    assert_eq!(gnome_font("'Cantarell 11'").as_deref(), Some("Cantarell"));
    assert_eq!(gnome_font("'Adwaita Sans 10.5'").as_deref(), Some("Adwaita Sans"));
    assert_eq!(kde_font("Noto Sans,10,-1,5,50,0").as_deref(), Some("Noto Sans"));
    assert_eq!(gnome_accent("'teal'\n"), Some(Color32::from_rgb(33, 144, 164)));
    assert_eq!(gnome_accent("'mauve'"), None);
    assert_eq!(parse_rgb("61, 174,233"), Some(Color32::from_rgb(61, 174, 233)));
    assert_eq!(parse_rgb("61,174"), None);
}

#[test]
fn reads_only_the_named_kde_section() {
    let text = "[General]\nfont=Noto Sans,10\n[Colors:Window]\nBackgroundNormal=40,40,40\n";
    assert_eq!(
        kde_section_value(text, "Colors:Window", "BackgroundNormal").as_deref(),
        Some("40,40,40")
    );
    assert_eq!(kde_section_value(text, "Colors:Window", "font"), None);
    assert_eq!(
        kde_section_value(text, "General", "font").as_deref(),
        Some("Noto Sans,10")
    );
}

#[test]
fn every_desktop_gets_a_dark_and_a_light_look() {
    for desktop in DESKTOPS {
        let ctx = egui::Context::default();
        apply(
            &ctx,
            desktop,
            System {
                dark: true,
                accent: None,
            },
        );
        let dark = ctx.style().visuals.panel_fill;
        apply(
            &ctx,
            desktop,
            System {
                dark: false,
                accent: None,
            },
        );
        let light = ctx.style().visuals.panel_fill;
        assert!(
            luminance(dark) < 0.3 && luminance(light) > 0.7,
            "{desktop:?}: {dark:?} {light:?}"
        );
    }
}

#[test]
fn the_cosmic_look_uses_its_accent_unless_the_system_gives_one() {
    let ctx = egui::Context::default();
    apply(
        &ctx,
        Desktop::Cosmic,
        System {
            dark: true,
            accent: None,
        },
    );
    assert_eq!(ctx.style().visuals.selection.bg_fill, Color32::from_rgb(99, 208, 223));
    let custom = Color32::from_rgb(200, 30, 60);
    apply(
        &ctx,
        Desktop::Cosmic,
        System {
            dark: true,
            accent: Some(custom),
        },
    );
    assert_eq!(ctx.style().visuals.selection.bg_fill, custom);
}

#[test]
fn a_dim_accent_is_lifted_to_stay_readable_as_a_link() {
    let panel = Color32::from_gray(27);
    let dim = Color32::from_rgb(20, 20, 60);
    assert!(luminance(readable_on(dim, panel)) > luminance(dim));
    let bright = Color32::from_rgb(99, 208, 223);
    assert_eq!(readable_on(bright, panel), bright);
}
