//! The executable: the bones engine with the M410 native module composed in.
//!
//! This is the shipped engine's composition plus one `.module(...)` line. The
//! interface is the WASM extension in `extension/`; see [`Layout`] for where it is found.

use std::path::{Path, PathBuf};

use bones_kernel::logging::Logger;
use m410_rgb_app::M410Module;

const TITLE: &str = "Legion M410 RGB";

fn main() {
    let logger = Logger::default();
    let layout = Layout::find();
    let result = bones_kernel::Kernel::new()
        .logger(logger.clone())
        .window(TITLE, 460, 720)
        .renderer()
        .module(M410Module::new(logger))
        .extensions_dir(layout.extensions)
        .saves_dir(layout.saves)
        .devtools_from_env("BONES_DEVTOOLS_PORT")
        .offscreen_from_env("BONES_OFFSCREEN")
        .run();
    if let Err(error) = result {
        eprintln!("fatal: {error}");
        std::process::exit(1);
    }
}

/// Where the extension is loaded from and state is kept.
struct Layout {
    extensions: PathBuf,
    saves: PathBuf,
}

impl Layout {
    /// Beside the executable when installed (`extensions/` next to it); otherwise
    /// the source tree's `.bones/`, which `bones-develop build` fills.
    fn find() -> Self {
        let installed = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .filter(|dir| dir.join("extensions").is_dir());
        match installed {
            Some(dir) => Self {
                extensions: dir.join("extensions"),
                saves: dir.join("states"),
            },
            None => {
                let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.bones");
                Self {
                    extensions: root.join("extensions"),
                    saves: root.join("states"),
                }
            }
        }
    }
}
