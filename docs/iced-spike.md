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
- [x] Replace the fork's translated overlay pages with a custom `Slots` widget and
      verify mid-drag rendering and input gating on the tablet.
- [ ] Accessibility: TalkBack cannot drive the launcher — NO-GO; the exposure is
      switched off and the failure is recorded below.
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

## Scroll fade follow-up

The Iced app grids now consume the saved **Scroll fade** setting, including
pager pages, search results and folder members:

- **None:** ordinary clipping, without opacity changes.
- **Gradient:** Flutter's 32 dp top/bottom ramps, eased away within 32 dp of
  either real list boundary. Content that fits has no gradient.
- **Tile:** uniform icon/label/highlight opacity based on the visible cell
  fraction, becoming fully transparent when half the cell leaves the viewport.

The pinned renderer has no alpha-mask layer; gradient mode approximates the
mask using 1 dp clipped bands with primitive opacity, not a background overlay.
Wallpaper remains visible and page-transition/icon opacity still multiply in.
Only edge cells need repeated draws; the opaque middle is drawn once per cell.
Because opacity is applied per primitive rather than to a composited layer,
overlapping shadows and pressed highlights can differ slightly from Flutter.
On-device visual parity and frame pacing still need checking.

Verification (from `rust/`): `cargo test --workspace`. A separate GPU rendering
test checks transparent-surface output and is opt-in for machines with a wgpu
adapter: `cargo test -p qlaunch_iced rendered_fades --lib -- --ignored`.

## Accessibility — Task 6 result: NO-GO (2026-10-29)

**Decision: NO-GO.** Two hands-on attempts with TalkBack failed: the selector
stayed on the navigation bar and left/right swipes never entered the launcher,
so no announcement, exploration or double-tap flow could be used. The plan says
to stop and not start the pager work when any of the five acceptance items fails;
the user has since directed that the pager tasks proceed anyway, and that
accessibility exposure be switched off in the meantime.

**Accessibility exposure is off by default.** `ACCESSIBILITY_EXPOSURE_ENABLED`
in `LauncherAccessibilityView.kt` gates the host view, the published tree and
action dispatch together. The implementation is kept intact; the device
acceptance tests require the flag to be `true`. With it off the launcher is
verified to behave normally: no virtual nodes in `uiautomator dump` (a 1.7 KB
trampoline-only tree), tap still launches, and swiping still pages
(Home → Games with the wrapped title strip in landscape).

The earlier automated results below remain accurate for the code they tested;
they did not predict the hands-on failure, which is the whole reason this gate
was defined as a TalkBack check rather than an action-level check.

Tablet: iPlay60 mini Turbo, Android 14, TalkBack 17.1.0.1003738110.
TalkBack is now **disabled** on the tablet: with the launcher's exposure off,
TalkBack sees no launcher content, so leaving it enabled made the device unusable
(the user asked for it to be turned off). Settings are
`enabled_accessibility_services=null`, `accessibility_enabled=0` and
`touch_exploration_enabled=0`; the launcher was rechecked afterwards (a tap
launches, paging works). The updated release APK was built and installed with
`adb install -r`; its Home tree exposes the five expected apps. Wallpaper remains
visible.

Fresh verification: 125 iced library tests (plus binary/doc-test targets), 21
Kotlin unit tests (including 15 tree-diff tests), and all nine device acceptance
methods pass. Device methods
cover published order; Firefox launch and app/blank-grid menus; real search IME,
query/results updates and accessibility focus through `c`, `ca`, `cal`, `calc`;
vertical scrolling and page steps, focus retention/fallback and removal of old
page nodes; and an old-item/page-step smoke check. A deterministic Rust test
separately proves a valid request is rejected during settle and accepted after
motion is cancelled, without changing the frame/revision. Device acceptance
ran against the debug build; the final updated release was rebuilt and installed.
The additional methods cover Rename/Move screens, cancelling the system uninstall
confirmation, temporary-folder navigation, and TalkBack-off ordinary tap/long
press, injected hardware key events, a real Gboard key tap, and blank double tap /
pull-down reaching the existing lock/notification permission screen. The launcher's
ScreenLockService is not enabled: actual locking/notification expansion was not
tested, and no new accessibility permission was granted. Temporary edits/folder
were discarded by restoring the pre-check store byte-for-byte before release
installation. Screenshots remain outside the repository.

