# Launcher raster-cache device investigation — 2026-10-23

## Conclusion

Keep full-resolution raster caching in Flutter. On this device, the complete
current launcher fits comfortably in a single full-resolution cache: **11.36 MiB
for all 58 combined tiles and three tab labels**. Warm paging and complete
repeat scrolling required **zero additional captures and zero evictions**.

A low-resolution/full-resolution streaming system is not justified by this
working set. The more useful work is productionizing retention, accounting for
appearance generations, handling memory pressure, and improving first-use capture.

The existing 128-entry limit is sufficient for the current single-style data set.
Increasing it to 1024 did not improve ordinary warm paging. It did prevent
recapture when returning to a previously used appearance after multiple changes.

This is an investigation report, not an implemented cache-policy change.

## Device and original configuration

| Property | Observed value |
| --- | --- |
| Device | ALLDOCUBE iPlay60 mini Turbo |
| Android / ABI | Android 14 / arm64-v8a |
| SoC property | SM6450 |
| Physical display | 1200 × 1920, 320 dpi, 90 Hz |
| Test orientation | Landscape |
| Flutter logical viewport / pixel ratio | 960 × 600 / 2.0 |
| RAM reported by `/proc/meminfo` | 7,678,032 KiB, approximately 7.32 GiB |
| Available RAM at initial inspection | 4,112,316 KiB, approximately 3.92 GiB |
| Initial thermal status | 0; no reported thermal throttling |
| Original mode | Paged, tiles layout, gradient edge fade |
| Original appearance | Icons and labels visible, shadows enabled, blur 0 |
| Icon / label size | 40 / 14 logical pixels |
| Columns | 6 landscape, 4 portrait |
| Flutter | 3.47.7 stable; Dart 3.13.5 |

The display's nominal 90 Hz frame interval is **11.11 ms**. Flutter's integration
performance summary uses a fixed **16 ms** budget in this Flutter version
(`flutter_test/lib/src/frame_timing_summarizer.dart`); those counts alone cannot
establish smooth 90 Hz rendering.

Every visible group was exercised, including the empty Home group:

| Tab | Items |
| --- | ---: |
| Apps | 39 |
| Home | 0 |
| Games | 19 |
| Total | 58 |

## Method

Two profile-mode integration runs used the real Rust backend, installed apps,
existing groups, and persisted native icon cache. They did not create synthetic
apps or alter group membership.

A temporary diagnostic harness and temporary raster-cache counters recorded:

- Actual captured image dimensions and estimated pixel bytes (`width × height × 4`).
- Retained entries, live images including evicted-but-displayed images, captures,
  evictions, unique signatures, and peak live estimated image bytes.
- Source-icon bytes, Flutter decoded-image-cache bytes, and process RSS.
- Flutter frame build/raster timings and, in the follow-up, counts above 11.11 ms.

Both runs forced paged mode, icons and labels visible, shadows enabled, and blur
5 to exercise the known expensive shadow path. The original stored blur was 0;
these measurements deliberately tested blurred rendering rather than assuming
blur was already enabled on the installed configuration.

Each variant visited every tab in order, flung each scrollable grid until its
bottom was reached, and scrolled back toward the top. It then repeated the full
circuit and performed three further paging-only circuits. All 58 item tiles were
captured, demonstrating coverage beyond the empty Home page and beyond just the
initial visible rows.

The first run compared live rendering, separate icon/label caching, combined
caching at the existing 128-entry cap, and combined caching at an experimental
1024-entry cap. The latter changed only capacity, not resolution or rendering.

The follow-up compared the two combined-cache capacities, added 12 horizontal
flings of 420 logical pixels at 2400 logical pixels/second, and verified that all
tabs were visited. Each fling settled before the next; this was not an overlapping
gesture/reversal stress test. It then changed blur 5 → 6 → 7 → 5, traversing and
scrolling every group at each setting.

The first circuits were cold **raster** caches, not clean-install/process-start
benchmarks: icons could already be warmed, and the native disk cache was retained.

## 1. Full-resolution memory working set

