# Full-Resolution Launcher Cache Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task, or superpowers:subagent-driven-development if the user chooses delegated execution. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve buttery-smooth paging by retaining complete full-resolution launcher working sets, eliminating unnecessary live blurred-text rendering, and reducing first-use stalls without sacrificing interaction, appearance, or predictable memory use.

**Architecture:** Keep rendering, raster-image ownership, and caching in Dart/Flutter. Replace the spike's entry-only LRU with a byte-accounted, lease-based image store that prioritizes the current appearance and retains two recent appearances when capacity permits. Add reproducible all-tab benchmarks first, then introduce capture/prewarming optimizations behind measured acceptance gates; ship only variants that improve the real device workload.

**Tech Stack:** Existing Flutter 3.47.7 / Dart 3.13.5, `dart:ui`, Flutter rendering/scheduler APIs, `flutter_test`, `integration_test`, existing Rust backend and Android tooling. No new runtime package or Rust bridge API is planned.

**Spec:** `docs/cache-device-investigation-2026-10-23.md`, interpreted with the user's explicit preference for keeping all current full-resolution launcher images in memory to avoid paging stutter. The implementation choices below are proposed engineering decisions, not already measured outcomes.

## Global Constraints

- Retain the existing user-selected font, wrapping, ellipsis, icon opacity, shadow colour, offset, and blur; improve reuse rather than reducing visual quality.
- Keep full-resolution images at the actual device pixel ratio. No low-resolution tier, sharpening transition, persistent rendered-tile cache, or independent Rust text renderer.
- Retain all current launcher tiles whenever the working set fits the proposed **64 MiB raster soft target**. Keep the existing **32 MiB source-icon** and **64 MiB decoded-image** limits initially; measure them separately, not as one guaranteed total-process cap.
- Retain **current plus two recently activated appearance/geometry generations**, subject to byte pressure. **4096 discoverable entries** is a secondary metadata guard, not a preallocation or promise to retain that many images.
- Images currently displayed or being handed to a widget remain valid until the final lease is released, even above the target. Report pinned overage explicitly; never claim the soft target is a hard process/GPU-memory ceiling.
- Priority under capacity pressure is: reclaim obsolete generations; reclaim unused historical images; then unused current-generation images. Pinned images remain discoverable and usable.
- The device report's **11.36 MiB** full-resolution working set is measured only for 58 items, 960 × 600 logical landscape viewport, DPR 2, 40 dp icons, and 14 dp labels. Do not extrapolate universal fit or performance guarantees.
- Frame acceptance uses the display's actual refresh rate: nominal **11.11 ms at 90 Hz** on the current tablet. Flutter's built-in summary uses a fixed **16 ms** threshold and is supplementary.
- Preserve tap, long press, blank-space gestures, accessibility labels, focus/action semantics, continuous scrolling, paging wrap, fades, and reduced-motion behavior.
- Preserve the Rust model as the authority for settings, discovery, organization, and launch behavior. Avoid generated bridge/model edits.
- Save and restore the installed APK and every changed device setting around profile runs. Traverse **all tabs**, including empty Home, and scroll nonempty groups fully.
- Do not commit, install new dependencies, or start implementing this plan as part of merely writing/reviewing it. During execution, commits require separate user authorization.

## Review Focus

1. A cached tile is remounted before its icon readiness callback: show the completed matching raster immediately, but never capture a still-empty icon (Task 3).
2. Pressure, replacement, and async completion overlap a transition: do not dispose a displayed image or publish stale pixels; keep byte counters accurate (Tasks 2, 5, 6).
3. Font scaling, locale/direction, rotation, and page geometry change: cache keys must reflect actual rendered pixels, while ordinary navigation/usage updates must preserve hits (Task 4).
4. More items or an oversized active working set exceed the target: avoid a capture/eviction loop, bound speculative work, and account for pinned overage (Tasks 2, 7).
5. Rapid reverse gestures, hidden routes, backgrounding, and setting changes race prewarming: preserve responsiveness and cancel stale jobs without painting them into the wrong generation (Tasks 6–8).

---

## File/Responsibility Map