The device test exposed a bridge focus bug: `ExploreByTouchHelper.INVALID_ID`
was treated as a removed focused node, generating synthetic focus events while
the helper retained no focus. Absent focus is now filtered out, and fallback
focus uses the provider's real focus action instead of merely emitting an event.
The search test failed before this fix and passes afterwards.

Review also identified obsolete focus across screen changes: root invalidation
alone does not clear the helper's old focused ID. The bridge now clears removed
focus while the old node still exists and transfers it to the first live node
on the new screen. The new diff regression failed (`expected 2, got null`), then
passed; the device menu test confirms live focus before explicitly focusing a
menu control. The reviewer’s motion-test false-positive finding is addressed by
the deterministic valid-request Rust test and by treating the device race as a
smoke check, not proof of timing.

An additional real bounds bug was found by the TalkBack-off rename test:
`.height(INPUT_HEIGHT).center_y(Fill)` overwrote the fixed input height with Fill,
stretching the card and moving drawn actions away from their semantic targets.
The regression test `input_entry_keeps_the_height_used_by_accessibility` failed
with `Fill != Fixed(36)` before the fix; `.center_y(INPUT_HEIGHT)` now keeps the
view and semantic geometry consistent, and ordinary Back/Cancel taps pass.

Bounds samples (physical pixels, tablet density 320 / 2 px per dp):
- Home Firefox cell: `(0,140)-(300,455)`; icon `(60,150)-(240,330)` and label
  lie inside this target; the next rendered row begins at the matching pitch.
- Folder title: `(115,43)-(1186,140)`; rendered title starts at x=115 and lies
  inside this header. Header/cell targets have no painted border, so their
  outer edges cannot independently be pixel-measured from the screenshot.
- Search result Calculator: `(0,122)-(300,437)`; icon begins at `(60,133)`;
  the 11 px top inset is intentional cell padding, not an offset of the node.
- Search field: semantics and pixel-detected blue border both
  `(14,47)-(1088,119)` — 0 px discrepancy.
- Overlay rename input: semantics and pixel-detected blue border both
  `(334,883)-(866,947)` — 0 px discrepancy. Back's reported
  `(334,1034)-(866,1099)` now contains the rendered action and ordinary taps work.

Hands-on attempt (2026-10-29): TalkBack's selector stayed on the navigation bar
and left/right swipes did not reach the launcher. Cause: provider nodes were not
marked important for accessibility, so services that do not request unimportant
views never saw the virtual tree. `LauncherAccessibilityView` now sets
`isImportantForAccessibility` on every populated node; the placeholder for a
racing id sets it too. New test
`i_nodesAreExposedWithoutIncludingUnimportantViews` failed before the fix
(`importantForAccessibility: false` for Firefox) and passes after.

Observable evidence after the fix (with TalkBack temporarily enabled, release
build): the focused
accessibility window is the QLaunch window on cold start (ids 701–704) where it
was the navigation bar (676) before; this held across three cold starts, and
opening search kept it on QLaunch. That is machine evidence only — it shows
TalkBack moved into the app window, not that speech or swipes are correct.

Second hands-on attempt after the `isImportantForAccessibility` fix still failed:
the selector stayed on the navigation bar and swipes did not enter the launcher.
The gate is therefore closed as NO-GO, and the machine evidence above (focused
accessibility window moving to QLaunch) is not sufficient to reopen it. This is
the check that mattered: earlier automated passes, including the fixed-flag test,
did not predict it. Per the plan this failure should also stop the pager work; the
user directed that Tasks 7-9 continue regardless, and the exposure was switched
off by default so the launcher behaves as it did before the spike.

