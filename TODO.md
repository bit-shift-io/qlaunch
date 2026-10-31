# TODO

## Investigate intermittent blank screen in the Iced Android launcher

- [ ] Find and fix the rendering/lifecycle failure observed on Pixel 8 on
  2026-11-01 (`bitshift.qlaunch`, version `1.0.0-iced-spike`; installed Oct 31).

### Observations

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

### Suggested debugging

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

### Local evidence

Captured files are outside the repository under `/tmp/opencode/` (temporary,
not durable): `qlaunch-blank.png`, `qlaunch-blank-process.log`,
`qlaunch-blank-all.log`, `qlaunch-restarted.png`,
`qlaunch-restarted-process.log`, and `qlaunch-restarted-surfaces.txt`.
The pre-replacement Iced APK is `qlaunch-iced-before-replacement.apk` there.
Keep device logs private/review them before sharing. Private configuration could
not be backed up with `run-as` because the installed release was not debuggable.
