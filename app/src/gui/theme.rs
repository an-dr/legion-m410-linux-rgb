//! Makes egui follow the desktop: its interface font, light or dark mode, accent
//! colour, and its spacing and corners. egui draws its own widgets, so this
//! resembles the platform without being its real controls.
//!
//! Ported from `src/theme.rs` of github.com/an-dr/cross-wall (MIT, same author),
//! keeping the Linux desktops: COSMIC, GNOME and KDE.

use std::path::PathBuf;

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};

use super::colour::{luminance, on_colour};
use super::desktop::Desktop;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct System {
    pub dark: bool,
    pub accent: Option<Color32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    corner: u8,
    body: f32,
    button_padding: [f32; 2],
    item_spacing: [f32; 2],
    dark_panel: u8,
    light_panel: u8,
    accent: Color32,
}

fn metrics(desktop: Desktop) -> Metrics {
    match desktop {
        Desktop::Kde => Metrics {
            corner: 3,
            body: 13.5,
            button_padding: [8.0, 5.0],
            item_spacing: [6.0, 6.0],
            dark_panel: 42,
            light_panel: 239,
            accent: Color32::from_rgb(61, 174, 233),
        },
        Desktop::Gnome => Metrics {
            corner: 9,
            body: 15.0,
            button_padding: [12.0, 7.0],
            item_spacing: [10.0, 10.0],
            dark_panel: 36,
            light_panel: 250,
            accent: Color32::from_rgb(53, 132, 228),
        },
        Desktop::Cosmic => Metrics {
            corner: 8,
            body: 14.0,
            button_padding: [12.0, 6.0],
            item_spacing: [8.0, 8.0],
            dark_panel: 27,
            light_panel: 242,
            accent: Color32::from_rgb(99, 208, 223),
        },
    }
}

/// Applies colours and metrics. Called again when the system mode or accent changes.
pub fn apply(ctx: &egui::Context, desktop: Desktop, system: System) {
    let look = metrics(desktop);
    let accent = system.accent.unwrap_or(look.accent);
    ctx.set_visuals(visuals(&look, system.dark, accent));
    ctx.style_mut(|style| {
        let spacing = &mut style.spacing;
        spacing.item_spacing = look.item_spacing.into();
        spacing.button_padding = look.button_padding.into();
        spacing.interact_size.y = look.body + 2.0 * look.button_padding[1];
        spacing.window_margin = Margin::same(16);
        spacing.scroll = egui::style::ScrollStyle::floating();
        for (text, size, family) in [
            (TextStyle::Small, look.body * 0.85, FontFamily::Proportional),
            (TextStyle::Body, look.body, FontFamily::Proportional),
            (TextStyle::Button, look.body, FontFamily::Proportional),
            (TextStyle::Heading, look.body * 1.5, FontFamily::Proportional),
            (TextStyle::Monospace, look.body, FontFamily::Monospace),
        ] {
            style.text_styles.insert(text, FontId::new(size, family));
        }
    });
}

fn visuals(look: &Metrics, dark: bool, accent: Color32) -> egui::Visuals {
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let shade = |offset: i16| {
        let base = if dark { look.dark_panel } else { look.light_panel } as i16;
        let value = (base + if dark { offset } else { -offset }).clamp(0, 255) as u8;
        Color32::from_gray(value)
    };
    let text = if dark {
        Color32::from_gray(240)
    } else {
        Color32::from_gray(28)
    };
    let radius = CornerRadius::same(look.corner);
    v.panel_fill = shade(0);
    v.window_fill = shade(6);
    v.faint_bg_color = shade(6);
    v.extreme_bg_color = if dark { shade(-10) } else { Color32::WHITE };
    v.hyperlink_color = readable_on(accent, shade(0));
    v.selection.bg_fill = accent;
    v.selection.stroke = Stroke::new(1.0_f32, on_colour(accent));
    v.window_corner_radius = CornerRadius::same(look.corner.saturating_add(4));
    v.menu_corner_radius = radius;
    v.window_stroke = Stroke::new(1.0_f32, shade(22));
    for (state, fill) in [
        (&mut v.widgets.noninteractive, shade(0)),
        (&mut v.widgets.inactive, shade(16)),
        (&mut v.widgets.hovered, shade(26)),
        (&mut v.widgets.active, shade(34)),
        (&mut v.widgets.open, shade(26)),
    ] {
        state.corner_radius = radius;
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.expansion = 0.0;
        state.fg_stroke = Stroke::new(1.0_f32, text);
        state.bg_stroke = Stroke::NONE;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, shade(16));
    v
}

/// A dark accent is unreadable as link text on a dark panel, and the reverse.
fn readable_on(accent: Color32, panel: Color32) -> Color32 {
    if (luminance(accent) - luminance(panel)).abs() >= 0.3 {
        return accent;
    }
    let toward: f32 = if luminance(panel) < 0.5 { 255.0 } else { 0.0 };
    let mix = |c: u8| (c as f32 * 0.4 + toward * 0.6) as u8;
    Color32::from_rgb(mix(accent.r()), mix(accent.g()), mix(accent.b()))
}

