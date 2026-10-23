# Pixel app-return stutter investigation — 2026-10-23

## Finding

**Normal app switching is clearing the launcher's warm caches.** On the connected
Pixel 8, opening Calculator generates Android `onTrimMemory(20)`
(`TRIM_MEMORY_UI_HIDDEN`). This notification means the UI is hidden; it does not
establish that the system is low on memory.

Flutter forwards this level as a framework memory-pressure warning. Our
`HomeScreen.didHaveMemoryPressure` then evicts unused raster images, marks pinned
images for removal when released, releases source icon providers, and clears
Flutter's decoded image cache. Returning and swiping forces expensive tile
rendering/capture again. Repeated use warms the cache until the next app switch.

A minimal diagnostic experiment filtering **only level 20** at the Android
activity boundary prevented the cache clear and eliminated the large reproduced
raster stalls in two app-return cycles.

## Device and method

- Pixel 8 (`shiba`), Android 17, arm64; connected serial `3B031FDJH001KZ`.
- Flutter 3.47.7, profile build, production entry point and real persisted data.
- Observed refresh rate: **60 Hz** (16.67 ms frame budget).
- Existing appearance retained: icon size **54**, label size **18**, shadow blur
  **2**, portrait columns **3**, paged mode; DPR **2.625**.
- Seven groups, containing 24/9/15/9/9/9/4 items (79 total).
- Temporary instrumentation logged native trim levels, Dart lifecycle/pressure
  callbacks, store counters/bytes, decoded-image bytes, and Flutter frame timings.
- Each swipe phase issued 14 leftward horizontal swipes (940,1150 → 100,1150,
  120 ms), circulating through all seven pages. Each app-return phase opened
  Calculator for three seconds, pressed Home, and immediately performed the same
  swipes. Warm-only phases preceded and separated the return phases.
- Repeated the sequence after adding one experimental condition:
  `if (level == TRIM_MEMORY_UI_HIDDEN) return` before `super.onTrimMemory(level)`.
- No settings or group membership were changed. This was horizontal page
  coverage, not a full vertical traversal of every group.

## Direct evidence

First unfiltered app switch:

1. Warm cache: **102 entries / 38,310,712 bytes**, zero capacity evictions.
2. Native callback: **`onTrimMemory level=20`**.
3. Dart memory-pressure callback received during the app-switch sequence.
4. After the handler: **19 entries / 6,420,000 bytes**, **83 evictions**.
5. Decoded icon cache: **6,290,600 bytes → 0**.
6. Return/swipes: **128 additional successful captures** and worst raster frame
   **210.240 ms**.

The next unfiltered switch again generated level 20, removed 70 unpinned images,
and led to 111 additional captures and a **320.772 ms** raster frame.
The launcher process survived both cycles; this was not a process restart.

| Profile scenario | Additional successful captures | Additional evictions | Worst raster frame |
| --- | ---: | ---: | ---: |
| Original: warm swipes before app switch | 0 | 0 | 8.299 ms |
| Original: Calculator return 1 + swipes | 128 | 83 | 210.240 ms |
| Original: warm swipes after return | 0 | 0 | 11.239 ms |
| Original: Calculator return 2 + swipes | 111 | 70 | 320.772 ms |
| Level-20 filter: warm swipes before app switch | 0 | 0 | 10.525 ms |
| Level-20 filter: Calculator return 1 + swipes | 0 | 0 | 11.419 ms |
| Level-20 filter: warm swipes after return | 0 | 0 | 11.106 ms |
| Level-20 filter: Calculator return 2 + swipes | 0 | 0 | 8.734 ms |

In the filtered build, level 20 still appeared in the native diagnostic log, but
there was no Dart memory-pressure event. Cache retention stayed at **83 entries /
31,921,200 bytes** throughout both return cycles. Decoded images also remained
cached. Initial cache populations differed between launches; the decisive
comparison is the within-run cache loss/capture deltas at each switch.

These are observed worst frames in short runs, not population p99 estimates or a
guarantee that all gestures always meet their deadline. The probe also counted
frames over 8.333 ms; that diagnostic threshold is stricter than the device's
observed 60 Hz budget and must not be called a missed-frame count. Some capture
attempts failed during swipes in both builds; these were not independently
investigated and did not prevent the filtered run from retaining its successful
captures across app switches.

## Code path and why larger tiles expose it

- `android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt` inherits Flutter's
  trim handling without distinguishing UI-hidden notifications.
