mod atomic;
mod location;
mod migrate;

pub use location::{config_root, config_root_from};

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::model::{Group, Id, Item, Model, ModelError};
use crate::settings::Settings;
use crate::usage::Usage;

pub const SCHEMA_VERSION: u32 = 1;

const DOCUMENT_NAME: &str = "qlaunch.json";
const QUARANTINE_NAME: &str = "qlaunch.json.corrupt";
const BACKUP_NAME: &str = "qlaunch.json.bak";
const ICONS_DIR: &str = "icons";

/// The whole of what is stored on disk: one versioned JSON document.
///
/// `settings` and `usage` may be absent (they then take the defaults); every
/// other field is required, since a document without them is damaged, not
/// merely old.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub schema: u32,
    #[serde(default)]
    pub settings: Settings,
    /// What has been launched, and when. Absent in a document written before
    /// the search screen existed, which is why it defaults rather than failing.
    #[serde(default)]
    pub usage: Usage,
    pub root_group_id: Id,
    pub groups: Vec<Group>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub model: Model,
    pub settings: Settings,
    /// The document's usage, or the defaults when it had none.
    pub usage: Usage,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("the document is not valid JSON of the expected shape: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unsupported schema version {0}")]
    UnsupportedSchema(u32),
    #[error("the document is not a valid model: {0}")]
    Invalid(#[from] ModelError),
    #[error("{0:?} is not a plain file name")]
    InvalidIconName(String),
    #[error(
        "the saved layout could not be read, so nothing will be written over it until a load succeeds"
    )]
    WritesBlocked,
}

