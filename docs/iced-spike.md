# Iced feasibility spike

Experimental comparison build on `dev/iced`, not a production replacement.

## Scope / checklist

- [x] Build without Flutter, Dart or flutter_rust_bridge in the dependency tree.
- [x] Reuse the Rust model, configuration, discovery, search and launch APIs.
- [x] Android HOME activity, system wallpaper, real icons and matching geometry.
- [x] Group navigation, touch paging, search, frequent/recent apps, folder opening.
- [x] Finger-following page transitions and Flutter-matched long press, tap, double
      tap and pull-down gestures.
- [x] Long-press menus for apps and blank space, backed by the core APIs.
- [x] Build/install on the connected Pixel 5a; record visual/lifecycle findings.
- [x] Measure release frame pacing and app-side CPU frame work against the saved
      Flutter release artifact, keeping the two metrics separate.
- [x] Document reproducible performance comparisons and remaining gaps.

Kotlin/JNI remains the OS adapter. No backend rewrite is needed. The experimental
application ID is `bitshift.qlaunch`, replacing Flutter on the authorised test phone
and reusing its configuration. Save the baseline APK and private files first.
The original sources remain for reference, but the iced APK does not build them.

Settings editing, group editing, backup/restore UI, custom shortcut editors,
exact scroll-fade parity and production hardening are outside this spike.
Existing saved settings, shortcuts and folders should be rendered where possible.

Dependencies are pinned to iced_android v0.1.1 and its matching iced and
android-activity forks. This is community-maintained mobile support, not official
iced support. Workability must be established on-device before adoption.

## Accessibility — Task 6 acceptance in progress (2026-10-29)

**Decision: pending; Tasks 7–9 are not authorised to proceed.** Automated
accessibility actions pass, but this is not a substitute for hands-on TalkBack
speech, swipe/double-tap and touch-exploration acceptance.

Tablet: iPlay60 mini Turbo, Android 14, TalkBack 17.1.0.1003738110.
TalkBack and touch exploration remain enabled for the manual check. The updated
release APK was built and installed with `adb install -r`; its Home tree exposes
the five expected apps. Wallpaper remains visible.

Fresh verification: 123 iced library tests, 20 Kotlin unit tests (including 14
tree-diff tests), and all five device acceptance methods pass. Device methods
cover published order; Firefox launch and app/blank-grid menus; real search IME,
query/results updates and accessibility focus through `c`, `ca`, `cal`, `calc`;
vertical scrolling and page steps, focus retention/fallback and removal of old
page nodes; and dropping an old item action during page motion. Device acceptance
ran against the debug build; release startup/tree was separately checked.

The device test exposed a bridge focus bug: `ExploreByTouchHelper.INVALID_ID`
was treated as a removed focused node, generating synthetic focus events while
the helper retained no focus. Absent focus is now filtered out, and fallback
focus uses the provider's real focus action instead of merely emitting an event.
The search test failed before this fix and passes afterwards.

Still required before a go decision:
- Hands-on spoken traversal, forward/backward swipes, double-tap and exploration.
- Folder navigation and Rename/Move/Uninstall flows (without destructive changes).
- Screenshot comparison of app/folder/search/overlay bounds within 2 dp.
- TalkBack-off ordinary gesture, hardware-keyboard and soft-keyboard regressions.
- Restore the pre-test accessibility-service settings after manual acceptance.

### Device-test safety

Gradle's connected-test cleanup uninstalled QLaunch and cleared its private
layout during this run. Android backup restore did not recover it. The on-device
`Download/qlaunch-backup-2026-10-23-085707.json` did: Apps 26, Home 5, Games 19,
with the expected labels verified after restoration. This recovers the Oct-23
settings/usage too; changes since that backup cannot be guaranteed recovered.
A fresh private-store snapshot is retained outside the repository at
`/tmp/opencode/qlaunch-pre-release-20261029.json`. No personal images were added.

Do **not** use `connectedDebugAndroidTest` on this tablet. Build with
`assembleDebugAndroidTest`, install target/test APKs using `adb install -r`, and
run each method of `AccessibilityAcceptanceTest` in a separate `am instrument`
invocation. AndroidJUnitRunner destroys activities between methods; recreating
the NativeActivity in the same process aborts in the pinned native glue.
The driver preserves TalkBack using `FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES`.
`UiAutomation.executeShellCommand` does not interpret pipes; filter its output
in Kotlin. Repeated Home opens search by design, so do not use it as an
unconditional test reset. Gradle configuration-cache reuse was verified.

## Building (no Flutter SDK required)

