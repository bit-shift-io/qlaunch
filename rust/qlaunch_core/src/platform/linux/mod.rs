mod desktop_file;
mod icons;

use std::fs;
use std::path::PathBuf;

use crate::model::{AppCategory, AppEntry, AppRef};
use crate::platform::{AppSource, LaunchError, PlatformError};

pub use desktop_file::{
    DesktopEntry, applications_dirs_from, data_roots, data_roots_from, dirs_to_scan,
    dirs_to_scan_from, discover_applications, exec_argv, locale_from, parse_desktop,
};
pub use icons::{icon_dirs_from, resolve_icon, resolve_icon_themed};

pub struct LinuxSource {
    /// The `applications` directories, highest priority first.
    dirs: Vec<PathBuf>,
    icon_dirs: Vec<PathBuf>,
    current_desktop: Option<String>,
    locale: Option<String>,
    theme: Option<String>,
}

impl LinuxSource {
    /// A source over the XDG data roots, with the desktop, locale and icon
    /// theme (`XDG_CURRENT_THEME`, which few desktops set) taken from the
    /// environment now.
    pub fn new() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self::from_parts(
            data_roots(),
            var("XDG_CURRENT_DESKTOP"),
            locale_from(
                var("LC_ALL").as_deref(),
                var("LC_MESSAGES").as_deref(),
                var("LANG").as_deref(),
            ),
            var("XDG_CURRENT_THEME"),
        )
    }

    /// A source over exactly `data_roots`, ignoring the environment: nothing
    /// but the arguments decides what it finds, apart from the fixed
    /// `/usr/share/pixmaps` icon directory. Applications are read from
    /// `<root>/applications` and icons from `<root>/icons`, `theme` is searched
    /// before `hicolor`, and `current_desktop` and `locale` are as for
    /// [`discover_applications`].
    pub fn from_parts(
        data_roots: Vec<PathBuf>,
        current_desktop: Option<String>,
        locale: Option<String>,
        theme: Option<String>,
    ) -> Self {
        Self {
            dirs: applications_dirs_from(&data_roots),
            icon_dirs: icon_dirs_from(&data_roots),
            current_desktop,
            locale,
            theme,
        }
    }

    /// A source over exactly `dirs`, ignoring the environment: no desktop
    /// filtering, no locale and no icon theme. Its only icon directory is
    /// `/usr/share/pixmaps`, as `dirs` do not say where their icons are.
    pub fn new_with_dirs(dirs: Vec<PathBuf>) -> Self {
        Self {
            dirs,
            icon_dirs: icon_dirs_from(&[]),
            current_desktop: None,
            locale: None,
            theme: None,
        }
    }

    /// The first `<applications dir>/<id>` that is a file, which is how the
    /// desktop-file id maps to a file: nothing else is read or parsed.
    fn desktop_file(&self, id: &str) -> Option<PathBuf> {
        if !icons::is_plain_name(id) {
            return None;
        }
        self.dirs
            .iter()
            .map(|dir| dir.join(id))
            .find(|path| fs::metadata(path).is_ok_and(|m| m.is_file()))
    }

    fn icon_path(&self, id: &str, size_px: u32) -> Option<PathBuf> {
        let bytes = fs::read(self.desktop_file(id)?).ok()?;
        let entry = parse_desktop(&String::from_utf8_lossy(&bytes), id, self.locale.as_deref());
        resolve_icon_themed(
            &entry.icon?,
            size_px,
            &self.icon_dirs,
            self.theme.as_deref(),
        )
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

    /// The PNG bytes of the icon named by the app's desktop file, or `None`:
    /// for an unknown id, an entry without an icon, an icon that cannot be
    /// found or read, and an `svg` or `xpm` icon, which is not rasterised. An
    /// icon is cosmetic, so none of these is an error.
    fn icon_png(&self, app: &AppRef, size_px: u32) -> Result<Option<Vec<u8>>, PlatformError> {
        Ok(self
            .icon_path(&app.package, size_px)
            .filter(|path| icons::is_png(path))
            .and_then(|path| fs::read(path).ok()))
    }

    fn launch(&self, _app: &AppRef) -> Result<(), LaunchError> {
        Err(LaunchError::Unsupported(
            "launching .desktop applications is not implemented yet".into(),
        ))
    }
}
