mod fake;
pub mod linux;

use thiserror::Error;

use crate::model::{AppEntry, AppRef};

pub use fake::FakeSource;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum PlatformError {
    #[error("platform I/O failed: {0}")]
    Io(String),
    #[error("the platform could not find the requested resource")]
    NotFound,
    #[error("not supported on this platform: {0}")]
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum LaunchError {
    #[error("no app {}/{} to launch", .0.package, .0.activity)]
    NotFound(AppRef),
    #[error("launch failed: {0}")]
    Failed(String),
    #[error("launching is not supported on this platform: {0}")]
    Unsupported(String),
}

pub trait AppSource: Send + Sync {
    fn list_apps(&self) -> Result<Vec<AppEntry>, PlatformError>;
    fn icon_png(&self, app: &AppRef, size_px: u32) -> Result<Option<Vec<u8>>, PlatformError>;
    fn launch(&self, app: &AppRef) -> Result<(), LaunchError>;
}