Remaining accessibility work, if it is ever resumed: find why TalkBack's linear
navigation does not enter the launcher window (the focused window moves, but
swipes stay on the navigation bar), then re-run the five hands-on checks and the
nine automated device methods with `ACCESSIBILITY_EXPOSURE_ENABLED = true`.
The service settings as found at the start of this continuation were restored
after the TalkBack-off checks, and TalkBack was then disabled outright once the
gate failed and the exposure was switched off (see above). The pre-spike service
state is not recorded reliably enough to claim it was restored.
Move destinations were verified without executing a move; actual uninstall was
cancelled. These are safety-limited checks, not proof of the destructive flows.
Physical external-keyboard hardware was not available; its key-event path was
tested through Android input injection.

Verification limitation: the debug native glue also intermittently aborted on
window resize (`glue.rs:498`, absent window); isolated reruns passed. No native
fork change was made. This is a separate lifecycle risk, not a successful test.
Package-scoped Clippy (`--no-deps`) and formatting pass. Strict Clippy including
dependencies fails on existing `qlaunch_core` module-inception/collapsible-if
warnings; the core was not edited to suppress them.

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
The folder fixture additionally requires `-e allowTemporaryFolder true`; take
an external store snapshot first and restore it afterwards. The TalkBack-off
method requires externally disabling TalkBack, then restoring the service list.

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

Translation of the neighbour pages was drawn through iced's overlay path; on that
iced_android build the translated `Float` layers could overlap and the title strip
could disappear in mid-drag screenshots, although the settled page was correct.
That is now replaced by the custom `Slots` widget (see "Page renderer" below),
which clips once and draws each slot itself, so the fork's overlay path is no
longer on the paging path at all.

Rotation was not verified on device: the saved settings pin the activity to
portrait, and `wm user-rotation` has no effect while that lock is applied. A resize
during an animation, which exercises the same target-retaining code, is covered by
a unit test instead. Pull-down-on-a-scrolled-page and vertical scrolling were not
exercised because no saved group has enough items to scroll.

Do not use this APK to judge final feature-complete maintenance cost. It is useful
for validating native integration, grid/search rendering, app-return behaviour,
gesture handling, motion and the reduced UI/core boundary.

## Page renderer (2026-10-30)

`rust/qlaunch_iced/src/slots.rs` adds a three-slot widget that replaces the
`float`/`Stack` pages and the translated title strip. Both are one clipped layer
in which each slot is drawn at `child_bounds + translation`: pages translate by
`(index - 1 - offset) * width`, the strip's thirds translate together by
`-offset * width / 3`. Two pure functions own that geometry, so the layout, the
draw calls and the tests share one source.

The widget also owns input gating, which the overlay path could not do:

| Interaction | Slots that receive input |
| --- | --- |
| `None` (dragging or settling) | none, except a redraw request to the centre |
| `Centre` (settled pages) | the committed page only |
| `All` (settled strip) | all three title slots |

Each slot's content gets the cursor and clip shifted by the inverse translation,
and a slot whose group key changes gets a fresh widget tree, so a scroll offset
cannot leak from one group into the next. `operate` forwards to the same slots
as input, which keeps `scroll_to("apps")` and the scroll resets working.

Device results (tablet, release build, landscape 1920x1200):

- A slow drag in both directions and across a wrap was recorded with
  `screenrecord` and inspected frame by frame at 5 fps. Mid-drag frames show the
  committed page, both neighbours and the whole title strip at once, with the
  strip's opacity following the offset; no overlap, dropout or flicker.
- A drag starting on an icon (Firefox) paged instead of launching: focus stayed
  on `bitshift.qlaunch`.
- A tap on an icon launched it; a tap 0.1-0.25 s after a horizontal drag release
  was dropped and the page still settled (`Interaction::None` working); at
  0.4-0.6 s, by which time the page had settled, the same tap launched the app,
  which is the intended behaviour for a settled page.
- A title step 0.15 s into a vertical drag landed correctly and did not launch
  anything. An earlier attempt to check this was invalid (the app was in search
  mode, where no title strip exists) and is not counted as evidence.