| Retained content, one appearance | Entries | Estimated bytes | MiB |
| --- | ---: | ---: | ---: |
| Combined tiles plus tabs | 61 | 11,908,928 | 11.36 |
| Separate icons, labels, and tabs | 119 | 2,614,816 | 2.49 |
| Source-icon cache, either mode | 58 providers | 1,484,480 | 1.42 |
| Flutter decoded-icon cache, either mode | 58 | 1,484,480 | 1.42 |
| Combined raster + source + decoded estimates | — | 14,877,888 | 14.19 |
| Separate raster + source + decoded estimates | — | 5,583,776 | 5.33 |

Each combined item capture was **320 × 160 physical pixels**, or 204,800 bytes
(200 KiB). There were 58. The tab images were 60 × 36, 70 × 36, and 82 × 36,
adding 30,528 bytes in total. Separate icon images were 80 × 80; label dimensions
varied with their text.

These are pixel-payload/accounting estimates, not exact graphics-driver allocation
sizes or total application memory. They exclude GPU allocation alignment,
intermediate render targets, layers, engine caches, object overhead, and other
application allocations. Source-byte counters also do not account for every
possible reference retained elsewhere.

Process RSS varied substantially with profile/integration machinery and garbage
collection. For example, the first run's RSS after warm paging was 257,495,040
bytes in combined-128 mode versus 266,571,776 in separate mode despite separate
images having fewer pixel bytes. This is **not evidence that separate caching
uses more graphics memory**. Do not infer mode-specific cache costs by subtracting
these sequential whole-process samples.

Android `dumpsys meminfo` reported Graphics as zero even while Flutter was running;
that field is not a trustworthy measurement of these Vulkan image allocations on
this device. A profile-process sample reported 141,921 KiB PSS / 265,052 KiB RSS;
the pre-investigation installed app sample reported 80,138 KiB PSS / 178,768 KiB
RSS. Different builds and workloads make these unsuitable for an isolated cache
overhead comparison.

## 2. Paging and scrolling results

First run, all at blur 5. Times below are milliseconds; captures are additional
captures during that measured action (the three tab images were already warm).

| Rendering | Action | Raster p99 | Worst raster | New captures | Evictions |
| --- | --- | ---: | ---: | ---: | ---: |
| Live | First full circuit and scroll | 88.757 | 111.471 | 0 | 0 |
| Live | Repeat full circuit and scroll | 83.632 | 107.730 | 0 | 0 |
| Live | Warm paging | 102.618 | 133.467 | 0 | 0 |
| Combined, 128 | First full circuit and scroll | 10.922 | 57.415 | 58 | 0 |
| Combined, 128 | Repeat full circuit and scroll | 6.827 | 8.160 | 0 | 0 |
| Combined, 128 | Warm paging | 8.095 | 8.136 | 0 | 0 |
| Separate, 128 | First full circuit and scroll | 37.127 | 60.927 | 116 | 0 |
| Separate, 128 | Repeat full circuit and scroll | 7.171 | 11.815 | 0 | 0 |
| Separate, 128 | Warm paging | 9.653 | 9.940 | 0 | 0 |
| Combined, 1024 | First full circuit and scroll | 8.638 | 62.786 | 58 | 0 |
| Combined, 1024 | Repeat full circuit and scroll | 7.440 | 7.638 | 0 | 0 |
| Combined, 1024 | Warm paging | 8.262 | 8.432 | 0 | 0 |

Warm combined caching reduced raster p99 by roughly **12.7×** relative to live
blurred rendering in this run. Both cached modes eliminated the persistent large
blur stalls after warming. Small differences between cached modes/capacities are
not statistically established performance differences.

**First use is still expensive.** In the follow-up, combined-128's first full
circuit had raster p99 37.762 ms and worst 77.981 ms; its repeat circuit was
7.342 / 7.700 ms. A handful of cold frames can substantially change p99 in a small
sample. Increasing retention capacity does not remove the first render/capture.

### Follow-up: actual swipes and 90 Hz accounting

| Combined capacity / action | Raster p99 | Worst raster | New captures | Raster frames >11.11 ms / collected timing frames | Build frames >11.11 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| 128 / repeat full circuit | 7.342 | 7.700 | 0 | 0 / 162 | 0 |
| 128 / warm paging | 7.744 | 8.263 | 0 | 0 / 154 | 2 |
| 128 / fast horizontal swipes | 11.397 | 14.938 | 0 | 14 / 1142 | 6 |
| 1024 / repeat full circuit | 7.791 | 13.844 | 0 | 2 / 170 | 1 |
| 1024 / warm paging | 8.179 | 8.804 | 0 | 0 / 149 | 4 |
| 1024 / fast horizontal swipes | 11.473 | 14.691 | 0 | 13 / 1146 | 3 |

