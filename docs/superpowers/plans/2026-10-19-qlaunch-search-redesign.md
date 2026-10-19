# QLaunch Search Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Home-button search panel with a full-screen search that shows nothing until you type, offers Frequent/Recent apps before typing, opens Settings from a gear, and draws results as home-screen cells — backed by new launch-usage tracking in the Rust core.

**Architecture:** The Rust core records one `(count, last_used_ms)` per app on every successful launch, persists it in the existing `qlaunch.json` document (schema stays 1; the new field is `#[serde(default)]`), and answers `frequent_apps` / `recent_apps`. The Flutter side reuses the grid cell widget on a computed item list, and the panel becomes a full screen with a sliding search box, a two-tab strip, and a gear.

**Tech Stack:** Rust (`flutter_rust_bridge` 2.13.0, `serde`, `serde_json`), Flutter 3.47.7 / Dart 3.13.5, JNI Android host.

**Spec:** `docs/superpowers/specs/2026-10-19-qlaunch-search-design.md`

## Global Constraints

- The stored schema **stays `SCHEMA_VERSION = 1`**. `usage` is added to `Document` with `#[serde(default)]` so a pre-change document loads unchanged and an older build ignores the field. Do **not** bump the schema.
- "Empty query" means empty after trimming, on both the core and the UI.
- `search_all_apps("")` returns an **empty** list (behaviour change); all other matching is untouched.
- **Only a launch that succeeds is recorded.** A failed launch records nothing.
- Usage results (`frequent_apps` / `recent_apps`) skip apps that are uninstalled or no longer in the model; they return a new `UsageHit { app, label }`, not `SearchHit`.
- A failed usage save surfaces through the existing `save_problem` path; it never fails the launch.
- All text styles come from `lib/src/ui/label_style.dart`; cells use `AppCell`, which already follows the Font size settings.
- Any change to public functions in `crate::api` requires `flutter_rust_bridge_codegen generate` (repo root) **and** `cargo build --release --manifest-path rust/qlaunch_core/Cargo.toml`, or `test/core_smoke_test.dart` fails on a content-hash mismatch.
- **Do not `git commit`** — the environment forbids it. Record each task in `.superpowers/sdd/2026-10-19-qlaunch-search-redesign/progress.md` with `commits: none`.
- Rust: unit tests live in `rust/qlaunch_core/tests/*.rs`; run with `cargo test` from `rust/qlaunch_core`. Dart: `flutter test`, `flutter analyze lib test`.
- Ignore analyzer errors under `rust_builder/cargokit/build_tool/*` (vendored).

## Review Focus

Inputs and conditions the spec implies but no single task's happy path exercises; each has a test in the task that owns the code:

1. **A usage entry for an app that is uninstalled or absent from the model** — must be skipped, not shown and not panic (Task 3).
2. **A usage entry for an app whose label changed** — shows the current label, not a stored one (Task 3).
3. **A whitespace-only query** — shows the tabs, never the whole app list (Tasks 1, 7).
4. **`limit` of 0, or larger than the number of entries** — empty set / all entries, no panic (Task 1).
5. **Back with no panel open** — still a no-op (Task 8).

---

### Task 1: Core usage model

**Files:**
- Create: `rust/qlaunch_core/src/usage/mod.rs`
- Modify: `rust/qlaunch_core/src/lib.rs` (add `pub mod usage;` beside `pub mod search;`)
- Test: `rust/qlaunch_core/tests/usage.rs`

