use std::io;
use std::path::{Path, PathBuf};

use super::StoreError;

const VENDOR_DIR: &str = "bitshift";
const APP_DIR: &str = "qlaunch";

/// Where the store lives, from the values of the environment that decide it.
/// Reads nothing itself, so it can be tested without touching the process
/// environment.
///
/// `xdg_config_home` is used only when it is an absolute path: the XDG Base
/// Directory specification requires that, and an empty or relative value would
/// put the configuration wherever the launcher happened to be started. Such a
/// value is ignored in favour of `<home>/.config`. With neither, there is no
/// place to keep anything and the result is an error.
pub fn config_root_from(
    xdg_config_home: Option<&Path>,
    home: Option<&Path>,
) -> Result<PathBuf, StoreError> {
    let base = match xdg_config_home.filter(|p| p.is_absolute()) {
        Some(xdg) => xdg.to_path_buf(),
        None => home
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "neither XDG_CONFIG_HOME nor a home directory is available",
                )
            })?
            .join(".config"),
    };
    Ok(base.join(VENDOR_DIR).join(APP_DIR))
}

/// The store directory for this user: `$XDG_CONFIG_HOME/bitshift/qlaunch`, or
/// `~/.config/bitshift/qlaunch`. See [`config_root_from`] for the rules.
pub fn config_root() -> Result<PathBuf, StoreError> {
    let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    config_root_from(xdg.as_deref(), dirs::home_dir().as_deref())
}
