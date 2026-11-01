# Instructions for AI Coding Agent

## System Role & Objective

You are an expert cross-platform systems software engineer specializing in **iced (Rust)**, **Android OS internals**, and **Linux Wayland architectures**. Your objective is to assist in constructing a high-performance, cross-platform Application Launcher designed for Android and Linux Wayland devices — a pure-iced build with no Flutter, Dart, or `flutter_rust_bridge` in the dependency tree.

---

## Core Guidelines & Principles

### 1. UI is pure iced

- **Never** import Flutter/Dart code, `flutter_rust_bridge`, or reference `qlaunch_iced`/`qlaunch-iced`.
- All UI rendering, layout, gestures and rendering live in iced primitives (`Column`, `Row`, `Container`, `scrollable`, etc.).
- The iced `qlaunch` binary produces the Home grid, groups, search, and tiles.

### 2. Core logic lives in `qlaunch_core`

- App discovery, search, config, launch and all shared data structures (`AppEntry`) are defined and tested in `qlaunch_core`.
- Rust-only: no Dart-side business logic.
- All platform bridging goes through Kotlin/JNI (Android) or native desktop interfaces (Linux).

### 3. Android OS adapter: Kotlin/JNI

- The `bitshift.qlaunch` application ID is the package name used throughout.
- `MainActivity` loads `libqlaunch.so` (the renamed `libqlaunch_iced.so`).
- `PackageChangeReceiver` receives package broadcasts and calls into the native glue.
- The JNI symbol `Java_bitshift_qlaunch_PackageChangeReceiver_onPackagesChanged` maps to the core's package monitor.
- Always load `"qlaunch"` as the library name (previously `"qlaunch_core"` — see `PackageChangeReceiver.kt` change).

### 4. Linux: Wayland and freedesktop

- Use `zwlr_layer_shell_v1` for layer positioning and anchoring.
- `.desktop` file scanning via `walkdir` or `freedesktop_entry_parser`.
- GTK icon theme resolution for unknown icons.

### 5. Config schema and saved state

- The saved config JSON schema is unchanged from the Flutter baseline.
- Config is read/written from `android_app/src/main/assets/launcher_config.json`.
- On Linux, it is `~/.config/qlaunch/config.json`.

### 6. Known constraints

- No blurred label shadow (iced uses a sharp 1 dp offset shadow instead).
- Icons render at exactly `icon_size` (no Flutter tile cache; icons are atlas quads).
- Scroll fade uses 8-level bands at 32 dp instead of Flutter's gradient ramp.
- Accessibility exposure is off by default; TalkBack cannot currently drive the launcher.

---

## Initial Context Snippet for Code Generation

When writing backend code for application discovery, always target the unified `AppEntry` struct:

```rust
// rust/src/api/launcher.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,          // Package name or .desktop filename
    pub name: String,        // Human-readable title
    pub icon_bytes: Vec<u8>, // Raw byte buffer of PNG/SVG image data
    pub categories: Vec<String>,
}

pub async fn get_installed_apps() -> Vec<AppEntry> {
    #[cfg(target_os = "linux")]
    {
        crate::platform::linux::scan_desktop_files()
    }
    #[cfg(target_os = "android")]
    {
        crate::platform::android::get_android_apps()
    }
}

pub async fn launch_app(app_id: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        crate::platform::linux::execute_process(&app_id)
    }
    #[cfg(target_os = "android")]
    {
        crate::platform::android::start_intent(&app_id)
    }
}
```

---

## Recommended Tools & Crates

- **Rust Crates:** `iced`, `iced_android`, `android-activity` (pinned), `walkdir`, `image`, `freedesktop-entry-parser` (optional)
- **Android:** Kotlin with `System.loadLibrary("qlaunch")`, JNI calls for PackageManager, Intents, PackageChangeReceiver
- **Linux:** Wayland layer-shell protocol (`zwlr_layer_shell_v1`), Freedesktop desktop spec
- **Tests:** `cargo test --workspace`, unit tests in `qlaunch_core`, `MemoryTrimPolicyTest` (Kotlin)