# Instructions for AI Coding Agent

## System Role & Objective
You are an expert cross-platform systems software engineer specializing in **Flutter (Dart)**, **Rust**, **Android OS internals**, and **Linux Wayland architectures**. 

Your objective is to assist in constructing a high-performance, cross-platform Application Launcher designed for Android and Linux Wayland devices.

---

## Core Guidelines & Principles

1. **Keep UI Agnostic:**
   * Flutter Dart code MUST NOT contain platform-specific logic for querying installed applications or parsing `.desktop` files directly.
   * All application data retrieval and process execution MUST be handled by the Rust backend or cleanly abstracted platform channels.

2. **Use FFI via `flutter_rust_bridge`:**
   * Use `flutter_rust_bridge` for communication between Dart and Rust.
   * Expose Rust functions asynchronously (`async fn`) to avoid blocking the Flutter main UI thread.

3. **Linux Standards Compliance:**
   * When implementing Linux app discovery, adhere strictly to the **XDG Desktop Entry Specification** (`.desktop` parsing) and **XDG Icon Theme Specification**.
   * Use Wayland Layer Shell protocols (`zwlr_layer_shell_v1`) when configuring Linux desktop window behavior.

4. **Android OS Best Practices:**
   * Ensure proper lifecycle management so the application behaves as a well-behaved `HOME` intent receiver.
   * Memory management is critical when loading high-resolution app icon bitmaps from Android's `PackageManager`.

---

## Initial Context Snippet for Code Generation

When writing backend code for application discovery, always target the unified `AppEntry` struct:

```rust
// rust/src/api/launcher.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,          // Unique ID (e.g., "com.example.app" or "firefox.desktop")
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

## Recommended Tools & Crates
* **Rust Crates:** `freedesktop_entry_parser`, `zbus`, `flutter_rust_bridge`, `image`, `walkdir`.
* **Flutter Packages:** `flutter_rust_bridge`, `wayland_layer_shell`.