- `frame-work` lines were emitted for each motion.

Known limits found while verifying:

- The debug native build can abort in `android-activity`'s window-resize glue
  (`glue.rs:498`, an absent window behind a `debug_assert`). The release build
  compiles the assert out. Isolated reruns pass, so this is an intermittent
  lifecycle hazard in the pinned fork, not a resolved issue.
- One run left a cell highlighted after paging. Instrumenting the set/clear
  paths and sweeping the tap delay (0.1-0.6 s) showed every press cleared within
  ~23 ms and did not reproduce it, so no fix was made on a guess; it is recorded
  as an unreproduced cosmetic finding.
- Accessibility exposure is off by default; see the NO-GO section above.

## Unified chrome font (2026-10-30)

The Material chrome — dialogs, long-press menus, the Settings/Groups/Advanced
screens, buttons, chips, sliders, the snackbar and the app bar — now draws in the
platform font at fixed Material sizes. It no longer reads the launcher's Text
settings; those drive only the home interface (grid labels, the tab strip, folder
items and shortcut labels) through `view::label()`.

This fixes blank chrome text on Android. `theme::Text::from_settings` had set the
whole UI's type from the Font-size and Typeface settings using iced's generic
families, and cosmic-text maps those to `Open Sans` / `DejaVu Serif` /
`Noto Sans Mono`, which Android never loads (it has no fontconfig). Only the
labels, which used the platform default `Roboto`, rendered. The chrome type scale
is now fixed (`theme::Text::new()`), the OS font scale still applies globally via
`scale_factor`, and label typefaces resolve to families that exist per platform
(`fonts::label_font`): Android uses `Roboto`, `Droid Sans Mono` and a `Noto Serif`
loaded from `/system/fonts`; Linux keeps the generic families.

Verified on the tablet (Android 14, release): menus, dialogs and Settings render,
and the grid and tab strip follow Normal, Serif and Mono while the chrome stays
sans-serif.

## Tablet frame measurement (2026-10-30)

Tablet: iPlay60 mini Turbo, Android 14, 1200x1920, 90 Hz, release build, saved
layout unchanged, five runs. The tablet was in landscape, where the profiling
script's default swipe coordinates (which assume portrait) land on the navigation
bar, so the same script and method were run with landscape coordinates
(`y=600`, x 1800->100 alternating) from a copy under `/tmp`. No repository script
was changed for this.

| Run | Frames | p50 | p95 | p99 | max | over 1.5x |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 126 | 11.14 ms | 11.15 ms | 11.45 ms | 22.28 ms | 0.8% |
| 2 | 126 | 11.14 ms | 11.16 ms | 33.41 ms | 155.93 ms | 1.6% |
| 3 | 126 | 11.14 ms | 11.16 ms | 11.36 ms | 33.42 ms | 0.8% |
| 4 | 126 | 11.14 ms | 11.15 ms | 22.27 ms | 167.08 ms | 1.6% |
| 5 | 126 | 11.14 ms | 11.16 ms | 11.50 ms | 33.41 ms | 0.8% |
| median | 126 | 11.14 ms | 11.16 ms | 11.50 ms | 33.42 ms | 0.8% |

The median cadence is 11.14 ms, which is the 90 Hz period (11.11 ms), and matches
the previously recorded tablet baseline (p50/p95/p99 11.14/11.15/11.17 ms). Two of
the five runs contain a single large interval (156 ms and 167 ms) plus elevated
p99, so the median row must not be quoted as "no hitches": occasional long frames
still occur. App-side work stayed small: `update` p50 0.03 ms / p95 0.25-0.27 ms,
`view` p50 0.76-0.77 ms / p95 0.91-0.99 ms over ~216 frames per drag.

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
Flutter code in the iced APK. Motion held the display's cadence (60 Hz on the
phone, 90 Hz on the tablet) and app-side frame work was well below a frame budget.
The paging path no longer depends on the fork's translated overlay layers, because
a custom `Slots` widget owns that geometry and its input gating.

