//! The window's look, which follows the desktop while the app runs: theme, accent,
//! font and display scale.
//!
//! After `src/app/look.rs` of github.com/an-dr/cross-wall (MIT, same author),
//! without its custom window frame and user-chosen theme.

use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use super::desktop::Desktop;
use super::theme::{self, System};
use super::window;

const THEME_POLL: Duration = Duration::from_secs(5);
const SCALE_POLL: f64 = 1.0;

/// The smallest the window may be dragged to.
pub const MINIMUM_SIZE: (i32, i32) = (400, 560);

pub struct Look {
    desktop: Desktop,
    system: System,
    system_rx: Receiver<System>,
    styled: bool,
    display_scale: f32,
    scale_checked_at: f64,
}

impl Look {
    pub fn new() -> Self {
        let desktop = Desktop::detect();
        // Read before the window opens, so the first frame is already right.
        let system = theme::read_system(desktop);
        let (system_tx, system_rx) = channel();
        std::thread::spawn(move || loop {
            std::thread::sleep(THEME_POLL);
            if system_tx.send(theme::read_system(desktop)).is_err() {
                return;
            }
        });
        Self {
            desktop,
            system,
            system_rx,
            styled: false,
            display_scale: 1.0,
            scale_checked_at: f64::NEG_INFINITY,
        }
    }

    /// A look with nothing to follow, so a test does not start threads or read the desktop.
    #[cfg(test)]
    pub(crate) fn fixed(desktop: Desktop) -> Self {
        let (_, system_rx) = channel();
        Self {
            desktop,
            system: System {
                dark: true,
                accent: None,
            },
            system_rx,
            styled: true,
            display_scale: 1.0,
            scale_checked_at: f64::INFINITY,
        }
    }

    /// Follows mode, accent and display changes while the app runs.
    pub fn follow(&mut self, ctx: &egui::Context) {
        if !self.styled {
            self.styled = true;
            theme::apply(ctx, self.desktop, self.system);
            window::set_minimum_size(MINIMUM_SIZE.0, MINIMUM_SIZE.1);
            let (ctx, desktop) = (ctx.clone(), self.desktop);
            // The font lookup can start a process, so it stays off the frame thread.
            std::thread::spawn(move || {
                let system = theme::read_font(desktop);
                if system.is_some() {
                    theme::install_font(&ctx, system);
                }
            });
        }
        while let Ok(system) = self.system_rx.try_recv() {
            if system != self.system {
                self.system = system;
                theme::apply(ctx, self.desktop, self.system);
            }
        }
        let time = ctx.input(|i| i.time);
        if time - self.scale_checked_at >= SCALE_POLL {
            self.scale_checked_at = time;
            self.display_scale = theme::display_scale();
        }
    }

    pub fn scale(&self) -> f32 {
        self.display_scale.clamp(0.5, 6.0)
    }
}
