# Launcher Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep all page icons decoded when they fit bounded caches, and remove unnecessary icon and background work without changing appearance or durable storage semantics.

**Architecture:** A bounded session provider cache complements Flutter's bounded decoded cache. An idle-priority, cancellable warmer resolves actual images in page-priority order. Rust supplies per-app revisions and maintains its disk cache incrementally; Android coalesces package notifications.

**Tech Stack:** Flutter/Dart, Rust, flutter_rust_bridge 2.13.0, Kotlin/JNI; existing dependencies only.

**Spec:** `docs/superpowers/specs/2026-10-22-launcher-performance-design.md`

## Global Constraints

- Provider cache: 32 MiB source bytes, 1000 entries; decoded cache: 64 MiB, 1000 entries.
- One speculative icon operation at a time; foreground requests must not queue behind it.
- Pause speculative work during scrolling, covered routes/search, and non-resumed lifecycle.
- No configuration schema change, weakened durability, visual change or new dependency.
- Regenerate bridge code with `flutter_rust_bridge_codegen generate` after public Rust API changes, then `cargo build --release --manifest-path rust/qlaunch_core/Cargo.toml` before bridge smoke tests.
- Run `flutter analyze lib test`, `flutter test`, and `cargo test --manifest-path rust/qlaunch_core/Cargo.toml`.
- Keep a task checklist here; leave changes uncommitted unless the user requests commits.
- A Pixel 8 is now connected and authorised over ADB. Automated tests cannot prove battery or frame-time improvements; collect device evidence separately.

## Review Focus

1. Cache invalidation during a pending load must never resurrect stale entries (Tasks 1, 4, 5).
2. A working set larger than the budgets must not cause endless speculative churn (Task 3).
3. Hidden icons, background lifecycle, scrolling and disposal must stop new speculative work (Tasks 2, 3).
4. Same-filename shortcut replacements must invalidate the old provider (Task 1).
5. Burst/continuous Android broadcasts must finish every pending result and avoid unbounded delay (Task 6).

## File map

- `lib/src/state/icon_cache.dart`: bounded provider/source-byte cache, shortcut entries and invalidation.
- `lib/src/state/icon_warmer.dart` (new): prioritised idle scheduling and cancellation.
- `lib/src/state/launcher_state.dart`: cache access, revision adoption, indexed items, event refresh coordination.
- `lib/src/ui/home_screen.dart`: page priority, lifecycle/route/search/scroll/memory-pressure gates.
- `lib/src/ui/app_cells.dart`: synchronous retained-provider reads and stable pending requests.
- `lib/main.dart`: decoded-image cache limits.
- `rust/qlaunch_core/src/api/api.rs`, `api/state.rs`: icon revision snapshots and selective invalidation.
- `rust/qlaunch_core/src/icons/store.rs`: shared accounting and synchronised cache writes.
- `android/app/src/main/kotlin/bitshift/qlaunch/PackageChangeReceiver.kt`: coalesced refresh worker.
- Corresponding Dart/Rust tests plus a Kotlin coalescer unit test under `android/app/src/test/kotlin/bitshift/qlaunch/`.

---

### Task 0: Preserve the phone and capture baseline

- [ ] Confirm the connected device identity before each install/test operation. Preserve the installed APK locally for rollback.
- [ ] Export the current configuration through the launcher's built-in backup and copy that export to the host; verify it parses and retain its checksum. The installed release build rejects `run-as`, so private-file backup is not currently available directly.
- [ ] Do not uninstall, clear app data, factory-reset, or change the default launcher. Use in-place, same-signature upgrades only. If a complete backup cannot be verified, stop before changing settings or installing test builds.
- [ ] Preserve shortcut icon files and other private data by retaining app data during upgrade. A configuration export alone is not a complete backup of shortcut image assets. Prefer read-only profiling of the installed release first.
- [ ] Record baseline frame timings/memory with repeatable swipes; avoid launching unrelated apps or changing system settings.
- [ ] After testing, restore the original configuration if anything changed, restore any deliberately changed system settings, and verify page order, groups, icon/label settings, shortcuts and default-launcher status. Report any restoration limitation explicitly.

### Task 1: Bounded application and shortcut provider cache

**Files:** Modify `lib/src/state/icon_cache.dart`, `lib/src/state/launcher_state.dart`, `lib/src/ui/app_cells.dart`; extend `test/icon_cache_test.dart`, `test/app_grid_test.dart`, `test/fake_backend.dart`.

**Interfaces:** Keep `resolve(String package, String activity, int sizePx)`. Add synchronous `peek(String package, String activity, int sizePx)`, `resolveItem(String id, String iconReference, int sizePx)`, `invalidateApp(String package, String activity)`, `invalidateItem(String id)`, `clear()`, and retained-byte/entry getters. Inject cache limits for tests. Distinguish pending, cached-null and absent internally.

