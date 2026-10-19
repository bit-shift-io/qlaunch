# QLaunch Android Discovery — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `flutter run -d <android>` show the device's installed apps in the same group pages the Linux build already renders, with icons, and launch one by tapping it.

**Architecture:** The Flutter skin and the Rust core already work (Plan 1). What is missing is a second `AppSource`: `AndroidSource`, which asks the OS through `QLaunchBridge.kt` over JNI. The OS-facing calls are hidden behind an `AndroidHost` trait so the mapping/self-filter logic is unit-tested on the host, while the JNI implementation (`JniHost`) is thin and verified on the device. Rust picks the source by target (`cfg(target_os = "android")`); Dart does not change.

**Tech Stack:** Rust 1.98 (`jni` 0.21 on Android, `serde`/`serde_json`), `flutter_rust_bridge` 2.13.0, Flutter 3.47.7 / Dart 3.13.5, Kotlin (Android SDK 34), `cargokit` (already wired via `rust_builder/`).

**Spec:** `docs/superpowers/specs/2026-10-18-qlaunch-flutter-rust-design.md` (§7.3 JNI boundary, §7.4 Android host, §8.5 discovery/reconciliation).

## Global Constraints

- Android applicationId `bitshift.qlaunch`; app label `QLaunch`; `minSdk 34`, compile/target latest.
- Discovery is `queryIntentActivities(MAIN + LAUNCHER)` **minus our own package** (§8.5).
- Category: `ApplicationInfo.category == CATEGORY_GAME` or `FLAG_IS_GAME` → `AppCategory::Game`; everything else `AppCategory::App` (§8.5).
- `(package, activity)` is unique across the model; `AppRef.activity` is the Activity class name on Android.
- `AppSource` is **synchronous** (Plan 1's deviation, kept); FRB dispatches the `async` API off the Dart isolate.
- Rust resolves/owns platform access; Dart contains no platform branching and does not change in this plan.
- No network access from any code path.
- Events reach Dart through exactly one stream, `events() -> Stream<CoreEvent>`.
- `flutter_rust_bridge` crate version and the `pubspec.yaml` version stay `2.13.0`.
- Licence GPL-3.0; the root `LICENSE` is untouched.
- Reference only, do not port: `origin/java` (`PackageMgr.java`, `AppIconHelper.java`, `MainActivity.java`).

## Review Focus

The five inputs/conditions most likely to bite, each with the check pinned to the task that owns the code. Android's glue is device-verified rather than unit-tested: the "test" is the on-device observation named in the task.

1. **Package visibility (API 30+).** Without a `<queries>` entry for `MAIN`/`LAUNCHER`, `queryIntentActivities` returns an empty or partial list and the grid looks silently empty. Expected: every launcher activity the OS will report. Checked in Task 2 (on-device: the Apps page is populated).
2. **`FindClass` on a non-Java thread.** A class looked up from a thread attached by Rust uses the wrong class loader and fails; the `QLaunchBridge` class ref must be captured during `JNI_OnLoad` (the thread that called `System.loadLibrary`) and reused. Checked in Task 2 (on-device: calls made off the main thread succeed).
3. **A pending JNI exception left uncleared.** After a Kotlin exception or a failed lookup, an uncleared pending exception makes every later JNI call abort. Expected: a failed icon or launch is a normal error, never a crash. Checked in Task 3 and Task 4 (on-device: a missing icon draws a placeholder; a bad launch reports an error).
4. **Icon conversion.** Adaptive/vector/`ColorDrawable` icons must render to a bitmap without the old `(BitmapDrawable) drawable` `ClassCastException`, and a missing icon must yield `null` so the placeholder draws. Checked in Task 3.
5. **Unicode, emoji and CJK labels** from `PackageManager` must survive the JSON→Rust parse and draw without panic. Checked in Task 1 (`parse_list_json` fixture + mapping).

---

### Task 1: Android platform seam — `AndroidSource` over an `AndroidHost`

Pure Rust, no JNI. This is the layer a later task's JNI code plugs into, and the only part of Android discovery that is unit-tested on the host.

**Files:**
- Create: `rust/qlaunch_core/src/platform/android/mod.rs`
- Modify: `rust/qlaunch_core/src/platform/mod.rs`
- Create: `rust/qlaunch_core/tests/android_source.rs`

**Interfaces:**
- Consumes: `crate::platform::{AppSource, PlatformError, LaunchError}` (Plan 1), `crate::model::{AppEntry, AppRef, AppCategory}` (Plan 1).
- Produces:
  - `pub struct AndroidActivity { pub package: String, pub activity: String, pub label: String, pub is_game: bool }`
  - `pub struct AndroidList { pub self_package: String, pub activities: Vec<AndroidActivity> }`
  - `pub trait AndroidHost: Send + Sync { fn list(&self) -> Result<AndroidList, PlatformError>; fn icon_png(&self, app: &AppRef, size_px: u32) -> Result<Option<Vec<u8>>, PlatformError>; fn launch(&self, app: &AppRef) -> Result<(), LaunchError>; }`
  - `pub struct AndroidSource<H: AndroidHost> { host: H }` with `pub fn new(host: H) -> Self`
  - `pub fn parse_list_json(json: &str) -> Result<AndroidList, PlatformError>`
  - `impl<H: AndroidHost> AppSource for AndroidSource<H>`

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/android_source.rs`:

```rust
use qlaunch_core::model::{AppCategory, AppRef};
use qlaunch_core::platform::android::{
    AndroidActivity, AndroidHost, AndroidList, AndroidSource, parse_list_json,
};

struct FakeHost {
    list: AndroidList,
    icon: Option<Vec<u8>>,
}

impl AndroidHost for FakeHost {
    fn list(&self) -> Result<AndroidList, qlaunch_core::platform::PlatformError> {
        Ok(self.list.clone())
    }
    fn icon_png(
        &self,
        _app: &AppRef,
        _size_px: u32,
    ) -> Result<Option<Vec<u8>>, qlaunch_core::platform::PlatformError> {
        Ok(self.icon.clone())
    }
    fn launch(&self, _app: &AppRef) -> Result<(), qlaunch_core::platform::LaunchError> {
        Ok(())
    }
}

fn host() -> FakeHost {
    FakeHost {
        list: parse_list_json(
            r#"{"selfPackage":"bitshift.qlaunch","activities":[
                {"package":"org.mozilla.firefox","activity":"org.mozilla.firefox.App","label":"Firefox","isGame":false},
                {"package":"com.example.zombie","activity":"com.example.zombie.Main","label":"Zombie","isGame":true},
                {"package":"bitshift.qlaunch","activity":"bitshift.qlaunch.MainActivity","label":"QLaunch","isGame":false},
                {"package":"jp.example","activity":"jp.example.Main","label":"日本 ゲーム 🙂","isGame":false}
            ]}"#,
        )
        .unwrap(),
        icon: Some(vec![1, 2, 3]),
    }
}