Requires Rust with `aarch64-linux-android`, an Android SDK with platform 36,
NDK 28.2.13676358, Python 3 and a JDK compatible with the Gradle wrapper.
The checkout's existing Android Gradle wrapper is used. The original Flutter
template ignores its scripts/JAR; on a fresh clone, install Gradle 9.8.1 (the build
script also accepts `gradle` on PATH), or restore/generate that wrapper first.

```sh
ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build-iced.sh release
adb install -r build/app/outputs/apk/release/app-release.apk
adb shell am start -W -n bitshift.qlaunch/.MainActivity
adb logcat -s qlaunch-iced
```

Host preview: `cd rust && cargo run -p qlaunch_iced --bin qlaunch-iced` (Wayland).
This is a windowed preview, not a Linux layer-shell integration.

```sh
cd rust
cargo test -p qlaunch_core --no-default-features
cargo tree -p qlaunch_iced --no-default-features | grep -Ei 'flutter|dart'
```

The last command should find nothing. Keep `-p qlaunch_iced` on builds: a
workspace-wide build also selects the original core's default Flutter feature.

## Comparison cautions

The original installed Flutter baseline was a **debug** build. Do not
compare its memory/startup/frame times with an iced release and call that a win.
An existing Flutter release artifact was subsequently saved and installed for
release-mode smoke measurements; its exact source revision was not established.
Build both in release with the same configuration, font/display settings and
device temperature, then run the same cold-start, repeated app-return, page-swipe,
vertical-scroll and search sequences. Record APK size, foreground/background PSS,
and a Perfetto trace of actual rendered frames and CPU/GPU work. Use several runs
and compare medians and tail frame times, not one launch.

`adb shell am start -W` measures Activity launch, not necessarily first usable
native-rendered frame. `dumpsys gfxinfo` describes Android View/HWUI rendering and
does not reliably measure Flutter or iced's native rendering; use Perfetto's
FrameTimeline/SurfaceFlinger tracks instead. The Rust `core ready` log measures
data initialization only.

During development the saved baseline is outside the repo at
`/tmp/opencode/qlaunch-iced-baseline/`: `flutter.apk`, `flutter-data.tar`,
`qlaunch.json`, screenshots and memory snapshots. The data archive is private;
do not commit it. Restoring the APK with `adb install -r .../flutter.apk` preserves
the shared configuration schema. Do not uninstall or clear app data to switch.

## Integration findings

- The existing JNI/Kotlin OS adapter works with iced; it did not need replacing.
  Rust calls the same model/search/icon/launch functions directly. The Flutter
  feature on the core is optional and disabled by the iced crate.
- `NativeActivity.onCreate` resets the Android window to RGB565. Setting
  `window.setFormat(PixelFormat.RGBA_8888)` **after** `super.onCreate` fixes real
  system-wallpaper transparency with Vulkan's inherited alpha mode on the Pixel.
- Keep iced's `web-colors` feature: disabling it enables linear-light sRGB
  rendering, which does not match Android's inherited premultiplied composition
  for this transparent window (washed-out translucent text/placeholders).
- The UI uses the platform font, a sharp contrasting 1 dp text shadow, configured
  icon/label sizes, columns, opacity, wallpaper dim and tile/list arrangements.
  Icons are loaded on background workers into a bounded ~24 MiB decoded working
  set; there is no Dart image pipeline or whole-cell raster cache.
- The daemon follows iced_android's close/reopen-window lifecycle pattern to free
  GPU resources while invisible. The process/core and last page stay alive.
- The phone initially used Pixel Launcher as its default Home. QLaunch was made
  Home for lifecycle testing using `adb shell cmd package set-home-activity
  bitshift.qlaunch/.MainActivity`. Restore Pixel Launcher if desired with
  `adb shell cmd package set-home-activity
  com.google.android.apps.nexuslauncher/.NexusLauncherActivity`.

## Gestures and transitions

The gesture classifier uses the same constants as the Flutter launcher:

| Gesture | Constant |
| --- | --- |
| Long press | 500 ms |
| Touch slop before a drag | 18 px |
| Double tap window / movement limit | 300 ms / 100 px |
| Pull-down trigger | 48 px |
| Fling velocity | 300 px/s |
| Page transition | `page_transition_ms`, default 300 ms, `easeOutCubic` |

Dragging horizontally moves the current page and its neighbours with the finger,
wraps in both directions, commits each page as its width is crossed and settles
with a velocity bias when the release exceeds the fling threshold. The title
strip translates and fades with the same progress. A drag that starts on an icon
pages instead of launching; a tap still launches. Title buttons switch by one
page with the same animation, and tapping the centre title opens search.

Blank presses are inferred from geometry: iced_android's `scrollable` captures
every touch before a wrapper `mouse_area` sees it, so the app treats a press below
the title strip or folder header (and above the navigation inset) as blank.
Chrome presses therefore do nothing rather than locking or paging.