**Interfaces:**
- Consumes: `crate::model::AppRef`.
- Produces:
  - `pub struct Usage { pub apps: Vec<AppUsage> }` with `Default` (derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default`; `apps` is `#[serde(default)]`).
  - `pub struct AppUsage { pub package: String, pub activity: String, pub count: u32, pub last_used_ms: i64 }` (derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`).
  - `pub struct UsageHit { pub app: AppRef, pub label: String }` (derive `Debug, Clone, PartialEq, Eq`).
  - `pub fn record(&mut self, app: &AppRef, now_ms: i64)` — increments `count` (saturating) and sets `last_used_ms`, or inserts a new entry with `count: 1`.
  - `pub fn frequent(&self, limit: usize, label_of: impl Fn(&AppRef) -> Option<String>) -> Vec<UsageHit>` — keeps entries where `label_of` is `Some`, sorts by `count` desc, then `last_used_ms` desc, then `label` asc; truncates to `limit`.
  - `pub fn recent(&self, limit: usize, label_of: impl Fn(&AppRef) -> Option<String>) -> Vec<UsageHit>` — same, ordering by `last_used_ms` desc, then `count` desc, then `label` asc.

- [ ] **Step 1: Write the failing tests** in `rust/qlaunch_core/tests/usage.rs`

```rust
use qlaunch_core::model::AppRef;
use qlaunch_core::usage::{Usage, UsageHit};

fn app(package: &str) -> AppRef {
    AppRef { package: package.into(), activity: "Main".into() }
}

/// Every app resolves to a label equal to its package, except `gone`.
fn label_of(a: &AppRef) -> Option<String> {
    (a.package != "gone").then(|| a.package.to_uppercase())
}

fn labels(hits: &[UsageHit]) -> Vec<&str> {
    hits.iter().map(|h| h.label.as_str()).collect()
}

#[test]
fn a_first_launch_inserts_with_count_one() {
    let mut u = Usage::default();
    u.record(&app("a"), 100);
    assert_eq!(u.apps.len(), 1);
    assert_eq!(u.apps[0].count, 1);
    assert_eq!(u.apps[0].last_used_ms, 100);
}

#[test]
fn a_repeat_launch_increments_and_moves_the_time_forward() {
    let mut u = Usage::default();
    u.record(&app("a"), 100);
    u.record(&app("a"), 200);
    assert_eq!(u.apps.len(), 1);
    assert_eq!(u.apps[0].count, 2);
    assert_eq!(u.apps[0].last_used_ms, 200);
}

#[test]
fn frequent_orders_by_count_then_recency_then_label() {
    let mut u = Usage::default();
    for _ in 0..3 { u.record(&app("often"), 1); }
    for _ in 0..1 { u.record(&app("rare"), 9); }
    u.record(&app("tie_a"), 5);
    u.record(&app("tie_a"), 5); // count 2
    u.record(&app("tie_b"), 7); // count 2, more recent
    assert_eq!(labels(&u.frequent(10, label_of)), ["OFTEN", "TIE_B", "TIE_A", "RARE"]);
}

#[test]
fn recent_orders_by_last_used_then_count_then_label() {
    let mut u = Usage::default();
    u.record(&app("older"), 10);
    u.record(&app("newer"), 30);
    assert_eq!(labels(&u.recent(10, label_of)), ["NEWER", "OLDER"]);
}

#[test]
fn entries_that_do_not_resolve_are_skipped() {
    let mut u = Usage::default();
    u.record(&app("gone"), 50);
    u.record(&app("here"), 10);
    assert_eq!(labels(&u.frequent(10, label_of)), ["HERE"]);
    assert_eq!(labels(&u.recent(10, label_of)), ["HERE"]);
}

#[test]
fn zero_and_oversized_limits_do_not_panic() {
    let mut u = Usage::default();
    u.record(&app("a"), 1);
    assert!(u.frequent(0, label_of).is_empty());
    assert_eq!(u.recent(99, label_of).len(), 1);
}

#[test]
fn usage_round_trips_through_json() {
    let mut u = Usage::default();
    u.record(&app("a"), 42);
    let json = serde_json::to_string(&u).unwrap();
    assert_eq!(serde_json::from_str::<Usage>(&json).unwrap(), u);
    assert_eq!(serde_json::from_str::<Usage>("{}").unwrap(), Usage::default());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml --test usage`
Expected: FAIL — `unresolved import qlaunch_core::usage`.

- [ ] **Step 3: Implement `rust/qlaunch_core/src/usage/mod.rs`**

Implement the structs and methods from the Interfaces block. `frequent`/`recent` build a `Vec<(AppUsage, String)>` of the resolvable entries, sort, then map to `UsageHit`. `record` uses `saturating_add(1)`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml --test usage`
Expected: PASS (7 tests).

- [ ] **Step 5: Record in the ledger** (`.superpowers/sdd/2026-10-19-qlaunch-search-redesign/progress.md`), `commits: none`.

---

### Task 2: Persist usage in the document

**Files:**
- Modify: `rust/qlaunch_core/src/store/mod.rs`
- Modify: `rust/qlaunch_core/src/api/state.rs` (only the `persist` signature and its callers)
- Test: `rust/qlaunch_core/tests/store.rs`, `rust/qlaunch_core/tests/store_location.rs`

**Interfaces:**
- Consumes: `Usage` (Task 1).
- Produces:
  - `Document { schema, settings, usage, root_group_id, groups, items }`, `usage` is `#[serde(default)] pub usage: Usage`.
  - `Loaded { model, settings, usage, warnings }`.
  - `pub fn save(&self, model: &Model, settings: &Settings, usage: &Usage) -> Result<(), StoreError>`.
  - `interpret` returns `Loaded` carrying the document's usage (default when the field was absent).
  - `unreadable` / `write_defaults` supply `Usage::default()`.
  - `state::persist(store, model, settings, usage)`.

- [ ] **Step 1: Write the failing tests** in `rust/qlaunch_core/tests/store.rs`

```rust
#[test]
fn usage_round_trips_through_save_and_load() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let m = Model::new_default();
    let mut usage = Usage::default();
    usage.record(&AppRef { package: "p".into(), activity: "Main".into() }, 7);
    s.save(&m, &Settings::default(), &usage).unwrap();
    assert_eq!(s.load().unwrap().usage, usage);
}

#[test]
fn a_document_without_usage_loads_with_defaults_and_is_not_repaired() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let m = Model::new_default();
    let doc = json!({
        "schema": SCHEMA_VERSION,
        "root_group_id": m.root_group_id.clone(),
        "groups": m.groups.clone(),
        "items": m.items.clone(),
    });
    std::fs::create_dir_all(s.document_path().parent().unwrap()).unwrap();
    std::fs::write(s.document_path(), serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
    let loaded = s.load().unwrap();
    assert_eq!(loaded.usage, Usage::default());
    assert!(loaded.warnings.is_empty(), "a missing usage field is not damage");
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml --test store usage`
Expected: FAIL — `save` takes 2 arguments / no field `usage`.

- [ ] **Step 3: Implement the changes**

Add the field, thread it through `save`, `interpret`, `Loaded`, `unreadable`, `write_defaults`, `write_back`, and `state::persist`. Then update every existing `save(...)` and `Document { ... }` construction site — the compiler lists them; they include `tests/store.rs` (the `write_document` helper, the `expect`/`read_document` closures, and ~20 `s.save(...)` calls) and `tests/store_location.rs` — passing `&Usage::default()` (and `usage: Usage::default()` in literals). Import `qlaunch_core::usage::Usage` where needed.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml --test store --test store_location`
Expected: PASS, both suites green.

- [ ] **Step 5: Record in the ledger**, `commits: none`.

---

### Task 3: API — record launches, answer queries, empty search

**Files:**
- Modify: `rust/qlaunch_core/src/api/state.rs` (`App` gains `usage`, `install` loads it, `persist` calls pass it)
- Modify: `rust/qlaunch_core/src/api/api.rs`
- Modify: `rust/qlaunch_core/src/search/mod.rs` and `rust/qlaunch_core/tests/search.rs`
- Test: `rust/qlaunch_core/tests/api.rs`, `rust/qlaunch_core/tests/search.rs`

**Interfaces:**
- Consumes: `Usage` (Task 1), `Store::save(.., usage)` + `Loaded.usage` (Task 2).
- Produces (all `pub`, in `crate::api::api`):
  - `pub use crate::usage::UsageHit;`
  - `pub async fn frequent_apps(limit: u32) -> Vec<UsageHit>`
  - `pub async fn recent_apps(limit: u32) -> Vec<UsageHit>`
  - `launch_app` records the launch in `app.usage` after `source.launch` returns `Ok`, then persists.

- [ ] **Step 1: Write the failing tests**

In `rust/qlaunch_core/tests/search.rs`, replace `empty_or_whitespace_query_returns_every_candidate_in_input_order` with:

```rust
#[test]
fn empty_or_whitespace_query_returns_nothing() {
    let cs = cands(&["Zeta", "Alpha", "Mid"]);
    assert!(search(&cs, "").is_empty());
    assert!(search(&cs, "  \t ").is_empty());
}
```

In `rust/qlaunch_core/tests/api.rs`:

```rust
#[test]
fn a_successful_launch_is_counted_and_a_failed_one_is_not() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    block_on(api::launch_app("org.alpha".into(), "org.alpha.desktop".into())).unwrap();
    block_on(api::launch_app("org.alpha".into(), "org.alpha.desktop".into())).unwrap();
    let err = block_on(api::launch_app("org.ghost".into(), "org.ghost.desktop".into()));
    assert!(err.is_err());
    let frequent = block_on(api::frequent_apps(10));
    assert_eq!(frequent.len(), 1);
    assert_eq!(frequent[0].label, "Alpha");
}

#[test]
fn usage_survives_a_reinit() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    block_on(api::launch_app("org.beta".into(), "org.beta.desktop".into())).unwrap();
    // init again over the same directory
    api::init_with_source(
        Some(dir.path().to_path_buf()),
        Box::new(FakeSource::new(fixtures())),
        Vec::new(),
    ).unwrap();
    assert_eq!(block_on(api::recent_apps(10))[0].label, "Beta");
}

#[test]
fn usage_queries_skip_apps_that_are_not_in_the_model() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    block_on(api::launch_app("org.alpha".into(), "org.alpha.desktop".into())).unwrap();
    // Remove the item, but its usage entry stays on disk.
    let snap = block_on(api::snapshot()).unwrap();
    let id = snap.items.iter().find(|i| i.label == "Alpha").unwrap().id.clone();
    block_on(api::remove_item(id)).unwrap();
    assert!(block_on(api::frequent_apps(10)).is_empty());
}

