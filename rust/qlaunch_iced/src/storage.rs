//! Reading backups in shared Downloads.
//!
//! Android attributes a `MediaStore` file to the install that wrote it, so after
//! a reflash the scoped query cannot see the backups the previous install wrote
//! without "All files access". Elsewhere Downloads is a plain directory that
//! needs no permission, so access is always granted.

/// Whether the app can list backups written by a previous install.
#[cfg(target_os = "android")]
pub(crate) fn has_all_files_access() -> bool {
    qlaunch_core::platform::android::jni_host::JniHost::has_all_files_access().unwrap_or(false)
}

#[cfg(not(target_os = "android"))]
pub(crate) fn has_all_files_access() -> bool {
    true
}

/// Opens the system "All files access" screen, or does nothing off Android.
#[cfg(target_os = "android")]
pub(crate) fn request_all_files_access() -> Result<(), String> {
    qlaunch_core::platform::android::jni_host::JniHost::request_all_files_access()
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
pub(crate) fn request_all_files_access() -> Result<(), String> {
    Ok(())
}
