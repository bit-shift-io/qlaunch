use serde::Deserialize;

use crate::model::{AppCategory, AppEntry, AppRef};
use crate::platform::{AppSource, LaunchError, PlatformError};

#[cfg(target_os = "android")]
pub mod jni_host;

#[derive(Debug, Clone, PartialEq)]
pub struct AndroidActivity {
    pub package: String,
    pub activity: String,
    pub label: String,
    pub is_game: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AndroidList {
    pub self_package: String,
    pub activities: Vec<AndroidActivity>,
}

pub trait AndroidHost: Send + Sync {
    fn list(&self) -> Result<AndroidList, PlatformError>;
    fn icon_png(&self, app: &AppRef, size_px: u32) -> Result<Option<Vec<u8>>, PlatformError>;
    fn launch(&self, app: &AppRef) -> Result<(), LaunchError>;
}

#[derive(Deserialize)]
struct RawList {
    #[serde(rename = "selfPackage")]
    self_package: String,
    activities: Vec<RawActivity>,
}

#[derive(Deserialize)]
struct RawActivity {
    package: String,
    activity: String,
    label: String,
    #[serde(rename = "isGame")]
    is_game: bool,
}

/// Parses the JSON `QLaunchBridge.listLaunchActivitiesJson` returns. Kept here,
/// not in the JNI module, so it is testable on the development machine.
pub fn parse_list_json(json: &str) -> Result<AndroidList, PlatformError> {
    let raw: RawList = serde_json::from_str(json)
        .map_err(|e| PlatformError::Io(format!("bad app list JSON: {e}")))?;
    Ok(AndroidList {
        self_package: raw.self_package,
        activities: raw
            .activities
            .into_iter()
            .map(|a| AndroidActivity {
                package: a.package,
                activity: a.activity,
                label: a.label,
                is_game: a.is_game,
            })
            .collect(),
    })
}

pub struct AndroidSource<H: AndroidHost> {
    host: H,
}

impl<H: AndroidHost> AndroidSource<H> {
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: AndroidHost> AppSource for AndroidSource<H> {
    fn list_apps(&self) -> Result<Vec<AppEntry>, PlatformError> {
        let list = self.host.list()?;
        Ok(list
            .activities
            .into_iter()
            // Our own package must never appear (§8.5), so a launcher cannot
            // list itself.
            .filter(|a| a.package != list.self_package)
            .map(|a| AppEntry {
                package: a.package,
                activity: a.activity,
                label: a.label,
                category: if a.is_game {
                    AppCategory::Game
                } else {
                    AppCategory::App
                },
            })
            .collect())
    }

    fn icon_png(&self, app: &AppRef, size_px: u32) -> Result<Option<Vec<u8>>, PlatformError> {
        self.host.icon_png(app, size_px)
    }

    fn launch(&self, app: &AppRef) -> Result<(), LaunchError> {
        self.host.launch(app)
    }
}