| File | Responsibility |
| --- | --- |
| `lib/src/ui/raster_tile.dart` | Existing public facade, widget capture/display, mode selection; delegate storage and scheduling. Keep existing imports usable. |
| `lib/src/ui/raster_cache_stats.dart` — new | Immutable byte/counter snapshots; no per-frame reporting or Flutter state notifications. |
| `lib/src/ui/raster_image_store.dart` — new | Keyed ownership, leases, byte accounting, current/recent-generation retention, pressure trimming. |
| `lib/src/ui/raster_cache_identity.dart` — new | Shared appearance/geometry and per-visual key construction. |
| `lib/src/ui/raster_capture_scheduler.dart` — new, measured gate | Deduplicated visible capture work and speculative idle jobs; cancellation and bounded submission. |
| `lib/src/ui/raster_warmer.dart` — new, measured gate | Offscreen full-resolution rendering using the same visual widgets/keys/constraints as the visible tree. |
| `lib/src/ui/app_cell.dart` | Consume valid cached hits immediately; keep image readiness exclusively as a new-capture gate; extract reusable visual content only for Task 7. |
| `lib/src/ui/app_cells.dart`, `grid_geometry.dart` | Compute shared per-grid geometry once and expose identical geometry to visible and warm rendering. |
| `lib/src/ui/home_screen.dart` | Activate generation; coordinate foreground/route/scroll eligibility; reconcile items; handle pressure. |
| `lib/src/ui/tab_strip.dart`, `group_scroller.dart` | Typed raster identities for tabs and headings; retain live outer animation/gesture layers. |
| `lib/main.dart` | Explicit production cache configuration after acceptance; tests bypassing `main()` remain configurable. |
| `integration_test/raster_cache_performance_test.dart` — new | Permanent real-backend, all-tab memory/performance benchmark. |
| `integration_test/support/raster_performance.dart` — new | Timing collection, refresh-rate budgets, scenario result schema, compact JSON output. |
| `integration_test/group_transition_performance_test.dart` | Preserve the existing visibility matrix; update mode/config access if required. |
| `test/raster_image_store_test.dart`, `raster_tile_test.dart`, `raster_cache_identity_test.dart` — new | Ownership/accounting, warm-hit behavior, key correctness. |
| `test/raster_capture_scheduler_test.dart`, `raster_warmer_test.dart` — gated new | Scheduling/cancellation and offscreen/live visual equivalence. |
| Existing `test/app_cell_test.dart`, `app_grid_test.dart`, `home_screen_test.dart`, `tab_strip_test.dart`, `group_pager_test.dart` | Integration with existing interaction, lifecycle, and layout behavior. |
| `docs/scroll-performance.md` | Supported production configuration, performance results, and reproducible commands. |

### Execution order and experiment rules

Tasks 1–5 establish the reliable production cache. Task 6 tests capture scheduling;
Task 7 tests offscreen prewarming; Task 8 investigates remaining swipe cost. Task 9
selects and enables the measured production configuration. Complete tasks in that
order; do not combine all experiments into one opaque before/after result.

If an experiment fails its gate, retain its findings in the performance document
and remove its production path. The deliverable is the best measured behavior,
not shipping every proposed mechanism. Record the decision before moving on.

## Task 1: Make the device investigation reproducible

**Files:** Create `raster_cache_stats.dart`, `integration_test/raster_cache_performance_test.dart`, and `integration_test/support/raster_performance.dart`; modify the facade in `raster_tile.dart` and `docs/scroll-performance.md`.

**Interfaces:**

- `RasterCacheStats`: immutable `int` fields `entryCount`, `retainedBytes`, `pinnedBytes`, `retiredLiveBytes`, `liveBytes`, `peakLiveBytes`, `hits`, `misses`, `captures`, `evictions`, `captureFailures`, `captureMicros`, `captureMaxMicros`; `Map<String, Object> toJson()`.
- `TileRasterCache.statistics` → `RasterCacheStats`; `TileRasterCache.resetStatistics()` → `void`, resetting event counters and setting the peak to current live bytes without dropping images.
- `RasterScenarioResult`: `String name`, `double refreshRateHz`, `List<FrameTiming> timings`, `RasterCacheStats before/after`, `int sourceBytes/decodedBytes`, `Map<String, Object> toJson()`. Include frame counts, build/raster p50/p90/p99/worst, and counts above `1000000 / refreshRateHz` microseconds.
- `Future<RasterScenarioResult> measureRasterScenario({required String name, required WidgetTester tester, required Future<void> Function() action, required int Function() sourceBytes})` in the integration support file. Read refresh rate from the active Flutter view's display and reject nonpositive values unless an explicit benchmark override is supplied.

- [ ] Add the immutable snapshot and read-only facade instrumentation to the existing cache. Count image bytes once on ownership transfer; keep retired live images accounted until disposal. Distinguish consumer hits/misses from internal capture deduplication lookups. Preserve the current default and 128-entry policy in this task.
- [ ] Write the permanent harness to enumerate `state.pageIds`, print group counts and original settings, navigate every tab, and scroll each grid to both extrema. Assert every group was visited and every nonempty grid reached its bottom. Follow the existing backend-injection pattern; do not hardcode Apps/Games names, group UUIDs, or Home as useful content.
- [ ] Add scenarios for cold raster circuit, repeat circuit, warm tab paging, settled fast swipes, immediate direction reversals, and blur **5 → 6 → 7 → 5**. Record settings-change capture work separately from traversal capture work. Restore only fields the harness changes, in `addTearDown`, before ending the run.
- [ ] Add compact JSON phase summaries with cache snapshots, source/decoded bytes, viewport, DPR, mode, and timings. Flush pending timing callbacks at phase boundaries; use the same collection window for percentiles and refresh-budget counts. Label OS RSS/PSS samples separately from raster byte estimates.
- [ ] Run `dart analyze lib/src/ui/raster_cache_stats.dart lib/src/ui/raster_tile.dart integration_test/raster_cache_performance_test.dart integration_test/support/raster_performance.dart` and the existing focused raster consumers' tests. Expected: no analysis errors and existing tests pass.
- [ ] Save the installed APK and device configuration, then run the new profile harness in legacy `none`, `separate`, and `combined` modes. Expected: reproducible per-phase JSON, full group coverage, all settings restored. Capture at least three samples per main comparison with alternated run order; keep the baseline artifacts before implementing Task 2.

