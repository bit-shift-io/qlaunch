use std::ffi::c_void;
use std::sync::OnceLock;

use jni::objects::GlobalRef;
use jni::sys::{JNI_VERSION_1_6, jint};
use jni::{JNIEnv, JavaVM};

use crate::api::events::CoreEvent;
use crate::model::AppRef;
use crate::platform::android::{AndroidHost, AndroidList, parse_list_json};
use crate::platform::{LaunchError, PlatformError};

pub(crate) static ANDROID_VM: OnceLock<JavaVM> = OnceLock::new();
pub(crate) static BRIDGE_CLASS: OnceLock<GlobalRef> = OnceLock::new();

/// The class Rust looks up, fixed by the Kotlin package (§7.1).
const BRIDGE: &str = "bitshift/qlaunch/QLaunchBridge";

pub struct JniHost;

impl JniHost {
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

    fn icon_png(&self, app: &AppRef, size_px: u32) -> Result<Option<Vec<u8>>, PlatformError> {
        let size = size_px as i32;
        JniHost::with_env(|env| {
            let class = JniHost::class()?;
            let pkg = env.new_string(&app.package).map_err(jni_error)?;
            let act = env.new_string(&app.activity).map_err(jni_error)?;
            let result = env.call_static_method(
                class,
                "iconPng",
                "(Ljava/lang/String;Ljava/lang/String;I)[B",
                &[(&pkg).into(), (&act).into(), size.into()],
            );
            match result {
                Ok(value) => {
                    let object = value.l().map_err(jni_error)?;
                    if object.is_null() {
                        return Ok(None);
                    }
                    let arr: jni::objects::JByteArray = object.into();
                    let bytes = env.convert_byte_array(&arr).map_err(jni_error)?;
                    Ok(Some(bytes))
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