/// The one safety property of the store: **it never writes a document it has
/// not read.** A `load` that finds the document but cannot read it blocks
/// every later `save` and `quarantine` until a `load` succeeds, so a model
/// built from defaults can never replace the document that was not read.
///
/// | read result | writes afterwards |
/// |---|---|
/// | no load yet (`Store::new`) | allowed |
/// | `NotFound` / `NotADirectory` (absent) | allowed; the defaults are created |
/// | any other error (unreadable) | **refused** (`StoreError::WritesBlocked`) |
/// | read succeeded, even if corrupt | allowed; a corrupt file is already moved aside |
///
/// The block is shared by every clone of a `Store`, and is an atomic flag so
/// `Store` stays `Send + Sync` without a lock.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
    writes_blocked: Arc<AtomicBool>,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Store {
            root,
            writes_blocked: Arc::new(AtomicBool::new(false)),
        }
    }

    fn check_writable(&self) -> Result<(), StoreError> {
        if self.writes_blocked.load(Ordering::Acquire) {
            Err(StoreError::WritesBlocked)
        } else {
            Ok(())
        }
    }

    pub fn document_path(&self) -> PathBuf {
        self.root.join(DOCUMENT_NAME)
    }

    fn icons_dir(&self) -> PathBuf {
        self.root.join(ICONS_DIR)
    }

    /// The path of the custom icon called `name` in `root/icons/`.
    ///
    /// `name` comes from `icon_override`, which comes from a file a person can
    /// edit, so it must be a single plain path component: no separators, no
    /// `.` or `..`, nothing absolute, not empty. Anything else is an error and
    /// is never joined onto the path.
    pub fn icon_path(&self, name: &str) -> Result<PathBuf, StoreError> {
        let mut components = Path::new(name).components();
        let plain = matches!(
            (components.next(), components.next()),
            (Some(Component::Normal(_)), None)
        );
        let clean = !name.contains(['/', '\\', '\0']);
        if !plain || !clean {
            return Err(StoreError::InvalidIconName(name.to_string()));
        }
        Ok(self.icons_dir().join(name))
    }

    /// Reads the document, and decides what the read means. The three cases
    /// are kept strictly apart, because only the last two may touch the disk:
    ///
    /// - **Absent**: reading fails with `NotFound`, or with `NotADirectory`
    ///   (a directory above the document is a file, so no document can exist).
    ///   The defaults are written and returned. This is the only case in which
    ///   a document is created from nothing. If the write fails (the store
    ///   directory is read-only, or cannot be created) the defaults are
    ///   returned all the same, with a warning.
    /// - **Unreadable**: reading fails any other way (permission denied, the
    ///   path is a directory, any other I/O error). The file may be someone's
    ///   real document, so nothing is written, replaced or quarantined. The
    ///   in-memory defaults are returned with a warning, and the store refuses
    ///   every `save` and `quarantine` (`StoreError::WritesBlocked`) until a
    ///   later `load` succeeds. See [`Store`].
    /// - **Read**: the bytes were read. If they are corrupt or not a valid
    ///   model the document is quarantined (see below); otherwise it is used,
    ///   and repaired if need be. Quarantine moves a file aside, so it only
    ///   ever runs on a file that was read successfully.
    ///
    /// What counts as damage and what counts as repairable, for the clamped
    /// numeric settings:
    /// - an **integer that fits in `i64`** but is outside the permitted range
    ///   is **clamped to the nearest bound and written back**, so the user's
    ///   entry is not thrown away;
    /// - anything else is **corruption and is quarantined**: a value that is
    ///   not an integer (`40.5`), that does not fit in `i64` (`1e30`), or that
    ///   is the wrong JSON type altogether (`"big"`).
    ///
    /// That boundary is deliberate: a fractional or astronomically large
    /// number in a settings file is not worth a lenient number parser.
    ///
    /// A document that is corrupt, or that is not a valid model, is moved aside
    /// (see [`Store::quarantine`]), replaced by the defaults, and described in
    /// `warnings`. That holds even when the defaults then cannot be written:
    /// the quarantine has happened and is reported.
    ///
    /// Settings are clamped on the way in. If the document that results differs
    /// from the one that was read (settings clamped, duplicate `AppRef`s
    /// removed, or the schema migrated), the repaired document is written back
    /// at once, so each warning is reported once. A clean document is not
    /// rewritten. Before that write the original bytes are copied aside to
    /// `qlaunch.json.bak` (see [`Store::back_up`]), since a repair can remove
    /// content, and the warnings name the copy. If the backup cannot be made
    /// the document is left as it was; if the write fails the repaired model is
    /// still returned. Either way there is a further warning.
    pub fn load(&self) -> Result<Loaded, StoreError> {
        let bytes = match fs::read(self.document_path()) {
            Ok(bytes) => bytes,
            Err(e) if is_absent(&e) => {
                self.writes_blocked.store(false, Ordering::Release);
                return Ok(self.write_defaults(Vec::new()));
            }
            Err(e) => {
                self.writes_blocked.store(true, Ordering::Release);
                return Ok(self.unreadable(&e));
            }
        };
        self.writes_blocked.store(false, Ordering::Release);
        match interpret(&bytes) {
            Ok((mut loaded, repaired)) => {
                if repaired {
                    self.write_back(&bytes, &mut loaded);
                }
                Ok(loaded)
            }
            Err(reason) => {
                let kept = self.quarantine()?;
                let warning = format!(
                    "The saved layout could not be used ({reason}). It was kept as {} and a fresh layout was created.",
                    kept.display()
                );
                Ok(self.write_defaults(vec![warning]))
            }
        }
    }

    /// Writes the document atomically (see `atomic::write_atomic`: concurrent
    /// saves are last-writer-wins, and the file is always one complete
    /// document). Refused with `WritesBlocked` while the document is unreadable
    /// (see [`Store`]). A model that [`Model::validate`] rejects
    /// is refused, since `load` would only quarantine it again.
    pub fn save(&self, model: &Model, settings: &Settings, usage: &Usage) -> Result<(), StoreError> {
        self.check_writable()?;
        model.validate()?;
        let document = Document {
            schema: SCHEMA_VERSION,
            settings: settings.clone(),
            usage: usage.clone(),
            root_group_id: model.root_group_id.clone(),
            groups: model.groups.clone(),
            items: model.items.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&document)?;
        fs::create_dir_all(self.icons_dir())?;
        atomic::write_atomic(&self.document_path(), &bytes)?;
        Ok(())
    }

    /// Stores a custom icon for `item` and returns the file name to put in
    /// `icon_override`. The name is derived from the item id, never supplied.
    pub fn write_icon(&self, item: &Id, png: &[u8]) -> Result<String, StoreError> {
        if item.is_empty() {
            return Err(StoreError::InvalidIconName(String::new()));
        }
        let name = format!("{item}.png");
        let path = self.icon_path(&name)?;
        fs::create_dir_all(self.icons_dir())?;
        atomic::write_atomic(&path, png)?;
        Ok(name)
    }

    /// Moves the document aside and returns the path it was moved to.
    ///
    /// The first quarantine is `qlaunch.json.corrupt`. An earlier quarantine is
    /// never overwritten: later ones take the first free name in the series
    /// `qlaunch.json.corrupt.1`, `.2`, and so on. The earliest file is the one
    /// most likely to hold the user's real setup. A quarantined file failed to
    /// load at all; compare [`Store::back_up`].
    pub fn quarantine(&self) -> Result<PathBuf, StoreError> {
        self.check_writable()?;
        let to = self.free_name(QUARANTINE_NAME)?;
        fs::rename(self.document_path(), &to)?;
        atomic::sync_dir(&self.root)?;
        Ok(to)
    }

    /// Keeps `bytes`, the document as it was read, as `qlaunch.json.bak` and
    /// returns that path. Never overwrites: later backups are `.bak.1`, `.bak.2`
    /// and so on. A backup is of a document that loaded fine and was then
    /// repaired; compare [`Store::quarantine`].
    fn back_up(&self, bytes: &[u8]) -> Result<PathBuf, StoreError> {
        let to = self.free_name(BACKUP_NAME)?;
        atomic::write_atomic(&to, bytes)?;
        Ok(to)
    }

    /// The first unused name in the series `base`, `base.1`, `base.2`, ... in
    /// the store directory. A dangling symlink counts as used.
    fn free_name(&self, base: &str) -> Result<PathBuf, StoreError> {
        let mut path = self.root.join(base);
        let mut n = 0u32;
        while path_taken(&path)? {
            n += 1;
            path = self.root.join(format!("{base}.{n}"));
        }
        Ok(path)
    }

    /// Replaces the document with the repaired `loaded`, after keeping the
    /// original `bytes`. Failures become warnings, not errors: the repaired
    /// model is fine in memory.
    fn write_back(&self, bytes: &[u8], loaded: &mut Loaded) {
        match self.back_up(bytes) {
            Ok(kept) => {
                loaded.warnings.push(format!(
                    "The saved layout was repaired. The original was kept as {}.",
                    kept.display()
                ));
                if let Err(e) = self.save(&loaded.model, &loaded.settings, &loaded.usage) {
                    loaded.warnings.push(format!(
                        "The repaired layout could not be saved ({e}); it will be repaired again next time."
                    ));
                }
            }
            Err(e) => loaded.warnings.push(format!(
                "The saved layout needs repair, but its original could not be kept ({e}), so it was left as it is."
            )),
        }
    }

    /// The defaults for a document that is there but could not be read. Writes
    /// nothing: see [`Store::load`].
    fn unreadable(&self, e: &io::Error) -> Loaded {
        Loaded {
            model: Model::new_default(),
            settings: Settings::default(),
            usage: Usage::default(),
            warnings: vec![format!(
                "The saved layout at {} could not be read ({e}). It was left as it is, and a temporary default layout is being used.",
                self.document_path().display()
            )],
        }
    }

    /// Returns the defaults after trying to write them. A store that cannot be
    /// written (a read-only or blocked directory) must not stop the app from
    /// starting, so a failed write is a warning: the defaults are still
    /// returned and the write is tried again at the next save.
    fn write_defaults(&self, mut warnings: Vec<String>) -> Loaded {
        let model = Model::new_default();
        let settings = Settings::default();
        let usage = Usage::default();
        if let Err(e) = self.save(&model, &settings, &usage) {
            warnings.push(format!(
                "The layout could not be saved to {} ({e}); changes will not be kept until it can be.",
                self.root.display()
            ));
        }
        Loaded {
            model,
            settings,
            usage,
            warnings,
        }
    }
}