**Checkpoint:** The baseline reproduces the large live-blur stall, and warm combined traversal has no recaptures for the current 58-item working set. A failure to reproduce is a measurement issue to resolve before optimization, not a reason to relax assertions.

## Task 2: Introduce byte-accounted ownership and working-set retention

**Files:** Create `lib/src/ui/raster_image_store.dart` and `test/raster_image_store_test.dart`; modify `raster_tile.dart` and `raster_cache_stats.dart`.

**Interfaces:**

```dart
class RasterCacheKey {
  const RasterCacheKey({required Object generation, required String kind,
    required String owner, required Object pixels});
  // Value equality/hash across all four fields.
}
class RasterImageLease {
  ui.Image get image;
  double get pixelRatio;
  Size get logicalSize;
  RasterCacheKey get key;
  void release(); // Idempotent; exactly one underlying reference released.
}
class RasterImageStore {
  RasterImageStore({int maximumBytes = 64 << 20,
    int maximumEntries = 4096, int recentGenerations = 2});
  RasterCacheStats get statistics;
  void resetStatistics();
  void activateGeneration(Object generation);
  bool contains(RasterCacheKey key); // No hit/miss accounting or LRU touch.
  RasterImageLease? acquire(RasterCacheKey key);
  RasterImageLease insertAndAcquire(RasterCacheKey key, ui.Image image,
    double pixelRatio, Size logicalSize);
  void invalidateOwner(String owner);
  void retainOwners(Set<String> owners);
  bool canWarm({required RasterCacheKey key, required int estimatedBytes});
  void trimForMemoryPressure();
  void clear();
}
```

The facade exposes `RasterImageStore get store`. Add
`RasterCacheScope({required RasterImageStore store, required Object generation,
required Widget child})` in `raster_tile.dart`, with
`static RasterCacheScope? maybeOf(BuildContext context)`. Consumers resolve an
explicit test dependency first, then this scope, then the session facade. The
scope carries dependencies/identity; it does not own or dispose the store.

The store owns an inserted image. A live insert acquires its first lease **before**
trimming; the returned lease is usable even if the image alone exceeds the target.
`clear`/invalidation detach discoverable keys, retiring pinned images until release.
Normal capacity eviction never detaches pinned keys. `retainedBytes` includes
discoverable pinned images; `pinnedBytes` is its pinned subset, not extra memory.
`liveBytes = retainedBytes + retiredLiveBytes`; count each image allocation once,
regardless of the number of leases. Peak includes insertion-before-trim transients.

- [ ] Write behavioral store tests using small real `ui.Image` fixtures created through `PictureRecorder` and `Picture.toImage`; dispose fixture pictures and release every lease. Use test targets small enough to force eviction without allocating large images.
- [ ] Cover unequal-size LRU entries, current-generation preference over recent generations, three complete generations fitting, fourth-generation retirement, generation reactivation, and a target smaller than one displayed image. Assert bytes, discoverability, and final image disposal—not private map layout.
- [ ] Add overlap tests: replacement under a live lease, two leases to one allocation, clear while displayed, repeated release, owner removal, and pinned overage shrinking after release. The old image must remain drawable until its final release, and detached live bytes must eventually reach zero.
- [ ] Run `flutter test test/raster_image_store_test.dart`. Expected initially: missing API/behavior failures. Implement the ownership model, O(1) running byte totals, per-generation LRU, and event counters; avoid recalculating sums over all images during captures or frames.
- [ ] Make `canWarm` decline existing keys and jobs that would require eviction of a current-generation entry or exceed the soft target after reclaiming historical unpinned entries. Speculative work must not churn the current working set. Live rendering remains allowed above the target while pinned.
- [ ] Replace the facade's static map/refcount implementation with one session store, preserving mode selection until Task 9. Use leases in `RasterTile`; eliminate duplicate image-disposal paths. Provide an injectable store for focused tests and the benchmark rather than changing global constants to simulate capacity. Until Task 4 migrates consumers to typed keys, adapt the old signature to `RasterCacheKey(generation: 0, kind: 'legacy', owner: 'legacy', pixels: signature)`; remove this adapter in Task 4.
- [ ] Run `flutter test test/raster_image_store_test.dart test/app_cell_test.dart test/app_grid_test.dart test/tab_strip_test.dart test/home_screen_test.dart`. Expected: ownership and existing consumer behavior pass.

**Checkpoint:** At one appearance the current device retains all 61 measured combined entries without eviction. Three measured styles occupy about 34.1 MiB and returning to the first requires zero new item captures under the 64 MiB target.