- [ ] Write tests asserting same-key concurrent loads share one call; an LRU hit survives eviction; retained bytes never exceed an injected budget; null entries count toward the entry cap; errors retry; old-generation completion after clear/invalidation is not retained.
- [ ] Add widget tests asserting page recreation reuses providers, unrelated notifications do not flash placeholders, shortcut recreation reads once, requested shortcut decode size is physical display size, and same-filename replacement reads again.
- [ ] Run `flutter test test/icon_cache_test.dart test/app_grid_test.dart` and confirm the new cases fail for the intended missing behaviour.
- [ ] Implement a shared bounded LRU with separate pending requests; retain source-byte costs and use `ResizeImage` for shortcut PNGs. Remove the always-async wrapper on the cached success path; use synchronous lookups in cells and stable pending futures. Invalidate shortcut mutations explicitly.
- [ ] Rerun the focused tests and record results here.

### Task 2: Actual image warming with deterministic cancellation

**Files:** Create `lib/src/state/icon_warmer.dart`, `test/icon_warmer_test.dart`; modify `lib/main.dart`.

**Interfaces:** `IconWarmer` accepts injected idle scheduler and image-warm callback for unit tests. Expose `replaceQueue(...)`, `setEnabled(bool)`, `cancel()`, `dispose()` and one progress/budget callback; use immutable request identities covering app/shortcut, revision and physical size.

- [ ] Write tests asserting priority order, at most one operation in flight, duplicate elimination, error isolation, cancellation before scheduled callbacks, and late completion after dispose never starting another load.
- [ ] Write a real-engine widget test with RGBA bytes asserting warming populates `PaintingBinding.instance.imageCache` before an `Image` widget mounts; clean up test cache/listeners afterward.
- [ ] Run `flutter test test/icon_warmer_test.dart` and confirm failures.
- [ ] Implement idle-priority scheduling using Flutter's scheduler; resolve/warm the image with listener cleanup on success and error. Configure 64 MiB/1000 decoded-cache limits at app startup, not on every widget rebuild.
- [ ] Rerun warmer and raw-image tests; ensure no scheduled polling or continuing frames remain when disabled/empty.

### Task 3: All-page warming within budgets and lifecycle gates

**Files:** Modify `lib/src/ui/home_screen.dart`, `lib/src/state/launcher_state.dart`; extend `test/home_screen_test.dart`, `test/launcher_state_test.dart`, `test/icon_warmer_test.dart`.

**Interfaces:** HomeScreen supplies current/adjacent/remaining page requests to Task 2. LauncherState exposes cached providers and retained-cost information from Task 1; index `itemsIn(String groupId)` during `_adopt` instead of filtering every call.

- [ ] Add tests for current/adjacent/remaining ordering, hidden/zero-size/uninstalled items, duplicate apps, shortcuts, size changes, navigation reprioritisation and snapshots that remove queued items.
- [ ] Add tests asserting no new speculative requests while scrolling, backgrounded, search-covered or route-covered; resume appropriately; disposal and memory pressure cancel the queue. After pressure, no immediate automatic refill occurs.
- [ ] Add oversized-working-set tests asserting a completed pass does not evict-and-rewarm forever and no attempt is made to pin live widgets outside Flutter's cache budget.
- [ ] Run the new tests and confirm expected failures.
- [ ] Replace `_warmIcons` loops with one owned warmer, using lifecycle/scroll/route visibility gates and actual decode warming. Enforce projected source and decoded budgets; end a speculative pass when full. Implement `didHaveMemoryPressure` to cancel and clear unused cache references, without forced live-image eviction.
- [ ] Run `flutter test test/home_screen_test.dart test/launcher_state_test.dart test/icon_warmer_test.dart` and record results.

### Task 4: Selective app invalidation across the bridge

**Files:** Modify `rust/qlaunch_core/src/api/api.rs`, `rust/qlaunch_core/src/api/state.rs`, `lib/src/state/launcher_state.dart`, snapshot fixtures and generated bridge outputs; extend `rust/qlaunch_core/tests/api.rs`, `rust/qlaunch_core/tests/icon_disk_cache.rs`, `test/launcher_state_test.dart`.

**Interfaces:** Add `IconRevision { app: AppRef, revision: i64 }` and `ModelSnapshot.icon_revisions: Vec<IconRevision>` using `icon_tokens`. Dart compares old/new revisions on snapshot adoption; invalidates only changed/removed apps and cancels obsolete warm requests.

