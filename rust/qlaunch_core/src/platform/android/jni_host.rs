use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::OnceLock;

use jni::objects::GlobalRef;
use jni::sys::{JNI_VERSION_1_6, jint};
use jni::{JNIEnv, JavaVM};

use crate::api::events::CoreEvent;
use crate::model::{AppRef, IconImage};
use crate::platform::android::{
    AndroidHost, AndroidList, icon_from_int_slots, parse_backup_names_json, parse_list_json,
};
use crate::platform::{LaunchError, PlatformError};

pub(crate) static ANDROID_VM: OnceLock<JavaVM> = OnceLock::new();
pub(crate) static BRIDGE_CLASS: OnceLock<GlobalRef> = OnceLock::new();

/// The class Rust looks up, fixed by the Kotlin package (§7.1).
const BRIDGE: &str = "bitshift/qlaunch/QLaunchBridge";

pub struct JniHost;

impl JniHost {
    pub fn call_void(method: &str, strings: &[String]) -> Result<(), String> {
        Self::with_env(|env| {
            let values = strings
                .iter()
                .map(|s| env.new_string(s).map_err(jni_error))
                .collect::<Result<Vec<_>, _>>()?;
            let args: Vec<jni::objects::JValue> = values.iter().map(|s| s.into()).collect();
            let signature = format!("({})V", "Ljava/lang/String;".repeat(strings.len()));
            match env.call_static_method(Self::class()?, method, &signature, &args) {
                Ok(_) => Ok(()),
                Err(e) => {
                    clear_pending(env);
                    Err(jni_error(e))
                }
            }
        })
        .map_err(|e| e.to_string())
    }

    pub fn call_string(method: &str) -> Result<String, String> {
        Self::with_env(|env| {
            let result =
                env.call_static_method(Self::class()?, method, "()Ljava/lang/String;", &[]);
            match result {
                Ok(value) => {
                    let string: jni::objects::JString = value.l().map_err(jni_error)?.into();
                    let out: String = env.get_string(&string).map_err(jni_error)?.into();
                    Ok(out)
                }
                Err(e) => {
                    clear_pending(env);
                    Err(jni_error(e))
                }
            }
        })
        .map_err(|e| e.to_string())
    }

    pub fn call_string_arg(method: &str, arg: &str) -> Result<String, String> {
        Self::with_env(|env| {
            let value = env.new_string(arg).map_err(jni_error)?;
            let result = env.call_static_method(
                Self::class()?,
                method,
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[(&value).into()],
            );
            match result {
                Ok(value) => {
                    let string: jni::objects::JString = value.l().map_err(jni_error)?.into();
                    let out: String = env.get_string(&string).map_err(jni_error)?.into();
                    Ok(out)
                }
                Err(e) => {
                    clear_pending(env);
                    Err(jni_error(e))
                }
            }
        })
        .map_err(|e| e.to_string())
    }

    pub fn new() -> Self {
        JniHost
    }

    /// Runs `f` on a thread attached to the JVM.
    fn with_env<T>(
        f: impl FnOnce(&mut JNIEnv) -> Result<T, PlatformError>,
    ) -> Result<T, PlatformError> {
        let vm = ANDROID_VM
            .get()
            .ok_or_else(|| PlatformError::Unsupported("the JVM is not available".into()))?;
        let mut env = vm
            .attach_current_thread()
            .map_err(|e| PlatformError::Io(format!("attach failed: {e}")))?;
        f(&mut env)
    }

    fn class() -> Result<&'static GlobalRef, PlatformError> {
        BRIDGE_CLASS
            .get()
            .ok_or_else(|| PlatformError::Unsupported("the bridge class was not captured".into()))
    }

    /// The app's private files directory (`Context.filesDir`), where the store
    /// keeps `qlaunch.json`.
    pub fn files_dir() -> Result<PathBuf, PlatformError> {
        let path: String = JniHost::with_env(|env| {
            let class = JniHost::class()?;
            match env.call_static_method(class, "filesDirPath", "()Ljava/lang/String;", &[]) {
                Ok(value) => {
                    let object = value.l().map_err(|e| {
                        clear_pending(env);
                        jni_error(e)
                    })?;
                    let java_str: jni::objects::JString = object.into();
                    env.get_string(&java_str).map_err(jni_error).map(Into::into)
                }
                Err(e) => {
                    clear_pending(env);
                    Err(jni_error(e))
                }
            }
        })?;
        Ok(PathBuf::from(path))
    }
}