## Task 3: Make completed cache hits bypass icon readiness

**Files:** Modify `raster_tile.dart` and `app_cell.dart`; create `test/raster_tile_test.dart`; extend `test/app_cell_test.dart`.

**Interfaces:** Rename `RasterTile.cacheable` to `captureReady` at all call sites. Its meaning is solely permission to create a **new** image, not permission to reuse an existing one. Add optional `RasterImageStore? store` injection for standalone widget tests; the session facade is the fallback.

- [ ] Write a regression test with a matching prepopulated raster and `captureReady: false`: the very first build displays `RawImage`, does not mount the live visual child, and does not schedule a capture. Remount the same item and assert capture count is unchanged.
- [ ] Write the complementary miss test: false readiness displays the live child; changing readiness to true permits capture only after paint. Hold an image provider's completion to prove no empty icon is frozen; complete or fail it and verify the final icon/placeholder is captured correctly.
- [ ] Add tests for a ready-image lease surviving a widget rebuild/capacity trim, signature change during pending capture, unmount before callback, and capture failure followed by a later successful frame. Failures count in diagnostics and do not enter an infinite post-frame retry loop; requeue only after a new frame/readiness/signature opportunity.
- [ ] Run `flutter test test/raster_tile_test.dart test/app_cell_test.dart` and observe the new regression fail against the old ordering.
- [ ] Implement build order: validate/reuse the widget's current matching lease; otherwise acquire a store hit; only on a miss check `captureReady` and schedule capture. Check mounted state, generation/key, constraints, paint readiness, and cache mode again before and after asynchronous capture.
- [ ] Keep gestures/semantics outside cached content as today. Add cached-mode tap, long-press, label-semantics, icon failure, and uninstalled-opacity assertions. Do not make the production path rebuild repeatedly just to resolve a cache hit.
- [ ] Run the focused tests, then repeat warm paging/remount scenarios on device. Expected: zero warm captures; no temporary live blurred-label subtree for matching cached hits. Record the timing effect independently of later scheduler changes.

## Task 4: Make identity, invalidation, and geometry reuse deliberate

**Files:** Create `raster_cache_identity.dart` and `test/raster_cache_identity_test.dart`; modify `app_cell.dart`, `app_cells.dart`, `grid_geometry.dart`, `home_screen.dart`, `tab_strip.dart`, and `group_scroller.dart`.

**Interfaces:**

- `Object rasterGeneration(BuildContext context, Settings settings)` returns a value-equal record of actual pixel-affecting appearance/environment: effective cache mode, layout, icon/label visibility and sizes, icon alpha, label typeface/colour/shadow/blur, active column count, logical viewport, DPR, safe-area padding, effective scaled label size, locale, text direction, and effective default font/style inputs used by these visuals. Do not use snapshot identity, current group, animation alpha, usage counts, or wallpaper alpha.
- `AppCellGeometry appCellGeometry(BuildContext context, Settings settings, BoxConstraints constraints, {double? measuredLabelHeight})` returns immutable `cellSize`, `iconSide`, `labelHeight`, `devicePixelRatio`, and `scaledLabelSize`. Pass the grid's once-measured label height in the regular path; compute it only when omitted for standalone cells. Preserve the existing formulas in `grid_geometry.dart`.
- `RasterCacheKey appCellRasterKey({required Object generation, required Item item, required ImageProvider? icon, required Settings settings, required AppCellGeometry geometry, required String kind})` supports kinds `tile`, `icon`, `label`; carries every pixel-affecting item/provider input and exact constraints. Label keys may omit icon-provider identity when the icon cannot affect their pixels.
- `RasterCacheKey labelRasterKey({required Object generation, required String kind, required String owner, required Object pixels})` supports kinds `tab`, `heading`. All identities use owner namespaces `item:<id>`, `group:<id>`; tabs and headings share the group owner but differ in kind.
- Export a complete `RasterCacheKey` from `raster_tile.dart` for each `RasterTile` constructor. Retire the untyped `signature` parameter after updating all consumers and the benchmark.

