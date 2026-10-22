# Continuous scroller performance — 2026-10-22

## Full-resolution raster-cache benchmark

`integration_test/raster_cache_performance_test.dart` visits every visible tab,
scrolls every nonempty group to the bottom, and measures cold/repeat circuits,
warm paging, fast swipes, immediate reversals, and blur 5 → 6 → 7 → 5. Home being
empty does not reduce coverage. Settings changed by the benchmark are restored.

```sh
JAVA_HOME=/opt/jbr-21 PATH="/opt/jbr-21/bin:$PATH" \
  flutter drive --profile --no-dds -d DEVICE \
  --driver=test_driver/performance.dart \
  --target=integration_test/raster_cache_performance_test.dart
```

`RASTER_SAMPLES` defaults to 3 with alternating mode order; `RASTER_MODES` defaults
to `all` or accepts a single mode (`none`, `separate`, `combined`). Set
`--dart-define=RASTER_TRACE=true` for an additional Dart/Embedder/GC trace.
Each `RASTER_RESULT` JSON line contains exact action-window timing percentiles,
counts over the display's refresh budget and 16 ms, image-payload byte estimates,
captures/evictions, and separate source/decoded icon bytes. Byte estimates are not
whole-process or graphics-driver allocation measurements. Only consumer lookups
count as hits/misses; internal capture deduplication does not.

## Follow-up: Games ↔ Apps stutter in paged mode

The device was in **paged mode** when the remaining stutter was reported. The
continuous-scroller measurements below did not cover that rendering path. A new
benchmark repeatedly alternates Games ↔ Apps, verifies the destination after
each tap, and tests all four icon/text visibility combinations in both modes.

### Root cause

The main repeatable stall was the **blurred text shadow**, not Rust bridge work.
With the original `blurRadius: 2`, paged transitions reached 106–116 ms raster
p99 and worst frames up to 142 ms. Disabling icons did not remove the stall;
disabling label shadows did. Removing only the edge fade left ~104 ms worst
frames. Turning off shadows while keeping the edge fade reduced worst frames
to ~13 ms. UI build time was much smaller than raster time.

The collected Embedder timeline shows the Impeller/Vulkan graphics path
(`AllocatorVK`, `SurfaceFrame::Encode`, `SurfaceFrame::Submit`), GPU submission
waits and display lag. Real-backend instrumentation recorded **zero icon fetches
and zero snapshot calls during the warm transition samples**, including the
bad samples. This rules out those bridge calls as the cause of the reproduced
repeated warm-transition stalls; it does not claim every possible startup or
package-update workload is free of bridge cost.

### Change and results

The shadow remains enabled and contrasting (light for black text, dark for
light text), with its original one-pixel offset. Its blur is removed, producing
a crisper shadow instead of a soft halo. The page crossfade and edge fade stay
enabled. No Rust/API/cache changes are needed for this fix.

Same tablet, profile mode, 39 Apps items and 19 Games items, ten back-and-forth
pairs per sample. These are raster p99 times in milliseconds:

| Mode / content | Original blurred shadow | Sharp shadow |
| --- | ---: | ---: |
| Paged, icons + text | 106.201 | 9.601 |
| Paged, text only | 115.704 | 6.153 |
| Paged, icons only | 11.565 | 8.016 |
| Paged, neither | 5.797 | 1.903 |
| Continuous, icons + text | 15.361 | 3.804 |
| Continuous, text only | 14.991 | 3.737 |
| Continuous, icons only | 4.239 | 2.909 |
| Continuous, neither | 6.450 | 7.558 |

### Shadow blur setting

Shadow blur is now a user setting (`label_shadow_blur`, 0-10, default 0) with a
slider under Text. At 0 the shadow stays the sharp offset that measured well
above. Raising it brings the old per-label GPU cost back for live text, so the
slider is meant to be used with the whole-tile raster cache
(`--dart-define=TILE_RASTER=combined` for a test build; the default remains
live text). The cache key includes the blur value, so each new value captures
the visible tiles once and then reuses the image.