use qlaunch_core::platform::AppSource;

#[test]
fn the_launcher_lists_games_last_and_drops_its_own_package() {
    let source = AndroidSource::new(host());
    let apps = source.list_apps().unwrap();
    let packages: Vec<_> = apps.iter().map(|a| a.package.as_str()).collect();
    assert_eq!(
        packages,
        ["org.mozilla.firefox", "com.example.zombie", "jp.example"]
    );
    assert!(!packages.contains(&"bitshift.qlaunch"));
}

#[test]
fn the_game_flag_routes_to_the_games_category_and_everything_else_to_app() {
    let source = AndroidSource::new(host());
    let apps = source.list_apps().unwrap();
    let by = |p: &str| apps.iter().find(|a| a.package == p).unwrap().category.clone();
    assert_eq!(by("com.example.zombie"), AppCategory::Game);
    assert_eq!(by("org.mozilla.firefox"), AppCategory::App);
}

#[test]
fn a_unicode_label_survives_the_parse_and_the_mapping() {
    let source = AndroidSource::new(host());
    let apps = source.list_apps().unwrap();
    let jp = apps.iter().find(|a| a.package == "jp.example").unwrap();
    assert_eq!(jp.label, "日本 ゲーム 🙂");
    assert_eq!(jp.activity, "jp.example.Main");
}