- [ ] Write table-driven identity tests: blur/size/font/colour/icon replacement, bounds, DPR, text scaling, locale/RTL, and orientation changes miss; paging position, outer opacity/fade animation, usage updates, and group order changes do not change pixels/keys. Preserve separate variants when actual geometry differs.
- [ ] Add a home-widget test that activates one shared generation for tabs and grid; different cell bounds must not repeatedly switch the session's current generation. Assert **5 → 6 → 7 → 5** reuses an existing generation and the fourth distinct generation bounds historical retention.
- [ ] Add rename, app/icon revision, deletion, and move tests. Item content/provider changes miss safely; removing an owner prunes all its generations; moving an unchanged item preserves a hit when its actual geometry is unchanged. Do not invalidate the entire launcher on each backend snapshot.
- [ ] Run `flutter test test/raster_cache_identity_test.dart test/app_cell_test.dart test/app_grid_test.dart test/home_screen_test.dart test/tab_strip_test.dart` to establish the failing identity/invalidation cases.
- [ ] Centralize generation/key creation and activate the generation in HomeScreen's build path before descendants build. Wrap the launcher content, including tabs, headings, and search, in `RasterCacheScope` using that store/generation. `RasterTile` reads this context/session value; it never activates generations independently. Standalone tests explicitly supply a generation.
- [ ] Compute row metrics once per grid layout/context and pass `AppCellGeometry` or the measured label height into cells; remove the per-cell repeated `TextPainter('Ag\nAg')` work on the normal grid path. Standalone AppCell can compute the same metrics as a fallback. Preserve existing small-cell constraints and all three layouts.
- [ ] Reconcile removed item/group owners on actual content changes. Use key content to handle rename/provider changes; old unpinned revisions of the same owner/kind/generation can be pruned on replacement, without discarding a different live geometry variant. Cover frequent same-generation icon revisions so stale buffers are eventually reclaimed.
- [ ] Run the focused tests and `flutter test test/group_pager_test.dart`. Profile warm build p99 before/after metric reuse; record a neutral result honestly rather than claiming an unmeasured gain.

## Task 5: Coordinate memory pressure and lifecycle

**Files:** Modify `home_screen.dart`, `raster_tile.dart`, and `raster_image_store.dart`; extend `test/home_screen_test.dart` and `test/raster_image_store_test.dart`.

**Interfaces:** `TileRasterCache.trimForMemoryPressure()` delegates to the session store and cancels speculative work. Keep `HomeScreen` as the existing `WidgetsBindingObserver`; do not add a second independent observer for the same policy. Extend `HomeScreen`'s optional dependencies with `RasterImageStore? rasterStore` for tests; use the session store when omitted, and pass the resolved store to `RasterCacheScope` so rendering, generation activation, reconciliation, and pressure handling use the same instance.

- [ ] Test memory pressure while outgoing/incoming page images are pinned: both remain drawable/discoverable; unused images are released; source/decoded clearing still occurs. Mark preserved images for removal once their last lease releases, unless an explicit subsequent foreground reuse re-admits them.
- [ ] Test repeated pressure, route/search hiding, background/resume, and screen disposal. Trim must not change user settings, unexpectedly navigate, or synchronously rebuild the entire grid. Clear stale retired source-provider keys so reported source-map clearing is not mistaken for complete buffer reclamation.
- [ ] Run `flutter test test/home_screen_test.dart test/raster_image_store_test.dart`; implement trim order and hook the existing pressure callback, then rerun to pass. Pressure handling itself must not start warming immediately.
- [ ] Add a benchmark-only pressure phase after warming; inject the Flutter memory-pressure notification, verify visible integrity, then revisit all groups. Record reclaimed bytes, required recaptures, and recovery time. Pressure recovery is a distinct scenario and is not held to the zero-recapture warm-navigation contract.

## Task 6: Benchmark a shared visible-capture scheduler

**Files:** Create `raster_capture_scheduler.dart` and `test/raster_capture_scheduler_test.dart`; modify `raster_tile.dart`, `home_screen.dart`, and the permanent raster benchmark.

**Interfaces:**

```dart
class RasterCaptureScheduler {
  RasterCaptureScheduler({int maximumVisiblePerFrame = 4,
    Duration submissionBudget = const Duration(milliseconds: 2)});
  void enqueueVisible({required Object owner, required RasterCacheKey key,
    required bool Function() isCurrent,
    required Future<void> Function() capture});
  void enqueueSpeculative({required RasterCacheKey key,
    required bool Function() isCurrent,
    required Future<void> Function() capture});
  void cancelOwner(Object owner);
  void cancelSpeculative();
  void setSpeculationEnabled(bool enabled);
  void dispose();
}
```

Visible jobs run post-paint; speculative jobs run only when idle and when there is
no pending visible job. Deduplicate by full key, with multiple live subscribers:
unmounting the first subscriber must not cancel another widget's valid request.
For speculative work permit one in-flight job; callbacks check eligibility again
at execution. `capture` completes when publication or failure is settled.

- [ ] Test deduplication, subscriber cancellation, generation changes, newest visible request replacing obsolete work, pressure cancellation, and no publication after unmount/dispose. Use an injected frame/idle scheduling seam in tests, not wall-clock sleeps.
- [ ] Test visible priority over speculative work and bounded submissions per frame. Capture failure releases the slot so the queue makes progress. Pin an existing hit before any deferred work.
- [ ] Run `flutter test test/raster_capture_scheduler_test.dart test/raster_tile_test.dart`, implement the scheduler, and rerun to pass. Instrument queue length, wait time, submissions, cancellations, and failures in benchmark summaries; avoid production per-frame logging.
- [ ] Compare existing immediate capture against **2**, **4**, and effectively unbounded visible submissions per frame, with a **2 ms synchronous submission** soft budget. A stopwatch measures UI-side work only; it does not measure GPU completion or guarantee a 2 ms raster cost. Never delay required visible captures merely because a gesture is active.
- [ ] Measure cold p99/worst, total frames over the actual refresh budget, and time until all visible tiles are cached. A candidate ships only if it meets the gates below; otherwise retain immediate visible capture and use the scheduler solely for eligible speculative work in Task 7.

