use std::collections::HashMap;
use std::sync::Mutex;

use crate::model::{AppEntry, AppRef};

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

    pub fn launched(&self) -> Vec<AppRef> {
        self.launched.lock().expect("launch log poisoned").clone()
    }
}

impl AppSource for FakeSource {
    fn list_apps(&self) -> Result<Vec<AppEntry>, PlatformError> {
        Ok(self.apps.clone())
    }

    fn icon_png(&self, app: &AppRef, _size_px: u32) -> Result<Option<Vec<u8>>, PlatformError> {
        Ok(self.icons.get(app).cloned())
    }

    fn launch(&self, app: &AppRef) -> Result<(), LaunchError> {
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
