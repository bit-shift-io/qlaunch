# Cross-Platform Launcher (Android & Linux Wayland)

## Executive Summary
This project aims to build a responsive, touch-friendly Application Launcher designed to run seamlessly on both **Android** and **Linux Wayland** environments (targeting x86 tablets initially and ARM64 Linux phones in the future). 

By leveraging **Flutter** for the UI engine and **Rust** for core business logic and system interfacing, the architecture ensures native graphics performance, smooth animations, and high cross-platform code reuse.

---

## Key Tech Stack

| Layer | Android Target | Linux Target |
| :--- | :--- | :--- |
| **UI Framework** | Flutter (Dart) | Flutter (Dart) |
| **Core Logic** | Rust (`cargo-ndk`) | Rust (`cargo`) |
| **FFI Bridge** | `flutter_rust_bridge` | `flutter_rust_bridge` |
| **OS Interfacing** | Kotlin/JNI (`PackageManager`, Intents) | Freedesktop `.desktop` parser, `std::process` |
| **Compositor Target** | Android System Home | Wayland Compositor (e.g., `phoc`, `sway`, `labwc`) |
| **Shell Integration**| `android.intent.category.HOME` | `wayland_layer_shell` (`zwlr_layer_shell_v1`) |

---

## Architectural Strategy

To prevent platform-specific code from leaking into the UI, all system queries are abstracted behind a unified Rust interface via `flutter_rust_bridge`.

```
                    ┌───────────────────────────────────────┐
                    │          Flutter UI Engine            │
                    │   (App Grid, Search Bar, Gestures)    │
                    └──────────────────┬────────────────────┘
                                       │
                             flutter_rust_bridge (FFI)
                                       │
                    ┌──────────────────┴────────────────────┐
                    │           Rust Core Crate             │
                    │      (Unified Launcher Model)         │
                    └──────────┬─────────────────┬──────────┘
                               │                 │
                   (Android)   │                 │   (Linux)
                               ▼                 ▼
                     Android JNI Bridge   Freedesktop Parser
                     (Package Manager)    (/usr/share/applications)
```

### Shared Data Model
```rust
pub struct AppEntry {
    pub id: String,          // Package name or .desktop filename
    pub name: String,        // App display name
    pub icon_bytes: Vec<u8>, // PNG/SVG raw byte array for Flutter display
    pub categories: Vec<String>,
}
```

---

## Platform Scope & Compositors

1. **Android Setup:**
   * Declared as a Home screen replacement (`android.intent.category.HOME`).
   * Handles system shortcuts, home button intercepts, and native intent dispatching.

2. **Linux Wayland Setup:**
   * Acts as a **Wayland Layer Shell Client** (`zwlr_layer_shell_v1`).
   * Configured via `wayland_layer_shell` to occupy the `BACKGROUND` or `OVERLAY` layer depending on whether it runs as a full home screen or an app drawer overlay.
   * Target Compositors: **Phoc** (Mobile/Touch focus) and **Sway/Labwc** (Development & Tiling/Stacking testing).