## Task 7: Warm full-resolution tiles during real idle time

**Files:** Create `raster_warmer.dart` and `test/raster_warmer_test.dart`; modify `app_cell.dart`, `app_cells.dart`, `grid_geometry.dart`, `home_screen.dart`, and the raster benchmark.

**Interfaces:**

- Extract `AppCellVisual` from AppCell's current visual body: plain `item`, `icon`, `settings`, `AppCellGeometry geometry`, and optional icon-frame callback. It renders the existing live pixels, with no gesture recognizers and no nested `RasterTile`. AppCell composes it with its existing gesture, opacity, semantics, and cache wrappers. Do not duplicate layout/text rendering in the warmer.
- `RasterWarmRequest({required RasterCacheKey key, required AppCellGeometry geometry, required int estimatedBytes, required Future<Widget?> Function() buildReadyVisual})`. A null result means not currently warmable; do not cache an empty intermediate icon.
- `RasterWarmer({required RasterImageStore store, required RasterCaptureScheduler scheduler})`; `replaceQueue(Iterable<RasterWarmRequest> requests)`, `setEnabled(bool enabled)`, `cancel()`, `dispose()`.
- `Future<ui.Image> renderWarmVisual({required Widget visual, required AppCellGeometry geometry, required MediaQueryData mediaQuery, required ThemeData theme, required TextDirection textDirection, required Locale locale})` owns a detached render pipeline and disposes its widget/render tree in `finally`. The caller owns the returned image until insertion.

- [ ] Test a warmed visual's pixels/dimensions against a live capture at identical geometry, DPR, theme, locale/direction, text scale, and icon readiness. Include multiline/ellipsized labels, blurred shadows, missing icons, uninstalled items, and all layout styles. Use semantic assertions separately; detached rendering must not add focusable/accessible offscreen content.
- [ ] Build one detached Flutter layout/paint pipeline with `BuildOwner`, `PipelineOwner`, a render root and `RenderRepaintBoundary`; flush layout/compositing/paint before capture. Supply the same inherited visual inputs as the live tree. Do not use an `Offstage` or zero-opacity widget in the displayed page: those do not supply a reliably painted capture, and never submit the detached render root to the screen surface.
- [ ] Test that a completed ready icon is used; warming waits on the existing provider/decode path rather than capturing a missing frame. Reuse the source/decoded caches and existing `warmIconImage` helper; do not fetch the same icon through a new backend path.
- [ ] Test zero speculative work after scroll starts, search/route hiding, backgrounding, pressure, or generation change; recheck before allocation and before publication. Dispose images produced after cancellation. Bound speculative work to **one in-flight image**, with no next job until the previous image is settled.
- [ ] Implement queue order: missing visible captures first (Task 6), offscreen items in current group, adjacent groups, then remaining groups. Skip keys already cached or queued. Check `canWarm` before resolving/rendering and again before insertion; use exact physical capture dimensions for the estimate. Pause the warmer without blocking visible rendering.
- [ ] Insert a warmed image using `insertAndAcquire`, then immediately release the warmer's lease. Reject stale-generation results instead of reactivating an old generation. Queue replacement is revision-stamped; ordinary paging must reprioritize existing work rather than invalidate completed images.
- [ ] Run `flutter test test/raster_warmer_test.dart test/app_cell_test.dart test/app_grid_test.dart test/home_screen_test.dart test/raster_capture_scheduler_test.dart` until the behavior/equivalence tests pass.
- [ ] On device measure **immediate first flick after launch** separately from **first flick after idle warm-up**. Report warm-up duration, completed/missing items, peak bytes, and any warming-induced over-budget frames. Repeat with 58, 200, and 500 synthetic items in a separate fake-backend fixture (never persist synthetic items to the real model).
- [ ] Ship offscreen warming only if exact-pixel/interaction checks pass and its acceptance gates pass. If the detached pipeline has unacceptable overhead or unreliable parity, remove the production warmer and retain the completed hit/retention improvements; document the result rather than silently substituting a lower-resolution image.

## Task 8: Profile residual swipe composition before changing paging

**Files:** Modify `integration_test/raster_cache_performance_test.dart` and `docs/scroll-performance.md`; modify `group_pager.dart`, `tab_strip.dart`, or `scroll_edge_fade.dart` only when a measured bottleneck identifies the owner; extend `test/group_pager_test.dart` / `tab_strip_test.dart` for any resulting behavioral change.

**Interfaces:** Extend benchmark scenarios `warm_fast_swipes`, `warm_reversals`, and `warm_tab_paging` with timeline markers and counts of widget builds, paint/capture work, and backend calls where feasible. Reuse the existing `TimelineTask` and bridge instrumentation patterns from `group_transition_performance_test.dart`.

