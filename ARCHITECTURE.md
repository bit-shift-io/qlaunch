# qlaunch architecture

A pure **iced + Rust** Android launcher (the Pixel test phone / `dev/iced`), with a
Linux desktop target used for development. There is no Flutter, Dart or bridge:
the old Dart tree was purged and must not be reintroduced.

The design rule running through the whole codebase is **the core is the source of
truth**. Rust (not the UI, and not Kotlin) owns the model, the settings rules and
the persistence. The iced UI renders a snapshot and sends mutations back; the
Kotlin layer only answers OS questions and performs OS actions.

---

## Crates and build

| Path | What it is |
| :--- | :--- |
| `rust/qlaunch_core` | Platform-independent core: model, settings, store, search, usage, platform traits and adapters. Builds `rlib` + `staticlib` + `cdylib` (the cdylib carries the JNI exports). |
| `rust/qlaunch` | The iced UI (`App`, `Message`, `view`/`update`) and JNI-facing entry points. Builds `cdylib` (Android) + the `qlaunch` desktop binary. |
| `android/` | Gradle app, Kotlin OS adapter, manifest (HOME/LAUNCHER role), accessibility service, device-admin lock receiver. |
| `scripts/build.sh` | arm64 Android build: cargo → `jniLibs` → Gradle. |
| `run.sh` / `run_adb.sh` | Run on desktop / build-install-launch on a device. |

Toolchain: Rust edition 2024, iced pinned to the `iced_android-v0.1.1` fork and
`iced_android v0.1.1`, Android NDK `28.2.13676358`, JDK 17 (`JAVA_HOME=/opt/jbr-21`).

```sh
# Android release APK
ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build.sh release
adb install -r build/app/outputs/apk/release/app-release.apk

# Desktop (needs a Wayland/X session)
./run.sh

# Tests and lints
cd rust && cargo test --workspace
cd rust && cargo clippy --workspace --all-targets
```

`cargo test --workspace` is the gate. A number of clippy warnings are pre-existing
and intentional; do not "fix" unrelated warnings in a feature change.

---

## Data flow

```
Kotlin (OS)  ──JNI──►  qlaunch_core  ──snapshot()/CoreEvent──►  qlaunch (iced)  ──render
     ▲                     │                                        │
     └───────── QLaunchBridge ◄──────────────── core::* calls ──────┘
```

- `qlaunch_core::api::state` holds the live `App` (model + settings + usage + store)
  behind a global lock.
- The UI reads `core::snapshot()`, which returns a `Clone`-able `ModelSnapshot`.
  It subscribes to `CoreEvent`s (`ModelChanged`, `AppsChanged`, `HomePressed`, …)
  and re-reads the snapshot when one arrives.
- Mutations are `async fn core::*(...) -> Result<ModelSnapshot, String>`. The UI
  runs them off the render thread through `App`'s `work(...)` helper
  (`tokio::spawn_blocking`), then applies the returned snapshot.
- Platform access goes through traits so the core is testable: `AppSource`
  (`list_apps`, `icon_image`, `launch`, `watch`, backups) with an Android
  implementation over JNI (`platform/android/jni_host.rs`) and a Linux
  implementation over `.desktop` files, plus `FakeSource` for tests.
- The Android host calls `QLaunchBridge`, whose methods marshal JSON in/out of
  Kotlin. The JNI symbol names and the `MainActivity` external methods are the
  contract between the two languages; keep them in sync.

### Snapshot invariants

These are core rules. The UI must consume them, never re-derive them:

- `page_ids` is the page order. Do not recompute it from groups.
- `home_group_id` is the group holding the Home role. Do not scan roles in the UI.
- `items` is in **display order within each group**. Filter by `group_id` and use
  the given order; do not sort again.
- `folder_icons` names the member a folder tile should draw. Folder tiles are
  `Item`s with `folder_group_id: Some(id)`; their members have `group_id == id`.
  (The grid now draws the first four members itself; see `folder_icon.rs`.)