/// Whether `e` means the document is not there. `NotADirectory` counts: it is
/// what reading `root/qlaunch.json` reports when `root`, or a directory above
/// it, is a file, and no document can exist there either.
fn is_absent(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

/// Whether anything (even a dangling symlink) is at `path`.
fn path_taken(path: &Path) -> Result<bool, StoreError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Turns the bytes of a document into a usable model, or says why not. The
/// flag says whether the result differs from what was read, so it needs
/// writing back.
fn interpret(bytes: &[u8]) -> Result<(Loaded, bool), StoreError> {
    let mut value: Value = serde_json::from_slice(bytes)?;
    let found = value
        .get("schema")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    let mut migrated = false;
    if let Some(schema) = found.filter(|&n| n != SCHEMA_VERSION) {
        value = migrate::migrate(schema, value)?;
        migrated = true;
    }
    let document: Document = serde_json::from_value(value)?;
    if document.schema != SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchema(document.schema));
    }
    let read = document.clone();

    let usage = document.usage;
    let mut settings = document.settings;
    let mut warnings = clamp_settings(&mut settings);
    let mut model = Model {
        root_group_id: document.root_group_id,
        groups: document.groups,
        items: document.items,
    };
    warnings.extend(dedupe_app_refs(&mut model));
    model.validate()?;

    let repaired = migrated
        || settings != read.settings
        || model.groups != read.groups
        || model.items != read.items;
    Ok((
        Loaded {
            model,
            settings,
            usage,
            warnings,
        },
        repaired,
    ))
}