#[test]
fn empty_search_no_longer_lists_every_app() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    assert!(block_on(api::search_all_apps(String::new())).is_empty());
}
```

Note: `FakeSource` must succeed for the known fixtures and fail for an unknown app (it already does). Confirm the launch-error path by reading `rust/qlaunch_core/src/platform/fake.rs`.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml --test api --test search`
Expected: FAIL — no `frequent_apps`/`recent_apps`; the empty-search test fails.

- [ ] **Step 3: Implement**

- `search/mod.rs`: return `Vec::new()` when the trimmed query is empty.
- `state.rs`: add `usage: Usage` to `App`; in `install`, take `usage` from `Loaded` (or `Usage::default()` when there is no store) and put it on the `App`; add `pub(crate) fn launch_error`-style helper if needed; change `persist(store, model, settings, usage)` and its four callers (`install`, `commit_model`, `mutate_settings`, and the icon path at ~line 371) to pass `&app.usage`.
- `api.rs`:
  - `launch_app`: clone the source under the lock; call `launch`; on `Ok`, re-lock with `state::with_app`, call `app.usage.record(&app_ref, now_ms())`, and persist the model+settings+usage into `app.save_problem` (ignore an `Err` from persist as the existing warning path does, or propagate as today). Get `now_ms` from `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` as `i64` milliseconds, saturating on error.
  - `frequent_apps`/`recent_apps`: under `with_app`, build `label_of` from `app.model.items` (map `(package, activity) -> label`, only `!uninstalled` and `app.is_some()`), call `app.usage.frequent(limit as usize, ..)` / `recent`, and return. Return an empty vec when not initialised.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml`
Expected: PASS, whole suite green.

- [ ] **Step 5: Record in the ledger**, `commits: none`.

---

### Task 4: Regenerate the bridge and rebuild the host library

**Files:**
- Modify (generated): `lib/src/rust/**`
- Modify (build output): `rust/target/release/libqlaunch_core.so`

- [ ] **Step 1: Regenerate**

Run: `flutter_rust_bridge_codegen generate` (repo root).
Expected: `lib/src/rust/api/api.dart` gains `frequentApps`/`recentApps`; a new `lib/src/rust/usage.dart` holds `UsageHit`.

- [ ] **Step 2: Rebuild the release library**

Run: `cargo build --release --manifest-path rust/qlaunch_core/Cargo.toml`
Expected: success; the `.so` is newer than the regenerated Dart.

- [ ] **Step 3: Verify the hash matches**

Run: `flutter test test/core_smoke_test.dart`
Expected: PASS (no content-hash mismatch).

- [ ] **Step 4: Record in the ledger**, `commits: none`.

---

### Task 5: Dart backend and state

**Files:**
- Modify: `lib/src/state/launcher_backend.dart`
- Modify: `lib/src/state/launcher_state.dart`
- Modify: `test/fake_backend.dart`
- Test: `test/launcher_state_test.dart`

**Interfaces:**
- Consumes: generated `rust.frequentApps({required int limit})`, `rust.recentApps(...)`, `UsageHit`.
- Produces:
  - `LauncherBackend.frequentApps(int limit)` / `recentApps(int limit)` → `Future<List<UsageHit>>`; `RustBackend` forwards.
  - `LauncherState.frequentApps(int limit)` / `recentApps(int limit)` — like `searchAllApps`: a throw is reported and yields `const []`.
  - `LauncherState.backRequests` (int) — incremented on `CoreEvent_BackPressed()` in `_onEvent` (replacing the current ignore), plus `_notify()`.
  - `FakeBackend.frequentAppsResults` / `recentAppsResults` (`List<UsageHit> Function(int limit)?`), and `usageQueries` (`List<String>` recording `'frequent(N)'`/`'recent(N)'`).

- [ ] **Step 1: Write the failing tests** in `test/launcher_state_test.dart`

```dart
test('a back press increments backRequests and notifies', () async {
  final fake = FakeBackend();
  final state = LauncherState(fake);
  await state.load();
  final before = state.backRequests;
  var notified = false;
  state.addListener(() => notified = true);
  fake.emit(const CoreEvent.backPressed());
  await pumpEventQueue();
  expect(state.backRequests, before + 1);
  expect(notified, isTrue);
});

test('usage queries pass the limit through and tolerate a failure', () async {
  final fake = FakeBackend();
  final state = LauncherState(fake);
  await state.load();
  fake.frequentAppsResults = (limit) => [
    UsageHit(app: AppRef(package: 'p', activity: 'Main'), label: 'P'),
  ];
  expect((await state.frequentApps(24)).single.label, 'P');
  expect(fake.usageQueries, ['frequent(24)']);

  fake.failure = StateError('boom');
  final reports = captureReports();
  expect(await state.recentApps(24), isEmpty);
  expect(reports, isNotEmpty);
});
```

(If `launcher_state_test.dart` has an existing `pumpEventQueue`/`captureReports` pattern, follow it. `captureReports` comes from `test/fake_backend.dart`.)

- [ ] **Step 2: Run to verify they fail**

Run: `flutter test test/launcher_state_test.dart`
Expected: FAIL — no `backRequests` / `frequentApps`.

- [ ] **Step 3: Implement** the interface additions across the three files.

- [ ] **Step 4: Run to verify they pass**

Run: `flutter test test/launcher_state_test.dart`
Expected: PASS.

- [ ] **Step 5: Record in the ledger**, `commits: none`.

---

### Task 6: Extract the shared cells grid

**Files:**
- Create: `lib/src/ui/app_cells.dart`
- Modify: `lib/src/ui/app_grid.dart`
- Test: `test/app_grid_test.dart` (must stay green)

**Interfaces:**
- Consumes: `AppCell`, `LauncherState`, `Settings`, `Item`.
- Produces: `class AppCells extends StatefulWidget` with
  `AppCells({required LauncherState state, required Settings settings, required List<Item> items, ValueChanged<Item>? onItemTap, ValueChanged<Item>? onItemLongPress})`.
  It renders the `GridView.builder` currently in `AppGrid` (same delegate, same icon-future cache) and keeps `AppGrid`'s default tap (launch `item.app` via `state.launchApp`).

- [ ] **Step 1: Move the grid into `AppCells`** — cut the `GridView.builder` and the `_icons`/`_iconFor`/`_launch` members out of `app_grid.dart` verbatim, replacing `widget.state`/`widget.groupId` with the new `state`/`settings`/`items` fields. `AppGrid` becomes a `StatelessWidget` that keeps its `ListenableBuilder`, computes `settings` and `itemsIn(groupId)`, and returns `AppCells(...)`.

- [ ] **Step 2: Run the existing grid tests to confirm no behaviour change**

Run: `flutter test test/app_grid_test.dart test/app_cell_test.dart`
Expected: PASS, unchanged.

- [ ] **Step 3: Record in the ledger**, `commits: none`.

---

### Task 7: The search screen

**Files:**
- Modify: `lib/src/ui/app_search.dart`
- Modify: `test/app_search_test.dart`
- Test: `test/snapshot_builders.dart` (may need a hit/usage builder helper)

**Interfaces:**
- Consumes: `LauncherState.frequentApps`/`recentApps`/`searchAllApps`, `AppCells` (Task 6).
- Produces: `AppSearch({required LauncherState state, required VoidCallback onDone, required Future<void> Function() onOpenSettings})` with keys `fieldKey`, `closeKey`, `gearKey`, `frequentTabKey`, `recentTabKey`, `resultsKey`.

Behaviour to implement:
- A `FocusNode`; autofocus on open.
- An `AnimationController` (~200 ms) driving a `SlideTransition` (`Offset(0,-1)` → `Offset.zero`) on the box row.
- Row: gear `IconButton` (key `gearKey`) → `await widget.onOpenSettings(); _focus.requestFocus();`, then the `TextField`, then the close button.
- State: `String _query`, `_Tab _tab = _Tab.frequent`, and one `Future<List<Item>> _items`.
- `_query.trim().isEmpty` → show the two-tab strip (Frequent selected by default) and the grid for the selected tab (`frequentApps(24)` / `recentApps(24)`, mapped to `Item`s). Otherwise hide the tabs and show `searchAllApps(_query)` mapped to `Item`s.
- Mapping: `Item(id: '${app.package}/${app.activity}', groupId: '', label: label, labelOverridden: false, app: app, uninstalled: false, order: index)`.
- Grid: `AppCells(state, settings, items, onItemTap: (item) { state.launchApp(item.app!.package, item.app!.activity); widget.onDone(); })`.
- No "no matches" text is required; an empty grid is acceptable.

- [ ] **Step 1: Write the failing tests** (rewrite `test/app_search_test.dart`)

Cover, at minimum:
```dart
testWidgets('an empty query shows the tabs and asks for frequent apps', ...);
// expect(fake.usageQueries, ['frequent(24)']); expect(fake.searchQueries, isEmpty);
testWidgets('a whitespace query still shows the tabs', ...); // enterText '   '
testWidgets('the Recent tab asks for recent apps', ...); // tap recentTabKey -> 'recent(24)'
testWidgets('typing hides the tabs and shows the search results', ...);
testWidgets('the gear asks to open settings and refocuses on return', ...);
testWidgets('tapping a result launches and closes', ...);
testWidgets('the close button closes without launching', ...);
testWidgets('the search box and cells take the Font size', ...); // field fontSize == labelSize
```

Update `pumpSearch` to pass `onOpenSettings: () async {}` (and a real one where the gear test needs to observe the call).

- [ ] **Step 2: Run to verify they fail**

Run: `flutter test test/app_search_test.dart`
Expected: FAIL.

- [ ] **Step 3: Implement the screen.**

- [ ] **Step 4: Run to verify they pass**

Run: `flutter test test/app_search_test.dart`
Expected: PASS.

- [ ] **Step 5: Record in the ledger**, `commits: none`.

---

### Task 8: Wire Back and the gear into the home screen

**Files:**
- Modify: `lib/src/ui/home_screen.dart`
- Test: `test/home_screen_test.dart`

**Interfaces:**
- Consumes: `LauncherState.backRequests` (Task 5), `AppSearch(onOpenSettings:)` (Task 7).
- Produces: `_HomeScreenState` tracks `_seenBackRequests` and, in `_onStateChanged`, closes the search if it is open when `backRequests` moved; `_openSettings` returns `Future<void>` (the `Navigator.push` future) and is passed to `AppSearch`.

- [ ] **Step 1: Write the failing tests** in `test/home_screen_test.dart`

```dart
testWidgets('a back press closes the open search', ...);
// open search (Home press), emit backPressed, pump -> AppSearch gone
testWidgets('a back press with no search open does nothing', ...);
// emit backPressed, pump -> no exception, no route change
```

- [ ] **Step 2: Run to verify they fail**

Run: `flutter test test/home_screen_test.dart`
Expected: FAIL — back does not close the search.

- [ ] **Step 3: Implement.**

- [ ] **Step 4: Run to verify they pass**

Run: `flutter test test/home_screen_test.dart`
Expected: PASS.

- [ ] **Step 5: Full checks**

Run: `flutter analyze lib test` then `flutter test`
Expected: analyze clean (ignoring the vendored `rust_builder` errors); all tests pass.

- [ ] **Step 6: Record in the ledger**, `commits: none`.

---

### Task 9: Device verification

**Files:** none (verification only; record evidence in the ledger).

- [ ] **Step 1: Build and install**

Run: `flutter build apk --debug` (with `JAVA_HOME=/usr/lib/jvm/java-27`), then `adb install -r build/app/outputs/flutter-apk/app-debug.apk`. Re-assert the default home app: `adb shell cmd package set-home-activity --user 0 bitshift.qlaunch/.MainActivity`.

- [ ] **Step 2: Verify on device** — a Home press opens the panel with the search box sliding down, focused; before typing the Frequent/Recent tabs show and no list of every app appears; typing filters; the gear opens Settings and returning keeps the query and focus; Back closes the panel; tapping a result launches. Launch two apps, reopen, and confirm they appear under Frequent/Recent.

- [ ] **Step 3: Record the evidence** (commands and observations) in the ledger, `commits: none`.

If a `wm size` override is used to test overflow/resize, reset it immediately (`adb shell wm size reset`) in the same command chain.

---

## Self-Review

**Spec coverage:** §5 layout/tabs/gear/back → Tasks 7, 8; §5 cells → Task 6; §6 usage model → Task 1; §6 persistence/schema → Task 2; §6 recording/queries/empty-query → Task 3; bridge → Task 4; Dart glue → Task 5; approved choices (default Frequent, home-screen layout) → Task 7; device → Task 9.

**Type consistency:** `Usage`/`AppUsage`/`UsageHit` are defined once in Task 1 and referenced verbatim later; `frequent`/`recent` take `usize` limits and the API takes `u32` (cast once in Task 3); `Store::save` gains `&Usage` in Task 2 and every later caller passes it; `backRequests` is defined in Task 5 and consumed in Task 8; `AppCells`' constructor is fixed in Task 6 and used in Task 7.

**Proportion:** the plan is shorter than the code it describes; bodies appear only where the algorithm is not determined by the signature and tests.
