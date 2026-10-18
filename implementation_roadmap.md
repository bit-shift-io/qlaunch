# Implementation Roadmap

This document outlines the step-by-step phased approach for building the cross-platform Flutter/Rust launcher.

---

## Phase 1: Core Foundation & FFI Setup
- [ ] **Task 1.1:** Initialize Flutter workspace with Linux and Android targets enabled.
- [ ] **Task 1.2:** Initialize Rust library crate within the project repository.
- [ ] **Task 1.3:** Configure `flutter_rust_bridge` codegen pipeline between Dart and Rust.
- [ ] **Task 1.4:** Establish shared data structures (`AppEntry`, `LaunchRequest`, `LauncherConfig`).

---

## Phase 2: Linux Integration & App Discovery
- [ ] **Task 2.1:** Implement Freedesktop `.desktop` file parser in Rust (scanning `/usr/share/applications` and `~/.local/share/applications`).
- [ ] **Task 2.2:** Build Linux icon resolver in Rust (resolving GTK icon themes and reading PNG/SVG files into byte streams).
- [ ] **Task 2.3:** Implement process launcher in Rust using `std::process::Command`.
- [ ] **Task 2.4:** Integrate `wayland_layer_shell` in Flutter to configure layer positioning, anchors, and keyboard focus under Wayland.

---

## Phase 3: Android Integration
- [ ] **Task 3.1:** Configure `AndroidManifest.xml` with intent filters for `android.intent.category.HOME` and `android.intent.category.DEFAULT`.
- [ ] **Task 3.2:** Write Kotlin/JNI bindings or use Flutter Platform Channels to query `PackageManager` for launcher apps and icons.
- [ ] **Task 3.3:** Implement Android intent launcher for starting selected applications.
- [ ] **Task 3.4:** Handle Android home button and back button events cleanly.

---

## Phase 4: UI & Touch-First Experience (Flutter)
- [ ] **Task 4.1:** Design responsive App Grid with touch-friendly spacing and dynamic scaling (for tablet & phone screen sizes).
- [ ] **Task 4.2:** Implement instant App Search bar with fuzzy filtering driven by the Rust backend.
- [ ] **Task 4.3:** Add smooth gesture support (swipe up for drawer, pinch-to-edit, swipe-down dismiss).
- [ ] **Task 4.4:** Support app category tabs and custom icon/grid layout persistent settings.

---

## Phase 5: Hardware & Device Testing
- [ ] **Task 5.1:** Test Linux build on Wayland compositors (`sway` for PC dev, `phoc` for touch testing).
- [ ] **Task 5.2:** Deploy and test touch input, virtual keyboard interaction, and scaling on an x86 Linux tablet.
- [ ] **Task 5.3:** Build Android APK and test home-screen lifecycle on Android tablet/phone devices.