One requirement is not met: accessibility. TalkBack cannot drive the launcher
(see the NO-GO section), so this build is not usable by a screen-reader user, and
the exposure is switched off rather than shipped half-working.

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

## Release comparison against main (2026-10-31)

Rebuilt both UIs as release APKs on the same day and ran the same measurement on
the tablet: iPlay60 mini Turbo, Android 14, 1200x1920 at density 320, 90 Hz. The
iced APK is the `dev/iced` working tree; the Flutter APK is `main` at `d232c78`.
Both were installed with `adb install -r` over the same saved layout (same debug
key, so no data was cleared) and settled before each run.

Frame pacing uses the same workload as the section above: five runs of one
continuous 2 s horizontal drag plus settle, alternating direction, captured with
`scripts/profile-iced.sh`. Keep in mind the two app-side rows are not
interchangeable; iced's own `frame-work` log has no Flutter equivalent here.

| Measurement | Flutter release (main) | Iced spike release |
| --- | ---: | ---: |
| SF interval p50 (median run) | 11.14 ms | 11.14 ms |
| SF interval p95 | 11.17 ms | 11.17 ms |
| SF interval p99 | 11.37 ms | 11.48 ms |
| SF interval max | 22.29 ms | 11.54 ms |
| Intervals over 1.5x | 0.8% | 0.0% |
| App-side `update` p50 / p95 | not collected | 0.02 / 0.10 ms |
| App-side `view` p50 / p95 | not collected | 0.65 / 0.92 ms |
| Foreground total PSS | 107,647 KiB (~105 MiB) | 79,119 KiB (~77 MiB) |
| APK bytes | 57,884,425 | 18,920,790 |

Both hold the 90 Hz cadence through the drag and settle. The medians are within
one histogram bucket of each other, so this is parity, not a win: the only clear
difference is that the Flutter run set shows a larger first-run spike (78 ms cold,
then 22 ms) while iced's median run peaked at 11.54 ms with one 22 ms run. Read
that as "iced is no worse", not as "iced is faster".

The memory and size gaps are the same shape as the earlier smoke rows but are
still **not apples-to-apples**: the Flutter APK bundles three ABIs while the
spike packages arm64 only, and the Flutter artifact carries more complete
functionality. The PSS figures are single foreground snapshots with different
working-set histories. `am start -W` was also recorded (iced WaitTime 3 ms,
Flutter 12 ms) but measures Activity launch, not first usable frame, so it is not
treated as a startup benchmark. The device was left running the iced release
afterwards.

Commands: `bash scripts/build-iced.sh release` (with `JAVA_HOME` pointing at a
full JDK; the default `java-25-openjdk` on this host is a JRE and Gradle rejects
it) and, from a `main` worktree,
`JAVA_HOME=<jdk21> flutter build apk --release`.

## Tile rendering: Flutter's raster cache vs iced (2026-10-31)

Flutter (`main`) rendered each app cell — icon, label and shadow — into a raster
tile and reused it (`TileRasterCache`; modes `combined`, `separate`, `none`, see
`docs/scroll-performance.md`). That cache existed mainly because **live text was
expensive, and a *blurred* label shadow cost ~100 ms raster p99** on paged
transitions; caching the whole tile amortised it to roughly 10 ms.

Iced has no per-widget render-to-texture cache, and the measurements say it does
not need one:

- Shaped text is cached by content value (`paragraph::Plain` only reshapes when
  the string or its bounds change), so labels are not reshaped each frame.
- Icons are one atlas quad each with a stable handle; there is no per-frame
  decode or re-upload.
- **There is no glyph blur at all.** `label_shadow_blur` is stored and exposed in
  Settings but is never read by the iced UI: the shadow is a sharp 1 dp offset
  copy behind the foreground (`view.rs`). Iced's only `blur_radius` is the quad
  shadow used by surfaces such as the dialog card, which text cannot use.
- App-side frame work on the tablet: `update` p50 0.02 ms, `view` p50 0.65 ms
  against an 11.1 ms (90 Hz) budget.

