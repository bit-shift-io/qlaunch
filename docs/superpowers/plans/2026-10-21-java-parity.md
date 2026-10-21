# Java Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore snappy grid-aligned crossfade navigation, original title behavior, complete missing shortcuts/settings/reset, and add long-press actions and screen locking.

**Architecture:** Flutter owns gestures/layout/animation; Rust owns persistent mutations and platform dispatch; Kotlin handles Android window, intent-result and device-lock operations. Preserve the updated search and group-management flows.

**Tech Stack:** Flutter/Dart, Rust, flutter_rust_bridge 2.13, Kotlin/Android.

**Spec:** `docs/superpowers/specs/2026-10-21-java-parity-design.md` and adjacent parity audit.

## Global Constraints

- Keep schema 1 compatibility; regenerate public Rust bindings.
- No commits or branch changes without explicit request.
- Preserve existing working tree; execute natively in this session as requested by “build it”.
- Tests use isolated configuration; never reset device user data.

## Review Focus

- Interrupted gestures/model changes must not leave invisible grids (Task 1).
- Narrow cells/large fonts must not overlap or shrink labels (Task 2).
- Item taps/long presses/double taps must not trigger blank-space actions (Task 5).
- Cancelled shortcut requests or deleted destination groups must not create misplaced items (Task 3).
- Reset save failure preserves the old model and icon references (Task 4).

### Task 1: Pager motion and linked three-title strip

Files: `lib/src/ui/group_pager.dart`, `lib/src/ui/tab_strip.dart`, `lib/src/ui/home_screen.dart`; tests `test/group_pager_test.dart`, `test/tab_strip_test.dart`.
Interfaces: controller exposes continuous `ValueNotifier<double> position`, nearest-cycle `goToGroup` and directional `step`; GroupPager takes configured columns. Titles consume the controller position.

- [ ] Add regression tests for crossfade at half-page, column-pitch translation, interrupted swipe recovery, animated nearest navigation and one-page immobility; run focused tests red.
- [ ] Implement continuous transforms, command-aware animation, structural immediate reconciliation, transparent previous/current/next titles and accessible neighbour navigation.
- [ ] Run focused tests green; replace tests asserting superseded all-tabs/underline/pips behavior.

### Task 2: Content-sized cells

Files: `lib/src/ui/app_cell.dart`, `lib/src/ui/app_cells.dart`, new `lib/src/ui/grid_geometry.dart`; tests `test/app_cell_test.dart`, `test/app_grid_test.dart`.
Interfaces: shared geometry computes row extent from settings, width and text scaler. Labels use two lines; no whole-cell scaling.

- [ ] Add two-line/scaled-text/hidden-icon geometry regressions; run red.
- [ ] Implement layout-specific padding and extents with stable column origins; retain session icon cache and per-group scroll offsets.
- [ ] Run focused tests green.

### Task 3: Shortcut and lock backend

Files: `rust/qlaunch_core/src/api/api.rs`, `api/state.rs`, new `api/shortcuts.rs`, `model/mod.rs`, Android `jni_host.rs`; Kotlin `QLaunchBridge.kt`, `MainActivity.kt`, new `LockDeviceAdminReceiver.kt`, XML/manifest; generated bindings.
Interfaces: async `shortcut_providers`, `begin_shortcut(provider, target_group_id)`, `add_command_shortcut(target_group_id,label,command)`, `add_lock_shortcut(target_group_id)`, `launch_shortcut(id)`, `item_icon(id)`, `lock_screen`. Provider/result transport uses JSON at JNI only; shortcut payloads remain typed Rust data. Add a `LockScreen` payload variant.

- [ ] Add isolated Rust API tests for lock shortcut round-trip, unknown/deleted group refusal, command validation, cancellation/no item, typed payload validation and item icon retrieval; run red.
- [ ] Implement core validation/persistence and platform dispatch; Android chooser results feed Rust with captured group identity, typed extras, supplied icon, cancellation/error events. Apply data/type together. Lock uses minimal force-lock admin permission flow; Linux uses loginctl.
- [ ] Regenerate bindings, build host library and run Rust tests green. Compile Android native changes later with full APK.

### Task 4: Display settings and atomic reset

Files: Rust `api/api.rs`, `api/state.rs`; Kotlin `QLaunchBridge.kt`, `MainActivity.kt`; Dart backend/state/settings screen.
Interfaces: `factory_reset() -> ModelSnapshot`; `apply_display_settings` private Android dispatch after init/update/reset; store saves before swap.

- [ ] Add reset tests verifying defaults, empty usage, rediscovered apps, and save failure preserving old state; run red.
- [ ] Implement reset and owned-icon/cache cleanup after successful replacement, native orientation/status-bar application on main thread, and Dart cache invalidation/reset navigation.
- [ ] Run Rust and state/settings widget tests green.

### Task 5: Launcher menu, shortcuts UI and double-tap lock

Files: Dart `launcher_backend.dart`, `launcher_state.dart`, `home_screen.dart`, `app_settings_dialog.dart`, `app_cell.dart`, `app_cells.dart`, new `shortcut_picker.dart`; tests fake backend and home/shortcut editor tests.
Interfaces: state wraps Task 3/4 APIs; menu captures current group; built-in Lock Screen appears alongside Android providers or Linux command form.

- [ ] Add regressions for blank-space menu destinations, double-tap lock count, item gesture exclusion, shortcut launch/move/remove and menu dismissal; run red.
- [ ] Implement bounded context menu, provider/command/built-in picker, supplied icon rendering, shortcut-aware editor and launch. Home/Back dismiss menu before other actions. Keep search behavior unchanged.
- [ ] Run focused tests green.

### Task 6: Verify and document

- [ ] Run `flutter analyze lib test integration_test`, `flutter test`, `cargo test --manifest-path rust/qlaunch_core/Cargo.toml`.
- [ ] Build Linux and APK with documented Java environment; inspect device availability and test fresh install where available without resetting user data.
- [ ] Update README, roadmap, audit and execution ledger with delivered behavior and precise verification limitations.

Self-review: all design sections mapped to Tasks 1–6; locks are covered by Tasks 3/5. Execution is inline; no commits.
