# TODO

## Elastic scroll feedback is switched off (2026-11-01)

- [x] Set `elastic_scroll::ENABLED` to `false`. `wrap` now returns its scroller
  unchanged, so the edge bounce, row spread and velocity stretch never run. The
  wrapper, spring, stretch code and their tests are retained; flip the constant
  to `true` to restore the behaviour.
- This is a scope decision to focus on other work, **not** a root-cause fix for
  the blank screen below. There is no evidence yet that the feedback caused it.

## Intermittent blank/frozen launcher: reproduced and fixed (2026-11-01)

- [x] Reproduced on the connected Pixel 5a (Android 14): the app closes its only
  iced window when the activity that hosts it is destroyed and recreated in the
  same process, because the destroyed instance's late `onStop` arrives after the
  replacement's `onStart`. The window is then gone while the activity is
  `RESUMED`, focused and visible, so the last frame stays frozen and no input
  reaches the event loop; screen off/on recreated the surface and recovered it.
- [x] Fixed by never closing the window on background (`window_action`), and by
  clearing a stale background flag from `window::Event::Focused`.
- See "Root cause and fix" below. The fixed release is now the installed
  `bitshift.qlaunch`; the Oct 28 APK it replaced could not be updated in place,
  so it was uninstalled and reinstalled (see "Installing the fix").

### Root cause and fix (2026-11-01)

Evidence from `logcat` with lifecycle instrumentation, on the
`bitshift.qlaunch.blanktest` side-by-side build, forcing an in-place activity
restart (`am start -f 0x10008000`):

```
onPause            (old activity, window 150278188)
onCreate/onStart/onResume  (new activity, window 164215634)
foreground true  window=Some(Id(1))   <- new instance keeps the window
onStop             (old activity)
onDestroy          (old activity, finishing=true)
foreground false window=Some(Id(1))   <- STALE signal, from the destroyed instance
window closed background=true         <- pre-fix: the app closes its only window
```

- Both `onStart(true)` and `onStop(false)` are static JNI calls that carry no
  activity identity, so `android-activity`'s "spurious JVM callback" guard does
  not cover them: the stale `false` wins.
- winit's Android backend never destroys the window by itself
  (`MainEvent::TerminateWindow` only emits `Suspended`), so the only `Closed` is
  one this app asks for. Closing on background therefore turned a rare
  interleaving into a permanent, windowless process.
- Fix: `window_action` returns `Nothing` for a background transition, so the
  window is kept; `Foreground(true)` still opens one if it is really gone. The
  stale `background = true` is repaired by `window::Event::Focused`, which always
  belongs to the live instance. Tests:
  `backgrounding_keeps_the_window_open`,
  `window_focus_clears_a_stale_background_flag`.

Verification on device (A/B, same sequence): the pre-fix APK logs
`window closed background=true` and then takes no input (`FROZEN`, activity
`RESUMED`/focused, stale frame); the fixed APK logs
`foreground false ... action=Nothing`, keeps rendering and stays responsive.
A stress loop of 4 x 8 lifecycle transitions (Home, page swipe, app switch,
recents, notification shade, screen off/on, search with keyboard, in-place
activity restart) left the fixed build healthy throughout.

Diagnostics kept for a recurrence: `MainActivity` logs every lifecycle callback
with the activity-instance window hash (`adb logcat -s qlaunch-iced:*`), and the
Rust side logs `window opened/closed`, `foreground <bool> window=... action=...`,
`window resized` and every touch-down.

### Installing the fix

`adb install -r` of the new build fails against the installed `bitshift.qlaunch`
with `INSTALL_FAILED_UPDATE_INCOMPATIBLE`: the Oct 28 APK was signed with a debug
key that is not on this machine (`~/.android/debug.keystore` matches the new
build, not the installed one), so it cannot be updated in place. The fix was
first verified as a side-by-side package (`bitshift.qlaunch.blanktest`, built
with `-PqlaunchAppId=...`).

The device is a test phone and its layout data was expendable, so on 2026-11-01
the installed package was uninstalled and the fixed release installed as the real
`bitshift.qlaunch`, then set as the default Home. The side-by-side package was
removed. The fresh install shows an empty default layout; the launcher's own
Advanced > Restore can load `qlaunch-backup-2026-10-21-185054.json` from
Downloads to bring the old layout back. The same 8-step lifecycle loop passed on
this real install (3 cycles plus an idle check).