#[test]
fn icon_and_launch_delegate_to_the_host() {
    let source = AndroidSource::new(host());
    let app = AppRef { package: "org.mozilla.firefox".into(), activity: "org.mozilla.firefox.App".into() };
    assert_eq!(source.icon_png(&app, 64).unwrap(), Some(vec![1, 2, 3]));
    assert!(source.launch(&app).is_ok());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test android_source`
Expected: FAIL — `platform::android` does not exist.

- [ ] **Step 3: Implement the seam**

Add to `rust/qlaunch_core/src/platform/mod.rs`:

```rust
pub mod android;
// linux is desktop-only; its deps and APIs are not built for Android.
#[cfg(not(target_os = "android"))]
pub mod linux;
```

(`fake` stays `mod fake;` and is re-exported as before.)

`rust/qlaunch_core/src/platform/android/mod.rs`:

```rust
use serde::Deserialize;

use crate::model::{AppCategory, AppEntry, AppRef};
use crate::platform::{AppSource, LaunchError, PlatformError};

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
    let raw: RawList =
        serde_json::from_str(json).map_err(|e| PlatformError::Io(format!("bad app list JSON: {e}")))?;
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
            // Our own package must never appear (§8.5).
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
```

`AppEntry` and `AppCategory` must derive `Clone`; they already do (Plan 1). If `AppCategory` is not `Clone`, add it — it is used by value in the test.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test android_source`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core/src/platform rust/qlaunch_core/tests/android_source.rs
git commit -m "feat(android): AndroidSource over an AndroidHost seam, with the list JSON parser"
```

---

### Task 2: JNI foundation, discovery, and platform selection (on-device)

Wires Rust to the OS. Ends with the device showing its installed apps in the existing grid.

**Files:**
- Create: `rust/qlaunch_core/src/platform/android/jni_host.rs`
- Modify: `rust/qlaunch_core/src/platform/android/mod.rs` (add `#[cfg(target_os = "android")] pub mod jni_host;`)
- Modify: `rust/qlaunch_core/src/lib.rs` (declare the `JNI_OnLoad` export)
- Modify: `rust/qlaunch_core/Cargo.toml` (Android-only `jni` dependency)
- Modify: `rust/qlaunch_core/src/api/api.rs` (platform source selection)
- Create: `android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt`
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt`
- Modify: `android/app/src/main/AndroidManifest.xml`
- Modify: `android/app/build.gradle.kts`

**Interfaces:**
- Consumes: Task 1's `AndroidHost`, `AndroidList`, `AndroidSource`, `parse_list_json`.
- Produces:
  - `pub struct JniHost` implementing `AndroidHost`, constructed with `JniHost::new()`.
  - `pub(crate) static ANDROID_VM: OnceLock<jni::JavaVM>` and `pub(crate) static BRIDGE_CLASS: OnceLock<jni::objects::GlobalRef>` in `jni_host`.
  - `#[unsafe(no_mangle)] pub extern "system" fn JNI_OnLoad(...) -> jint` exported from the crate root.
  - `QLaunchBridge.kt` static surface (called by Rust): `install(Context)`, `getAppContext(): Context`, `listLaunchActivitiesJson(): String`.

- [ ] **Step 1: Add the Android-only JNI dependency**

In `rust/qlaunch_core/Cargo.toml`:

```toml
[target.'cfg(target_os = "android")'.dependencies]
jni = "0.21"
```

Run: `cargo check --manifest-path rust/Cargo.toml`
Expected: PASS on the host (the target block is inert).

- [ ] **Step 2: Write the Kotlin bridge**

`android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt` — the whole OS surface Rust may call. Methods are `@JvmStatic` so Rust finds them as static methods on `bitshift/qlaunch/QLaunchBridge`.

```kotlin
package bitshift.qlaunch

import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import org.json.JSONArray
import org.json.JSONObject

/// Everything the Rust core asks the OS for. Rust owns all data and logic;
/// this class only answers questions and performs OS actions.
object QLaunchBridge {
    private var appContext: Context? = null

    @JvmStatic
    fun install(context: Context) {
        appContext = context.applicationContext
    }

    @JvmStatic
    fun getAppContext(): Context =
        appContext ?: throw IllegalStateException("QLaunchBridge.install was not called")

    /// Every launcher activity, as the JSON shape Rust's `parse_list_json` reads.
    /// `selfPackage` lets Rust drop our own package, and is the package-name
    /// source of truth when the debug build's applicationId differs.
    @JvmStatic
    fun listLaunchActivitiesJson(): String {
        val context = getAppContext()
        val pm = context.packageManager
        val intent = Intent(Intent.ACTION_MAIN, null).addCategory(Intent.CATEGORY_LAUNCHER)
        val resolved = pm.queryIntentActivities(intent, 0)
        val activities = JSONArray()
        for (ri in resolved) {
            val ai = ri.activityInfo
            val app = ai.applicationInfo
            val isGame = (app.flags and ApplicationInfo.FLAG_IS_GAME) != 0 ||
                app.category == ApplicationInfo.CATEGORY_GAME
            activities.put(
                JSONObject()
                    .put("package", ai.packageName)
                    .put("activity", ai.name)
                    .put("label", ai.loadLabel(pm)?.toString() ?: ai.name)
                    .put("isGame", isGame)
            )
        }
        return JSONObject()
            .put("selfPackage", context.packageName)
            .put("activities", activities)
            .toString()
    }

    /// `PackageManager.ResolveInfoFlags`/`queryIntentActivities` overloads differ
    /// across API levels; the `Int` form above is valid on API 34.
    private fun unusedPm(pm: PackageManager) = pm
}
```

(Remove the unused `unusedPm` helper — it is there only to make the `PackageManager` import used if the linter complains; if `loadLabel`/`queryIntentActivities` already use it, drop both the helper and nothing else.)

- [ ] **Step 3: Load the library and install the context at startup**

`android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt`:

```kotlin
package bitshift.qlaunch

import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity

class MainActivity : FlutterActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        // Loaded before the Flutter engine starts so JNI_OnLoad runs on this
        // (app class loader) thread and Rust can capture the JVM and bridge
        // class. See §7.3.
        System.loadLibrary("qlaunch_core")
        super.onCreate(savedInstanceState)
        QLaunchBridge.install(applicationContext)
    }
}
```

- [ ] **Step 4: Capture the JVM and bridge class in `JNI_OnLoad`**

`rust/qlaunch_core/src/platform/android/jni_host.rs`:

```rust
use std::ffi::c_void;
use std::sync::OnceLock;

use jni::objects::GlobalRef;
use jni::sys::{jint, JNI_VERSION_1_6};
use jni::{JNIEnv, JavaVM};

use crate::model::AppRef;
use crate::platform::android::{AndroidList, parse_list_json};
use crate::platform::{AndroidHost, LaunchError, PlatformError};

pub(crate) static ANDROID_VM: OnceLock<JavaVM> = OnceLock::new();
pub(crate) static BRIDGE_CLASS: OnceLock<GlobalRef> = OnceLock::new();

/// The class name Rust looks up, fixed by the Kotlin package (§7.1).
const BRIDGE: &str = "bitshift/qlaunch/QLaunchBridge";

pub struct JniHost;

impl JniHost {
    pub fn new() -> Self {
        JniHost
    }

    /// Runs `f` with an attached thread. Any pending exception from `f`'s last
    /// call is cleared by the call sites; see [`JniHost::call`].
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
            let value = env
                .call_static_method(
                    class,
                    "listLaunchActivitiesJson",
                    "()Ljava/lang/String;",
                    &[],
                )
                .map_err(jni_error)?;
            let object = value.l().map_err(jni_error)?;
            let java_str = env.get_string(&object.into()).map_err(jni_error)?;
            Ok(String::from(java_str))
        })?;
        parse_list_json(&json)
    }

    fn icon_png(&self, app: &AppRef, size_px: u32) -> Result<Option<Vec<u8>>, PlatformError> {
        // Filled in by Task 3. Returning Ok(None) keeps the placeholder path.
        let _ = (app, size_px);
        Ok(None)
    }

    fn launch(&self, app: &AppRef) -> Result<(), LaunchError> {
        // Filled in by Task 4.
        let _ = app;
        Err(LaunchError::Unsupported("launch is not wired yet".into()))
    }
}

/// Maps a `jni::errors::Error` to a `PlatformError`. The `jni` crate leaves a
/// pending exception after a failed call; clear it before it aborts the next
/// call (Review Focus 3).
fn jni_error(_e: jni::errors::Error) -> PlatformError {
    PlatformError::Io("a JNI call failed".into())
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
```

Declare the module in `rust/qlaunch_core/src/platform/android/mod.rs`:

```rust
#[cfg(target_os = "android")]
pub mod jni_host;
```

And the export in `rust/qlaunch_core/src/lib.rs`:

```rust
#[cfg(target_os = "android")]
pub use crate::platform::android::jni_host::JNI_OnLoad;
```

Note: `JNI_OnLoad` must be reachable from the cdylib. `pub use` re-exports it; if the linker still drops it, mark the re-export `#[used]` or call it from a `#[no_mangle]` shim in `lib.rs`. Verify with `nm -D` on the built `.so` in Step 7.

- [ ] **Step 5: Select the source by target**

In `rust/qlaunch_core/src/api/api.rs`, replace the `use crate::platform::linux::LinuxSource;` line and the `state::install(root, Box::new(LinuxSource::new()), warnings)` call:

```rust
#[cfg(not(target_os = "android"))]
use crate::platform::linux::LinuxSource;
#[cfg(target_os = "android")]
use crate::platform::android::{AndroidSource, jni_host::JniHost};

/// The platform's `AppSource`. Rust resolves this; Dart never chooses.
fn platform_source() -> Box<dyn AppSource> {
    #[cfg(target_os = "android")]
    {
        Box::new(AndroidSource::new(JniHost::new()))
    }
    #[cfg(not(target_os = "android"))]
    {
        Box::new(LinuxSource::new())
    }
}
```

and in `init`: `state::install(root, platform_source(), warnings)`.

Run: `cargo test --manifest-path rust/Cargo.toml`
Expected: PASS — all Plan 1 tests still pass and `android_source` passes.

- [ ] **Step 6: Declare package visibility and the app label**

In `android/app/src/main/AndroidManifest.xml`, set `android:label="QLaunch"` on `<application>`, and add a `<queries>` child of `<manifest>` (sibling of `<application>`):

```xml
<!-- Required on API 30+: without this, queryIntentActivities(MAIN+LAUNCHER)
     returns an empty list and the launcher looks empty (Review Focus 1). -->
<queries>
    <intent>
        <action android:name="android.intent.action.MAIN"/>
        <category android:name="android.intent.category.LAUNCHER"/>
    </intent>
</queries>
```

In `android/app/build.gradle.kts`, set the minimum SDK to the spec's value:

```kotlin
minSdk = 34
```

- [ ] **Step 7: Verify on the device**

Run: `flutter run -d 6bfcc911` (the attached Android 14 device; use the id from `flutter devices` if it differs).

Expected, in this order:
1. The build succeeds and the app opens.
2. The **Apps** page is populated with the device's launcher apps (not empty), and the **Games** page contains at least one game if the device has one.
3. No `FATAL EXCEPTION` / `UnsatisfiedLinkError` in the log. Watch with: `adb logcat --pid=$(adb shell pidof bitshift.qlaunch)`.

If the grid is empty, check, in order: the `<queries>` block landed in the manifest; `adb shell cmd package query-activities -a android.intent.action.MAIN -c android.intent.category.LAUNCHER` returns rows; `JNI_OnLoad` ran (temporary `Log`/`println` if needed). This is the JNI foundation check (Review Focus 2).

- [ ] **Step 8: Commit**

```bash
git add rust/qlaunch_core android
git commit -m "feat(android): JNI bridge and PackageManager discovery; apps render on device"
```

---

### Task 3: Icons (on-device)

**Files:**
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt` (add `iconPng`)
- Modify: `rust/qlaunch_core/src/platform/android/jni_host.rs` (`icon_png`)

**Interfaces:**
- Consumes: Task 2's `with_env`/`class` helpers.
- Produces: `QLaunchBridge.iconPng(package: String, activity: String, sizePx: Int): ByteArray?`, and a real `JniHost::icon_png`.

- [ ] **Step 1: Add `iconPng` to `QLaunchBridge.kt`**

```kotlin
import android.content.ComponentName
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.drawable.BitmapDrawable
import android.graphics.drawable.Drawable
import java.io.ByteArrayOutputStream

// inside object QLaunchBridge:

/// The activity's icon as PNG bytes at `sizePx`, or null if it cannot be had.
/// Draws through a Canvas rather than casting to BitmapDrawable, so adaptive,
/// vector and colour icons all work (Review Focus 4).
@JvmStatic
fun iconPng(packageName: String, activity: String, sizePx: Int): ByteArray? {
    val context = getAppContext()
    return try {
        val drawable = context.packageManager
            .getActivityIcon(ComponentName(packageName, activity))
        val bitmap = drawableToBitmap(drawable, sizePx)
        ByteArrayOutputStream().use { out ->
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, out)
            out.toByteArray()
        }
    } catch (e: Exception) {
        null
    }
}

