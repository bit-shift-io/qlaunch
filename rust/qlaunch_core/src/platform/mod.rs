pub mod android;
pub mod backup;
mod fake;
// linux is desktop-only; its dependencies and APIs are not built for Android.
#[cfg(not(target_os = "android"))]
pub mod linux;

use thiserror::Error;

use crate::model::{AppEntry, AppRef, IconImage};

pub use backup::{backup_name, is_backup_name, sort_backup_names};
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
    fn icon_image(&self, app: &AppRef, size_px: u32) -> Result<Option<IconImage>, PlatformError>;
    fn launch(&self, app: &AppRef) -> Result<(), LaunchError>;

    /// The PNG bytes for an icon named by a platform reference, such as an
    /// Android resource. Used when a shortcut's icon is not stored as a file.
    fn icon_reference(
        &self,
        reference: &str,
        size_px: u32,
    ) -> Result<Option<Vec<u8>>, PlatformError> {
        let _ = (reference, size_px);
        Err(PlatformError::Unsupported(
            "icon references are not supported on this platform".into(),
        ))
    }

    /// Calls `on_change` after the installed apps change, once per burst of
    /// changes, until the returned handle is dropped. The callback only says
    /// "something changed": the caller re-reads `list_apps`.
    fn watch(&self, on_change: Box<dyn Fn() + Send + Sync>) -> Result<WatchHandle, PlatformError> {
        let _ = on_change;
        Err(PlatformError::Unsupported(
            "watching is not supported on this platform".into(),
        ))
    }

    /// Writes `bytes` as the backup `name` in the platform's Downloads folder.
    ///
    /// `name` always comes from [`backup::backup_name`]; an implementation must
    /// reject anything that is not a backup name rather than trust it as a path.
    fn write_backup(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        let _ = (name, bytes);
        Err(PlatformError::Unsupported(
            "config backups are not supported on this platform".into(),
        ))
    }

    /// The backup names in the platform's Downloads folder, newest first.
    fn list_backups(&self) -> Result<Vec<String>, PlatformError> {
        Err(PlatformError::Unsupported(
            "config backups are not supported on this platform".into(),
        ))
    }

    /// The bytes of the backup `name` from the platform's Downloads folder.
    /// `name` is checked as in [`AppSource::write_backup`].
    fn read_backup(&self, name: &str) -> Result<Vec<u8>, PlatformError> {
        let _ = name;
        Err(PlatformError::Unsupported(
            "config backups are not supported on this platform".into(),
        ))
    }
}

/// Keeps a [`AppSource::watch`] running until it is dropped.
///
/// The handle runs `stop` exactly once, when dropped. `stop` must not panic
/// and must not block indefinitely, and once it returns the callback must not
/// be called again.
pub struct WatchHandle {
    stop: Box<dyn Fn() + Send + Sync>,
}

impl WatchHandle {
    /// The handle runs `stop` exactly once, when it is dropped.
    pub fn new(stop: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            stop: Box::new(stop),
        }
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        (self.stop)();
    }
}