So the one Flutter feature not reproduced is the **blurred label shadow** — which
is exactly the cost the tile cache existed to hide. In iced the shadow is free, so
there is nothing to bake; the remaining work is reducing work per frame (build
only on-screen rows, stop the scroll-fade rebuilding whole cells, and remove the
O(results x items) search scan), not caching pixels. A true offscreen tile cache
(a `Headless` render per tile uploaded as an image) would add a texture per tile
and invalidation on every icon/label/theme/size change for a gain the numbers do
not justify.

### Icon size defect (fixed)

Iced clamps a fixed-size image to the available cell (`limits.resolve`), and an
image drawn with `ContentFit::Contain` then scales down to fit. So whenever a cell
was narrower or shorter than `icon_size` the icon shrank instead of staying put.
The probe logged the same app at `100x100` (normal), `95x100` (a brief 420 dp
start-up size) and `100x52`. It showed on the phone and, per the report, on
desktop when the window was small. Fixed by sizing every row to hold a full icon
(`grid::row_height`, now including list layout, which always clipped by 2 dp) and
by reducing the column count when it cannot fit (`grid::fit_columns`). Icons now
render at exactly `icon_size`; verified on device in portrait and landscape,
including at the 420 dp transient.

A second, related defect sat behind the same symptom: iced's flex **compresses**
fixed-height children that overflow their parent, and the two non-committed page
slots wrapped their rows in `container(rows).height(Fill)` rather than a
scrollable. A group taller than the page was therefore squashed to the page
height, shrinking every icon in it (logged as a 66 dp cell / 52 dp icon). All
three slots now use a scrollable, so rows are always laid out at their true
height; only the committed slot takes scroll input or shows a scrollbar.

## Per-frame work after the scrolling fixes (2026-10-31)

Four changes came out of the "small icons" report and the request to stop
rebuilding tiles:

1. **Icons cannot shrink** — columns are reduced to fit `icon_size`
   (`grid::fit_columns`) and every row, list included, leaves room for a full
   icon (`grid::row_height`).
2. **Off-screen rows are no longer built** — the grid emits a fixed-height spacer
   above and below the rows its viewport can show (`grid::row_window`). Row
   height is constant, so the spacers reproduce the scroll extent exactly: the
   scrollbar, the scroll-fade origin and the accessibility bounds are unchanged.
   Work per frame is now bounded to about one viewport of rows however long the
   group is.
3. **The scroll fade is banded at 8 levels per 32 dp ramp** instead of 1 dp.
   Each band with a partial alpha costs one rebuilt cell, so the old step rebuilt
   an edge cell up to ~64 times a frame; it is now bounded at ~16.
4. **Search no longer scans every item per result.** Results are looked up in an
   app index rebuilt with the snapshot (`index_items`), replacing the
   O(results x items) scan.

The `frame-work` log now reports iced's own `layout` and `draw` phases beside
`update` and `view`, measured by a pass-through probe wrapping the launcher tree
(`probe.rs`), so the phases the app never calls directly are visible.

Tablet, 90 Hz, five runs of the same 2 s drag workload, release build:

| Phase (app-side, per frame) | p50 | p95 | max |
| --- | ---: | ---: | ---: |
| `update` | 0.01-0.02 ms | 0.08-0.14 ms | 0.35 ms |
| `view` | 0.60-0.64 ms | 0.80-0.94 ms | 1.84 ms |
| `layout` | 0.32-0.34 ms | 0.46-0.54 ms | 1.48 ms |
| `draw` | 0.06-0.11 ms | 0.15-0.21 ms | 0.53 ms |

Frame pacing was unchanged at parity (SF interval p50 11.14 ms, median p99
11.39 ms, no interval over 1.5x). Total app-side work is about 1.05 ms of an
11.1 ms budget (~9%).

The `draw` figure is the useful surprise: it is ~0.1 ms, so the scroll fade's
per-band cell rebuilds were never a large cost, and the banding change bounds
them rather than rescuing the frame budget. `view` and `layout` are the phases
that carry the per-frame CPU work, which is why the row window (2) matters for
large groups and the others do not.