### Original observations (Pixel 8, 2026-11-01)

- The entire launcher content was black; Android status/navigation bars remained
  visible. The activity was `RESUMED`, focused, visible and reported drawn.
- The app process and its `android_main` thread were still alive. Captured logs
  contained no native panic, fatal exception or wgpu error explaining the failure.
- Force-stopping the app restored the full Home grid on automatic relaunch,
  without clearing data. This shows the saved layout was still usable.
- Suspect stale rendering/surface state after a lifecycle transition, **not a
  confirmed root cause**. The triggering sequence is unknown.
- Android `dumpsys gfxinfo` reported zero HWUI frames, but that alone is not
  evidence of failure for the native Vulkan renderer.

### Debugging checklist (kept for the next lifecycle failure)

1. Capture evidence **before restarting** the next blank instance: screenshot,
   `adb logcat -d -b all -v threadtime`, `adb shell dumpsys activity activities`,
   `adb shell dumpsys window`, `adb shell dumpsys SurfaceFlinger`, and
   `adb shell ps -T -p <pid>` (get PID with
   `adb shell pidof bitshift.qlaunch`). Start a continuous logcat capture before
   reproduction so the triggering transition is not lost from the ring buffer.
2. Add timestamped lifecycle/surface logging across `MainActivity`, Iced/winit
   and the native Android glue: pause/resume, stop/start, focus, window creation/
   destruction, suspend/resume, surface size and surface reconfiguration. Include
   activity/window identity so events for replaced instances can be distinguished.
3. Trace redraw requests, redraw handling, successful frame presentation and
   surface errors. Determine whether the event loop stops requesting frames,
   rendering stalls, or frames are presented to a stale/invisible surface.
   Rate-limit frame logs and retain the last successful presentation time.
4. Exercise screen off/on and unlock, switching apps and returning Home, opening/
   closing notifications, search keyboard show/hide, and rotation in another app
   followed by returning to the portrait launcher. Test long idle/background
   periods too. Record the exact sequence if it reproduces.
5. Compare the blank and healthy SurfaceFlinger layer/buffer state. If the native
   thread is stuck, capture its stack in a debuggable build and inspect whether it
   is waiting for lifecycle/input events or GPU/surface synchronization. Preserve
   the release APK/configuration first; do not uninstall or clear user data to
   obtain a debug build.
6. Inspect the pinned Android/Iced/winit glue's suspend/resume and window teardown
   paths. `docs/iced-spike.md` already records a separate intermittent resize abort
   at `glue.rs:498` (absent window); investigate whether lifecycle handling is
   related, without assuming it explains this blank screen.
7. Once a cause is established, add a regression test for the failing transition
   and repeat the device lifecycle sequence in release mode. Restart recovery is
   a workaround, not proof of a fix.
8. Force the suspected transition directly (`am start -f 0x10008000` restarts the
   activity in place) and A/B the same sequence against the pre-fix APK.

### Local evidence

Captured files are outside the repository under `/tmp/opencode/` (temporary, not
durable). The 2026-11-01 lifecycle work produced, in
`/tmp/opencode/qlaunch-blank-test/`:

- `repro-2026-11-01-a/` and `repro-prefix-2026-11-01/`: the pre-fix freeze
  (screenshot, `logcat`, `dumpsys window/activity/SurfaceFlinger`, threads).
- `stress-fixed/results.txt`: 4 x 8 transitions healthy on the fixed build.
- `qlaunch-instrumented-blanktest.apk` (pre-fix) and
  `qlaunch-installed-2026-10-28.apk` (the original installed APK, kept so its
  replacement key/signature can be compared).
- `analyze.py` (blank detection), `capture.sh` (evidence bundle),
  `alive.sh` (input liveness), `stress.sh` (lifecycle loop).

The older Pixel 8 captures (`qlaunch-blank.png`, `qlaunch-blank-process.log`,
`qlaunch-blank-all.log`, `qlaunch-restarted.png`,
`qlaunch-restarted-process.log`, `qlaunch-restarted-surfaces.txt`) were gone by
2026-11-01. Keep device logs private/review them before sharing. The installed
release is not debuggable, so private configuration cannot be read with
`run-as`.