impl Default for JniHost {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidHost for JniHost {
    fn list(&self) -> Result<AndroidList, PlatformError> {
        let json = JniHost::with_env(|env| {
            let class = JniHost::class()?;
            match env.call_static_method(
                class,
                "listLaunchActivitiesJson",
                "()Ljava/lang/String;",
                &[],
            ) {
                Ok(value) => {
                    let object = value.l().map_err(|e| {
                        clear_pending(env);
                        jni_error(e)
                    })?;
                    let java_str: jni::objects::JString = object.into();
                    let out: String = env.get_string(&java_str).map_err(jni_error)?.into();
                    Ok(out)
                }
                Err(e) => {
                    clear_pending(env);
                    Err(jni_error(e))
                }
            }
        })?;
        parse_list_json(&json)
    }

    fn icon_image(&self, app: &AppRef, size_px: u32) -> Result<Option<IconImage>, PlatformError> {
        let size = size_px as i32;
        JniHost::with_env(|env| {
            let class = JniHost::class()?;
            let pkg = env.new_string(&app.package).map_err(jni_error)?;
            let act = env.new_string(&app.activity).map_err(jni_error)?;
            let result = env.call_static_method(
                class,
                "iconRgba",
                "(Ljava/lang/String;Ljava/lang/String;I)[I",
                &[(&pkg).into(), (&act).into(), size.into()],
            );
            match result {
                Ok(value) => {
                    let object = match value.l() {
                        Ok(object) => object,
                        Err(e) => {
                            clear_pending(env);
                            let _ = e;
                            return Ok(None);
                        }
                    };
                    if object.is_null() {
                        return Ok(None);
                    }
                    let arr: jni::objects::JIntArray = object.into();
                    let len = match env.get_array_length(&arr) {
                        Ok(len) if len >= 2 => len as usize,
                        Ok(_) => return Ok(None),
                        Err(e) => {
                            clear_pending(env);
                            let _ = e;
                            return Ok(None);
                        }
                    };
                    let mut raw = vec![0i32; len];
                    if let Err(e) = env.get_int_array_region(&arr, 0, &mut raw) {
                        clear_pending(env);
                        let _ = e;
                        return Ok(None);
                    }
                    Ok(icon_from_int_slots(&raw))
                }
                Err(e) => {
                    clear_pending(env);
                    let _ = e;
                    Ok(None)
                }
            }
        })
    }

    fn launch(&self, app: &AppRef) -> Result<(), LaunchError> {
        JniHost::with_env(|env| {
            let class = JniHost::class()?;
            let pkg = env.new_string(&app.package).map_err(jni_error)?;
            let act = env.new_string(&app.activity).map_err(jni_error)?;
            let result = env.call_static_method(
                class,
                "launch",
                "(Ljava/lang/String;Ljava/lang/String;)V",
                &[(&pkg).into(), (&act).into()],
            );
            match result {
                Ok(_) => Ok(()),
                Err(e) => {
                    clear_pending(env);
                    Err(jni_error(e))
                }
            }
        })
        .map_err(platform_to_launch)
    }

    fn write_backup(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        let body = String::from_utf8_lossy(bytes).into_owned();
        JniHost::call_void("writeBackup", &[name.to_string(), body]).map_err(PlatformError::Io)
    }

    fn list_backups(&self) -> Result<Vec<String>, PlatformError> {
        let json = JniHost::call_string("listBackups").map_err(PlatformError::Io)?;
        parse_backup_names_json(&json)
    }

    fn read_backup(&self, name: &str) -> Result<Vec<u8>, PlatformError> {
        let body = JniHost::call_string_arg("readBackup", name).map_err(PlatformError::Io)?;
        Ok(body.into_bytes())
    }
}

impl JniHost {
    /// Opens the system wallpaper chooser. There is no result; the change comes
    /// back as [`Java_bitshift_qlaunch_MainActivity_onWallpaperChanged`].
    pub fn choose_wallpaper() -> Result<(), PlatformError> {
        JniHost::with_env(|env| {
            let class = JniHost::class()?;
            match env.call_static_method(class, "chooseWallpaper", "()V", &[]) {
                Ok(_) => Ok(()),
                Err(e) => {
                    clear_pending(env);
                    Err(jni_error(e))
                }
            }
        })
    }