Long-pressing an app opens an overlay menu with **Rename**, **Move**, **Uninstall**
(app tiles only), **Remove** and **Cancel**. Rename edits the label through the
core config; Move lists the other non-root, non-folder groups and appends to the
destination; Uninstall opens Android's confirmation. Long-pressing blank space
opens the launcher menu with **Create Folder** (name step), **Add Shortcut**
(system shortcut providers), **Lock Screen Shortcut**, **Notifications Shortcut**,
**Choose Wallpaper**, **Settings**, **Edit Groups** and **Close**. Settings and
Edit Groups show "Not ported in this spike"; the shortcut commands create tiles in
the current page group through the core APIs.

Double-tapping blank space locks the screen; double-tapping an icon or the title
strip does not. Pulling down from blank space at the top opens the notification
shade (the accessibility service must be enabled, otherwise Android opens its
accessibility settings). A pull-down on a page with content is intended to scroll
the group instead (not exercised here; see gaps). Back closes a sub-menu, then the
menu, then search, then a folder; it leaves the launcher itself visible.

## Deliberate gaps

This is a small comparison UI, not a commitment to adopt iced. Frequent/Recent
switch by title or horizontal swipe and use core history. Back does not exit the
launcher.

Not implemented: settings/group editors (the menu entries only show a notice),
custom shortcut or icon editors beyond the system provider picker, backup/restore,
scroll-edge fades, continuous vertical group scrolling, blurred shadows, exact
two-line ellipsis and accessibility semantics parity. Labels are clipped to two
lines rather than ellipsized; folder placeholder art is approximate; list/centred
arrangements are previews, not pixel parity. A group's scroll offset resets to the
top when paging away and back, and multi-page jumps are not animated. Android
shortcuts already in the config still launch through the core adapter.

Translation of the neighbour pages is drawn through iced's overlay path; on this
iced_android build the translated `Float` layers can overlap and the title strip
can disappear in mid-drag screenshots, although the settled page is correct. This
is a rendering-fork limitation, not a state bug.

Rotation was not verified on device: the saved settings pin the activity to
portrait, and `wm user-rotation` has no effect while that lock is applied. A resize
during an animation, which exercises the same target-retaining code, is covered by
a unit test instead. Pull-down-on-a-scrolled-page and vertical scrolling were not
exercised because no saved group has enough items to scroll.

Do not use this APK to judge final feature-complete maintenance cost. It is useful
for validating native integration, grid/search rendering, app-return behaviour,
gesture handling, motion and the reduced UI/core boundary.

## Frame measurement (2026-10-28)

Two independent numbers are collected, and they are not interchangeable:

1. **SurfaceFlinger presentation cadence** for the launcher layer, as p50/p95/p99/
   max milliseconds between actual presents, plus the share of intervals over 1.5x
   the refresh period.
2. **App-side CPU frame work** from the iced process's own `frame-work` log: the
   time spent in `update` and in `view` per frame, as p50/p95/max. This excludes
   GPU submission and composition, so it is not a total frame time.

`scripts/profile-iced.sh` wakes the device, runs one continuous 2 s horizontal drag
plus its settle per run (alternating direction, so paging wraps), repeated five
times, and prints both summaries. A continuous drag avoids counting the idle gaps
between separate swipes as slow frames. `scripts/latency.py` parses either the
timestamp rows from `dumpsys SurfaceFlinger --latency` or, when that sampler is
disabled, the per-layer `present2present` histogram from
`dumpsys SurfaceFlinger --timestats -dump`, which reports 1 ms buckets.

On this Android 14 user build `--latency` returns only the refresh period even for
system layers, and `dumpsys gfxinfo` reports zero HWUI frames because both UIs
render natively. The timestats histogram is therefore the presentation source here;
histogram percentiles are accurate to the bucket width.

Workload: Pixel 5a (1080x2400, 60 Hz), Android 14, Adreno 620, five runs, release
builds (`flutter-release.apk` and the iced release APK) installed over the same
saved layout, app settled before the run.

| Measurement | Flutter release | Iced spike release |
| --- | ---: | ---: |
| Presented frames per run (median) | 133 | 142 |
| SF interval p50 / p95 / p99 | 16 / 16 / 17 ms | 16 / 16 / 16 ms |
| SF interval max | 17 ms | 17 ms |
| SF intervals over 25 ms | 0.0% | 0.0% |
| App-side `update` p50 / p95 / max | not collected | 0.01 / 0.03-0.04 / 0.05-0.32 ms |
| App-side `view` p50 / p95 / max | not collected | 0.21-0.40 / 0.43-0.50 / 0.46-0.84 ms |