/// Puts the desktop's interface font first, after the icon font. egui's fonts
/// stay as fallbacks for characters it lacks, and a font that cannot be read
/// leaves egui's own in use.
pub fn install_font(ctx: &egui::Context, system: Option<Vec<u8>>) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    if let Some(bytes) = system {
        fonts
            .font_data
            .insert("system".into(), egui::FontData::from_owned(bytes).into());
        if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
            family.insert(0, "system".into());
        }
    }
    ctx.set_fonts(fonts);
}

/// Can start a process, so it stays off the frame thread.
pub fn read_font(desktop: Desktop) -> Option<Vec<u8>> {
    font_path(desktop).and_then(|path| std::fs::read(path).ok())
}

fn font_path(desktop: Desktop) -> Option<PathBuf> {
    match desktop {
        Desktop::Kde => fc_match(
            &kde_value("General", "font")
                .and_then(|f| kde_font(&f))
                .unwrap_or("Noto Sans".into()),
        ),
        Desktop::Gnome => fc_match(
            &command_output("gsettings", &["get", "org.gnome.desktop.interface", "font-name"])
                .and_then(|text| gnome_font(&text))
                .unwrap_or("Cantarell".into()),
        ),
        Desktop::Cosmic => fc_match("Open Sans"),
    }
}

fn fc_match(family: &str) -> Option<PathBuf> {
    command_output("fc-match", &["-f", "%{file}", family])
        .map(PathBuf::from)
        .filter(|p| p.exists())
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `'Cantarell 11'` gives `Cantarell`.
fn gnome_font(value: &str) -> Option<String> {
    let name = value.trim().trim_matches('\'');
    let (family, size) = name.rsplit_once(' ')?;
    size.parse::<f32>().ok().map(|_| family.to_string())
}

/// `Noto Sans,10,-1,5,50,...` gives `Noto Sans`.
fn kde_font(value: &str) -> Option<String> {
    value.split(',').next().filter(|f| !f.is_empty()).map(String::from)
}

fn kde_section_value(text: &str, section: &str, key: &str) -> Option<String> {
    let header = format!("[{section}]");
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == header;
        } else if inside {
            if let Some((name, value)) = line.split_once('=') {
                if name.trim() == key {
                    return Some(value.trim().to_string());
                }
            }
        }
    }
    None
}

fn kde_value(section: &str, key: &str) -> Option<String> {
    let path = dirs::config_dir()?.join("kdeglobals");
    kde_section_value(&std::fs::read_to_string(path).ok()?, section, key)
}

fn parse_rgb(value: &str) -> Option<Color32> {
    let mut parts = value.split(',').map(|p| p.trim().parse::<u8>());
    Some(Color32::from_rgb(
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ))
}

fn gnome_accent(value: &str) -> Option<Color32> {
    Some(match value.trim().trim_matches('\'') {
        "blue" => Color32::from_rgb(53, 132, 228),
        "teal" => Color32::from_rgb(33, 144, 164),
        "green" => Color32::from_rgb(58, 148, 74),
        "yellow" => Color32::from_rgb(200, 136, 0),
        "orange" => Color32::from_rgb(237, 91, 0),
        "red" => Color32::from_rgb(230, 45, 66),
        "pink" => Color32::from_rgb(213, 97, 153),
        "purple" => Color32::from_rgb(145, 65, 172),
        "slate" => Color32::from_rgb(111, 131, 150),
        _ => return None,
    })
}

/// The desktop's own light or dark setting and accent colour. Starts processes
/// or reads files, so it stays off the frame thread.
pub fn read_system(desktop: Desktop) -> System {
    match desktop {
        Desktop::Gnome => System {
            dark: command_output("gsettings", &["get", "org.gnome.desktop.interface", "color-scheme"])
                .is_some_and(|v| v.contains("prefer-dark")),
            accent: command_output("gsettings", &["get", "org.gnome.desktop.interface", "accent-color"])
                .and_then(|v| gnome_accent(&v)),
        },
        Desktop::Kde => System {
            dark: kde_value("Colors:Window", "BackgroundNormal")
                .and_then(|v| parse_rgb(&v))
                .is_some_and(|c| luminance(c) < 0.5),
            accent: kde_value("Colors:Selection", "BackgroundNormal").and_then(|v| parse_rgb(&v)),
        },
        Desktop::Cosmic => System {
            dark: dirs::config_dir()
                .and_then(|d| std::fs::read_to_string(d.join("cosmic/com.system76.CosmicTheme.Mode/v1/is_dark")).ok())
                .is_none_or(|v| v.trim() == "true"),
            accent: None,
        },
    }
}

/// Physical pixels per interface point. bones reports no display scale, so ask SDL.
/// A native Wayland window is already scaled by the compositor.
pub fn display_scale() -> f32 {
    use sdl3::sys::{keyboard, video};
    // SAFETY: plain SDL queries with no pointer kept past the call; SDL returns null for none.
    unsafe {
        let driver = video::SDL_GetCurrentVideoDriver();
        if !driver.is_null() && std::ffi::CStr::from_ptr(driver).to_bytes() == b"wayland" {
            return 1.0;
        }
        let window = keyboard::SDL_GetKeyboardFocus();
        let scale = if window.is_null() {
            video::SDL_GetDisplayContentScale(video::SDL_GetPrimaryDisplay())
        } else {
            video::SDL_GetWindowDisplayScale(window)
        };
        if scale > 0.0 {
            scale
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests;
