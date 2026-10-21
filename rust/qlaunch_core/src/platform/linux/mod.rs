mod desktop_file;
mod icons;
pub(crate) mod launch;
mod watch;

use std::fs;
use std::path::PathBuf;

use crate::model::{AppCategory, AppEntry, AppRef, IconImage};
use crate::platform::backup::{is_backup_name, sort_backup_names};
use crate::platform::{AppSource, LaunchError, PlatformError, WatchHandle};
use crate::store::atomic;

pub use desktop_file::{
    DesktopEntry, applications_dirs_from, data_roots, data_roots_from, dirs_to_scan,
    dirs_to_scan_from, discover_applications, exec_argv, locale_from, parse_desktop,
};
pub use icons::{icon_dirs_from, resolve_icon, resolve_icon_themed};
pub use launch::{
    build_command, is_on_path, is_on_path_in, terminal_emulator, terminal_emulator_from,
};

pub struct LinuxSource {
    /// The `applications` directories, highest priority first.
    dirs: Vec<PathBuf>,
    icon_dirs: Vec<PathBuf>,
    current_desktop: Option<String>,
    locale: Option<String>,
    theme: Option<String>,
    terminal_emulator: Option<String>,
    /// Where config backups are written and read. `None` when the desktop has
    /// no Downloads folder (no home directory), which makes backups unsupported.
    downloads: Option<PathBuf>,
}

impl LinuxSource {
    /// A source over the XDG data roots, with the desktop, locale, icon
    /// theme (`XDG_CURRENT_THEME`, which few desktops set) and terminal
    /// emulator (see [`terminal_emulator`]) taken from the environment now.
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
            terminal_emulator(),
        )
    }

    /// A source over exactly `data_roots`, ignoring the environment: nothing
    /// but the arguments decides what it finds, apart from the fixed
    /// `/usr/share/pixmaps` icon directory. Applications are read from
    /// `<root>/applications` and icons from `<root>/icons`, `theme` is searched
    /// before `hicolor`, and `current_desktop` and `locale` are as for
    /// [`discover_applications`]. `terminal_emulator` runs applications that
    /// ask for a terminal; without one they cannot be launched.
    pub fn from_parts(
        data_roots: Vec<PathBuf>,
        current_desktop: Option<String>,
        locale: Option<String>,
        theme: Option<String>,
        terminal_emulator: Option<String>,
    ) -> Self {
        Self {
            dirs: applications_dirs_from(&data_roots),
            icon_dirs: icon_dirs_from(&data_roots),
            current_desktop,
            locale,
            theme,
            terminal_emulator,
            downloads: dirs::download_dir(),
        }
    }

    /// Replaces the Downloads folder backups go to. For tests, so they never
    /// touch the person's real downloads.
    pub fn with_downloads(mut self, downloads: Option<PathBuf>) -> Self {
        self.downloads = downloads;
        self
    }

    /// A source over exactly `dirs`, ignoring the environment: no desktop
    /// filtering, no locale, no icon theme and no terminal emulator. Its only icon directory is
    /// `/usr/share/pixmaps`, as `dirs` do not say where their icons are.
    pub fn new_with_dirs(dirs: Vec<PathBuf>) -> Self {
        Self {
            dirs,
            icon_dirs: icon_dirs_from(&[]),
            current_desktop: None,
            locale: None,
            theme: None,
            terminal_emulator: None,
            downloads: dirs::download_dir(),
        }
    }

    fn downloads_dir(&self) -> Result<PathBuf, PlatformError> {
        self.downloads.clone().ok_or_else(|| {
            PlatformError::Unsupported("there is no Downloads folder to save a backup in".into())
        })
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

    /// The desktop file's modification time in milliseconds, used as the icon
    /// cache token so a rewritten entry (a package update) misses the cache.
    fn desktop_mtime_ms(&self, id: &str) -> i64 {
        self.desktop_file(id)
            .and_then(|path| fs::metadata(path).ok())
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or(0)
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
            .map(|(id, entry)| {
                // A rewritten desktop file (a package update) changes this, so
                // the persisted icon cache misses rather than staying stale.
                let last_update_time = self.desktop_mtime_ms(&id);
                AppEntry {
                    package: id,
                    activity: String::new(),
                    category: category_of(&entry),
                    label: entry.name,
                    last_update_time,
                }
            })
            .collect())
    }

    /// The PNG bytes of the icon named by the app's desktop file, or `None`:
    /// for an unknown id, an entry without an icon, an icon that cannot be
    /// found or read, and an `svg` or `xpm` icon, which is not rasterised. An
    /// icon is cosmetic, so none of these is an error.
    fn icon_image(&self, app: &AppRef, size_px: u32) -> Result<Option<IconImage>, PlatformError> {
        Ok(self
            .icon_path(&app.package, size_px)
            .filter(|path| icons::is_png(path))
            .and_then(|path| fs::read(path).ok())
            .map(IconImage::png))
    }

    /// Starts the app's `Exec` line without a shell and without waiting for
    /// it. An unknown id is `NotFound`; an entry that cannot be read, has no
    /// usable `Exec`, or cannot be spawned is `Failed`; a terminal entry with
    /// no emulator is `Unsupported`.
    fn launch(&self, app: &AppRef) -> Result<(), LaunchError> {
        let id = &app.package;
        let path = self
            .desktop_file(id)
            .ok_or_else(|| LaunchError::NotFound(app.clone()))?;
        let bytes = fs::read(&path)
            .map_err(|e| LaunchError::Failed(format!("could not read {id}: {e}")))?;
        let entry = parse_desktop(&String::from_utf8_lossy(&bytes), id, self.locale.as_deref());
        let exec = entry
            .exec
            .ok_or_else(|| LaunchError::Failed(format!("{id} has no Exec line to run")))?;
        let argv = build_command(&exec, entry.terminal, self.terminal_emulator.as_deref())?;
        launch::spawn(&argv)
    }

    /// Calls `on_change` once, 250 ms after the last of any burst of changes
    /// to the source's `applications` directories. A directory that does not
    /// exist is skipped, and if none exists the handle is inert. If some
    /// exist but none can be watched (not a directory, permission, the
    /// inotify limit) that is an error. `on_change` runs on a thread of its
    /// own; if it panics, no further notifications are sent.
    fn watch(&self, on_change: Box<dyn Fn() + Send + Sync>) -> Result<WatchHandle, PlatformError> {
        watch::watch(&self.dirs, on_change)
    }

    fn write_backup(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        if !is_backup_name(name) {
            return Err(PlatformError::Unsupported(format!(
                "{name:?} is not a backup file name"
            )));
        }
        let dir = self.downloads_dir()?;
        fs::create_dir_all(&dir).map_err(|e| PlatformError::Io(e.to_string()))?;
        atomic::write_atomic(&dir.join(name), bytes).map_err(|e| PlatformError::Io(e.to_string()))
    }

    fn list_backups(&self) -> Result<Vec<String>, PlatformError> {
        let Some(dir) = &self.downloads else {
            return Ok(Vec::new());
        };
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(PlatformError::Io(e.to_string())),
        };
        let names = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
            .map(|entry| entry.file_name().to_string_lossy().into_owned());
        Ok(sort_backup_names(names))
    }

    fn read_backup(&self, name: &str) -> Result<Vec<u8>, PlatformError> {
        if !is_backup_name(name) {
            return Err(PlatformError::Unsupported(format!(
                "{name:?} is not a backup file name"
            )));
        }
        fs::read(self.downloads_dir()?.join(name)).map_err(|e| PlatformError::Io(e.to_string()))
    }
}