---

## Configuration and persistence

`qlaunch_core::store` writes one document plus icon files.

- Root: Android uses `Context.filesDir`; Linux uses
  `$XDG_CONFIG_HOME/bitshift/qlaunch` or `~/.config/bitshift/qlaunch`
  (`store/location.rs`).
- `qlaunch.json` — the document (`Model` + `Settings` + usage), `SCHEMA_VERSION = 1`.
- `icons/` — PNGs written by `Store::write_icon`.
- `qlaunch.json.bak` — last good document kept before a repair.
- `qlaunch.json.corrupt[.N]` — quarantined documents; the earliest is the original.
- Backups are written to the platform Downloads folder (`qlaunch-backup.*.json`);
  `qlaunch-backup.pre-restore.json` is kept before a restore.

Load-time behaviour that future changes must preserve:

- `Settings` and the model use `#[serde(default)]`, so an older document loads with
  defaults for new fields. Adding a setting needs no migration.
- A missing/invalid field is repaired or quarantined; out-of-range numbers are
  clamped, not rejected. Saves refuse to write a document load would reject.
- A schema newer than `SCHEMA_VERSION` is quarantined, never reinterpreted.

---

## The iced UI (`rust/qlaunch/src`)

`lib.rs` is the hub: `App` (all UI state), `Message`, `update_inner`, `subscription`,
icon warming, dashboard refresh scheduling, and the JNI-facing functions.

Rendering is split so geometry is testable independently of drawing:

| Module | Responsibility |
| :--- | :--- |
| `view.rs` | Launcher tree: search, page grid, title strip, cells, labels, dashboard header. |
| `slots.rs` | Custom three-slot widget used for pages *and* the title strip: one clipped layer, per-slot translation and inverse cursor/clip, input gating. Geometry lives in pure `child_bounds`/`translation`. |
| `pager.rs` | Pure pager state: committed page, signed fractional offset, drag and settle animation. |
| `grid.rs` | Shared geometry: content width, row height, column fitting, title/search/folder rectangles. |
| `label.rs` | Measured single-line label. Short text centres when asked; text too wide is ellipsized at a character boundary and drawn from the left, with an optional contrast shadow. |
| `folder_icon.rs` | Folder-tile preview widget: 1 centred, 2 side by side, 3 triangle, 4 square. Empty (or not-yet-warmed) folders fall back to the generic glyph. |
| `dashboard.rs` | Dashboard widget (clock, alarm, weather, calendar) and its platform calls. Takes the page `alpha` so it fades during paging. |
| `folder.rs` | Content-sized folder modal (geometry + view). |
| `overlay.rs`, `dialog.rs`, `components.rs`, `theme.rs`, `icons.rs`, `fonts.rs`, `placeholder.rs`, `field.rs` | Menus and dialogs, reusable Material widgets, dark tokens/typography, bundled Material Icons glyphs, label fonts, generated placeholders, field underline. |
| `screens/` | `settings`, `groups`, `group_apps`, `advanced` full-screen pages. |
| `gesture.rs` | Pure gesture classifier (tap, long press, double tap, drag, swipe, pull-down). |
| `accessibility.rs` | Accessibility frame derived from state + the shared geometry. |
| `scroll_fade.rs`, `elastic_scroll.rs`, `stretch.rs`, `reorder_list.rs` | Scroll-edge fade, overscroll feedback, elastic stretch, grip-only reordering. |
| `metrics.rs`, `probe.rs` | Frame-phase timing and the `frame-work` probe. |
| `storage.rs` | Reading shared Downloads backups. |

### Shared components

Reusable UI lives in `components.rs`, with every colour, size and text style coming
from `theme.rs`. New screens and dialogs compose these; they do not invent one-off
styling. Brief result messages use the shared toast (`App.toast`, `Message::Toast`),
never a bespoke surface.

