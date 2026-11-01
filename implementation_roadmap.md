# Implementation Roadmap

This document outlines the step-by-step approach for building the pure-iced launcher.

Status as of 2026-11-01: Phases 1–4 are implemented. The Android build works, the launcher renders the Home grid, basic gestures, search and app launching work; Linux Wayland and Wayland layer-shell integration remain pending.

---

## Phase 1: Core Foundation & Platform Abstraction

- [x] **Task 1.1:** Initialize Rust workspace (`qlaunch_core` and `qlaunch` crates) with platform-aware settings (Android via `cargo-ndk`).
- [x] **Task 1.2:** Configure `android-activity` v0.6.1 (pinned from `iced_android-v0.1.1`) and its `[patch.crates-io]` entry.
- [x] **Task 1.3:** Add Kotlin/JNI adapter (`bitshift.qlaunch`) that exposes the core APIs to Android (PackageChangeReceiver, MainActivity, PackageChangeReceiver). Fix `PackageChangeReceiver` to load `libqlaunch.so` (previously `libqlaunch_iced.so`).
- [x] **Task 1.4:** Establish shared data structures (`AppEntry`, `LaunchRequest`, `LauncherConfig`) in `qlaunch_core`.

---

## Phase 2: Platform Integration & App Discovery

- [x] **Task 2.1:** Implement Android app discovery in Rust (via JNI, using `PackageManager`).
- [x] **Task 2.2:** Implement Linux app discovery in Rust (Freedesktop `.desktop` parser, `walkdir`).
- [x] **Task 2.3:** Build Linux icon resolver in Rust (GTK icon theme and PNG/SVG reading).
- [x] **Task 2.4:** Implement process launchers (`std::process::Command` for Linux, Android `Intent` dispatch for Android).
- [ ] **Task 2.5:** Integrate `wayland_layer_shell` in the Linux UI to configure layer positioning, anchors and focus.

---

## Phase 3: Search, Gestures & Core UI

- [x] **Task 3.1:** Build in-app search driven by the Rust search API, supporting frequent/recent apps.
- [x] **Task 3.2:** Implement gesture classifier (long-press, drag, double-tap, pull-down).
- [x] **Task 3.3:** Implement paged group navigation with a custom `Slots` widget, replacing the overlay pages from the pinned iced fork.
- [x] **Task 3.4:** Implement scrollable groups, cell layouts and edge opacity fades.
- [x] **Task 3.5:** Build the long-press menu (Rename/Move/Uninstall, folder creation, system shortcut providers) backed by the core APIs.

---

## Phase 4: Settings & Lifecycle Management

- [ ] **Task 4.1:** Implement persistent settings, group editing, backup/restore and factory reset.
- [ ] **Task 4.2:** Implement custom app/icon picker, notification expansion and quick-settings tile scaffolding.
- [x] **Task 4.3:** Fix the lifecycle bug: never close the window on background (`window_action` returns `Nothing`) and clear stale background flag on focus (`window::Event::Focused`).
- [x] **Task 4.4:** Ensure the accessibility service can expose virtual nodes (`isImportantForAccessibility`), and switch exposure off by default to disable TalkBack support.

---

## Phase 5: Hardware & Device Testing

- [ ] **Task 5.1:** Test Linux build on Wayland compositors (`sway` for PC dev, `phoc` for touch testing).
- [ ] **Task 5.2:** Deploy and test touch input, virtual keyboard interaction and scaling on an x86 Linux tablet.
- [x] **Task 5.3:** Build Android APK and test home-screen lifecycle on Android tablet/phone devices.