Tile caching was compared in all four icon/text combinations. With both on,
build p99 fell from ~12.7 to ~9.7 ms while raster p99 stayed at ~8.6-9.3 ms
with zero raster misses; sparse tiles got slower (empty cells raster average
1.09 -> 3.01 ms, icons only 2.72 -> 3.91 ms), which is why combined caching is
still opt-in. The cache waits for icon frames to decode before capturing (a
capture taken during decoding freezes an empty icon), and keeps at most 128
ref-counted entries so a blur slider cannot accumulate hundreds of MB.

The tab strip and the continuous-scroll section headings use the same cache, so
every visible label is covered. Tab fade is applied with `Opacity` outside the
image, which keeps the animated alpha from triggering recaptures while paging.
With labels only and blur 5, live text measured raster p99 113.2 ms (56 budget
misses); separate and combined caching measured 5.8 ms and 6.9 ms with no
misses. Captures for these fixed widgets all happen during warm-up.

All eight post-fix samples had zero raster-budget misses. Minor differences in
the empty/icon-only cases are run-to-run variation, not evidence that the shadow
change speeds up content that has no label shadow to draw.

Additional post-fix checks:

- Cold Dart source and decoded-image caches, persisted native cache retained:
  58 real icon bridge calls, 509 ms **summed asynchronous call latency**, raster
  p99 7.359 ms, worst 9.900 ms, zero raster-budget misses. The single snapshot
  call in this sample follows the explicit icons-enabled settings change.
- Actual horizontal swipe gestures, labels/shadows/icons/fade enabled: raster
  p99 9.726 ms, worst 12.391 ms, zero raster-budget misses and zero icon/snapshot
  calls during the swipes.
- The new on-device regression assertion was observed failing at 109.452 ms
  before the change and passing after it, against a 25 ms raster p99 limit.

### Run the targeted benchmark

```sh
JAVA_HOME=/opt/jbr-21 PATH="/opt/jbr-21/bin:$PATH" \
  flutter drive --profile --no-dds -d DEVICE \
  --driver=test_driver/performance.dart \
  --target=integration_test/group_transition_performance_test.dart
```

Keep the tab strip visible and the Apps/Games groups unhidden. The test uses
real installed apps, temporarily changes visibility/mode/shadow/fade settings,
and restores their original values in teardown. Its measured backend delegates
to the real Rust backend, counting and timing icon and snapshot calls.

`--dart-define=DIAGNOSTICS_ONLY=true` skips the eight-case matrix but retains
the shadow/fade comparisons, cold-cache pass and swipe pass.
`--dart-define=TRACE_TRANSITIONS=true` additionally saves an Embedder/Dart/GC
trace under `paged_trace` in `build/integration_response_data.json`.
`--dart-define=MAX_TRANSITION_RASTER_P99_MS=...` changes the device-specific
regression budget.

## Findings and changes

- The initial on-device prototype was installed in debug mode. Use profile for
  measurements and release for normal use.
- Crossing a section changed HomeScreen's active group, rebuilding GroupScroller,
  recreating every section/row descriptor and the visible list's widget subtree.
  This caused repeatable rendering stalls even after icons were warm.
- GroupScroller now retains its cycle and list subtree until the model snapshot,
  group order or grid geometry changes. Navigation commands still run immediately.
- Each embedded app row previously created its own GridView/Scrollable/viewport.
  Embedded rows now use ordinary equal-width cells, including empty slots in the
  last row. The device's initial view went from six Scrollables to one, with the
  same 25 mounted cells. Icon lookup, launch and long-press handling are shared
  with the regular grids.
- Section headings now have a tight central slot and centered text. Equal-width
  dividers keep the title at the screen center, including short titles.

## Controlled device comparison

iPlay60 mini Turbo, Android 14, profile mode, real installed groups and icons
(58 items, 3 groups). Twelve flings per pass, then twelve section-tab taps.
The following A/B isolates **list reuse**: both runs already use lightweight app
rows; the baseline temporarily disabled reuse. All numbers are milliseconds.

| Metric | Reuse disabled | Reuse enabled |
| --- | ---: | ---: |
| Warm scroll average UI | 1.047 | 0.768 |
| Warm scroll p99 UI | 11.112 | 3.231 |
| Warm scroll average raster | 6.059 | 4.571 |
| Warm scroll p99 raster | 49.514 | 15.746 |
| Warm scroll raster budget misses | 52 / 1213 | 13 / 1288 |
| Section navigation average raster | 7.417 | 4.463 |
| Section navigation p99 raster | 49.296 | 14.829 |
| Section navigation raster budget misses | 28 / 302 | 1 / 354 |

