# QLaunch Core and Linux Launcher — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust core (model, store, search, reconciliation) plus the Linux platform layer and the Flutter home screen, so that `flutter run -d linux` lists the machine's real `.desktop` applications in group pages and launches them by tapping.

**Architecture:** A single Rust crate `qlaunch_core` holds all data and logic, split into `model` (pure, no I/O), `store` (a PATH, not an OS), `search`, `reconcile`, `platform` (a trait with `fake` and `linux` implementations), and `api` (the `flutter_rust_bridge` surface). Dart is a UI shell: it holds the latest `ModelSnapshot`, subscribes to one `CoreEvent` stream, and calls mutation functions. Platform paths are resolved by Rust, never by Dart.

**Tech Stack:** Rust 1.98 (`serde`, `serde_json`, `uuid`, `dirs`, `thiserror`, `notify`, `tempfile` for tests), `flutter_rust_bridge` v2, Flutter 3.47.7 / Dart 3.13.5, `flutter_test`.

**Spec:** `docs/superpowers/specs/2026-10-18-qlaunch-flutter-rust-design.md`

## Global Constraints

- Flutter 3.47.7 stable / Dart 3.13.5; Rust 1.98.1; `flutter_rust_bridge` v2. The `flutter_rust_bridge` crate version in `rust/qlaunch_core/Cargo.toml` and the `flutter_rust_bridge` version in `pubspec.yaml` MUST be identical, and MUST match the installed `flutter_rust_bridge_codegen`.
- Licence GPL-3.0. The existing root `LICENSE` is retained; do not remove or replace it.
- Linux config root: `$XDG_CONFIG_HOME/bitshift/qlaunch/`, falling back to `$HOME/.config/bitshift/qlaunch/`. Document file: `qlaunch.json`. Icon files: `icons/<uuid>.png`.
- **Rust resolves every platform path.** Dart must never read an environment variable, call `Platform.isAndroid`, or build a path. The single exception is the test-only `config_root_override` argument to `init`.
- **No business logic in Dart.** Dart does not sort, filter, group, or decide visibility. It renders a `ModelSnapshot` and calls API functions.
- **No network access** from any code path.
- `(package, activity)` is unique across the whole model. `AppRef` with an empty `activity` is legal (Linux).
- Schema field `"schema"` is present in the document and equals `1`. Any other value runs the migration hook and, if unhandled, is treated as corrupt.
- Atomic writes: serialise → write temp file in the same directory → `fsync` → `rename` over the target.
- Default groups on first run, in this order: `Apps`, `Home`, `Games`, `Hidden`, with roles `Apps`, `Home`, `Games`, `Hidden`. Root carries role `Root`.
- Settings defaults, verbatim: `grid_columns_portrait=4`, `grid_columns_landscape=6`, `layout=tiles`, `tab_visibility=always`, `wallpaper_alpha=0`, `status_bar_visible=true`, `icons_visible=true`, `icon_size=40`, `icon_alpha=100`, `labels_visible=true`, `label_size=14`, `label_typeface=normal`, `label_colour=white`, `label_shadow=true`, `orientation=auto`.
- Ranges: `grid_columns_*` 1–12; `wallpaper_alpha` 0–100; `icon_size` 0–100; `icon_alpha` 0–100; `label_size` 0–100.
- Exactly one group holds each of `Root`, `Hidden`, `Apps`, `Games`. At most one holds `Home`.
- Events reach Dart through exactly one stream, `events() -> Stream<CoreEvent>`.

## Review Focus

The spec is a vision document; its silence on an input is not permission for that input to break the program. These are the five inputs and conditions most likely to bite someone, and each one has a test pinned to the task that owns the code.

1. **A `qlaunch.json` truncated mid-write** (power loss, killed process, disk full) must not silently reset the user's whole setup. A document that parses but fails validation, or fails to parse at all, is moved aside with a warning and defaults are rebuilt — never overwritten in place. *Test pinned in Task 5.*
2. **A `.desktop` `Exec=` line with quoted arguments, embedded spaces, a literal `%`, or `%%`** — a naive field-code stripper mangles these into wrong or empty commands. *Test pinned in Task 10.*
3. **An icon that cannot be resolved** — missing theme, icon name absent, SVG-only — must render a placeholder in the grid, not crash, not panic, not leave a blank cell. *Tests pinned in Tasks 11 and 19.*
4. **Unicode, emoji and CJK labels** must survive a store round-trip, sort correctly and case-fold for search without panicking on a multi-byte boundary. *Tests pinned in Tasks 3, 5 and 7.*
5. **Shortcut extras and labels containing `=`, `|`, `"` or a newline** must round-trip intact. The old app's `|Class=key=value|` string encoding was destroyed by exactly these characters. *Test pinned in Task 5.*

---

### Task 1: Repository scaffold, bridge wiring, value from Rust

**Files:**
- Create: `.gitignore`, `pubspec.yaml`, `analysis_options.yaml`, `lib/main.dart`, `rust/Cargo.toml`, `rust/qlaunch_core/Cargo.toml`, `rust/qlaunch_core/src/lib.rs`, `rust_builder/**` (generated), `flutter_rust_bridge.yaml`, `test/core_smoke_test.dart`, `linux/**` (generated)
- Modify: `README.md` (restore after `flutter create` clobbers it)

**Interfaces:**
- Consumes: nothing.
- Produces: `qlaunch_core::api::version() -> String`, and the Dart binding `rust.api.version()`.

- [ ] **Step 1: Create the Flutter project in place**

Run: `flutter create --platforms=linux,android --org bitshift --project-name qlaunch .`

`flutter create` overwrites `README.md`. Restore ours immediately: `git checkout README.md`.

- [ ] **Step 2: Create the Rust crate**

Run: `cargo new --lib rust/qlaunch_core` then add to `rust/Cargo.toml` a `[workspace]` with `members = ["qlaunch_core"]`.

`rust/qlaunch_core/Cargo.toml` `[dependencies]`: `flutter_rust_bridge`, `serde` (features `derive`), `serde_json`, `uuid` (features `v4`, `serde`), `dirs`, `thiserror`, `notify`.
`[dev-dependencies]`: `tempfile`.
`[lib]`: `crate-type = ["cdylib", "staticlib", "rlib"]`.

- [ ] **Step 3: Write the failing test**

`test/core_smoke_test.dart`:

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/api/api.dart' as rust;