private fun drawableToBitmap(drawable: Drawable, sizePx: Int): Bitmap {
    if (drawable is BitmapDrawable && drawable.bitmap != null) {
        return drawable.bitmap
    }
    val width = if (drawable.intrinsicWidth > 0) drawable.intrinsicWidth else sizePx.coerceAtLeast(1)
    val height = if (drawable.intrinsicHeight > 0) drawable.intrinsicHeight else sizePx.coerceAtLeast(1)
    val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
    val canvas = Canvas(bitmap)
    drawable.setBounds(0, 0, canvas.width, canvas.height)
    drawable.draw(canvas)
    return bitmap
}
```

- [ ] **Step 2: Implement `JniHost::icon_png`**

Make `with_env` error mapping clear the pending exception (Review Focus 3). Add a helper and use it here:

```rust
fn clear_pending(env: &mut JNIEnv) {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}
```

```rust
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
                let bytes = env.convert_byte_array(object.as_obj().into_raw()).map_err(jni_error)?;
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
```

Run: `cargo test --manifest-path rust/Cargo.toml` and `cargo build --manifest-path rust/Cargo.toml`
Expected: PASS / builds on the host (the Android module is not compiled on the host, so this proves the non-Android build is unaffected). The Android compilation is proven by Step 3.

- [ ] **Step 3: Verify on the device**

Run: `flutter run -d 6bfcc911`
Expected: each cell shows the app's real icon; an app with no resolvable icon shows the placeholder rather than a blank cell or a crash (Review Focus 4). Check the log for `ClassCastException`.

- [ ] **Step 4: Commit**

```bash
git add rust/qlaunch_core android
git commit -m "feat(android): package icons rendered to PNG over JNI"
```

---

### Task 4: Launching (on-device)

**Files:**
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt` (add `launch`)
- Modify: `rust/qlaunch_core/src/platform/android/jni_host.rs` (`launch`)

