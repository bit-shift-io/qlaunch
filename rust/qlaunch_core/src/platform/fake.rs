use std::collections::HashMap;
use std::sync::Mutex;

use crate::model::{AppEntry, AppRef, IconImage};

use super::{AppSource, LaunchError, PlatformError};

pub struct FakeSource {
    apps: Vec<AppEntry>,
    icons: HashMap<AppRef, Vec<u8>>,
    launched: Mutex<Vec<AppRef>>,
    fail_launch: bool,
}

impl FakeSource {
    pub fn new(apps: Vec<AppEntry>) -> Self {
        Self {
            apps,
            icons: HashMap::new(),
            launched: Mutex::new(Vec::new()),
            fail_launch: false,
        }
    }

    pub fn with_icon(mut self, app: AppRef, png: Vec<u8>) -> Self {
        self.icons.insert(app, png);
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
}