void main() {
  test('core reports its version', () async {
    expect(await rust.version(), startsWith('0.1.'));
  });
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `flutter test test/core_smoke_test.dart`
Expected: FAIL — the binding file does not exist yet.

- [ ] **Step 5: Implement `version` and generate bindings**

In `rust/qlaunch_core/src/api/api.rs`:

```rust
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
```

Declare `pub mod api;` in `lib.rs` and `pub mod api;` inside `api/mod.rs` as needed for codegen discovery. Add `flutter_rust_bridge.yaml` with `rust_input: crate::api` and `rust_root: rust/qlaunch_core`.

Run: `flutter_rust_bridge_codegen generate`.

- [ ] **Step 6: Run the test to verify it passes**

Run: `flutter test test/core_smoke_test.dart`
Expected: PASS.

- [ ] **Step 7: Verify the Linux app boots and shows the version**

Run: `flutter run -d linux`

Expected: a window opens. Change `lib/main.dart` to a `FutureBuilder` showing the returned version; the window displays `0.1.0`.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore: scaffold Flutter + Rust workspace with a working bridge"
```

---

### Task 2: Model — groups, items, roles and invariants

**Files:**
- Create: `rust/qlaunch_core/src/model/mod.rs`, `rust/qlaunch_core/src/model/ids.rs`, `rust/qlaunch_core/src/model/roles.rs`, `rust/qlaunch_core/src/model/error.rs`, `rust/qlaunch_core/tests/model_invariants.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `Id` (`= String`), `GroupRole`, `AppRef`, `ExtraKind`, `Extra`, `ShortcutPayload`, `Item`, `Group`, `AppEntry`, `AppCategory`, `ModelError`, and `Model::validate() -> Result<(), ModelError>`.

```rust
pub type Id = String;

pub enum GroupRole { Root, Apps, Games, Home, Hidden }

pub struct Group { pub id: Id, pub label: String, pub parent_id: Option<Id>,
                   pub role: Option<GroupRole>, pub order: i32 }

pub struct AppRef { pub package: String, pub activity: String }

pub enum ExtraKind { String, Bool, Int }
pub struct Extra { pub key: String, pub kind: ExtraKind, pub value: String }

pub enum ShortcutPayload {
    Android { action: Option<String>, data: Option<String>, mime_type: Option<String>,
              categories: Vec<String>, package: Option<String>, component: Option<String>,
              flags: i32, extras: Vec<Extra> },
    Command { command: String },
}

pub struct Item { pub id: Id, pub group_id: Id, pub label: String,
                  pub label_overridden: bool, pub app: Option<AppRef>,
                  pub shortcut: Option<ShortcutPayload>, pub uninstalled: bool,
                  pub order: i32, pub icon_override: Option<String> }

pub struct AppEntry { pub package: String, pub activity: String, pub label: String,
                      pub category: AppCategory }
pub enum AppCategory { Game, App, Unknown }

pub struct Model { pub root_group_id: Id, pub groups: Vec<Group>, pub items: Vec<Item> }
```

`ModelError` variants: `MissingRoot`, `DuplicateRole(GroupRole)`, `UnknownGroup(Id)`, `UnknownItem(Id)`, `DuplicateAppRef(AppRef)`, `RoleNotAssignable`, `GroupHoldsRole(Id)`, `NotAPage(Id)`.

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/model_invariants.rs` — assert: `Model::new_default()` validates; exactly one holder for each of `Root`/`Hidden`/`Apps`/`Games`; `Home` holder's label is `"Home"`; the default page order is exactly `["Apps", "Home", "Games"]` (Hidden is not a page); setting a second `Apps` role is not possible via the constructor — build a duplicate by hand and assert `validate()` returns `DuplicateRole(Apps)`; two items sharing an `AppRef` make `validate()` return `DuplicateAppRef`.

```rust
#[test]
fn duplicate_app_ref_is_rejected() {
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    m.add_app(AppRef { package: "a".into(), activity: "b".into() }, "A");
    m.add_app(AppRef { package: "a".into(), activity: "b".into() }, "A again");
    assert!(matches!(m.validate(), Err(ModelError::DuplicateAppRef(_))));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test model_invariants`
Expected: FAIL — the module does not exist.

- [ ] **Step 3: Implement the types and `new_default`**

`new_default` creates a root group with `role = Some(Root)`, `label = "_root_"`, `order = 0`, and the four default groups as its children with `order` 0, 1, 2, 999 and the labels and roles from the Global Constraints. `validate()` checks every invariant in the Global Constraints section, plus that `root_group_id` resolves and that every `Item.group_id` resolves.

`role_holder(role) -> Option<&Group>` returns the single holder.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test model_invariants`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(core): model types, default groups and invariant validation"
```

---

### Task 3: Model — mutation and ordering rules

**Files:**
- Create: `rust/qlaunch_core/src/model/mutate.rs`, `rust/qlaunch_core/tests/model_mutations.rs`
- Modify: `rust/qlaunch_core/src/model/mod.rs`

**Interfaces:**
- Consumes: Task 2's types.
- Produces: `Model::{pages, items_in, add_group, rename_group, move_group, set_group_role, delete_group, move_item, rename_item, remove_item, sort_group, add_app, home_group_id}`.

```rust
impl Model {
    pub fn pages(&self) -> Vec<&Group>;                    // root children, role != Hidden, by order
    pub fn items_in(&self, group: &Id) -> Vec<&Item>;      // by order
    pub fn add_group(&mut self, label: &str) -> Id;        // order = after the last page
    pub fn rename_group(&mut self, id: &Id, label: &str) -> Result<(), ModelError>;
    pub fn move_group(&mut self, id: &Id, new_order: i32) -> Result<(), ModelError>;
    pub fn set_group_role(&mut self, id: &Id, role: Option<GroupRole>) -> Result<(), ModelError>;
    pub fn delete_group(&mut self, id: &Id) -> Result<(), ModelError>;
    pub fn move_item(&mut self, id: &Id, to_group: &Id, new_order: i32) -> Result<(), ModelError>;
    pub fn rename_item(&mut self, id: &Id, label: &str) -> Result<(), ModelError>;
    pub fn remove_item(&mut self, id: &Id) -> Result<(), ModelError>;
    pub fn sort_group(&mut self, id: &Id) -> Result<(), ModelError>;
    pub fn add_app(&mut self, app: AppRef, label: &str) -> Id;
    pub fn home_group_id(&self) -> Option<Id>;
}
```

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/model_mutations.rs`:

```rust
#[test]
fn set_group_role_moves_the_role_and_never_duplicates() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    m.set_group_role(&g, Some(GroupRole::Apps)).unwrap();
    assert_eq!(m.role_holder(GroupRole::Apps).unwrap().id, g);
    assert!(m.validate().is_ok());
}

#[test]
fn root_hidden_apps_games_cannot_be_unset() {
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    assert!(matches!(m.set_group_role(&g, None), Err(ModelError::RoleNotAssignable)));
}

#[test]
fn deleting_an_ordinary_group_moves_its_items_to_apps() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    let a = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    let it = m.add_item_to(&g, "Signal");
    m.delete_group(&g).unwrap();
    assert_eq!(m.item(&it).unwrap().group_id, a);
}

#[test]
fn deleting_a_role_holding_group_is_refused() {
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Games).unwrap().id.clone();
    assert!(matches!(m.delete_group(&g), Err(ModelError::GroupHoldsRole(_))));
}

#[test]
fn sort_group_is_case_insensitive_and_unicode_safe() {
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    for l in ["zebra", "Ápple", "apple", "日本", "🙂 emoji"] { m.add_item_to(&g, l); }
    m.sort_group(&g).unwrap();
    let labels: Vec<_> = m.items_in(&g).iter().map(|i| i.label.clone()).collect();
    assert_eq!(labels, ["apple", "Ápple", "zebra", "日本", "🙂 emoji"]);
}
```

Add `add_item_to(group: &Id, label: &str) -> Id` and `item(&Id) -> Option<&Item>` to the `impl` for test ergonomics; they are part of the produced interface.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test model_mutations`
Expected: FAIL — methods do not exist.

- [ ] **Step 3: Implement the mutations**

`order` values are kept dense: after any insertion, removal or move, rewrite the affected list's `order` fields to `0..n` in list order. `move_group` and `move_item` take the target index and splice rather than assigning a raw `order`, so a caller cannot create gaps or ties. `delete_group` moves the victim's items to the `Apps` role holder and refuses if the victim holds any role. `home_group_id` returns the `Home` role holder's id, or `None`.

Sorting compares `label.to_lowercase()` — which is Unicode-aware in Rust — and falls back to the raw label on a tie, so the order is total and stable.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test model_mutations`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(core): group and item mutations, ordering and deletion rules"
```

---

### Task 4: Settings model and clamping

**Files:**
- Create: `rust/qlaunch_core/src/settings/mod.rs`, `rust/qlaunch_core/tests/settings.rs`
- Modify: `rust/qlaunch_core/src/lib.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `Settings`, `LayoutStyle`, `TabVisibility`, `Typeface`, `LabelColour`, `Orientation`, `SettingsPatch`, `Settings::clamp()`, `Settings::apply(&mut self, patch: &SettingsPatch)`.

```rust
pub enum LayoutStyle { Tiles, List, Centred }
pub enum TabVisibility { Always, LandscapeOnly, PortraitOnly, Never }
pub enum Typeface { Normal, Sans, Serif, Monospace }
pub enum LabelColour { White, Black, Grey }
pub enum Orientation { Auto, Landscape, Portrait }

pub struct Settings {
    pub grid_columns_portrait: u32, pub grid_columns_landscape: u32,
    pub layout: LayoutStyle, pub tab_visibility: TabVisibility,
    pub wallpaper_alpha: u8, pub status_bar_visible: bool,
    pub icons_visible: bool, pub icon_size: u32, pub icon_alpha: u8,
    pub labels_visible: bool, pub label_size: u32,
    pub label_typeface: Typeface, pub label_colour: LabelColour,
    pub label_shadow: bool, pub orientation: Orientation,
}

pub struct SettingsPatch {
    pub grid_columns_portrait: Option<u32>,
    pub grid_columns_landscape: Option<u32>,
    pub layout: Option<LayoutStyle>,
    pub tab_visibility: Option<TabVisibility>,
    pub wallpaper_alpha: Option<u8>,
    pub status_bar_visible: Option<bool>,
    pub icons_visible: Option<bool>,
    pub icon_size: Option<u32>,
    pub icon_alpha: Option<u8>,
    pub labels_visible: Option<bool>,
    pub label_size: Option<u32>,
    pub label_typeface: Option<Typeface>,
    pub label_colour: Option<LabelColour>,
    pub label_shadow: Option<bool>,
    pub orientation: Option<Orientation>,
}
```

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/settings.rs` — assert every default in the Global Constraints verbatim; assert `SettingsPatch { icon_size: Some(999), ..default() }` applied then clamped yields `icon_size == 100` (clamped, **not** reset to the default 40 — this is the §8.6 behaviour change); assert `grid_columns_portrait: Some(0)` clamps to `1`; assert an empty patch changes nothing.

```rust
#[test]
fn clamping_writes_the_clamped_value_not_the_default() {
    let mut s = Settings::default();
    s.apply(&SettingsPatch { icon_size: Some(999), ..Default::default() });
    s.clamp();
    assert_eq!(s.icon_size, 100);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test settings`
Expected: FAIL.

- [ ] **Step 3: Implement `Settings`, `Default`, `SettingsPatch::default`, `apply`, `clamp`**

`clamp` uses `clamp(min, max)` from the Global Constraints ranges. `apply` overwrites only the `Some` fields.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test settings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(core): settings model with clamping that keeps the user's value"
```

---

### Task 5: Store — the single document, atomically written

**Files:**
- Create: `rust/qlaunch_core/src/store/mod.rs`, `rust/qlaunch_core/src/store/atomic.rs`, `rust/qlaunch_core/src/store/migrate.rs`, `rust/qlaunch_core/tests/store.rs`
- Modify: `rust/qlaunch_core/src/lib.rs`

**Interfaces:**
- Consumes: Tasks 2–4.
- Produces: `SCHEMA_VERSION` (`u32 = 1`), `Document`, `Loaded`, `StoreError`, `Store`, `Loaded::{model, settings, warnings}`.

```rust
pub const SCHEMA_VERSION: u32 = 1;

pub struct Document { pub schema: u32, pub settings: Settings,
                      pub root_group_id: Id, pub groups: Vec<Group>, pub items: Vec<Item> }

pub struct Loaded { pub model: Model, pub settings: Settings, pub warnings: Vec<String> }

pub enum StoreError {
    Io(std::io::Error), Parse(serde_json::Error),
    UnsupportedSchema(u32), Invalid(ModelError),
}

pub struct Store { root: PathBuf }
impl Store {
    pub fn new(root: PathBuf) -> Self;
    pub fn document_path(&self) -> PathBuf;                  // root/qlaunch.json
    pub fn icon_path(&self, name: &str) -> PathBuf;          // root/icons/<name>
    pub fn load(&self) -> Result<Loaded, StoreError>;        // creates defaults if absent
    pub fn save(&self, model: &Model, settings: &Settings) -> Result<(), StoreError>;
    pub fn write_icon(&self, item: &Id, png: &[u8]) -> Result<String, StoreError>; // returns filename
    pub fn quarantine(&self) -> Result<PathBuf, StoreError>; // moves the bad file aside, returns where
}
```

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/store.rs`, each using `tempfile::tempdir()`:

```rust
#[test]
fn round_trips_unicode_and_hostile_shortcut_extras() {
    let dir = tempdir().unwrap();
    let s = Store::new(dir.path().to_path_buf());
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    m.add_shortcut(&g, "Café ☕ 日本", ShortcutPayload::Android {
        action: Some("a=b|c".into()), data: Some("line1\nline2".into()),
        mime_type: None, categories: vec!["x|y".into()],
        package: None, component: None, flags: 0,
        extras: vec![Extra { key: "k=1|2".into(), kind: ExtraKind::String,
                             value: "\"quoted\" | =".into() }],
    });
    s.save(&m, &Settings::default()).unwrap();
    let loaded = s.load().unwrap();
    assert_eq!(loaded.model, m);
        assert_eq!(loaded.model.items_in(&g)[0].label, "Café ☕ 日本");
}

#[test]
fn a_truncated_document_is_quarantined_and_defaults_rebuilt() {
    let dir = tempdir().unwrap();
    let s = Store::new(dir.path().to_path_buf());
    s.save(&Model::new_default(), &Settings::default()).unwrap();
    let mut bytes = std::fs::read(s.document_path()).unwrap();
    bytes.truncate(bytes.len() / 2);
    std::fs::write(s.document_path(), &bytes).unwrap();

    let loaded = s.load().unwrap();
    assert!(!loaded.warnings.is_empty());
    assert!(loaded.model.validate().is_ok());
    assert!(dir.path().join("qlaunch.json.corrupt").exists());
    assert!(!std::fs::read(s.document_path()).unwrap().is_empty());
}

#[test]
fn a_wrong_schema_is_quarantined_not_reinterpreted() {
    // write {"schema": 99, ...}, assert UnsupportedSchema path quarantines and rebuilds
}

#[test]
fn duplicate_app_refs_are_deduplicated_with_a_warning() {
    // hand-write a document with two items sharing an AppRef; assert one survives
    // and warnings mentions it
}
```

Add `Model::add_shortcut(&mut self, group: &Id, label: &str, payload: ShortcutPayload) -> Id`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test store`
Expected: FAIL.

- [ ] **Step 3: Implement `Store`**

`save`: `serde_json::to_vec_pretty(&Document { schema: SCHEMA_VERSION, .. })`, create `root` and `root/icons`, write to `root/.qlaunch.json.tmp`, `File::sync_all()` on the temp file, `fs::rename` over `qlaunch.json`, then `fsync` the directory. Serialisation must use `serde` derive with no custom field-name mangling, so `Enum` variants round-trip as the same strings the spec's sample JSON uses (`"app"`, `"android"`, `"command"`, `"tiles"`, …) — assert these exact strings in one test so a rename cannot silently break the on-disk format.

`load`: if the file is absent, build `Model::new_default()` and `Settings::default()`, `save`, and return with no warnings. If present, parse; on `Parse` failure, `quarantine()` (rename to `qlaunch.json.corrupt`, overwriting any previous quarantine), push a warning, and rebuild defaults, then **save** so the file is valid again. Run `migrate::migrate(doc)` when `schema != SCHEMA_VERSION`. Then `validate()`, and on failure quarantine and rebuild. Then de-duplicate `AppRef` duplicates, keeping the first by item order, recording a warning. `icons/` is created on demand.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test store`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(core): atomic single-document store with corruption quarantine"
```

---

### Task 6: Store — config root resolution and first-run behaviour

**Files:**
- Create: `rust/qlaunch_core/src/store/location.rs`, `rust/qlaunch_core/tests/store_location.rs`
- Modify: `rust/qlaunch_core/src/store/mod.rs`

**Interfaces:**
- Consumes: Task 5.
- Produces: `config_root() -> Result<PathBuf, StoreError>`.

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/store_location.rs`:

```rust
#[test]
fn config_root_honours_xdg_config_home() {
    std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-test");
    assert_eq!(config_root().unwrap(), PathBuf::from("/tmp/xdg-test/bitshift/qlaunch"));
}

#[test]
fn config_root_falls_back_to_home_dot_config() {
    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::set_var("HOME", "/tmp/home-test");
    assert_eq!(config_root().unwrap(), PathBuf::from("/tmp/home-test/.config/bitshift/qlaunch"));
}

#[test]
fn an_unwritable_config_root_does_not_panic() {
    // point XDG_CONFIG_HOME at a path under a read-only directory; assert load()
    // returns Ok with a warning and the app can still render a default model
}
```

These tests mutate process environment, so they must run serialised: use `#[serial]` from the `serial_test` dev-dependency, or gate the whole file behind `--test-threads=1`. Choose the `serial_test` attribute — it is a dev-dependency only.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test store_location`
Expected: FAIL.

- [ ] **Step 3: Implement `config_root`**

Read `XDG_CONFIG_HOME`; if unset or empty, use `dirs::home_dir()/.config`. Append `bitshift/qlaunch`. Return `StoreError::Io` only if neither variable yields a path. `Store::load` gains the behaviour that a failure to create the directory or write the document is a **warning**, not an error: the in-memory default model is still returned so the app starts (§Review Focus 5).

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test store_location`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(core): XDG config root resolution and read-only tolerance"
```

---

### Task 7: Search

**Files:**
- Create: `rust/qlaunch_core/src/search/mod.rs`, `rust/qlaunch_core/tests/search.rs`
- Modify: `rust/qlaunch_core/src/lib.rs`

**Interfaces:**
- Consumes: Task 2's `AppRef`.
- Produces: `MatchKind`, `SearchCandidate`, `SearchHit`, `search(&[SearchCandidate], &str) -> Vec<SearchHit>`.

```rust
pub enum MatchKind { Prefix, Substring, Subsequence }
pub struct SearchCandidate { pub app: AppRef, pub label: String }
pub struct SearchHit { pub app: AppRef, pub label: String, pub kind: MatchKind }
```

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/search.rs` — assert: an empty or whitespace query returns every candidate in input order; matching is case-insensitive (`"fir"` matches `"Firefox"`, `"FIR"` matches `"firefox"`); `Prefix` outranks `Substring` outranks `Subsequence`; a CJK query matches a CJK label; an emoji-containing label can be matched by its emoji and does not panic; a query longer than the label returns nothing.

```rust
#[test]
fn ranks_prefix_above_substring_above_subsequence() {
    let cs = cands(&["Web Browser", "Firefox", "ff"]); // fake
    let hits = search(&cs, "fi");
    assert_eq!(hits[0].kind, MatchKind::Prefix);       // "fi"
    assert!(hits.iter().any(|h| h.kind == MatchKind::Substring)); // "Firefox"
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test search`
Expected: FAIL.

- [ ] **Step 3: Implement `search`**

Lowercase both sides with `to_lowercase()`. Classify: `starts_with` → `Prefix`; `contains` → `Substring`; otherwise a two-pointer subsequence scan → `Subsequence`; else drop. Rank by `(kind, label.len(), label)`. Input order is the tie-breaker of last resort, so the function is deterministic.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test search`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(core): ranked, case-insensitive app search"
```

---

### Task 8: Platform trait and the fake source

**Files:**
- Create: `rust/qlaunch_core/src/platform/mod.rs`, `rust/qlaunch_core/src/platform/fake.rs`, `rust/qlaunch_core/tests/platform_fake.rs`
- Modify: `rust/qlaunch_core/src/lib.rs`

**Interfaces:**
- Consumes: Task 2's `AppEntry`, `AppRef`.
- Produces: `AppSource`, `PlatformError`, `LaunchError`, `FakeSource`.

```rust
pub enum PlatformError { Io(String), NotFound, Unsupported(String) }
pub enum LaunchError { NotFound(AppRef), Failed(String), Unsupported(String) }

pub trait AppSource: Send + Sync {
    fn list_apps(&self) -> Result<Vec<AppEntry>, PlatformError>;
    fn icon_png(&self, app: &AppRef, size_px: u32) -> Result<Option<Vec<u8>>, PlatformError>;
    fn launch(&self, app: &AppRef) -> Result<(), LaunchError>;
}

pub struct FakeSource { /* apps: Vec<AppEntry>, icons: HashMap<AppRef, Vec<u8>>,
                           launched: Mutex<Vec<AppRef>>, fail_launch: bool */ }
impl FakeSource {
    pub fn new(apps: Vec<AppEntry>) -> Self;
    pub fn with_icon(self, app: AppRef, png: Vec<u8>) -> Self;
    pub fn with_launch_failure(self) -> Self;
    pub fn launched(&self) -> Vec<AppRef>;
}
```

The trait is **synchronous**. `flutter_rust_bridge` v2 dispatches non-`sync` API functions off the Dart isolate, so Dart never blocks; the API layer (Task 14) is where `async` lives. This deviates from the spec's `async fn` on the trait itself and keeps the trait object-safe and trivially testable.

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/platform_fake.rs` — assert `list_apps` returns what was given; `icon_png` returns `None` for an app with no icon registered; `launch` records the app and `launched()` returns it in order; with `with_launch_failure()`, `launch` returns `Err(LaunchError::Failed(_))`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test platform_fake`
Expected: FAIL.

- [ ] **Step 3: Implement the trait and `FakeSource`**

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test platform_fake`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(core): AppSource trait and a scriptable fake implementation"
```

---

### Task 9: Reconciliation

**Files:**
- Create: `rust/qlaunch_core/src/reconcile.rs`, `rust/qlaunch_core/tests/reconcile.rs`
- Modify: `rust/qlaunch_core/src/lib.rs`

**Interfaces:**
- Consumes: Tasks 2, 3, 8.
- Produces: `ReconcileOutcome`, `reconcile(&mut Model, &[AppEntry]) -> ReconcileOutcome`.

```rust
#[derive(Default, PartialEq, Debug)]
pub struct ReconcileOutcome {
    pub added: Vec<Id>,
    pub marked_uninstalled: Vec<Id>,
    pub restored: Vec<Id>,
}
```

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/reconcile.rs`:

```rust
#[test]
fn routes_new_apps_by_category_to_the_role_holder() {
    let mut m = Model::new_default();
    let games = m.role_holder(GroupRole::Games).unwrap().id.clone();
    let apps = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    let out = reconcile(&mut m, &[
        entry("com.g", "G", AppCategory::Game),
        entry("com.t", "T", AppCategory::App),
        entry("com.u", "U", AppCategory::Unknown),
    ]);
    assert_eq!(out.added.len(), 3);
    assert_eq!(m.item(&out.added[0]).unwrap().group_id, games);
    assert_eq!(m.item(&out.added[1]).unwrap().group_id, apps);
    assert_eq!(m.item(&out.added[2]).unwrap().group_id, apps);
}

#[test]
fn a_reinstalled_app_returns_to_its_previous_group() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    let it = m.add_item_to(&g, "Signal");
    m.set_item_app(&it, appref("org.signal", "Main"));
    reconcile(&mut m, &[]);                                  // it disappears
    assert!(m.item(&it).unwrap().uninstalled);
    let out = reconcile(&mut m, &[entry("org.signal", "Main", AppCategory::App)]);
    assert_eq!(out.restored, vec![it.clone()]);
    assert_eq!(m.item(&it).unwrap().group_id, g);            // not the Apps group
    assert!(!m.item(&it).unwrap().uninstalled);
}

#[test]
fn a_reinstall_does_not_duplicate_and_does_not_override_a_user_label() {
    // user renames an item; it uninstalls and reinstalls; assert the label survives
}
```

Add `Model::set_item_app(&mut self, id: &Id, app: AppRef)`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test reconcile`
Expected: FAIL.

- [ ] **Step 3: Implement `reconcile`**

For every live entry: find the item whose `app` matches by `(package, activity)`. If found and `uninstalled`, clear the flag and record in `restored`. If found and active, it is unchanged. If not found, `add_app` into the `Games` role holder when `category == Game`, else the `Apps` role holder, and record it in `added`. Then every item with an `app`, not `uninstalled`, whose ref is absent from the live list, gets `uninstalled = true` and is recorded in `marked_uninstalled`. Items are never moved or deleted by reconciliation, which is what preserves the user's placement and label.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test reconcile`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(core): app list reconciliation that preserves user placement"
```

---

### Task 10: Linux — `.desktop` discovery and parsing

**Files:**
- Create: `rust/qlaunch_core/src/platform/linux/mod.rs`, `rust/qlaunch_core/src/platform/linux/desktop_file.rs`, `rust/qlaunch_core/tests/desktop_parse.rs`, `rust/qlaunch_core/tests/fixtures/applications/*.desktop`
- Modify: `rust/qlaunch_core/src/platform/mod.rs`

**Interfaces:**
- Consumes: Task 8's trait and Task 2's `AppEntry`.
- Produces: `DesktopEntry`, `parse_desktop(&str) -> DesktopEntry`, `exec_argv(&str) -> Vec<String>`, `discover_applications(&[PathBuf]) -> Vec<(String, DesktopEntry)>`, and `LinuxSource`.

```rust
pub struct DesktopEntry {
    pub id: String,               // desktop-file id, e.g. "firefox.desktop" or "org.x.Foo.desktop"
    pub name: String,             // locale-resolved
    pub icon: Option<String>,     // absolute path or theme name
    pub exec: Option<String>,
    pub terminal: bool,
    pub categories: Vec<String>,
    pub no_display: bool,
    pub hidden: bool,
    pub only_show_in: Vec<String>,
    pub not_show_in: Vec<String>,
}
pub fn parse_desktop(text: &str, id: &str, locale: Option<&str>) -> DesktopEntry;
pub fn exec_argv(exec: &str) -> Vec<String>;
pub fn dirs_to_scan() -> Vec<PathBuf>;   // XDG_DATA_HOME then XDG_DATA_DIRS, each + "/applications"
```

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/desktop_parse.rs`, reading fixtures from `tests/fixtures/applications/`:

```rust
#[test]
fn exec_with_quotes_spaces_and_percent_does_not_mangle() {
    assert_eq!(exec_argv("/usr/bin/foo --title=\"100% done\" %U"),
               ["/usr/bin/foo", "--title=100% done"]);
    assert_eq!(exec_argv("foo %% %f"), ["foo", "%"]);
    assert_eq!(exec_argv("env FOO=bar baz arg"), ["env", "FOO=bar", "baz", "arg"]);
}

#[test]
fn no_display_and_hidden_and_non_application_are_skipped() {
    // three fixtures; assert discover_applications() returns only the visible one
}

#[test]
fn the_same_id_in_two_directories_prefers_xdg_data_home() {
    // same filename in both dirs; assert the XDG_DATA_HOME copy's Name wins
}

#[test]
fn a_locale_name_is_preferred_and_a_missing_name_does_not_panic() {
    // fixture with Name and Name[de]; assert de when locale is "de"; assert a
    // fixture with no Name at all yields an empty label rather than a panic
}

#[test]
fn categories_containing_game_map_to_the_games_group() {
    // fixture with Categories=Game;ActionGame; assert AppCategory::Game
}
```

A literal `%%` must decode to `%`, and `%f`/`%F`/`%u`/`%U`/`%i`/`%c`/`%k` must be dropped without eating a neighbouring space. A `%` that is not a known field code and is not doubled is passed through literally.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test desktop_parse`
Expected: FAIL.

- [ ] **Step 3: Implement the parser and discovery**

`parse_desktop` reads only the `[Desktop Entry]` group, splits on the first `=`, tolerates duplicate keys (last wins) and lines with no `=` (ignored). `Name[xx]` is preferred when the locale matches, comparing on the language subtag. `Categories` splits on `;` with empty segments dropped. `exec_argv` implements the unquoting rules and the field-code table above; on unbalanced quotes it takes the remainder as one argument rather than failing.

`discover_applications` scans `dirs_to_scan()` in order, skips `no_display`, `hidden`, and entries whose `Type` is not `Application` (a `Type` key must be parsed and checked), applies `OnlyShowIn`/`NotShowIn` against `XDG_CURRENT_DESKTOP` (a `;`-separated list; absent means no filtering), and de-duplicates by `id` keeping the **first** occurrence — which is `XDG_DATA_HOME`, since it is scanned first.

`LinuxSource::new() -> Self` and `LinuxSource::new_with_dirs(Vec<PathBuf>) -> Self` (the latter for tests, holding an explicit list rather than reading the environment). `list_apps` maps a `DesktopEntry` to an `AppEntry { package: id, activity: "", label: name, category }`. Declare and implement `LinuxSource` against the `AppSource` trait, but leave `icon_png` returning `Ok(None)` and `launch` returning `Err(Unsupported)` for now — Tasks 11 and 12 fill them in.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test desktop_parse`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(linux): XDG .desktop discovery, parsing and Exec field codes"
```

---

### Task 11: Linux — icon resolution

**Files:**
- Create: `rust/qlaunch_core/src/platform/linux/icons.rs`, `rust/qlaunch_core/tests/linux_icons.rs`, `rust/qlaunch_core/tests/fixtures/icons/**`
- Modify: `rust/qlaunch_core/src/platform/linux/mod.rs`

**Interfaces:**
- Consumes: Task 10's `LinuxSource`.
- Produces: `resolve_icon(icon: &str, size_px: u32, dirs: &[PathBuf]) -> Option<PathBuf>`, and `LinuxSource::icon_png` from Task 8's trait.

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/linux_icons.rs`, using fixture icon directories:

```rust
#[test]
fn an_absolute_icon_path_is_used_directly() {
    // Icon=/abs/path/foo.png -> Some(that path)
}

#[test]
fn a_theme_name_resolves_through_hicolor() {
    // fixture: hicolor/128x128/apps/foo.png, Icon=foo -> Some(...)
}

#[test]
fn the_largest_available_size_at_or_above_the_request_is_preferred() {
    // hicolor/48x48 and hicolor/256x256 present, size_px 128 -> the 256 one
}

#[test]
fn an_unresolvable_icon_returns_none_and_never_panics() {
    assert_eq!(resolve_icon("definitely-not-a-real-icon-xyz", 64, &dirs), None);
}

#[test]
fn icon_png_reads_the_bytes_and_returns_none_for_a_missing_file() {
    // assert the bytes equal the fixture file
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test linux_icons`
Expected: FAIL.

- [ ] **Step 3: Implement `resolve_icon` and `LinuxSource::icon_png`**

If `icon` is absolute and exists, use it. Otherwise walk each dir in `dirs` (plus `hicolor` and the current theme) looking for `<dir>/<theme>/<size>x<size>/apps/<name>.<ext>`, `<dir>/<theme>/<size>/apps/`, and `<dir>/<theme>/scalable/apps/`, extensions `png`, `svg`, `xpm` in that order. Prefer the smallest size at or above `size_px`; if none, the largest available. Parse `size` segments of the form `48x48` or `48`; ignore `symbolic` and `scalable` for the numeric preference. `icon_png` returns the file's bytes when the extension is `png`, and `Ok(None)` for `svg`/`xpm` for now — rasterising vector icons is a spec-listed builder choice (§10) and a placeholder in the UI is the accepted v1 behaviour. **No path in this function may panic on a malformed filename or a symlink loop.**

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test linux_icons`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(linux): icon theme resolution with PNG passthrough"
```

---

### Task 12: Linux — launching applications

**Files:**
- Create: `rust/qlaunch_core/src/platform/linux/launch.rs`, `rust/qlaunch_core/tests/linux_launch.rs`
- Modify: `rust/qlaunch_core/src/platform/linux/mod.rs`

**Interfaces:**
- Consumes: Tasks 10, 11.
- Produces: `build_command(exec: &str, terminal: bool, terminal_emulator: Option<&str>) -> Result<Vec<String>, LaunchError>`, and `LinuxSource::launch`.

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/linux_launch.rs`:

```rust
#[test]
fn builds_argv_with_no_shell() {
    assert_eq!(build_command("/usr/bin/foo --a \"b c\" %U", false, None).unwrap(),
               ["/usr/bin/foo", "--a", "b c"]);
}

#[test]
fn terminal_entries_are_wrapped_in_the_configured_terminal() {
    assert_eq!(build_command("/usr/bin/top", true, Some("foot")).unwrap(),
               ["foot", "-e", "/usr/bin/top"]);
}

#[test]
fn a_terminal_entry_without_an_emulator_is_unsupported_not_silent() {
    assert!(matches!(build_command("/usr/bin/top", true, None),
                     Err(LaunchError::Unsupported(_))));
}

#[test]
fn launching_records_the_spawn_and_an_unknown_id_is_not_found() {
    // LinuxSource over fixture dirs; launch a fixture whose Exec is
    // "/bin/sh -c \"echo hi > /tmp/.../marker\""; wait for the marker; assert it exists.
    // Launching an unknown AppRef returns Err(LaunchError::NotFound(_)).
}
```

The marker-file form of the last test is what makes launching verifiable without a desktop session — it asserts the process actually ran, not merely that a struct was constructed.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test linux_launch`
Expected: FAIL.

- [ ] **Step 3: Implement `build_command` and `LinuxSource::launch`**

`build_command` composes `[exec_argv...]`, or `[emulator, "-e", exec_argv...]` when `terminal` — reading the emulator from the argument, which `LinuxSource` supplies from `$TERMINAL`, then `x-terminal-emulator` if present on `PATH`, else `None`. Spawn with `std::process::Command`, `stdin`/`stdout`/`stderr` null, and `.spawn()` **without waiting**; a spawned child is detached by dropping the handle. Resolve the `AppRef` back to a `DesktopEntry` by `id`; an unknown id is `LaunchError::NotFound`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test linux_launch`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(linux): application launching without a shell"
```

---

### Task 13: Linux — watching the applications directories

**Files:**
- Create: `rust/qlaunch_core/src/platform/linux/watch.rs`, `rust/qlaunch_core/tests/linux_watch.rs`
- Modify: `rust/qlaunch_core/src/platform/linux/mod.rs`

**Interfaces:**
- Consumes: Tasks 8, 10.
- Produces: `LinuxSource::watch(&self, on_change: Box<dyn Fn() + Send + Sync>) -> Result<WatchHandle, PlatformError>`, `WatchHandle` (drops to stop).

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/linux_watch.rs`:

```rust
#[test]
fn creating_a_desktop_file_triggers_exactly_one_coalesced_callback() {
    let dir = tempdir().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let src = LinuxSource::new_with_dirs(vec![dir.path().to_path_buf()]);
    let _h = src.watch(Box::new(move || { let _ = tx.send(()); })).unwrap();
    std::fs::write(dir.path().join("new.desktop"),
        "[Desktop Entry]\nType=Application\nName=New\nExec=/bin/true\n").unwrap();
    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    // then assert the second poll finds it in list_apps()
}

#[test]
fn dropping_the_handle_stops_the_callbacks() { /* drop, write, assert recv times out */ }
```

Coalescing matters: a single file write can produce several inotify events, and an uncoalesced callback would rebuild the whole model per event. Assert the callback fires **once** per burst by draining `rx` after a `recv` and expecting `Empty`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test linux_watch`
Expected: FAIL.

- [ ] **Step 3: Implement watching**

Use `notify` on each directory in `dirs_to_scan()` that exists, non-recursively, watching create/modify/remove/rename. Coalesce with a `Duration::from_millis(250)` debounce on a dedicated thread: keep a pending flag, and fire `on_change` once when the debounce expires with no further events. `WatchHandle` owns the watcher and a shutdown signal; `Drop` signals and joins. A directory that does not exist is skipped rather than fatal.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test linux_watch`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/qlaunch_core
git commit -m "feat(linux): debounced application directory watching"
```

---

### Task 14: Bridge API — state, snapshot, mutations and the event stream

**Files:**
- Create: `rust/qlaunch_core/src/api/mod.rs`, `rust/qlaunch_core/src/api/state.rs`, `rust/qlaunch_core/src/api/events.rs`, `rust/qlaunch_core/tests/api.rs`
- Modify: `rust/qlaunch_core/src/lib.rs`

**Interfaces:**
- Consumes: Tasks 3–13.
- Produces: `ModelSnapshot`, `CoreEvent`, `SearchHit` (re-exported), and the functions below.

```rust
pub struct ModelSnapshot { pub root_group_id: Id, pub page_ids: Vec<Id>,
                           pub groups: Vec<Group>, pub items: Vec<Item>,
                           pub settings: Settings, pub warnings: Vec<String> }

pub enum CoreEvent { ModelChanged, AppsChanged, ShortcutCreated { item_id: Id },
                     ShortcutCancelled, WallpaperChanged, HomePressed,
                     BackPressed, Error(String) }

pub async fn init(config_root_override: Option<String>) -> Result<ModelSnapshot, String>;
pub async fn snapshot() -> Result<ModelSnapshot, String>;
pub async fn refresh_apps() -> Result<ModelSnapshot, String>;
pub async fn add_group(label: String) -> Result<ModelSnapshot, String>;
pub async fn rename_group(id: Id, label: String) -> Result<ModelSnapshot, String>;
pub async fn move_group(id: Id, new_order: i32) -> Result<ModelSnapshot, String>;
pub async fn set_group_role(id: Id, role: Option<GroupRole>) -> Result<ModelSnapshot, String>;
pub async fn delete_group(id: Id) -> Result<ModelSnapshot, String>;
pub async fn move_item(id: Id, to_group: Id, new_order: i32) -> Result<ModelSnapshot, String>;
pub async fn rename_item(id: Id, label: String) -> Result<ModelSnapshot, String>;
pub async fn set_item_icon(id: Id, png: Vec<u8>) -> Result<ModelSnapshot, String>;
pub async fn remove_item(id: Id) -> Result<ModelSnapshot, String>;
pub async fn sort_group(id: Id) -> Result<ModelSnapshot, String>;
pub async fn update_settings(patch: SettingsPatch) -> Result<ModelSnapshot, String>;
pub async fn home_group_id() -> Option<Id>;
pub async fn search_all_apps(query: String) -> Vec<SearchHit>;
pub async fn icon_for(package: String, activity: String, size_px: u32) -> Option<Vec<u8>>;
pub async fn launch_app(package: String, activity: String) -> Result<(), String>;
pub fn events() -> impl Stream<Item = CoreEvent>;
```

`page_ids` is the page order, computed by Rust from `Model::pages()`; Dart must not recompute it.

- [ ] **Step 1: Write the failing test**

`rust/qlaunch_core/tests/api.rs`:

```rust
#[test]
fn init_creates_the_config_root_and_the_first_run_model() {
    let dir = tempdir().unwrap();
    let snap = block_on(api::init(Some(dir.path().to_string_lossy().into()))).unwrap();
    assert!(dir.path().join("qlaunch.json").exists());
    assert_eq!(snap.page_ids.len(), 3);                        // Apps, Home, Games
    assert_eq!(snap.settings.grid_columns_portrait, 4);
}

#[test]
fn mutations_return_a_fresh_snapshot_and_persist() {
    let dir = tempdir().unwrap();
    block_on(api::init(Some(dir.path().into()))).unwrap();
    let snap = block_on(api::add_group("Chat".into())).unwrap();
    assert_eq!(snap.page_ids.len(), 4);
    assert!(std::fs::read_to_string(dir.path().join("qlaunch.json")).unwrap().contains("Chat"));
}

#[test]
fn a_mutation_emits_model_changed() {
    // subscribe to api::events(), add a group, assert ModelChanged arrives
}

#[test]
fn deleting_a_role_holding_group_returns_an_error_string_and_changes_nothing() {
    // assert Err(_) and that the snapshot is unchanged
}

#[test]
fn the_api_uses_a_linux_source_and_reports_discovered_apps() {
    // init with an override, refresh_apps, assert items exist for the fixtures
}
```

The API holds global state. Tests that use it must serialise — mark them `#[serial]` from `serial_test`, or expose `#[cfg(test)] fn reset_for_test()` and call it at the top of each. Use the latter: it keeps the test file honest about ordering.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --test api`
Expected: FAIL.

- [ ] **Step 3: Implement the API**

Global state: `static STATE: OnceLock<Mutex<Option<App>>>` where `App` holds `Store`, `Model`, `Settings`, the boxed `AppSource`, and a `broadcast::Sender<CoreEvent>`. `init` resolves the config root (Task 6) unless overridden, loads (Task 5), constructs the Linux source, reconciles once, saves, and returns the first snapshot. Every mutation: lock, apply, `save`, emit `ModelChanged`, return the new snapshot. `refresh_apps`: lock, `list_apps`, `reconcile`, `save`, emit `AppsChanged`. On Linux the source's `watch` callback calls the same `refresh_apps` path.

Errors cross the bridge as `String` (the `Err` type), formatted as `<variant>: <detail>`, because FRB does not transport arbitrary error enums without extra configuration.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --test api`
Expected: PASS.

- [ ] **Step 5: Regenerate bindings**

Run: `flutter_rust_bridge_codegen generate`

- [ ] **Step 6: Commit**

```bash
git add rust/qlaunch_core lib/src/rust
git commit -m "feat(core): flutter_rust_bridge API surface and event stream"
```

---

### Task 15: Dart — the state layer

**Files:**
- Create: `lib/src/state/launcher_state.dart`, `lib/src/state/launcher_backend.dart`, `test/launcher_state_test.dart`

**Interfaces:**
- Consumes: Task 14's generated bindings (`qlaunch/src/rust/api/api.dart`).
- Produces: `LauncherBackend` (abstract), `RustBackend` (real), `FakeBackend` (tests), `LauncherState` (a `ChangeNotifier`).

```dart
abstract class LauncherBackend {
  Future<ModelSnapshot> init({String? configRootOverride});
  Future<ModelSnapshot> snapshot();
  Future<ModelSnapshot> refreshApps();
  Future<ModelSnapshot> addGroup(String label);
  Future<ModelSnapshot> setGroupRole(String id, GroupRole? role);
  Future<ModelSnapshot> deleteGroup(String id);
  Future<ModelSnapshot> moveGroup(String id, int newOrder);
  Future<ModelSnapshot> moveItem(String id, String toGroup, int newOrder);
  Future<ModelSnapshot> renameItem(String id, String label);
  Future<ModelSnapshot> updateSettings(SettingsPatch patch);
  Future<List<SearchHit>> searchAllApps(String query);
  Future<Uint8List?> iconFor(String package, String activity, int sizePx);
  Future<void> launchApp(String package, String activity);
  Stream<CoreEvent> events();
}

class LauncherState extends ChangeNotifier {
  LauncherState(this._backend);
  ModelSnapshot? get snapshot;
  List<String> get pageIds;
  List<Group> get groups;
  List<Item> itemsIn(String groupId);
  Group? get homeGroup;
  Future<void> load({String? configRootOverride});
}
```

`LauncherState` is the only object that talks to `LauncherBackend`. It re-fetches on `ModelChanged` and `AppsChanged`.

- [ ] **Step 1: Write the failing test**

`test/launcher_state_test.dart` — with `FakeBackend`: after `load()` the snapshot is available and `notifyListeners` fired once; emitting `ModelChanged` re-fetches and notifies again; `itemsIn('apps')` returns only that group's items; `pageIds` comes from the snapshot and is not recomputed; an `Error` event surfaces as an error state rather than an exception.

- [ ] **Step 2: Run the test to verify it fails**

Run: `flutter test test/launcher_state_test.dart`
Expected: FAIL.

- [ ] **Step 3: Implement `LauncherBackend`, `RustBackend`, `FakeBackend`, `LauncherState`**

`RustBackend` forwards to the generated `rust.api.*` functions one-for-one, so there is no logic to get wrong. `FakeBackend` is a hand-written snapshot holder with a `StreamController` for events and a recorded list of `launchApp` calls, used by every later Dart test.

- [ ] **Step 4: Run the test to verify it passes**

Run: `flutter test test/launcher_state_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/src/state test/launcher_state_test.dart
git commit -m "feat(ui): Dart state layer over the bridge"
```

---

### Task 16: Dart — the group pager controller

**Files:**
- Create: `lib/src/ui/group_pager.dart`, `test/group_pager_test.dart`

**Interfaces:**
- Consumes: Task 15's `LauncherState`.
- Produces: `GroupPagerController`, `GroupPager` (a widget).

```dart
class GroupPagerController {
  GroupPagerController({required List<String> pageIds});
  static const int virtualPageCount = 100000;
  List<String> get pageIds;
  set pageIds(List<String> value);        // keeps the current group if it survives
  int get initialPage;                    // a large index whose group is pageIds.first
  String groupIdAt(int virtualIndex);     // pageIds[virtualIndex % pageIds.length]
  int virtualIndexFor(String groupId);    // nearest index whose group is groupId
  String currentGroupId(int currentPage);
}
```

- [ ] **Step 1: Write the failing test**

`test/group_pager_test.dart` — this pins the defect the whole pager redesign exists to prevent:

```dart
test('the group at an index is the group the user is looking at', () {
  final c = GroupPagerController(pageIds: ['apps', 'home', 'games']);
  for (var i = 0; i < 500; i++) {
    expect(c.groupIdAt(c.initialPage + i), ['apps', 'home', 'games'][i % 3]);
  }
  // and the round trip
  for (final g in ['apps', 'home', 'games']) {
    expect(c.groupIdAt(c.virtualIndexFor(g)), g);
    expect(c.currentGroupId(c.virtualIndexFor(g)), g);
  }
});

test('wrapping is symmetric in both directions', () {
  final c = GroupPagerController(pageIds: ['a', 'b', 'c']);
  expect(c.groupIdAt(c.initialPage - 1), 'c');
  expect(c.groupIdAt(c.initialPage + 3), 'a');
});

test('a single page never wraps', () {
  final c = GroupPagerController(pageIds: ['only']);
  expect(c.groupIdAt(c.initialPage - 1), 'only');
  expect(c.groupIdAt(c.initialPage + 1), 'only');
});

test('changing the page list keeps the current group when it survives', () { /* ... */ });
test('an emptied page list does not throw', () { /* ... */ });
```

There is **no** other place in the Dart code that converts an index to a group. Any widget needing to know the current group asks the controller.

- [ ] **Step 2: Run the test to verify it fails**

Run: `flutter test test/group_pager_test.dart`
Expected: FAIL.

- [ ] **Step 3: Implement `GroupPagerController` and `GroupPager`**

`GroupPager` is a `PageView.builder` with `itemCount: virtualPageCount`, `controller: PageController(initialPage: controller.initialPage)`, and `itemBuilder: (_, i) => gridFor(controller.groupIdAt(i))`. `groupIdAt` must be total — an empty `pageIds` returns an empty string, and every widget must handle that.

- [ ] **Step 4: Run the test to verify it passes**

Run: `flutter test test/group_pager_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/src/ui/group_pager.dart test/group_pager_test.dart
git commit -m "feat(ui): wrapping group pager with a single index mapping"
```

---

### Task 17: Dart — home screen, grid and the three cell layouts

**Files:**
- Create: `lib/src/ui/home_screen.dart`, `lib/src/ui/app_grid.dart`, `lib/src/ui/app_cell.dart`, `lib/src/ui/tab_strip.dart`, `test/app_cell_test.dart`, `test/app_grid_test.dart`
- Modify: `lib/main.dart`

**Interfaces:**
- Consumes: Tasks 15, 16.
- Produces: `HomeScreen`, `AppGrid`, `AppCell`, `TabStrip`.

- [ ] **Step 1: Write the failing test**

`test/app_cell_test.dart`:

```dart
testWidgets('tiles layout stacks icon above label', ...);
testWidgets('list layout puts icon left of label on one line', ...);
testWidgets('centred layout does not overlap label and icon', () async {
  // The old app drew the label on top of the icon in this layout. Assert the
  // label's rect does not intersect the icon's rect.
});
testWidgets('icons_visible=false hides the icon but keeps the label', ...);
testWidgets('labels_visible=false hides the label but keeps the icon', ...);
testWidgets('a null icon renders a placeholder and does not throw', ...);
testWidgets('a very long unicode label is ellipsised, not overflowed', ...);
```

`test/app_grid_test.dart`: the column count follows `grid_columns_portrait` in portrait and `grid_columns_landscape` in landscape; tapping a cell calls `launchApp` with that item's `(package, activity)`; tapping an item whose `app` is null does nothing and does not throw; a group with zero items renders an empty grid with the tab strip present.

- [ ] **Step 2: Run the test to verify it fails**

Run: `flutter test test/app_cell_test.dart test/app_grid_test.dart`
Expected: FAIL.

- [ ] **Step 3: Implement the widgets**

`AppCell` takes an `Item`, a resolved `Uint8List?` icon, the `Settings`, and an `onTap`. Icons load through `LauncherState`'s `iconFor` and are cached per `(package, activity, size)` in a map on `AppGrid`'s state, so scrolling does not re-request. Sizes come from `icon_size` (dp) and `label_size` (sp); alpha from `icon_alpha` and `wallpaper_alpha` is **not** applied here (it belongs to the background layer); typeface, colour and shadow come from `label_typeface`, `label_colour`, `label_shadow`. The centred layout is a `Column` with `mainAxisAlignment: MainAxisAlignment.center` around icon and label; the tiles layout is a `Column` with the label below; the list layout is a `Row`.

`HomeScreen` composes `TabStrip` (visibility from `tab_visibility` and the current orientation) with `GroupPager`, and hosts the `Scaffold`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `flutter test test/app_cell_test.dart test/app_grid_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib test
git commit -m "feat(ui): home screen, app grid and the three cell layouts"
```

---

### Task 18: Dart — settings screen and background layer

**Files:**
- Create: `lib/src/ui/settings_screen.dart`, `lib/src/ui/background_layer.dart`, `test/settings_screen_test.dart`, `test/background_layer_test.dart`
- Modify: `lib/main.dart`, `lib/src/ui/home_screen.dart`

**Interfaces:**
- Consumes: Tasks 4, 15, 17.
- Produces: `SettingsScreen`, `BackgroundLayer`.

- [ ] **Step 1: Write the failing test**

`test/settings_screen_test.dart`: every setting in the Global Constraints table has a control; changing `layout` calls `updateSettings` with a patch containing only `layout`; the displayed value after a change comes from the returned snapshot, not from local state (so a clamped value is reflected); `Restart Launcher` and `Factory Reset` are present and `Factory Reset` asks for confirmation before calling anything.

`test/background_layer_test.dart`: with `wallpaper_alpha == 0` the overlay is fully transparent; with `wallpaper_alpha == 100` the overlay is fully opaque; a null wallpaper renders a solid base colour without throwing.

- [ ] **Step 2: Run the test to verify it fails**

Run: `flutter test test/settings_screen_test.dart test/background_layer_test.dart`
Expected: FAIL.

- [ ] **Step 3: Implement `SettingsScreen` and `BackgroundLayer`**

`SettingsScreen` groups controls as Display / Window / Icon / Text / General / Advanced. Each control builds a `SettingsPatch` with exactly one field set. `BackgroundLayer` stacks the wallpaper bytes (or a solid colour) under a `Container(color: Colors.black.withOpacity(alpha / 100))`. On Linux `wallpaperPng()` is not part of this plan's API surface; the layer takes the bytes as a parameter so a later Android task can supply them.

- [ ] **Step 4: Run the test to verify it passes**

Run: `flutter test test/settings_screen_test.dart test/background_layer_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib test
git commit -m "feat(ui): settings screen and wallpaper background layer"
```

---

### Task 19: Linux end-to-end verification

**Files:**
- Create: `integration_test/linux_launcher_test.dart`, `test/fixtures/applications/*.desktop`
- Modify: `pubspec.yaml` (add `integration_test` from the Flutter SDK)

**Interfaces:**
- Consumes: everything above.
- Produces: no new production interface.

- [ ] **Step 1: Write the failing test**

`integration_test/linux_launcher_test.dart` — boots the app against `FakeBackend` with a fixture list, so it runs headless in CI:

```dart
testWidgets('discovery to launch', (tester) async {
  await tester.pumpWidget(const QLaunchApp(backendOverride: FakeBackend(...)));
  await tester.pumpAndSettle();
  expect(find.text('Firefox'), findsOneWidget);          // discovered and drawn
  await tester.tap(find.text('Firefox'));
  await tester.pumpAndSettle();
  expect((backend as FakeBackend).launched, [appref('firefox.desktop', '')]);
});
```

Also assert that an app whose icon resolves to `null` renders a placeholder cell (Review Focus 3).

- [ ] **Step 2: Run the test to verify it fails**

Run: `flutter test integration_test/linux_launcher_test.dart`
Expected: FAIL.

- [ ] **Step 3: Make the app injectable**

Add `QLaunchApp({LauncherBackend? backendOverride})` in `lib/main.dart`, defaulting to `RustBackend`. No other production change.

- [ ] **Step 4: Run the test to verify it passes**

Run: `flutter test integration_test/linux_launcher_test.dart`
Expected: PASS.

- [ ] **Step 5: Run against the real machine**

Run: `flutter run -d linux`

Expected: the window lists the real `.desktop` applications from this machine in the Apps page, games in Games, the tab strip shows all pages, tapping a row launches the application, and installing or removing a `.desktop` file updates the grid within about a second without a restart.

Record the outcome in the commit message — this is the first time the whole stack touches a real desktop, and what it looked like is worth writing down.

- [ ] **Step 6: Run the full suites**

Run: `cargo test --manifest-path rust/Cargo.toml` and `flutter test`
Expected: both PASS.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "test: end-to-end Linux launcher verification"
```

---

## Self-Review

**Spec coverage.** §7.1 layout → Task 1. §7.2 core modules → Tasks 2–9. §7.3 Android JNI → out of this plan (Plan 2). §7.4 Android host → Plan 2. §7.5 ephemeral/persistent split → Task 15 (`LauncherState` holds no durable state). §7.6 shortcuts → the `ShortcutPayload` type and its store round-trip are Task 5; the add/edit flow is Plan 3. §7.7 wallpaper → Task 18. §8.1 screens → Home (17), Settings (18); Group Editor and App Settings are Plan 3. §8.2 home screen → Tasks 16–17, except the Home button, which needs Android and is Plan 2. §8.3 drawers → Plan 3. §8.4 model → Tasks 2–3. §8.5 discovery/reconciliation → Tasks 9–10. §8.6 settings → Tasks 4, 18. §8.7 persistence → Tasks 5–6. §8.8 Linux → Tasks 10–13. §8.9 API surface → Task 14, minus the Android-only functions. §9 testing → every task. §10 builder calls → icon cache (Task 17), SVG rasterisation (Task 11), search ranking (Task 7), Linux command shortcuts (deferred), widget tree and theming (Tasks 17–18).

**Gaps found and addressed while checking:** the spec's `create_shortcut` and `begin_add_shortcut` are Android-only and moved out of Task 14; `set_item_icon` is in Task 14 but its UI is Plan 3, noted there. The spec's `GroupEditorActivity` role assignment lands in Task 14's `set_group_role` with its UI in Plan 3.

**Deliberate deviations from the spec, flagged for the reviewer.** (1) The `AppSource` trait is synchronous, not `async fn`; FRB v2 dispatches API calls off the Dart isolate, so Dart still never blocks, and the trait stays object-safe and trivially fakeable. (2) `move_group` and `move_item` take a target index and are dense-rewritten, where the spec says `new_order: i32`; the signature is the same shape but the semantics are splice-and-reindex rather than assign-and-sort.

**Proportion.** This plan is longer than the spec because it is the first plan and it carries the scaffolding. Tasks 2–14 are one file each or one concern each; no task's steps contain a full function body except where an algorithm is not determined by the signature and the tests (Tasks 3's reindexing, 10's field codes, 13's debounce).
