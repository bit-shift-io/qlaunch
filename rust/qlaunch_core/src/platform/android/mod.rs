use serde::Deserialize;

use crate::model::{AppCategory, AppEntry, AppRef, IconFormat, IconImage};
use crate::platform::{AppSource, LaunchError, PlatformError};

#[cfg(target_os = "android")]
pub mod jni_host;

#[derive(Debug, Clone, PartialEq)]
pub struct AndroidActivity {
    pub package: String,
    pub activity: String,
    pub label: String,
    pub is_game: bool,
    pub last_update_time: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AndroidList {
    pub self_package: String,
    pub activities: Vec<AndroidActivity>,
}

pub trait AndroidHost: Send + Sync {
    fn list(&self) -> Result<AndroidList, PlatformError>;
    fn icon_image(&self, app: &AppRef, size_px: u32) -> Result<Option<IconImage>, PlatformError>;
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
    #[serde(default, rename = "lastUpdateTime")]
    last_update_time: i64,
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
                last_update_time: a.last_update_time,
            })
            .collect(),
    })
}

/// Converts Android's packed `0xAARRGGBB` pixels, as `Bitmap.getPixels` returns
/// them, into the tightly packed RGBA bytes the bridge sends to Dart.
pub fn argb_pixels_to_rgba(pixels: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() * 4);
    for &argb in pixels {
        out.push(((argb >> 16) & 0xFF) as u8); // red
        out.push(((argb >> 8) & 0xFF) as u8); // green
        out.push((argb & 0xFF) as u8); // blue
        out.push(((argb >> 24) & 0xFF) as u8); // alpha
    }
    out
}

/// Turns the `[width, height, argb...]` array `QLaunchBridge.iconRgba` returns
/// into an [`IconImage`], or `None` when its shape is not exactly what the two
/// leading slots claim. Pure, so it is testable off-device.
pub fn icon_from_int_slots(slots: &[i32]) -> Option<IconImage> {
    if slots.len() < 2 {
        return None;
    }
    let (size, pixels) = slots.split_at(2);
    let (width, height) = (size[0], size[1]);
    if width <= 0 || height <= 0 {
        return None;
    }
    let width = width as u32;
    let height = height as u32;
    let count = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?;
    if pixels.len() != count {
        return None;
    }
    let pixels: Vec<u32> = pixels.iter().map(|&p| p as u32).collect();
    Some(IconImage {
        format: IconFormat::Rgba,
        width,
        height,
        bytes: argb_pixels_to_rgba(&pixels),
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
                last_update_time: a.last_update_time,
            })
            .collect())
    }

    fn icon_image(&self, app: &AppRef, size_px: u32) -> Result<Option<IconImage>, PlatformError> {
        self.host.icon_image(app, size_px)
    }

    fn launch(&self, app: &AppRef) -> Result<(), LaunchError> {
        self.host.launch(app)
    }
}