- Flutter 3.47.7's `FlutterActivityAndFragmentDelegate.onTrimMemory` sends low
  memory/framework pressure warnings when a first frame has rendered and
  `level >= TRIM_MEMORY_RUNNING_LOW` (10). Level 20 passes this numerical check.
  Reference: Flutter repository, tag 3.47.7,
  `engine/src/flutter/shell/platform/android/io/flutter/embedding/android/FlutterActivityAndFragmentDelegate.java`.
- `lib/src/ui/home_screen.dart`, `didHaveMemoryPressure`, applies the aggressive
  pressure response to that warning.
- `lib/src/ui/raster_image_store.dart`, `trimForMemoryPressure`, evicts all
  unpinned entries and marks pinned entries `discardOnRelease`.
- `lib/src/ui/raster_cache_identity.dart` includes the icon provider in combined
  tile identity. Releasing source providers can also change future tile keys.
- `lib/src/ui/raster_tile.dart` renders and synchronously submits captures for
  missed visible tiles. Real GPU work is not represented by the small synchronous
  submission-time counter; Flutter raster-frame timings show the resulting stalls.

The measured retained raster footprint was roughly **30–37 MiB**, below the
**64 MiB** target. Warm navigation had no capacity evictions. Increasing capacity
would not prevent this explicit pressure trim.

Larger icons/text increase full-resolution image payload and the work involved
in recreating tiles. The phone's blur setting also makes uncached labels more
expensive to render. This explains why the trigger is more noticeable with larger
appearance settings, but a controlled size/blur sweep was not performed. The
tablet's app-switch callbacks were not measured in this session. Earlier tablet
benchmarks principally covered in-launcher navigation and injected pressure, not
this natural app-switch sequence.

## Recommended implementation

Distinguish Android's **UI-hidden lifecycle hint** from **actual memory pressure**
before it becomes Flutter's untyped memory-pressure message. Preserve the bounded
working set on ordinary app switches, while retaining handling of genuine
low-memory notifications. Review the Android forwarding path when implementing
this so necessary renderer/plugin lifecycle handling is preserved.

Add a regression check around native trim-level classification and an on-device
app-return scenario checking cache deltas and raster timings at the enlarged
appearance. Validate genuine pressure recovery separately. A larger cache or
lower-resolution tile tier is not indicated by this reproduction.

## Cleanup

The temporary Dart probe and native logging/filter were removed. The original
installed APK was saved before profiling and restored afterward with
`adb install -r`. Only this report is retained in the repository; the experimental
condition is not a production fix.

## Implemented fix and verification

After the investigation, the requested production fix was added:

- `MainActivity.onTrimMemory` filters only `TRIM_MEMORY_UI_HIDDEN`, using the
  native `shouldForwardMemoryTrim` policy. Every other level is forwarded to
  Flutter unchanged. Existing `onStop`/`onResume` handling remains in place,
  including Flutter's renderer cleanup on stop.
- `MemoryTrimPolicyTest` verifies that UI-hidden is excluded and genuine pressure,
  moderate, background and unknown levels are forwarded. The UI-hidden regression
  was observed failing with unconditional forwarding before implementing the fix.
- No eager rendering of hidden groups was added. The existing in-memory store
  retains successfully rendered tiles under its existing limits; the icon warmer
  uses visible `pageIds`.

A fresh profile verification on the unlocked Pixel, with its original appearance,
repeated the seven-page swipe/Calculator-return sequence. Both returns retained
**102 entries / 38,310,712 bytes**, with **zero new successful captures / zero
evictions**, and worst raster frames **9.719 ms / 8.758 ms**. Script assertions
checked cache deltas and absence of framework memory-pressure events during both
returns.

Then `adb shell am send-trim-memory bitshift.qlaunch RUNNING_CRITICAL` exercised
real native pressure forwarding. A Dart memory-pressure event was received; the
store shrank from **102 to 25 entries**, evicting **77 unused entries** while
retaining displayed images. Decoded icon bytes fell to zero. Both pressure-event
delivery and actual eviction were asserted.

Checks:

- Android JVM suite: **6 tests passed** (`:app:testDebugUnitTest`).
- Flutter suite: **353 passed, 1 failed**. Existing failure:
  `test/home_screen_test.dart`, `icon prefetch scrolling pauses speculative loads
  until it ends` (line 822). It also fails in isolation; the Android-only change
  does not run in this host widget test. No Dart production code was changed.
- Root `flutter analyze` is blocked by unresolved dependencies in the nested
  `rust_builder/cargokit/build_tool` package, plus existing lint diagnostics.
  Scoped analysis of `lib integration_test test` has no errors, with one existing
  unused-import warning and four informational diagnostics.
- The ARM64 release APK built successfully from `lib/main.dart`.

Temporary verification instrumentation was removed. The fixed release supersedes
the original APK restoration described in the investigation cleanup above.