- [ ] Capture a real device timeline with raster caches fully warm and stats showing zero new captures. Include at least 12 settled swipes and 12 immediate-reversal sequences, checking all tab destinations remain reachable and the controller/tab strip agree after settling.
- [ ] Attribute remaining >refresh-budget frames using UI build, raster/compositing, GPU submission, and GC tracks. Do not equate total asynchronous bridge latency with blocked UI time. Record the dominant cause and the evidence before editing pager rendering.
- [ ] If rebuild work is dominant, remove redundant rebuild/metric allocation while preserving the pager's existing animated child separation. If compositing is dominant, compare one narrow layer change at a time, retaining opacity/translation/fade appearance and wrapping behavior. Retain no change without the swipe acceptance gate; if no attributable improvement is found, this task ends with the documented profile.
- [ ] For any retained change, add the specific regression test first: page wrap, interrupted/reversed gesture, column-dependent travel, reduced motion, or tab synchronization as appropriate. Run `flutter test test/group_pager_test.dart test/tab_strip_test.dart test/home_screen_test.dart`, then rerun the warm scenarios.

## Task 9: Enable the measured production policy and publish results

**Files:** Modify `lib/main.dart`, `raster_tile.dart`, the existing transition benchmark, and `docs/scroll-performance.md`; add/extend configuration and consumer tests.

**Interfaces:**

- `RasterCacheConfig`: immutable `maximumBytes = 64 << 20`, `maximumEntries = 4096`, `recentGenerations = 2`, `mode`, and booleans selecting only the scheduler/warmer variants accepted in Tasks 6–7.
- Extend `TileRasterMode` with `automatic`; keep `none`, `separate`, and `combined` explicit benchmark overrides.
- `TileRasterCache.configure(RasterCacheConfig config)` is called once at startup before widgets mount. Reconfiguration during benchmarks first unmounts/disables consumers and releases leases; do not dispose images under a live `RawImage`.
- `TileRasterCache.modeFor({required bool iconsVisible, required bool labelsVisible})` resolves automatic mode: **combined when icons + labels are visible; separate when only labels are visible; none when labels are hidden**. Icons already use Flutter's decoded-image cache, and the existing measurements showed extra sparse/empty raster-cache work is unhelpful. Tabs/headings remain independently cached whenever raster caching is enabled, including icon-only app-grid scenarios.
- `main()` defaults to automatic production configuration. Explicit `--dart-define=TILE_RASTER=none|separate|combined` overrides mode; add `automatic` as an explicit accepted value. Unknown values fail clearly in debug/profile and use automatic in release. Standalone widget tests that bypass `main()` retain an explicit disabled default and opt into cache behavior per test.

- [ ] Test automatic visibility policy and explicit overrides, including hidden icons, hidden labels, empty groups, and tab/heading caching independent of app-cell choice. Reset the session in focused tests so one test's mode/images cannot leak into another.
- [ ] Enable the supported configuration, remove throwaway-spike wording, and update existing benchmark mode switches to use the safe configure/reset path. Preserve the sharp default shadow and user-selected blur values.
- [ ] Run `flutter analyze` and `flutter test`. Expected: no errors and the full unit/widget suite passes. Format only changed Dart files. Do not run Rust tests just for a Dart-only change; run them if execution introduces Rust changes requiring a revised plan.
- [ ] Run the complete acceptance matrix below on the tablet in profile mode, alternating baseline/final order and saving JSON plus a short result table. Restore the user's original APK/settings after profiling unless the user explicitly requests installation of the new build.
- [ ] Update `docs/scroll-performance.md` with selected policy, actual memory/frame results, cold-vs-warm distinction, rejected experiments, and exact reproduction commands. Preserve the original investigation report as historical evidence rather than overwriting its measurements.
- [ ] Finish with a requirement-by-requirement review against this plan, a clean intended diff, and a summary of measured gains and remaining misses. Implementation is complete only after the selected production path meets the applicable gates.

## Acceptance Matrix and Performance Gates

### Required scenarios

| Scenario | Coverage / correctness checks |
| --- | --- |
| Current real dataset, landscape | All 3 tabs, all 58 items, full vertical extent, cold then repeated circuits |
| Current real dataset, portrait and rotate back | Correct geometry, no stretched/stale images, actual physical sizes counted; reactivation can reuse a retained matching generation |
| Warm navigation | Tab taps, settled 2400 dp/s swipes, immediate reversals; all destinations/controller state verified |
| Appearances | Blur 0, 5, 10; 5 → 6 → 7 → 5; icons/labels all four combinations; text-size and font changes |
| Accessibility / locale | Large text, RTL, multiline/ellipsis, screen-reader labels/actions; gestures retained |
| Content changes | Rename, icon replacement, uninstall, move, delete; no stale images or global snapshot-driven flush |
| Lifecycle | Search/settings visible, background/resume, disposal, pressure during transition; stale warm jobs never publish |
| Larger synthetic fixture | 200 and 500 items, constrained target, excessive pinned set; no speculative eviction/recapture loop |

