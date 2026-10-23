# Page Transition Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Save a 100–800 ms transition duration (300 ms default), animate tab navigation, and cycle left on Back when no route/search needs closing.

**Architecture:** Rust settings own the persisted duration and clamp its range; generated bridge types carry it to the existing Flutter settings slider and pager. Explicit commands and swipe settling share the duration; Back reuses the existing left-step command over visible pages.

**Tech Stack:** Rust/Serde, flutter_rust_bridge, Flutter/Dart, existing widget and on-device integration tests.

**Spec:** Approved conversation design: duration slider 100–800 ms/default 300 ms; direct finger tracking; animated tab clicks; Back closes existing route/search first, otherwise cycles left indefinitely. Test on the connected tablet.

## Global Constraints

- Preserve earlier uncommitted memory-trim fix.
- Use visible pages only; do not pre-render hidden groups.
- Preserve reduced-motion handling, page-list reconciliation, and nested vertical scrolling.
- Persist `page_transition_ms` without invalidating tile pixels or resetting page position.

## Review Focus

- Old settings without the new field load with 300 ms.
- Rapid Back/tab commands remain in the requested direction and can interrupt animation.
- Search/dialog/settings dismissal does not also step left.
- Empty or one-page launchers do not cycle or crash.
- Settings updates do not jump the pager or discard rendered tiles.

### Task 1: Persist duration and expose slider

**Files:** `rust/qlaunch_core/src/settings/mod.rs`, `rust/qlaunch_core/tests/settings.rs`, generated Rust/Dart bridge files, `lib/src/ui/settings_screen.dart`, `test/settings_screen_test.dart`, `test/snapshot_builders.dart`, `test/fake_backend.dart`.

**Interfaces:** `Settings.page_transition_ms: i64`, `SettingsPatch.page_transition_ms: Option<i64>`; generated `pageTransitionMs`; slider `SettingField.pageTransitionMs`.

- [x] Write tests for legacy default 300, range clamp 100/800, serialization/persistence, and slider patch isolation/range.
- [x] Observe failure, add the setting, regenerate bindings, update test builders.
- [x] Add the Display slider with millisecond units and run focused settings tests.

### Task 2: Animation and Back

**Files:** `lib/src/ui/group_pager.dart`, `lib/src/ui/home_screen.dart`, `test/pager_motion_test.dart`, `test/home_screen_test.dart`.

**Interfaces:** `GroupPager.transitionDuration: Duration`, default 300 ms; HomeScreen supplies saved duration. Back uses `_step(-1)` after route/search dismissal.

- [x] Write timing tests for tab commands and swipe release, and test actual HomeScreen tab clicks before and during animation.
- [x] Write Back tests covering repeated left wrapping, route/search dismissal priority, and one/empty page safety; observe failures.
- [x] Share selected duration across snap and explicit commands; fix any reproduced tab-animation reset; preserve reduced-motion jumps.
- [x] Run focused navigation/settings tests and ensure duration-only settings preserve raster generation.

### Task 3: Verify and deploy

- [x] Run full Flutter and Rust suites, scoped analysis, native tests, and diff checks; report pre-existing failures separately.
- [x] Exercise actual tab/swipe timing, duration persistence and Back cycling on the tablet, restoring settings changed by tests.
- [x] Build/install the release on the tablet and record results.

No commits are requested.