At 60 Hz, one frame has about 16.7 ms. These measurements show substantially
smaller stalls, not a guarantee that every frame meets its deadline.

Disabling the fade reduced the optimized average scroll raster time from 4.571
to 3.871 ms and p99 from 15.746 to 15.382 ms. It is a secondary cost; the fade
remains available. Warm and initial passes are similar, so cold icon loading
does not explain the repeated stalls in this workload.

## Reproduce

```sh
JAVA_HOME=/opt/jbr-21 PATH="/opt/jbr-21/bin:$PATH" \
  flutter drive --profile --no-dds -d DEVICE \
  --driver=test_driver/performance.dart \
  --target=integration_test/scroll_performance_test.dart
```

The benchmark uses `fullyLive` frame policy: animations render on normal engine
frames rather than only test pumps. Earlier exploratory runs with the default
test policy are not the results in the table above. `--no-dds` permits the
on-device integration binding to collect timeline/GC information.

It temporarily enables continuous scrolling, measures initial and warm passes,
section navigation, and a no-fade pass; teardown restores the original mode and
fade settings. Full metrics are written to `build/integration_response_data.json`.
The default warm-scroll raster p99 limit is 25 ms (1.5 frames at 60 Hz); override
with `--dart-define=MAX_SCROLL_RASTER_P99_MS=...` for other performance targets.

## Further work if larger app libraries expose more stalls

- Profile the remaining row-entry frames before adding more caches. Ordinary
  cells still measure label geometry individually; sharing those measurements
  is a candidate if UI work becomes dominant.
- The icon warm queue is rebuilt on section changes. Defer/reprioritize this
  work until scrolling stops if larger libraries make it significant.
- For very long sections or distant jumps, measure a varied-extent sliver with
  known heading/row extents instead of SliverList's dead-reckoning layout.
- A cheaper edge-only fade is worth considering if GPU fill/compositing becomes
  the limiting factor; the current A/B does not justify removing the effect.

## Capture scheduler experiment (Task 6, rejected for visible work)

`RasterCaptureScheduler` deduplicates captures by key, runs visible jobs
post-paint with a per-frame cap and a 2 ms synchronous submission budget, and
prioritises visible over speculative work. The permanent benchmark can select it
with `--dart-define=RASTER_SCHEDULER=immediate|2|4|unbounded`; the default stays
`immediate`.

Three combined-mode samples for `immediate` and `2`, one each for `4` and
`unbounded` (median of the cold full traversal, 90 Hz, all captures drained):

| Variant | cold worst raster | cold raster p99 | cold build p99 | over-budget frames | traversal ms |
| --- | --- | --- | --- | --- | --- |
| immediate | 78.3 ms | 41.8 ms | 16.6 ms | 2 | 1391 |
| 2/frame | 62.7 ms (−20%) | 62.7 ms (+50%) | 18.0 ms | 21 | 1858 (+34%) |
| 4/frame | 62.5 ms (−20%) | 51.9 ms | 14.3 ms | 13 | 1570 (+13%) |
| unbounded | 59.2 ms (−24%) | 39.7 ms | 16.3 ms | 3 | 1355 (−3%) |

Deferring captures lowers cold worst raster by about 20%, but the bounded
variants regress time-to-complete and cold raster p99 well past the 10% gate
(2/frame also trips 21 over-budget frames versus 2). The unbounded variant is
within noise at a single sample and is behaviourally close to immediate.

**Decision:** the Task 6 gate is not met, so production keeps immediate visible
capture (`TileRasterCache.immediateVisibleCapture = true`). The scheduler stays
in the tree for eligible speculative work only (Task 7). This is a retained
negative result, not a shipped optimisation.

## Offscreen warmer experiment (Task 7, rejected)

`AppCellVisual` was extracted from `AppCell` so a warmer could render exactly the
live pixels offscreen through a detached `RenderView`/`BuildOwner`/`PipelineOwner`
pipeline (`renderWarmVisual`). The visual extraction is retained and now backs the
live combined tile; the production warmer is not shipped.

