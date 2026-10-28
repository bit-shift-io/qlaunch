# Iced gestures and page transitions — design

Date: 2026-10-28
Status: proposed for the `dev/iced` comparison spike

## Context

`dev/iced` already runs a Flutter-free launcher on the Pixel 5a. It reuses the
Rust core through Kotlin/JNI, renders the saved groups with real icons, and
matches the Flutter launcher's static grid closely. It currently supports taps,
immediate horizontal swipes, title taps, search, folders, Home and Back.

The next comparison pass must exercise the interactions that dominate a
launcher's rendering workload, then measure frame pacing. The reported Flutter
baseline for this device is 9–12 ms per frame, so the iced version needs real
finger-following motion and overlays before frame numbers mean anything.

This remains a throwaway comparison build, not a production replacement.

## Intent

Add the launcher gestures users actually perform — long press, double tap and
pull-down — and make page changes follow the finger with the same settle
animation as Flutter. Then measure iced's frame intervals and CPU frame work on
the same device and settings, and record the result with the existing spike
notes.

## Goals

1. **Long press an app** opens a working menu: rename, move to another page, and
   uninstall (or remove, for shortcuts and placeholders).
2. **Long press blank space** opens the launcher menu: launcher settings entry,
   add shortcut, create folder, edit-groups entry, choose wallpaper.
3. **Double tap blank space** locks the screen through the core/accessibility
   adapter. Taps on apps must never lock the screen.
4. **Pull down from the top of the grid** opens the notification shade once per
   gesture, through the core adapter.
5. **Page transitions** follow the finger, wrap in both directions, settle with
   velocity and `page_transition_ms`, and animate the title strip in step.
6. **Measure** frame intervals on device plus the app's own update/view CPU time,
   and document them next to the Flutter baseline.

The menus only need to prove the interaction path. Settings, group editing and
shortcut-creation screens stay out of scope this pass; entries may show a clear
"not ported in this spike" notice instead of doing nothing.

## Non-goals

- A complete settings screen, group editor, or shortcut-creation flow.
- Blurred text shadows, scroll-edge fades, continuous vertical group scrolling,
  accessibility semantics parity, or animation of non-adjacent group jumps.
- Production hardening: no new permissions, no backup/restore UI, no migration
  work, no Play Store packaging.
- Rewriting any core rule in the UI. Menus call the existing core API.

## Constraints

- `qlaunch_iced` must remain buildable with `--no-default-features`, with no
  Flutter/Dart dependency.
- Core and JNI calls stay off iced's UI thread.
- Android gesture thresholds copy Flutter's where they exist:
  - long press: 500 ms (`kLongPressTimeout`)
  - touch slop: 18 logical px (`kTouchSlop`)
  - double-tap window: 300 ms (`kDoubleTapTimeout`)
  - double-tap slop: 100 logical px (`kDoubleTapSlop`)
  - pull-down trigger: 48 logical px
  - pager fling threshold: 300 logical px/s
- `page_transition_ms` is the existing core setting, clamped 100–800 ms,
  default 300 ms. Use `easeOutCubic`.
- Format only `qlaunch_iced`; `cargo fmt --all` touches unrelated generated core
  files.
- Strict Clippy applies to `qlaunch_iced` with `--no-deps`; the core has
  pre-existing lints outside this pass.

## Behavior decisions

### Gesture ownership

A touch is classified once, on the UI thread, from iced's raw touch events.

- **Down on an app:** the cell may launch on release, as today. The classifier
  also records the item for long press and cancels the pending launch whenever
  the finger moves beyond touch slop, a long press fires, a page drag wins, or a
  pull-down triggers.
- **Down on blank space:** the classifier owns the tap. The first short tap only
  records its time and position. A second short tap within 300 ms and 100 px
  locks the screen.
- **Long press:** a timer fires 500 ms after down. It only opens a menu if that
  same gesture is still active and has not moved beyond slop. Menu opening
  suppresses the eventual button release.
- **Page drag:** wins when horizontal travel exceeds slop and is more than 1.5×
  vertical travel. It then follows the finger until release.
- **Pull-down:** wins when the grid is at scroll offset zero, vertical travel
  exceeds 48 px, and vertical is more than 1.5× horizontal. It fires once, then
  the rest of the gesture is ignored.