Both builds held a 60 Hz presentation cadence through the drag and settle with no
interval above 25 ms. The iced frame count is slightly higher, most likely because
its settle takes a few frames longer. The Flutter app-side numbers were not
collected in a matching way, so this is a presentation-cadence comparison plus a
bound on iced's own CPU cost, not an end-to-end frame-time win.

The frequently quoted Flutter 9-12 ms DevTools number is frame build/raster work,
not presentation cadence. It cannot be compared directly with either row above
unless the same collection method is applied to both builds. The iced `view`
timings are sub-millisecond, but they cover only element-tree construction; they
show CPU headroom, not total frame cost.

These are five-run medians on one device, not a thermal or multi-device benchmark.
Use the script with the same layout, refresh rate and workload before drawing
conclusions.

## Can Dart/Flutter be removed?

Technically, for the exercised flows, yes: the Rust core plus the existing
Kotlin/JNI adapter provided saved configuration, groups/folders, real icons,
wallpaper, search, launching, system shortcut providers, lock/notifications
shortcuts, the wallpaper picker, uninstall and window recreation, with no Dart or
Flutter code in the iced APK. Motion stayed at 60 Hz and app-side frame work was
well below a frame budget.

Removal is not a drop-in yet. The original Dart UI remains the only complete
implementation and the reference for behaviour; its widget/integration tests and
tooling are Flutter-based, and the core crate still carries Flutter-only feature
flags and bridge bindings. A production switch would need the missing features
(settings and group editors, backup/restore, custom shortcut/icon editors, scroll
fades, accessibility semantics), rotation/resize and multi-device validation at
other refresh rates, release engineering without Flutter, and a maintenance plan
for the community `iced_android`/iced forks and the rendering limitations seen in
this spike. Until those exist, Flutter should remain the shipping UI; this spike
shows the core boundary is sound and estimates the remaining work rather than
proving a cheaper replacement.

## Device verification / preliminary measurements (2026-10-28)

Pixel 5a, Android 14, Adreno 620 / Vulkan. Release APK installed over the existing
package without clearing its layout. QLaunch is left as the default Home app.

Verified on-device: saved groups/folder contents and real icons, wallpaper,
title navigation, horizontal swipe starting on an app without launching it,
finger-following paging with wrapping and title-strip motion, folder
opening/Back, search with the real keyboard (`calc` → Calculator), launching
Calculator, returning Home, and foreground/background window recreation without
a crash. Long press on an app opened Rename/Move/Uninstall/Remove; rename and
rename-back succeeded, Move listed only other non-folder pages, and Uninstall
opened the system confirmation. Long press on blank space opened the launcher
menu; Create Folder reached its name step, Settings showed the notice, and Choose
Wallpaper opened the picker. Double tap on blank space locked the screen
(`mWakefulness=Dozing`); double tapping the title strip or an icon did not.
Pulling down from blank space at the top opened the notification shade once the
accessibility service was enabled. Back walked Move → Actions → closed, and the
launcher stayed visible. Back behaviour and the saved layout were unchanged after
reinstalling the release APK. Frame pacing and app-side CPU work are recorded in
the "Frame measurement" section; no thermal benchmark was performed. The final
screenshots are `iced-final-home.png`, `iced-final-apps.png` in the baseline
directory.

Single-run smoke measurements, **not controlled benchmark results**:

| Measurement | Saved Flutter release | Iced spike release |
| --- | ---: | ---: |
| Apps page total PSS | 135,001 KiB (~132 MiB) | 102,942 KiB (~101 MiB) |
| Background total PSS | 103,876 KiB (~101 MiB) | 48,049 KiB (~47 MiB) |
| Background reported graphics allocation | 41,092 KiB | 0 KiB |
| APK bytes | 57,884,425 | ~15,660,000 |

The APK sizes are **not apples-to-apples**: Flutter bundles three ABIs; this spike
only packages arm64. Uncompressed arm64 native libraries alone are ~19.6 MB for
Flutter's app/engine/core combined versus ~13.3 MB for iced/core combined. Memory
figures are snapshots with different working-set histories; the Flutter artifact
also has more complete functionality. These indicate feasibility, not proven
speedup. Full feature parity must be established before an adoption decision.

Verification commands passed: core tests with and without its Flutter feature
(416 tests per configuration), 76 iced library tests, six Python measurement
tests, `cargo fmt -p qlaunch_iced --check`, iced Clippy with `--no-deps -D
warnings`, and the Android release build. Workspace-wide strict Clippy reports
pre-existing core lints (`module_inception`, `collapsible_if`); the Android build
retains existing AGP/Kotlin deprecation warnings.