    /// Opens the system's uninstall confirmation for `package`.
    pub fn uninstall(package: &str) -> Result<(), PlatformError> {
        JniHost::with_env(|env| {
            let class = JniHost::class()?;
            let pkg = env.new_string(package).map_err(jni_error)?;
            match env.call_static_method(
                class,
                "uninstall",
                "(Ljava/lang/String;)V",
                &[(&pkg).into()],
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    clear_pending(env);
                    Err(jni_error(e))
                }
            }
        })
    }
}

/// Clears a pending exception left by a failed JNI call. Without this, the
/// pending exception aborts every later call (Review Focus 3).
pub(crate) fn clear_pending(env: &mut JNIEnv) {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}

fn jni_error(e: jni::errors::Error) -> PlatformError {
    PlatformError::Io(format!("a JNI call failed: {e}"))
}

fn platform_to_launch(e: PlatformError) -> LaunchError {
    LaunchError::Failed(e.to_string())
}

/// Exported: the JVM calls this when Kotlin does `System.loadLibrary`.
#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: *mut jni::sys::JavaVM, _reserved: *mut c_void) -> jint {
    let vm = match unsafe { JavaVM::from_raw(vm) } {
        Ok(vm) => vm,
        Err(_) => return JNI_VERSION_1_6,
    };
    if let Ok(mut env) = vm.get_env() {
        // This thread's class loader is the app's, so FindClass works here and
        // only here (Review Focus 2). Cache the class for later threads.
        if let Ok(class) = env.find_class(BRIDGE) {
            if let Ok(global) = env.new_global_ref(class) {
                let _ = BRIDGE_CLASS.set(global);
            }
        }
    }
    let _ = ANDROID_VM.set(vm);
    JNI_VERSION_1_6
}

/// The Home button, from `MainActivity.onNewIntent`. The event is emitted
/// rather than acted on here, so Dart decides what Home means.
#[unsafe(no_mangle)]
pub extern "system" fn Java_bitshift_qlaunch_MainActivity_onHomePressed(
    _env: JNIEnv,
    _this: jni::objects::JObject,
) {
    crate::api::state::emit(CoreEvent::HomePressed);
}

/// The Back button, from `MainActivity.onBackPressed`. Dart ignores it for
/// now; emitting it keeps the host's whole surface in one place.
#[unsafe(no_mangle)]
pub extern "system" fn Java_bitshift_qlaunch_MainActivity_onBackPressedEvent(
    _env: JNIEnv,
    _this: jni::objects::JObject,
) {
    crate::api::state::emit(CoreEvent::BackPressed);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_bitshift_qlaunch_MainActivity_onShortcutResult(
    mut env: JNIEnv,
    _this: jni::objects::JObject,
    group: jni::objects::JString,
    json: jni::objects::JString,
    error: jni::objects::JString,
) {
    let mut read = |s: &jni::objects::JString| -> String {
        env.get_string(s).map(Into::into).unwrap_or_default()
    };
    let group = read(&group);
    let json = read(&json);
    let error = read(&error);
    if !error.is_empty() {
        crate::api::state::emit(CoreEvent::Error(error));
    } else if json.is_empty() {
        crate::api::state::emit(CoreEvent::ShortcutCancelled);
    } else if let Err(error) = crate::api::shortcuts::accept_result(&group, &json) {
        crate::api::state::emit(CoreEvent::Error(error));
    }
}

/// The OS telling us an app was installed, removed or changed, from
/// `PackageChangeReceiver`. Refresh now; the core emits `AppsChanged` and Dart
/// refetches its snapshot. A failure (the core not yet started, say) is ignored:
/// the next `init` reconciles anyway.
#[unsafe(no_mangle)]
pub extern "system" fn Java_bitshift_qlaunch_PackageChangeReceiver_onPackagesChanged(
    _env: JNIEnv,
    _this: jni::objects::JObject,
) {
    let _ = crate::api::state::refresh_apps_now();
}
