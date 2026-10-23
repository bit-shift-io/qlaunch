use std::collections::HashMap;
use std::sync::Mutex;

use crate::model::{AppEntry, AppRef, IconImage};

use super::backup::{is_backup_name, sort_backup_names};
use super::{AppSource, LaunchError, PlatformError};

pub struct FakeSource {
    apps: Vec<AppEntry>,
    icons: HashMap<AppRef, Vec<u8>>,
    icon_references: HashMap<String, Vec<u8>>,
    launched: Mutex<Vec<AppRef>>,
    fail_launch: bool,
    /// Backups the fake Downloads folder holds, keyed by backup file name.
    backups: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeSource {
    pub fn new(apps: Vec<AppEntry>) -> Self {
        Self {
            apps,
            icons: HashMap::new(),
            icon_references: HashMap::new(),
            launched: Mutex::new(Vec::new()),
            fail_launch: false,
            backups: Mutex::new(HashMap::new()),
        }
    }

    /// Puts `bytes` in the fake Downloads folder under `name`, for a test that
    /// wants to restore a backup it did not create itself.
    pub fn with_backup(self, name: &str, bytes: Vec<u8>) -> Self {
        self.backups
            .lock()
            .expect("backup map poisoned")
            .insert(name.to_string(), bytes);
        self
    }

    /// The backup names the fake Downloads folder holds, newest first.
    pub fn backups(&self) -> Vec<String> {
        let names: Vec<String> = self
            .backups
            .lock()
            .expect("backup map poisoned")
            .keys()
            .cloned()
            .collect();
        sort_backup_names(names)
    }

    pub fn with_icon(mut self, app: AppRef, png: Vec<u8>) -> Self {
        self.icons.insert(app, png);
        self
    }

    pub fn with_icon_reference(mut self, reference: &str, png: Vec<u8>) -> Self {
        self.icon_references.insert(reference.to_string(), png);
        self
    }

    pub fn with_launch_failure(mut self) -> Self {
        self.fail_launch = true;
        self
    }

    /// Returns the apps that launched successfully, in call order. A failed
    /// launch is reported through `launch`'s return value and is not recorded.
    pub fn launched(&self) -> Vec<AppRef> {
        self.launched.lock().expect("launch log poisoned").clone()
    }
}

impl AppSource for FakeSource {
    fn list_apps(&self) -> Result<Vec<AppEntry>, PlatformError> {
        Ok(self.apps.clone())
    }

    fn icon_image(&self, app: &AppRef, _size_px: u32) -> Result<Option<IconImage>, PlatformError> {
        Ok(self.icons.get(app).cloned().map(IconImage::png))
    }

    fn icon_reference(
        &self,
        reference: &str,
        _size_px: u32,
    ) -> Result<Option<Vec<u8>>, PlatformError> {
        Ok(self.icon_references.get(reference).cloned())
    }

    fn launch(&self, app: &AppRef) -> Result<(), LaunchError> {
        let listed = self
            .apps
            .iter()
            .any(|entry| entry.package == app.package && entry.activity == app.activity);
        if !listed {
            return Err(LaunchError::NotFound(app.clone()));
        }
        if self.fail_launch {
            return Err(LaunchError::Failed("configured to fail".into()));
        }
        self.launched
            .lock()
            .expect("launch log poisoned")
            .push(app.clone());
        Ok(())
    }

    fn write_backup(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        if !is_backup_name(name) {
            return Err(PlatformError::Unsupported(format!(
                "{name:?} is not a backup file name"
            )));
        }
        self.backups
            .lock()
            .expect("backup map poisoned")
            .insert(name.to_string(), bytes.to_vec());
        Ok(())
    }

    fn list_backups(&self) -> Result<Vec<String>, PlatformError> {
        Ok(self.backups())
    }

    fn read_backup(&self, name: &str) -> Result<Vec<u8>, PlatformError> {
        if !is_backup_name(name) {
            return Err(PlatformError::Unsupported(format!(
                "{name:?} is not a backup file name"
            )));
        }
        self.backups
            .lock()
            .expect("backup map poisoned")
            .get(name)
            .cloned()
            .ok_or(PlatformError::NotFound)
    }
}