Window/lifecycle rule: the launcher keeps **one** iced window and never closes it
on background (`window_action` returns `Nothing`), because Android can deliver a
destroyed activity's late `onStop` after its replacement's `onStart`. See the
blank/frozen-screen notes in `docs/technical-notes.md`.

### Icons

`App.icons` is a bounded decoded working set (clear at ~24 MiB) warmed by
`App::warm_icons`, current page first then neighbours. Folder tiles draw their
first four members, so `warm_icons` also warms folder members of the visible pages
even though those members are not items of the page.

### Dashboard refresh

`DashboardTick` fires every 30 s while the dashboard is enabled and the app is
foregrounded. The clock and alarm refresh on every tick (cheap, local). Calendar
and weather refresh only when their configured interval has elapsed
(`dashboard_calendar_refresh_minutes`, `dashboard_weather_refresh_minutes`;
defaults 5 and 30; weather is a network call). Explicit actions — first load,
enabling the widget, changing the location, granting calendar permission — force an
immediate refresh and reset the due times.

---

## Android / Kotlin (`android/app/src/main/kotlin/bitshift/qlaunch`)

| File | Role |
| :--- | :--- |
| `MainActivity.kt` | `IcedActivity` subclass; lifecycle, Home/Back dispatch, accessibility publish, shortcut result, window focus. |
| `QLaunchBridge.kt` | The single surface the core asks the OS for: app list, icons, launch, uninstall, wallpaper, shortcuts, backups, display settings, accessibility. |
| `LauncherActions.kt` | Activity-bound actions and result round-trips (lock, notifications, shortcut begin/launch, apply display settings, publish a11y). |
| `DashboardAdapter.kt` | Clock/alarm/calendar/weather and per-category app pickers. |
| `PackageChangeReceiver.kt`, `PackageRefreshCoalescer.kt` | Coalesced package-change watch. |
| `MemoryTrimPolicy.kt` | Trims the icon cache on memory pressure. |
| `LauncherAccessibilityView.kt`, `AccessibilityDiff.kt` | TalkBack exposure (gated off while disabled). |
| `ScreenLockService.kt`, `LockDeviceAdminReceiver.kt` | Accessibility/device-admin screen lock. |

Adding a platform capability means: a trait method on the core `AppSource`/
`AndroidHost`, a JNI wrapper in `jni_host.rs`, a `@JvmStatic` method on
`QLaunchBridge`, and the Kotlin implementation.

---

## Where to change what

| Task | Start here |
| :--- | :--- |
| New setting | `qlaunch_core/src/settings/mod.rs` (field + patch + default + clamp + apply), `qlaunch_core/tests/settings.rs`, `qlaunch/src/screens/settings.rs`. `Settings` is `#[serde(default)]`, so old configs keep working. |
| New launcher UI | `qlaunch/src/view.rs`, with geometry in `grid.rs` and reusable widgets in `components.rs`. |
| New screen | `qlaunch/src/screens/`, wire a `Screen` variant and the dispatch in `view::launcher`. |
| Model/mutation rule | `qlaunch_core/src/model/`, then expose it through `qlaunch_core/src/api/api.rs`; the UI must not duplicate it. |
| Dashboard | `qlaunch/src/dashboard.rs` (view + refresh) and `DashboardAdapter.kt` (data). |
| Android permission/OS action | the Kotlin table above plus `jni_host.rs`. |

---

## Testing

- `cd rust && cargo test --workspace` — unit tests live next to the code; the core
  has integration tests under `qlaunch_core/tests/`.
- Geometry and pure state (pager, slots, grid, gestures, settings) are deliberately
  testable without a renderer. Keep new geometry pure and covered.
- Some renderer-dependent tests are `#[ignore]`d and need a headless wgpu adapter;
  run them explicitly with `cargo test -- --ignored` when relevant.
- On-device verification notes and measurements are in `docs/technical-notes.md`.
  Frame timing on a device: `adb logcat -s qlaunch` (the `frame-work` tag).