### Deterministic correctness/memory gates

- Warm complete traversal of the current real dataset: **0 new item captures**, **0 capacity evictions** after warm-up, at a fixed appearance.
- Blur 5 → 6 → 7 → 5 at the measured geometry: **0 new captures on final return**, with three complete raster generations expected near **34.1 MiB** and below **64 MiB**. Record any changed dimensions explaining a different payload.
- At quiescence, unpinned retention respects 64 MiB/4096 entries, with pinned overage explicitly reported. All clear/invalidation/disposal tests return live bytes to zero after final releases.
- No corrupted/disposed displayed images; no indefinite pending warm/capture jobs; no stale generation, missing-icon frame, or incorrect text/icon revision admitted.
- Pressure removes unused raster images and suppresses speculative rewarming until a subsequent eligible foreground opportunity; interactions continue working.

### Timing gates

Use **three samples per candidate**, then five if the decision is inconsistent.
Compare medians of per-run p99/worst and aggregate count ratios with identical
scenarios. Do not let extra idle frames dilute the action's miss rate.

- Warm tab paging and repeat full traversal: median raster p99 **≤10 ms** on this device; no >16 ms raster frames in at least two of three warmed samples; no >10% regression in median build p99 versus the retained baseline. Report residual >11.11 ms frames even when this gate passes.
- Settled fast swipes: median raster p99 **≤12 ms**, no >10% regression in build/raster p99, and no >1 percentage-point increase in refresh-budget miss rate. The investigation's approximately 1.2% raster miss rate is a baseline, not a zero-miss claim.
- A Task 8 pager/composition change additionally needs **≥20% relative reduction** in the aggregate above-refresh-budget raster-frame rate or **≥10% lower median raster p99**, while meeting all other gates. If the baseline has zero misses in a sample, use p99 rather than dividing by zero.
- A Task 6 visible scheduler ships only with **≥20% improvement** in median cold worst-raster time or cold over-budget-frame count, with no >10% regression in time-to-complete visible captures, cold build p99, or cold raster p99. Measure until all visible captures finish so moving work outside the scenario cannot manufacture a win.
- Task 7 idle prewarming: after a maximum **5 seconds eligible idle time** on the real 58-item dataset, first full circuit should require **0 new item captures**, have median raster p99 **≤10 ms**, and respect the memory target. Record actual warm-up time. Compare a fixed five-second warm-up window against an identical idle baseline: additional above-refresh-budget raster frames must be **≤1% of `refreshRateHz × 5` display intervals**, with no speculative raster frame >16 ms. Use this fixed denominator because a settled idle baseline can submit zero frames; report actual frame counts as well. First immediate flick must not regress build/raster p99 by >10%.
- No hard warm-up-time requirement for 200/500-item fixtures; correctness, cancellation, bounded memory, and no interference with active gestures remain required.

When the complete current-generation working set exceeds the target, revisiting
evicted offscreen content can legitimately require capture. The larger-fixture
gate forbids repeated capture/eviction of the **same displayed key** and speculative
thrashing; it does not promise zero recaptures for arbitrarily large collections.

These thresholds are proposed acceptance decisions, not promises that the current
implementation already satisfies them. If a gate fails, record the cause and
revise or drop that experiment; do not silently loosen the gate.

### Reproduction commands after the harness exists

```sh
flutter analyze
flutter test

JAVA_HOME=/opt/jbr-21 PATH="/opt/jbr-21/bin:$PATH" \
  flutter drive --profile --no-dds -d 6bfcc911 \
  --driver=test_driver/performance.dart \
  --target=integration_test/raster_cache_performance_test.dart

JAVA_HOME=/opt/jbr-21 PATH="/opt/jbr-21/bin:$PATH" \
  flutter drive --profile --no-dds -d 6bfcc911 \
  --driver=test_driver/performance.dart \
  --target=integration_test/group_transition_performance_test.dart
```

Task 1 defines any extra benchmark-only scenario/fixture flags and records them
with results. No benchmark flag should silently alter persisted user data.

## Self-Review / Coverage Record

- Full-resolution retention, source/decoded accounting separation, historical style reuse: Tasks 1–2 and 4.
- Warm remount/readiness bypass, no empty-icon capture, lifetime safety: Tasks 2–3 and 5.
- All actual pixel inputs, invalidation without navigation churn, repeated layout work: Task 4.
- Pressure recovery, active-image protection, stale async cancellation: Tasks 5–7.
- Cold capture and complete-cache prewarming: benchmark-gated Tasks 6–7, with explicit fallback if their complexity does not earn a measured gain.
- Residual swipe behavior with zero recaptures: Task 8; no attribution to cache capacity without evidence.
- Production enablement, sparse-mode choice, existing behavior, final evidence: Task 9.
- Interface ownership: store returns leases; only their final release disposes owned images. The scheduler schedules callbacks; the warmer owns detached pipelines; HomeScreen supplies eligibility and the shared generation. No competing disposal/observer policy.
- No implementation work or performance-success claim is implied by saving this plan.