Vertical scrolling keeps priority for primarily vertical movement that is not a
pull-down. Horizontal page drags do not scroll vertically.

### Page motion

The pager exposes a committed page, a fractional offset, and an optional
animation. Rendering shows the committed page's left neighbour, the committed
page, and its right neighbour, translated by the offset and clipped to the
content area. Wrapping is arithmetic on the page list, so one-page neighbours
always exist.

Dragging maps total horizontal finger travel to page units. Crossing a full page
width advances/retreats the committed neighbour continuously, so a long drag can
cross several groups without lifting. On release, velocity adds a half-page bias,
the result rounds to the nearest page, and the offset animates to that page over
`page_transition_ms` with `easeOutCubic`. A zero target settles back without
changing the committed page.

Explicit navigation (title taps, Home, Back, keyboard) uses the same animation
for one-page moves. A jump of more than one page is immediate; animating a long
jump is not required this pass.

The title strip renders the same fractional offset: three slots at 1/3 width,
each label's opacity `1 - |slot - offset| * 0.6`, clipped to the strip. Tapping a
side slot steps one page; tapping the centre keeps the spike's existing search
affordance.

Animation ticks come from `iced::window::frames()`, subscribed only while an
animation is active. Idle launcher frames must not be driven by a permanent
frame subscription.

### Menus and core actions

Menus are an in-app overlay (scrim plus card), not a navigation stack. Back
closes the overlay before closing search/folders. While an overlay is open, raw
touch gestures do not page or lock underneath it.

- **App menu:** Rename (text field), Move to group (list of eligible groups),
  Uninstall for app items, Remove for shortcuts/placeholders. Mutations call
  `rename_item`, `move_item`, `uninstall_app` or `remove_item` and replace the
  snapshot with the returned one.
- **Launcher menu:** Launcher Settings, Add Shortcut, Create Folder, Edit Groups,
  Choose Wallpaper. Settings and Edit Groups show a notice for this pass.
  Create Folder asks for a name and calls `add_folder` with the current group.
  Choose Wallpaper calls `choose_wallpaper`.
- **Add Shortcut:** Lock Screen and Notifications call
  `add_lock_shortcut`/`add_notifications_shortcut`; provider rows come from
  `shortcut_providers` and call `begin_shortcut`. The Android picker result
  arrives through the existing activity-result bridge and core snapshot event.
- Errors appear inside the overlay and can be dismissed; they never silently
  close a menu.

Eligible move destinations exclude the root group, folder groups, and the item's
current group. Appending uses the destination group's current item count as the
order, matching `move_item`'s dense-order contract.

### Measurement

Two numbers are reported separately and never conflated:

1. **Presentation intervals** from
   `dumpsys SurfaceFlinger --latency <launcher layer>` during a scripted gesture
   sequence. Report p50, p95, p99, max and the share over 1.5× the display
   refresh period.
2. **UI CPU frame work**: time spent inside the app's `update` plus `view` for
   frames during a gesture window, logged as p50/p95/max. This is not GPU time
   and is not comparable to Flutter's DevTools "frame time" without saying so.

Run each sequence several times on a cool device, after the app has settled, with
the same layout/settings, and report medians of the runs. The Flutter APK saved
in `/tmp/opencode/qlaunch-iced-baseline/` can be reinstalled for its own run; the
script must not swap APKs automatically.

## Acceptance

- All gestures work on the Pixel 5a in a portrait grid with one group, several
  groups, a folder, and an empty group.
- A swipe that starts on an icon never launches the app.
- A long press that drifts beyond slop never opens a menu.
- Double tap only locks from blank space.
- Pull-down only opens notifications when the grid is at the top.
- Page swipes wrap and settle without flicker or a jump at the hand-off.
- Back closes overlay, then search/folder, then leaves the launcher visible.
- `cargo test -p qlaunch_core --no-default-features`,
  `cargo test -p qlaunch_iced --no-default-features --lib`,
  `cargo fmt -p qlaunch_iced --check`, and
  `cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings`
  pass.
- The release APK builds and installs, and the measured results and remaining
  gaps are recorded in `docs/iced-spike.md`.

## Left to the builder

- Exact module decomposition beyond the named files, and whether the view stays
  in `lib.rs` or moves to `view.rs`.
- Menu visual details within the existing launcher style.
- Error wording and confirmation copy.
- Whether to add a small debug FPS readout; it is not required.