**Interfaces:**
- Consumes: Task 2's helpers.
- Produces: `QLaunchBridge.launch(package: String, activity: String)`, and a real `JniHost::launch`.

- [ ] **Step 1: Add `launch` to `QLaunchBridge.kt`**

```kotlin
import android.content.ComponentName

// inside object QLaunchBridge:

/// Starts the activity. Throws if the component is gone, so Rust can report it.
@JvmStatic
fun launch(packageName: String, activity: String) {
    val context = getAppContext()
    val intent = Intent(Intent.ACTION_MAIN).apply {
        addCategory(Intent.CATEGORY_LAUNCHER)
        component = ComponentName(packageName, activity)
        flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED
    }
    context.startActivity(intent)
}
```

- [ ] **Step 2: Implement `JniHost::launch`**

```rust
fn launch(&self, app: &AppRef) -> Result<(), LaunchError> {
    JniHost::with_env(|env| {
        let class = JniHost::class().map_err(platform_to_launch)?;
        let pkg = env.new_string(&app.package).map_err(jni_launch_err)?;
        let act = env.new_string(&app.activity).map_err(jni_launch_err)?;
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
                let _ = e;
                Err(LaunchError::Failed(format!("{}/{}", app.package, app.activity)))
            }
        }
    })
}
```

