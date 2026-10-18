mod desktop_file;

use std::path::PathBuf;

use crate::model::{AppCategory, AppEntry, AppRef};
use crate::platform::{AppSource, LaunchError, PlatformError};

pub use desktop_file::{
    DesktopEntry, dirs_to_scan, dirs_to_scan_from, discover_applications, exec_argv, locale_from,
    parse_desktop,
};

pub struct LinuxSource {
    dirs: Vec<PathBuf>,
    current_desktop: Option<String>,
    locale: Option<String>,
}

impl LinuxSource {
    /// A source over the XDG application directories, with the desktop and
    /// locale taken from the environment now.
    pub fn new() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self {
            dirs: dirs_to_scan(),
            current_desktop: var("XDG_CURRENT_DESKTOP"),
            locale: locale_from(
                var("LC_ALL").as_deref(),
                var("LC_MESSAGES").as_deref(),
                var("LANG").as_deref(),
            ),
        }
    }

    /// A source over exactly `dirs`, ignoring the environment: no desktop
    /// filtering and no locale.
    pub fn new_with_dirs(dirs: Vec<PathBuf>) -> Self {
        Self {
            dirs,
            current_desktop: None,
            locale: None,
        }
    }
}

impl Default for LinuxSource {
    fn default() -> Self {
        Self::new()
    }
}

/// `Game` and every subcategory of it (`ActionGame`, `BoardGame`, ...) are
/// games; everything else is an app.
fn category_of(entry: &DesktopEntry) -> AppCategory {
    if entry.categories.iter().any(|c| c.ends_with("Game")) {
        AppCategory::Game
    } else {
        AppCategory::App
    }
}

impl AppSource for LinuxSource {
    fn list_apps(&self) -> Result<Vec<AppEntry>, PlatformError> {
        let found = discover_applications(
            &self.dirs,
            self.current_desktop.as_deref(),
            self.locale.as_deref(),
        );
        Ok(found
            .into_iter()
            .map(|(id, entry)| AppEntry {
                package: id,
                activity: String::new(),
                category: category_of(&entry),
                label: entry.name,
            })
            .collect())
    }

    fn icon_png(&self, _app: &AppRef, _size_px: u32) -> Result<Option<Vec<u8>>, PlatformError> {
        Ok(None)
    }

    fn launch(&self, _app: &AppRef) -> Result<(), LaunchError> {
        Err(LaunchError::Unsupported(
            "launching .desktop applications is not implemented yet".into(),
        ))
    }
}