None of these actions evicted any entries. All reported zero integration-summary
16 ms raster-budget misses, but the explicit 90 Hz counters show occasional
smaller misses during swipes. The additional timing callback and integration
summary have slightly different batching boundaries, so their frame counts differ.

These residual frames occurred with **zero recaptures**, so simply increasing
capacity or providing low-resolution fallbacks would not address them. The data
does not isolate their remaining cause; paging composition and UI build work are
candidates for subsequent profiling, not established causes.

## 3. Appearance generations are a real retention issue

After warming blur 5, the follow-up traversed every group at blur 6, blur 7, and
then blur 5 again. Tab images could capture during the settings change before the
measured circuit; circuit capture counts below refer to the 58 item tiles.

| Capacity / destination blur | Retained entries after circuit | Retained MiB | Item captures during circuit | Circuit evictions | Raster p99 / worst ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| 128 / 5 initially warmed | 61 | 11.36 | 58 on first circuit | 0 | 37.762 / 77.981 |
| 128 / 6 | 122 | 22.71 | 58 | 0 | 34.107 / 69.694 |
| 128 / 7 | 128 | 23.33 | 58 | 55 | 35.456 / 61.355 |
| 128 / back to 5 | 128 | 23.33 | 58 | 58 | 39.664 / 64.832 |
| 1024 / 5 initially warmed | 61 | 11.36 | 58 on first circuit | 0 | 41.848 / 58.694 |
| 1024 / 6 | 122 | 22.71 | 58 | 0 | 39.334 / 67.058 |
| 1024 / 7 | 183 | 34.07 | 58 | 0 | 36.839 / 63.821 |
| 1024 / back to 5 | 183 | 34.07 | **0** | 0 | **6.010 / 8.471** |

The 128-entry cache's peak live estimated image bytes during this sequence were
24,667,584 (23.52 MiB), including the short insertion-before-eviction peak. The
1024-entry cache retained all three generations at 35,726,784 bytes (34.07 MiB).
At sampled boundaries there were no evicted-but-still-displayed images; that does
not prove this category is always zero under larger workloads.

This supports retaining complete useful working sets, but not retaining every
historical appearance indefinitely. Keeping a previous appearance can be useful
while the user is experimenting: immediately discarding all old appearances would
also require captures when returning to one. A production policy should explicitly
balance current-style priority with bounded recent-style reuse.

## 4. Code observations relevant to the next change

These are source observations, not independently isolated device experiments:

- `lib/src/ui/raster_tile.dart` currently bounds only entry count (128), not bytes.
  It can evict a referenced entry; reference counting protects the displayed image
  from disposal, but the entry is no longer discoverable by a later cache lookup.
- Appearance values live inside signatures, so old style generations remain until
  LRU eviction. There is no explicit current/recent-generation retention policy.
- Every missed tile schedules its own post-frame capture. There is no shared
  capture-work scheduler. Whether throttling captures improves first use needs a
  benchmark: delayed capture also prolongs live blurred-text rendering.
- `AppCell` resets `_iconReady` on a newly mounted cell. `RasterTile` bypasses
  lookup when `cacheable` is false, even if an already complete matching image
  exists. Safely allowing a valid existing hit while only gating new captures is
  a promising focused experiment; this investigation did not isolate its cost.
- `HomeScreen.didHaveMemoryPressure` clears source/decoded icon caches but does
  not trim the raster cache. Raster signatures contain image providers, which can
  themselves retain source bytes. Clearing the source map is not necessarily
  equivalent to releasing all source buffers.
- Icon warming already covers the current, neighboring, then remaining groups.
  It warms providers/decoded icons, not finished blurred labels or combined tiles.
- Raster caching remains opt-in through `TILE_RASTER`; the source default is
  `none`. A supported production configuration should make the selected policy
  deliberate rather than relying on a spike build flag.

## Recommendations, in priority order