A widget parity test rendered the same `AppCellVisual`, geometry, DPR, theme,
locale and text scale both live and through the detached pipeline and compared
raw RGBA. The pixels differed at a single channel byte (one antialiased edge) in
one of the parity cases, so the **exact-pixel correctness check fails**. Per the
plan, an unreliable-parity detached pipeline is removed rather than shipped at
lower fidelity. No speculative producer is wired up; the Task 6 scheduler remains
dormant infrastructure for a future warmer.

## Residual swipe composition (Task 8)

With the cache fully warm there are zero new captures during swipes, so the
remaining cost is not cache churn. Combined-mode warm swipes (three samples,
90 Hz):

| Scenario | build p99 | raster p99 | raster worst | frames over budget |
| --- | --- | --- | --- | --- |
| fast swipes | 4.4-4.6 ms | 10.8-10.9 ms | 14.3-16.9 ms | 9-10 of ~1144 |
| immediate reversals | 5.9-6.6 ms | 13.5-14.3 ms | 14.8-15.5 ms | 28-31 |

Build stays far under the budget; the small residual is raster/compositing
(~0.8% of fast-swipe frames), dominated by compositing rather than rebuilds. No
pager, tab-strip or fade change is retained: there is no single attributed owner
with a measured improvement, and the plan requires a >=20% reduction in
over-budget frames or >=10% lower raster p99 to keep a composition change. The
existing `group_pager_test.dart` and `tab_strip_test.dart` behavior is unchanged.
Set `--dart-define=RASTER_TRACE=true` to capture a Dart/Embedder/GC timeline if a
future investigation needs finer attribution.

## Selected production policy and acceptance (Task 9)

`main()` now calls `TileRasterCache.configure(RasterCacheConfig(...))` with the
automatic policy: 64 MiB / 4096-entry / two-recent-generation store, immediate
visible capture (the Task 6 scheduler and Task 7 warmer experiments are not
shipped). `--dart-define=TILE_RASTER=none|separate|combined|automatic` overrides
it; `automatic` resolves to **combined when icons and labels are visible**,
**separate when only labels are visible**, and **none when labels are hidden**.
Tabs and headings stay cached whenever caching is enabled. For the default
launcher appearance (icons + labels + shadow visible) automatic resolves to
combined, which is what the numbers below measure.

Profile results on the tablet (90 Hz, 58 items, all tabs/full scrolls, three
combined samples; baseline was `none` before any retention work):

| Scenario | build p99 | raster p99 | frames > 11.11 ms | captures |
| --- | --- | --- | --- | --- |
| warm tab paging | 4.9-5.5 ms | 10.32 / 11.13 / 11.50 ms | 0 / 2 / 2 | +58 first time, 0 after |
| repeat traversal | 4.8-5.6 ms | 9.12-10.00 ms | 0 | 0 |
| fast swipes | 4.4-4.6 ms | 10.8-10.9 ms | 9-10 of ~1144 | 0 |
| blur 5 -> 6 -> 7 -> 5 (final) | 5.0-5.8 ms | 7.9-9.2 ms | 0-1 | 0 on return |
| pressure trim (populated group) | 0.4-3.6 ms | 2.7-6.0 ms | 0 | 0 (revisit 110) |

Functional gates pass: warm complete traversal has **0 new captures / 0
evictions**; blur 5 -> 6 -> 7 -> 5 returns with **0 captures**; displayed images
never corrupt; live bytes return to zero after final releases. Timing: fast
swipes and repeat traversal meet their gates, and no warm frame exceeds 16 ms,
but **warm tab paging narrowly misses the 10 ms median raster-p99 gate** (median
11.13 ms) with 2 of ~150 frames over the 11.11 ms budget. The cause is the same
residual compositing cost Task 8 could not attribute to one owner; retaining the
working set is still strictly better than the pristine baseline's ~96-105 ms
paging stalls. Reported as a miss rather than loosened.

Reproduction:

```sh
flutter analyze
flutter test

JAVA_HOME=/opt/jbr-21 PATH="/opt/jbr-21/bin:$PATH" \
  flutter drive --profile --no-dds -d 6bfcc911 \
  --dart-define=RASTER_MODES=combined --dart-define=RASTER_SAMPLES=3 \
  --driver=test_driver/performance.dart \
  --target=integration_test/raster_cache_performance_test.dart
```
