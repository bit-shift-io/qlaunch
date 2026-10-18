mod atomic;
mod migrate;

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::model::{Group, Id, Item, Model, ModelError};
use crate::settings::Settings;

pub const SCHEMA_VERSION: u32 = 1;

const DOCUMENT_NAME: &str = "qlaunch.json";
const QUARANTINE_NAME: &str = "qlaunch.json.corrupt";
const ICONS_DIR: &str = "icons";

/// The whole of what is stored on disk: one versioned JSON document.
///
/// `settings` may be absent (it then takes the defaults); every other field is
/// required, since a document without them is damaged, not merely old.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub schema: u32,
    #[serde(default)]
    pub settings: Settings,
    pub root_group_id: Id,
    pub groups: Vec<Group>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub model: Model,
    pub settings: Settings,
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
}

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Store { root }
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

    /// Reads the document. When there is none, writes and returns the defaults.
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
    /// `warnings`. A document that cannot be *read* (permissions, not a file)
    /// is an error and is left alone.
    ///
    /// Settings are clamped on the way in. If the document that results differs
    /// from the one that was read (settings clamped, duplicate `AppRef`s
    /// removed, or the schema migrated), the repaired document is written back
    /// at once, so each warning is reported once. A clean document is not
    /// rewritten. If that write fails the repaired model is still returned,
    /// with a further warning.
    pub fn load(&self) -> Result<Loaded, StoreError> {
        let bytes = match fs::read(self.document_path()) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return self.write_defaults(Vec::new());
            }
            Err(e) => return Err(e.into()),
        };
        match interpret(&bytes) {
            Ok((mut loaded, repaired)) => {
                if repaired && let Err(e) = self.save(&loaded.model, &loaded.settings) {
                    loaded.warnings.push(format!(
                        "The repaired layout could not be saved ({e}); it will be repaired again next time."
                    ));
                }
                Ok(loaded)
            }
            Err(reason) => {
                let kept = self.quarantine()?;
                let warning = format!(
                    "The saved layout could not be used ({reason}). It was kept as {} and a fresh layout was created.",
                    kept.display()
                );
                self.write_defaults(vec![warning])
            }
        }
    }

    /// Writes the document atomically. A model that [`Model::validate`] rejects
    /// is refused, since `load` would only quarantine it again.
    pub fn save(&self, model: &Model, settings: &Settings) -> Result<(), StoreError> {
        model.validate()?;
        let document = Document {
            schema: SCHEMA_VERSION,
            settings: settings.clone(),
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
    /// most likely to hold the user's real setup.
    pub fn quarantine(&self) -> Result<PathBuf, StoreError> {
        let mut to = self.root.join(QUARANTINE_NAME);
        let mut n = 0u32;
        while path_taken(&to)? {
            n += 1;
            to = self.root.join(format!("{QUARANTINE_NAME}.{n}"));
        }
        fs::rename(self.document_path(), &to)?;
        atomic::sync_dir(&self.root)?;
        Ok(to)
    }

    fn write_defaults(&self, warnings: Vec<String>) -> Result<Loaded, StoreError> {
        let model = Model::new_default();
        let settings = Settings::default();
        self.save(&model, &settings)?;
        Ok(Loaded {
            model,
            settings,
            warnings,
        })
    }
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
