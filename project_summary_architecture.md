# Cross-Platform Launcher (Android & Linux Wayland)

## Executive Summary

This project builds a responsive, touch-friendly Application Launcher that runs on both **Android** and **Linux Wayland** environments. The UI is powered by **iced (Rust)** with the `iced_android` adapter, and all system interfacing is handled by a Rust core and Kotlin/JNI — no Flutter or Dart is involved.

## Key Tech Stack

| Layer | Android Target | Linux Target |
| :--- | :--- | :--- |
| **UI Framework** | iced + `iced_android` | iced (Wayland compositors) |
| **Core Logic** | Rust (`qlaunch_core`) | Rust (`qlaunch_core`) |
| **FFI Bridge** | Kotlin/JNI (`android-activity`) | Native desktop interfaces |
| **OS Interfacing** | Kotlin/JNI (`PackageManager`, Intents) | Freedesktop `.desktop` parser, `std::process` |
| **Compositor Target** | Android System Home | Wayland Compositor (e.g., `phoc`, `sway`, `labwc`) |
| **Shell Integration** | `android.intent.category.HOME` | `wayland_layer_shell` (`zwlr_layer_shell_v1`) |

## Architectural Strategy

All system queries are handled by the Rust core (`qlaunch_core`) via Kotlin/JNI on Android and native interfaces on Linux. The data model, discovery, search, launch and config APIs are shared across platforms.

### Shared Data Model

```rust
pub struct AppEntry {
    pub id: String,          // Package name or .desktop filename
    pub name: String,        // App display name
    pub icon_bytes: Vec<u8>, // PNG/SVG raw byte array
    pub categories: Vec<String>,
}
```

## Platform Scope

1. **Android Setup:**
   * Declared as a Home screen replacement (`android.intent.category.HOME`).
   * Handles system shortcuts, home button intercepts, and native intent dispatching via Kotlin/JNI.
   * The pinned Android fork `iced_android` v0.1.1 and its `android-activity` `[patch.crates-io]` provide the native activity glue.

2. **Linux Wayland Setup:**
   * Acts as a **Wayland Layer Shell Client** (`zwlr_layer_shell_v1`).
   * Configured via `wayland_layer_shell` to occupy the `BACKGROUND` or `OVERLAY` layer.
   * Target Compositors: **Phoc** (Mobile/Touch focus) and **Sway/Labwc** (Development & Tiling/Stacking testing).