/// Clamps `settings` and describes each value that had to move.
fn clamp_settings(settings: &mut Settings) -> Vec<String> {
    let before = settings.clone();
    settings.clamp();
    let mut warnings = Vec::new();
    macro_rules! report {
        ($($field:ident),* $(,)?) => {
            $(if before.$field != settings.$field {
                warnings.push(format!(
                    "Setting {} was {}, outside its permitted range; it was set to {}.",
                    stringify!($field), before.$field, settings.$field
                ));
            })*
        };
    }
    report!(
        grid_columns_portrait,
        grid_columns_landscape,
        icon_size,
        label_size,
        wallpaper_alpha,
        icon_alpha,
    );
    warnings
}

/// Two items cannot refer to the same application. Keeps the first in the
/// document's item list and removes the later ones, so the user's placement and
/// label survive, then renumbers the groups it removed from. One warning per
/// removed item.
fn dedupe_app_refs(model: &mut Model) -> Vec<String> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut warnings = Vec::new();
    let mut touched: Vec<Id> = Vec::new();

    model.items.retain(|item| {
        let Some(app) = &item.app else { return true };
        if seen.insert((app.package.clone(), app.activity.clone())) {
            return true;
        }
        warnings.push(format!(
            "Removed a duplicate entry \"{}\" for {}/{}; the application is already in the launcher.",
            item.label, app.package, app.activity
        ));
        if !touched.contains(&item.group_id) {
            touched.push(item.group_id.clone());
        }
        false
    });

    for group in &touched {
        let mut positions: Vec<usize> = (0..model.items.len())
            .filter(|&i| model.items[i].group_id == *group)
            .collect();
        positions.sort_by_key(|&i| (model.items[i].order, i));
        for (order, i) in positions.into_iter().enumerate() {
            model.items[i].order = order as i32;
        }
    }
    warnings
}