1. **Keep one full-resolution tier and retain all current launcher tiles when
   they fit.** This device's entire measured combined working set is small. Do
   not introduce low-resolution streaming based on this evidence.
2. **Add real byte accounting alongside a generous retention target.** Include
   retained, displayed-but-retired, and peak image bytes; keep source/decoded
   counters separate. A 32–64 MiB raster target is a candidate range, not a tested
   universal default. At the measured geometry, 32 MiB holds two full styles and
   64 MiB holds three with substantial headroom. Larger app sets, sizes, and
   orientations still need measurement. The experiment changed entry capacity,
   not an actual byte-budget policy.
3. **Prioritize complete current-style reuse and bound historical styles.** A
   small recent-style allowance can prevent the measured 5 → 6 → 7 → 5 recapture
   churn. Protect visible/transition images and avoid throwing away discoverable
   active entries merely to satisfy a soft target.
4. **Address cold capture separately from retention.** Investigate using valid
   raster hits before icon readiness gates new captures, then benchmark controlled
   warm-up/capture scheduling. Do not assume one capture per frame is better.
5. **Add raster-aware memory-pressure handling.** Trim unused/recent historical
   images first, preserving displayed images and rebuilding gradually afterward.
6. **Keep separate caching available as a memory-saving option.** Its measured
   image payload is about 78% smaller, and its warm behavior was good. It nearly
   fills the 128-entry limit at only 58 items, illustrating why entry count alone
   is a poor memory policy. Combined remains a sensible smoothness-first choice;
   the small warm timing differences here do not establish a universal winner.
7. **Investigate residual 90 Hz swipe misses with a timeline if needed.** The
   measured zero recaptures mean more capacity alone is not the answer there.

## Scope and evidence limits

- Measurements cover the current 58 items and all three tabs in landscape, not
  hundreds of apps, portrait/rotation transitions, larger fonts/icons, every blur
  value, or a low-memory device.
- There was no low-resolution prototype, memory-pressure injection, clean-install
  startup benchmark, or overlapping-gesture stress test.
- Variants ran sequentially. Cache counters and dimensions are direct observations;
  small timing/RSS differences can include warm-up, GC, and run-order effects.
- Diagnostic accounting adds some capture-path work, so cold frame times are not
  a perfectly uninstrumented production benchmark.
- Both profile integration runs completed with `All tests passed`.

Local raw logs are `/tmp/opencode/qlaunch-cache-investigation.log` and
`/tmp/opencode/qlaunch-cache-followup.log`. Memory snapshots are
`/tmp/opencode/qlaunch-memory-before.txt` and
`/tmp/opencode/qlaunch-memory-profile.txt`. These temporary artifacts are not
versioned; the key observations and limitations are preserved in this report.

Commands used for the temporary diagnostic harness:

```sh
JAVA_HOME=/opt/jbr-21 PATH="/opt/jbr-21/bin:$PATH" \
  flutter drive --profile --no-dds -d 6bfcc911 \
  --driver=test_driver/performance.dart \
  --target=integration_test/cache_investigation_test.dart

JAVA_HOME=/opt/jbr-21 PATH="/opt/jbr-21/bin:$PATH" \
  flutter drive --profile --no-dds -d 6bfcc911 \
  --dart-define=FOLLOWUP=true \
  --driver=test_driver/performance.dart \
  --target=integration_test/cache_investigation_test.dart
```

The temporary harness/instrumentation is removed after measurement. Its behavior
and variant definitions are described above; these commands require recreating
that diagnostic harness rather than running against an unchanged checkout.
The existing broader benchmark remains in
`integration_test/group_transition_performance_test.dart`.

## Cleanup and verification

- The temporary test harness and cache instrumentation were removed.
- `git diff --exit-code -- lib/src/ui/raster_tile.dart` confirmed the production
  cache file exactly matches its pre-investigation tracked state.
- The integration teardown restored the original settings fields changed by the
  harness, including blur 0, and completed successfully in both runs.
- The exact pre-investigation APK was saved before profiling and reinstalled with
  `adb install -r`; installation returned `Success`. The launcher was then started
  and confirmed as the focused activity with a running process.
- Only this report is left as a workspace change. No retention/rendering policy
  changes have been implemented.