with the small adapters:

```rust
fn platform_to_launch(e: PlatformError) -> LaunchError {
    LaunchError::Failed(e.to_string())
}
fn jni_launch_err(e: jni::errors::Error) -> LaunchError {
    LaunchError::Failed(format!("a JNI call failed: {e}"))
}
```

Run: `cargo test --manifest-path rust/Cargo.toml`
Expected: PASS.

- [ ] **Step 3: Verify on the device**

Run: `flutter run -d 6bfcc911`
Expected: tapping an app cell starts that app. Returning to QLaunch and tapping a different app starts that one. A component that no longer exists reports an error through `CoreEvent::Error` and the pending exception is cleared, so later taps still work (Review Focus 3).

- [ ] **Step 4: Commit**

```bash
git add rust/qlaunch_core android
git commit -m "feat(android): launch activities over JNI"
```

---

### Task 5: Finish — docs, full suites, and the release smoke test

**Files:**
- Modify: `README.md` (Status)
- Modify: `implementation_roadmap.md` (tick the Phase 3 tasks this plan completes)

**Interfaces:**
- Consumes: everything above.
- Produces: no new interface.

- [ ] **Step 1: Build the release library the smoke test loads**

Run: `cargo build --release --manifest-path rust/Cargo.toml`
Expected: builds `rust/target/release/libqlaunch_core.so` (the path `test/core_smoke_test.dart` loads).