- [ ] Add Rust tests that updating one app changes its snapshot revision but leaves a second app's persisted icon reusable; resetting/restoring still invalidates appropriately. Add Dart tests that an AppsChanged fetch preserves unchanged providers and discards changed/removed providers, including pending loads.
- [ ] Run affected tests before implementation and confirm intended failures (compile failures acceptable for missing API).
- [ ] Publish revisions, remove whole-disk-cache clears for ordinary package changes, adopt selective invalidation only after the revision snapshot arrives; retain generation invalidation for reset/restore.
- [ ] Regenerate bridge code and update every snapshot constructor/fixture. Build the release Rust library.
- [ ] Run Rust API/cache tests, Dart launcher-state/home tests and `test/core_smoke_test.dart`.

### Task 5: Incremental, concurrency-safe disk cache maintenance

**Files:** Modify `rust/qlaunch_core/src/icons/store.rs`; extend `rust/qlaunch_core/tests/icon_disk_cache.rs`.

**Interfaces:** Preserve `IconDiskCache` public methods; clones share a mutex-protected generation/accounting state. Scan lazily once and track paths, sizes and modification order. Use unique temporary filenames and serialise invalidation/write commits.

- [ ] Add tests for replacement accounting, oldest-first eviction, restart accounting, missing/unreadable files, concurrent different/same-key writes and clear racing a pending write. Add test-only accounting instrumentation proving a batch does not scan once per inserted icon.
- [ ] Run `cargo test --manifest-path rust/qlaunch_core/Cargo.toml --test icon_disk_cache` and confirm new cases fail.
- [ ] Implement incremental accounting and trim only over cap. Ensure a generation check and write commit cannot race `clear`; maintain best-effort cache semantics under I/O errors.
- [ ] Rerun focused tests repeatedly and the full Rust suite; record results.

### Task 6: Coalesce package broadcasts without idle wakeups

**Files:** Modify `android/app/src/main/kotlin/bitshift/qlaunch/PackageChangeReceiver.kt`; add a small testable coalescer and its Kotlin tests, and test configuration only if absent.

**Interfaces:** Coalescer accepts injected clock/scheduler, refresh callback and per-broadcast completion callback. One worker, 250 ms trailing quiet window, maximum 1 s delay from first pending event. Events during a running scan trigger a subsequent batch.

- [ ] Add deterministic tests for one event, update burst, continuous events, scan failure, and events during scanning; assert every completion fires once and no work remains scheduled when idle.
- [ ] Run the Kotlin unit-test target and confirm failures for the missing implementation.
- [ ] Integrate the coalescer with `goAsync()`, preserving library/bridge installation and off-main-thread scanning. Finish pending results in failure paths; do not add periodic timers or a permanently spinning worker.
- [ ] Run `./gradlew :app:testDebugUnitTest` from `android/` with the project's Java environment; build an Android release APK if the local toolchain permits.

### Task 7: Coalesce event refreshes and verify the delivery

**Files:** Modify `lib/src/state/launcher_state.dart`; extend `test/launcher_state_test.dart`; update this plan with evidence and remaining manual checks.

**Interfaces:** Add an internal event-refresh drain with one active snapshot fetch and at most one follow-up request. Preserve existing mutation generation handling and external-event delivery; do not globally ignore events during mutations.

- [ ] Add tests emitting many ModelChanged/AppsChanged events while a snapshot is held: one active fetch, at most one follow-up, final external state adopted; failures recover; older mutation results cannot replace newer adopted state; disposal causes no notifications.
- [ ] Run launcher-state tests and confirm the new coalescing assertions fail.
- [ ] Implement the bounded refresh drain and avoid speculative warm-queue rebuilds on Home/Back-only notifications.
- [ ] Run `flutter analyze lib test`, `flutter test`, and `cargo test --manifest-path rust/qlaunch_core/Cargo.toml`; inspect `git diff --check` and generated bridge consistency.
- [ ] Review changes against the design, especially listener ownership, pending-load invalidation, memory budgets and no idle wakeups. Record exact checks and blockers.
- [ ] When a device is available, capture cold/warm frame timings, retained memory and idle/background/update CPU traces. Until then mark battery/FPS outcomes unmeasured.

## Follow-up, not silently bundled into this delivery

Usage persistence requires a separate storage design/plan covering durable journal
or sidecar semantics, migration, backup/restore/reset, corruption and crash recovery.
Keep full durable usage saves unchanged here. Rendering-layer changes and search
debouncing are profiling-led follow-ups, not assumptions about measured bottlenecks.

## Plan self-review

- Scope covers bounded all-page icon warming and the non-storage audit findings.
- Usage persistence is explicitly separated to avoid weakening durability or
  improvising migration rules inside an icon optimisation.
- Bridge revision types are defined before their downstream consumers depend on them.
- Each review-focus condition has tests in its owning task.
- All timing/battery improvement claims remain gated on device measurements.