- [ ] **Step 2: Run the full Rust suite**

Run: `cargo test --manifest-path rust/Cargo.toml`
Expected: PASS — every Plan 1 suite plus `android_source`.

- [ ] **Step 3: Run the full Flutter suite**

Run: `flutter test`
Expected: PASS — including `test/core_smoke_test.dart`, which passes only once Step 1 has produced the release library.

- [ ] **Step 4: Update the status documents**

In `README.md`, change the Status line: Android and Linux both discover and launch apps; Android renders on device. In `implementation_roadmap.md`, tick Task 3.1 only if this plan added the HOME intent filters (it does **not** — that is the next plan), and leave the rest as they are. Note in the commit message what the device run looked like.

- [ ] **Step 5: Commit**

```bash
git add README.md implementation_roadmap.md
git commit -m "docs: Android discovery milestone"
```

---

## Self-Review

**Spec coverage.** §7.3 startup handshake → Task 2 (loadLibrary before `super.onCreate`, `JNI_OnLoad` captures the VM and class ref, `getAppContext`). §7.3 Kotlin surface → `listLaunchActivities`, `iconPng`, `launch` in Tasks 2–4 (the remaining methods are Android features not in this milestone). §7.4 host details → the HOME intent filters, `launchMode`, debug applicationId and the package receiver are the **next** plan; only the label and `minSdk` are set here. §8.5 discovery and category → Task 1 (self-filter, category) and Task 2 (the query). §8.7 config location — deferred to the next plan: this milestone uses `store::config_root`, which on Android currently falls back through `HOME`/`dirs`; a JNI `getFilesDir` resolution is a known gap, recorded here for the next plan. §9 testing → Task 1 unit tests, Tasks 2–4 on-device, Task 5 suites.

**Step scan.** Each step writes one test, adds one method, or runs one command. The one algorithm the signatures do not determine (the icon draw through `Canvas`) is given as a body. `JniHost::icon_png`'s byte-array conversion uses the `jni` 0.21 API, which the implementer should adjust to the exact version's method names (`convert_byte_array`, `as_obj`); the shape is what matters.

**Type consistency.** `AndroidActivity`/`AndroidList`/`AndroidHost`/`AndroidSource`/`parse_list_json` are defined once in Task 1 and consumed unchanged in Task 2. `JniHost` implements the Task 1 trait with `new()`, and `platform_source()` constructs it. `AppCategory`, `AppEntry`, `AppRef` are Plan 1 types. The JSON keys (`selfPackage`, `activities`, `package`, `activity`, `label`, `isGame`) match between `QLaunchBridge.kt` and `parse_list_json`.

**Review Focus.** Each of the five lines names the task and the check that pins it: 5→Task 1, 1 and 2→Task 2, 4→Task 3, 3→Tasks 3 and 4.

**Proportion.** Five tasks; only Task 1 carries real code, the rest are thin adapters plus a verification step. The plan is shorter than the spec it implements.

**Deliberate deviations, flagged.** (1) `listLaunchActivitiesJson` returns JSON rather than a structured JNI object: it keeps the marshalling surface to a string and makes the parse unit-testable (`parse_list_json`), at the cost of a serialise/parse hop per refresh. (2) `icon_png` is added to `JniHost` in Task 3 and `launch` in Task 4, each starting as a stub, so the trait is satisfied from Task 2 without stalling device verification on the whole surface.
