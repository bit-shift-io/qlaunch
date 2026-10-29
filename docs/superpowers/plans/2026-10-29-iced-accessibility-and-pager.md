# Iced Accessibility and Page Renderer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Flutter-free iced launcher navigable with TalkBack and replace the `Float` page/title overlays with one custom clipped `Slots` widget, without changing launcher rules or the pager state machine.

**Architecture:** Rust publishes a flat accessibility frame derived from app state and shared layout metrics; a transparent Kotlin `ExploreByTouchHelper` view maps it to Android virtual nodes and sends actions back over JNI. Pages and the title strip render three children in one clipped, translated layer through a custom `Slots` widget. Accessibility comes first and is the go/no-go; the pager work does not start unless TalkBack is credible.

**Tech Stack:** Rust 2024, `qlaunch_iced` on the `iced_android` v0.1.1 fork, existing `qlaunch_core` APIs, Kotlin/JNI, `androidx.customview:customview:1.1.0`, Android SDK/NDK, Python 3, `adb` and TalkBack.

**Spec:** `docs/superpowers/specs/2026-10-29-iced-accessibility-and-pager-design.md`

## Global Constraints

- `qlaunch_iced` must build with `--no-default-features`; no Flutter, Dart or `flutter_rust_bridge` dependency enters the iced build.
- Accessibility node bounds are **iced logical pixels** relative to the window origin. Kotlin alone scales them by `view.width / window.width` and `view.height / window.height`.
- Rust builds the tree in `update` from shared explicit metrics. No measured-layout pass and no copied padding numbers in `accessibility.rs`.
- Publishing is fire-and-forget. Never call the blocking `LauncherActions.onMain` / `FutureTask.get()` path from the iced thread.
- No permanent frame subscription. `window::frames()` stays limited to `pager.is_animating()`.
- Format only `qlaunch_iced` with `cargo fmt -p qlaunch_iced`; never run `cargo fmt --all`.
- Clippy: `cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings`.
- Do not run any git command. The user stages and commits.
- Preserve the tablet's saved layout, wallpaper and app data. Use `adb install -r`; never uninstall or clear data.
- Menus and node actions re-enter existing `Message`, gesture and overlay paths. Do not reimplement launcher rules in the accessibility module or in Kotlin.
- On the tablet the accessibility view must be transparent, non-clickable and must not request input focus; the iced IME view keeps keyboard focus.

## Review Focus

These are the input classes most likely to break and are pinned to the owning task's tests:

1. A TalkBack action arriving after a page/menu/screen change must be rejected by revision and must not launch an item on a hidden or departed page. (Task 4)
2. An accessibility focus rectangle and published bounds after a scroll must describe the settled fling, not the finger-up frame. (Tasks 4 and 6)
3. When the focused virtual node disappears, focus must move to a surviving node or the host, never remain on a dead id. (Task 5)
4. The full-screen accessibility view must consume no ordinary touch and must not steal the soft keyboard when TalkBack is off. (Tasks 5 and 6)
5. A vertical scroll fling decaying when a page step starts must keep receiving `RedrawRequested`, finish without a jump, and not freeze mid-flight. (Tasks 7 and 9)

---

### Task 1: Shared launcher and grid layout metrics

**Files:**
- Modify: `rust/qlaunch_iced/src/grid.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs:1182-1206` (`body_top`; add `body_top_for_layout`)
- Modify: `rust/qlaunch_iced/src/view.rs:26-100`
- Test: `rust/qlaunch_iced/src/grid.rs` tests and `rust/qlaunch_iced/src/lib.rs` tests

**Interfaces:**
- Consumes: `qlaunch_core::settings::{Settings, TabVisibility}`, `iced::{Rectangle, Size}`, `iced_android::Insets`.
- Produces:

```rust
pub const SEARCH_ROW_HEIGHT: f32 = 44.0;
pub const SEARCH_CONTENT_HEIGHT: f32 = 40.0;
pub const SEARCH_TABS_HEIGHT: f32 = 48.0;
pub const SEARCH_HORIZONTAL_PADDING: f32 = 8.0;
pub const SEARCH_GAP: f32 = 6.0;
pub const SEARCH_CLOSE_WIDTH: f32 = 48.0;
pub const FOLDER_BACK_WIDTH: f32 = 56.0;
pub const FOLDER_PADDING: f32 = 8.0;
pub const ERROR_LINE_HEIGHT: f32 = 18.0;
pub const WARNING_LINE_HEIGHT: f32 = 16.0;

pub fn tabs_visible(settings: &Settings, size: Size, page_count: usize) -> bool;
pub fn search_field(size: Size, insets: Insets) -> Rectangle;
pub fn search_close(size: Size, insets: Insets) -> Rectangle;
pub fn search_tabs(size: Size, insets: Insets) -> Rectangle;
pub fn search_tab(size: Size, insets: Insets, recent: bool) -> Rectangle;
pub fn title_strip(size: Size, insets: Insets, settings: &Settings) -> Rectangle;
pub fn folder_header(size: Size, insets: Insets, settings: &Settings) -> Rectangle;
pub fn folder_back(size: Size, insets: Insets, settings: &Settings) -> Rectangle;
pub fn folder_title(size: Size, insets: Insets, settings: &Settings) -> Rectangle;
pub fn body_top(
    size: Size,
    insets: Insets,
    settings: &Settings,
    searching: bool,
    search_history: bool,
    folder: bool,
    page_count: usize,
) -> f32;
pub fn viewport(
    size: Size,
    insets: Insets,
    settings: &Settings,
    searching: bool,
    search_history: bool,
    folder: bool,
    page_count: usize,
    footer_height: f32,
) -> Rectangle;
pub fn footer_height(error: bool, warnings: usize) -> f32;
```

`App::body_top_for_layout(&self) -> f32` calls `grid::body_top` with the real screen state. Existing `App::body_top` keeps its gesture meaning: searches return `f32::INFINITY`; launcher/folder return the same value as layout.

- [ ] **Step 1: Write the failing metric tests**

In `grid.rs`:

```rust
#[test] fn tabs_visibility_is_size_and_setting_driven();
#[test] fn search_geometry_uses_explicit_rows();          // field x = left+8, y = top+2, h 40, w = content_width-70; tabs y = top+44, halves of content_width
#[test] fn title_and_folder_headers_use_title_height();  // both h = title_height(settings)
#[test] fn folder_back_and_title_split_the_header();     // back x = left+8, w 56; title x = left+64, w = content_width-72
#[test] fn viewport_stops_above_footer_and_bottom_inset(); // tabs hidden, top 50, bottom 20, footer 36 => y 50, h 894
#[test] fn footer_height_counts_error_and_warnings();   // true, 2 => 18 + 2*16
```

In `lib.rs`:

```rust
#[test] fn body_top_for_layout_accounts_for_search_rows(); // top 10 => 102 with the history row, 54 without it
#[test] fn gesture_body_top_still_ignores_search();         // searching => INFINITY
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib grid::tests`
Expected: FAIL because the new metric functions and constants do not exist.

- [ ] **Step 3: Implement the metric functions and make the view use them**

Use `Rectangle::new(x, y, width, height)` and `Size` directly. `search_field` and `search_close` describe the content inside the row's `[2, SEARCH_HORIZONTAL_PADDING]` padding: field `x = insets.left + SEARCH_HORIZONTAL_PADDING`, `y = insets.top + 2`, `width = content_width - 2 * SEARCH_HORIZONTAL_PADDING - SEARCH_GAP - SEARCH_CLOSE_WIDTH`, height `SEARCH_CONTENT_HEIGHT`; close `x = insets.left + content_width - SEARCH_HORIZONTAL_PADDING - SEARCH_CLOSE_WIDTH`, same `y` and height; tabs start at `insets.top + SEARCH_ROW_HEIGHT`.

In `view.rs`, give the search row `.height(grid::SEARCH_ROW_HEIGHT)` and `.padding([2, grid::SEARCH_HORIZONTAL_PADDING])`, the history row `.height(grid::SEARCH_TABS_HEIGHT)`, the search field and close button `.height(grid::SEARCH_CONTENT_HEIGHT)`, the close button `.width(grid::SEARCH_CLOSE_WIDTH)`, the folder header `.height(grid::title_height(settings))` and `.padding([0, grid::FOLDER_PADDING])`, the folder back button `.width(grid::FOLDER_BACK_WIDTH).height(Fill)`, and wrap the error and warning lines in containers with `grid::ERROR_LINE_HEIGHT` / `grid::WARNING_LINE_HEIGHT`. These are the same values the tree will publish.

`App::body_top_for_layout` passes `self.query.is_empty()` as `search_history` and uses `grid::footer_height(self.error.is_some(), warnings.len())`; `App::body_top` is left as the gesture predicate.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib`
Expected: PASS, including the existing grid, pager, gesture and view tests.

- [ ] **Step 5: Format and lint**

Run:

```sh
cd rust
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
```

Expected: no changes required after formatting and no warnings.

---

### Task 2: Deterministic overlay card and scroll regions

**Files:**
- Modify: `rust/qlaunch_iced/src/overlay.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (scroll regions; `Message::Scrolled`)
- Modify: `rust/qlaunch_iced/src/view.rs` (search results `on_scroll`)
- Test: `rust/qlaunch_iced/src/overlay.rs` and `rust/qlaunch_iced/src/lib.rs`

**Interfaces:**
- Consumes: `qlaunch_core::model::{Group, Id, Item}`, `crate::{App, Message}`.
- Produces:

```rust
// lib.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Region {
    Group(Id),
    Search,
    OverlayList,
}
impl Region {
    pub(crate) fn widget_id(&self) -> &'static str; // apps / search-results / overlay-list
}

// App
scroll: HashMap<Region, f32>
Message::Scrolled(Region, f32)

// overlay.rs
pub(crate) const CARD_WIDTH: f32 = 320.0;
pub(crate) const CARD_PADDING: f32 = 12.0;
pub(crate) const CARD_GAP: f32 = 6.0;
pub(crate) const ACTION_HEIGHT: f32 = 36.0;
pub(crate) const INPUT_HEIGHT: f32 = 36.0;
pub(crate) const TEXT_HEIGHT: f32 = 18.0;
pub(crate) const LIST_HEIGHT: f32 = 240.0;
pub(crate) const APP_NAME_FIELD_ID: &str = "overlay-app-name";
pub(crate) const FOLDER_NAME_FIELD_ID: &str = "overlay-folder-name";

/// Deterministic, clipped card-title height: `(label_size * 1.17).max(22.0)`.
/// Matches the label's natural height at the default size and never reflows
/// the rows below it.
pub(crate) fn heading_height(settings: &qlaunch_core::settings::Settings) -> f32;

pub(crate) struct CardLayout {
    pub bounds: iced::Rectangle,
    pub title: iced::Rectangle,
    pub entries: Vec<EntryLayout>,
}
pub(crate) struct EntryLayout {
    pub key: String,
    pub bounds: iced::Rectangle,
}
pub(crate) enum EntryKind {
    Action { label: String, message: Message },
    Input { hint: String, value: String, widget_id: &'static str, on_input: fn(String) -> Message, on_submit: Message },
    Text { label: String, error: bool },
    List { rows: Vec<ActionRow> },
}
pub(crate) struct ActionRow { pub key: String, pub label: String, pub message: Message }
pub(crate) struct Entry { pub key: String, pub kind: EntryKind }

pub(crate) fn entries(app: &App, overlay: &Overlay) -> Vec<Entry>;
pub(crate) fn card(app: &App, overlay: &Overlay) -> CardLayout;
pub(crate) fn heading(overlay: &Overlay) -> String;
```

`entries` is the single source of card order for both the view and the tree. It appends `EntryKind::Text { label, error: true }` last when the open menu has an error; otherwise the error line is absent. `card` computes `x = ((window.width - CARD_WIDTH) / 2).max(0)`, `y = ((window.height - height) / 2).max(0)`, title height `heading_height(settings)`, then one row per entry with `CARD_GAP` between the title and each entry. Entry heights are `ACTION_HEIGHT`, `INPUT_HEIGHT`, `TEXT_HEIGHT`, and `LIST_HEIGHT`. Every entry's `x = bounds.x + CARD_PADDING` and `width = CARD_WIDTH - 2.0 * CARD_PADDING`; entry tops start at `bounds.y + CARD_PADDING + heading_height(settings) + CARD_GAP` and advance by the previous entry height plus `CARD_GAP`. The card height is `2.0 * CARD_PADDING + heading_height(settings) + sum(entry heights) + entries.len() as f32 * CARD_GAP`.

- [ ] **Step 1: Write the failing card and scroll tests**

In `overlay.rs`:

```rust
#[test] fn app_actions_follow_the_drawn_order();        // Rename, Move, Uninstall, Remove, Cancel
#[test] fn card_height_matches_rows_gaps_and_padding(); // 24 + 22 + 5*36 + 5*6 at default settings
#[test] fn rename_card_replaces_actions_with_an_input(); // one Input, Save, Back
#[test] fn move_card_excludes_the_current_group();       // rows from move_targets()
#[test] fn shortcut_card_keeps_loading_and_empty_states();
#[test] fn card_bounds_are_centred_and_inside_the_window();
```

In `lib.rs`:

```rust
#[test] fn scroll_offsets_are_tracked_per_region();      // Group("g"), Search and OverlayList do not collide
#[test] fn search_results_report_their_scroll_offset();  // Scrolled(Region::Search, 42.0)
#[test] fn overlay_list_reports_its_scroll_offset();     // Scrolled(Region::OverlayList, 42.0)
#[test] fn open_search_and_query_reset_the_search_offset();
#[test] fn switching_history_resets_the_search_offset();
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib overlay::tests`
Expected: FAIL because `entries`, `card`, `Region` and the new scroll events are absent.

- [ ] **Step 3: Implement `Region` and convert the scroll map**

Change `App::scroll` to `HashMap<Region, f32>`. View group scrolls publish `Region::Group(group)`. `at_top` and `reset_scroll` use `Region::Group(current_group_id)`. Search results add `.on_scroll(|viewport| Message::Scrolled(Region::Search, viewport.absolute_offset().y))`. The overlay list adds the same for `Region::OverlayList`.

`open_search`, `Message::Query` and `Message::History` reset `Region::Search` to `0.0` and batch `iced::widget::operation::scroll_to("search-results", AbsoluteOffset { x: Some(0.0), y: Some(0.0) })` so a new query or history tab never inherits a stale offset. They must not clear the field focus: keep `Message::Query`'s existing focus behaviour.

- [ ] **Step 4: Refactor the overlay view around `entries` and `card`**

Replace `app_card` / `launcher_card` construction with a shared card view that iterates `entries(app, overlay)` in order:

- `Action` becomes the existing full-width `button::text` with `height(ACTION_HEIGHT)`.
- `Input` becomes a `text_input` with the stable id, `height(INPUT_HEIGHT)`, and the entry's input/submit messages.
- `Text` becomes a fixed-height line; errors keep the existing red colour.
- `List` becomes `scrollable(column of action rows).id("overlay-list").height(LIST_HEIGHT)` with an `on_scroll` publishing `Region::OverlayList`.

Keep the scrim, `Noop` mouse area, `320` width, `12` padding, `6` spacing and rounded card style. Add `heading(overlay)` and use it as the card title, wrapped in a container of `heading_height(settings)` with `.clip(true)` so the title cannot reflow the rows. Give every `Action` `.height(ACTION_HEIGHT)` and every `Text` entry `.height(TEXT_HEIGHT)`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib`
Expected: PASS; all existing overlay/back/error tests still pass with the new entries.

- [ ] **Step 6: Format and lint**

Run:

```sh
cd rust
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
```

Expected: clean.

---

### Task 3: Accessibility frame builder

**Files:**
- Create: `rust/qlaunch_iced/src/accessibility.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod accessibility;`)
- Test: `rust/qlaunch_iced/src/accessibility.rs`

**Interfaces:**
- Consumes: `App`, `grid` metrics, `overlay::{card, entries, EntryKind, ActionRow}`, `Region`, `pager::Pager`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role { Button, Tab, Header, Text, TextField, Scroll }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Action {
    Click, LongClick, ScrollForward, ScrollBackward,
    ScrollLeft, ScrollRight, SetText, Dismiss,
}
impl Action { pub(crate) fn parse(value: &str) -> Option<Self>; }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextField { Search, AppName, FolderName }

#[derive(Debug, Clone)]
pub(crate) enum Target {
    Item(qlaunch_core::model::Id),
    Step(i32),
    Search,
    Back,
    SearchTab(bool),
    TextField(TextField),
    Scroll { region: Region, offset: f32, viewport: f32, content: f32 },
    Overlay(overlay::Message),
    Dismiss,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct Node {
    #[serde(skip)] pub key: String,
    #[serde(skip)] pub target: Target,
    pub id: i32,
    pub role: Role,
    pub label: String,
    pub bounds: [f32; 4],
    pub actions: Vec<Action>,
    pub selected: bool,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Frame {
    pub screen: String,
    /// Identifies the announced context; same as `screen`, except
    /// `launcher:<group-id>` so a page step announces the new page.
    pub announcement_key: String,
    pub window: [f32; 2],
    pub nodes: Vec<Node>,
}

pub(crate) struct NodeIds { /* by_key + next */ }
impl NodeIds {
    pub(crate) fn assign(&mut self, frame: &mut Frame); // stable ids, never reused
}

pub(crate) fn frame(app: &App) -> Option<Frame>;
pub(crate) fn announcement(app: &App) -> String;
```

- [ ] **Step 1: Write the failing tree tests**

Add a local `sample_app()` fixture that creates at least three pages, a folder, enough items to scroll, search results, and each overlay kind. Assertions:

```rust
#[test] fn launcher_frame_has_title_grid_items_and_footer();
#[test] fn launcher_frame_contains_only_the_current_page();
#[test] fn hidden_groups_and_closed_menus_are_absent();
#[test] fn item_bounds_are_row_major_and_clipped_to_the_viewport();
#[test] fn title_slots_use_wrapped_labels_and_actions(); // prev/current/next, current selected, search centre
#[test] fn grid_advertises_scroll_and_page_actions_only_when_available();
#[test] fn folder_frame_has_back_title_and_folder_items();
#[test] fn search_frame_has_field_tabs_results_and_tracked_offset();
#[test] fn overlay_frame_omits_launcher_nodes_and_follows_card_order();
#[test] fn overlay_field_has_value_click_and_set_text();
#[test] fn overlay_list_is_one_scroll_node_plus_visible_rows();
#[test] fn node_ids_survive_republish_and_are_never_reused();
#[test] fn bounds_never_escape_the_window();
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib accessibility::tests`
Expected: FAIL because `accessibility` does not exist.

- [ ] **Step 3: Implement the frame builder**

Build one flat list in visual order; use `grid`, `overlay::card` and `Region` geometry, never hard-coded row heights. Intersect every rectangle with the window and, for grid/search/overlay-list content, with its scroll viewport; omit zero-area results. Clip scroll offsets to `0..=max(0, content - viewport)`.

Screen keys: `launcher`, `folder:<id>`, `search`, `overlay:app:<step>`, `overlay:launcher:<step>`. `announcement` returns the heading for overlay/folder/search or `"<group>, page <n> of <m>"` for the launcher. `announcement_key` is the screen key except on the launcher, where it is `launcher:<group-id>`; use the same group id as the committed page.

Launcher nodes: three title tabs, `grid:container`, visible `item:<id>` in row-major order, then error/warning text. The title labels are exactly `"Previous page, <label>"`, `"<label>, search apps"` and `"Next page, <label>"`; the centre tab is selected. The grid label is `"<group>, page <n> of <m>, <count> apps"`. Folder nodes: `folder:back`, `folder:title`, `grid:container`, items. Search nodes: `search:field`, `search:close`, `search:frequent`, `search:recent` (use `grid::search_tab(size, insets, recent)` for their halves), a result scroll container and its result item nodes. Overlay nodes: `overlay:title`, entries in draw order, `overlay:list` plus visible rows for lists, error text. Use keys `item:<id>`, `overlay:action:<entry-key>`, `overlay:field:<app-name|folder-name>`, `overlay:text:<entry-key>`, `overlay:error`, `overlay:list` and `overlay:item:<row-key>` so focus survives a republish. Emit the error/warning footer after the grid on every screen that can show it.

Actions by node:

- title prev/next: `Click` → `Target::Step(-1/1)`; current: `Click` → `Target::Search`.
- grid: `LongClick` for launcher/folder blank, scroll up/down when the content can move, scroll left/right when `page_ids.len() > 1`. The grid's `Target` is `Target::Scroll`; page directions are dispatched from the action itself.
- items: `Click` and `LongClick`; search result items only `Click`.
- folder back: `Click` → `Target::Back`; folder title is a header.
- search field: `Click` and `SetText` → `Target::TextField(TextField::Search)`; close → `Target::Search`; tabs → `Target::SearchTab(bool)`.
- overlay field: `Click`/`SetText` → `Target::TextField(...)`; action rows → `Target::Overlay(message)`; overlay title and list nodes advertise `Dismiss` → `Target::Dismiss`.

- [ ] **Step 4: Implement stable ids**

`NodeIds::assign` walks the frame and gets or inserts `node.key` in `by_key`; new keys receive the next positive `i32`. It never reuses an id and never clears the map. The same semantic key keeps its id across republishes.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib accessibility::tests`
Expected: PASS.

- [ ] **Step 6: Format and lint**

Run:

```sh
cd rust
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
```

Expected: clean.

---

### Task 4: Publishing, quiescence and action dispatch

**Files:**
- Modify: `rust/qlaunch_iced/src/accessibility.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs`
- Modify: `rust/qlaunch_iced/Cargo.toml`
- Test: `rust/qlaunch_iced/src/accessibility.rs` and `rust/qlaunch_iced/src/lib.rs`

**Interfaces:**
- Consumes: `Frame`, `NodeIds`, `Target`, `Region`, `overlay`, `JniHost` on Android.
- Produces:

```rust
// Cargo.toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
[target.'cfg(target_os = "android")'.dependencies]
jni = "0.21"

// accessibility.rs
pub(crate) const QUIESCENCE: std::time::Duration = std::time::Duration::from_millis(150);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActionRequest {
    pub node_id: i32,
    pub action: Action,
    pub value: String,
    pub revision: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct Payload {
    pub revision: u64,
    pub screen: String,
    pub window: Window,
    pub nodes: Vec<Node>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announcement: Option<String>,
}
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub(crate) struct Window { pub width: f32, pub height: f32 }

pub(crate) struct Publisher { /* revision, fingerprint, announced */ }
impl Publisher {
    pub(crate) fn revision(&self) -> u64;
    /// Builds the next payload without committing it. A failed JNI call must
    /// not consume a revision or suppress the retry.
    pub(crate) fn prepare(&self, frame: &Frame, announcement: Option<String>) -> Option<Payload>;
    pub(crate) fn commit(&mut self, payload: &Payload, frame: &Frame);
}

pub(crate) struct State {
    pub(crate) ids: NodeIds,
    pub(crate) publisher: Publisher,
    pub(crate) frame: Option<Frame>,
    pub(crate) generation: u64,
    pub(crate) pending_scroll: bool,
}
impl State {
    pub(crate) fn scroll_started(&mut self) -> u64;
    pub(crate) fn scroll_settled(&mut self, generation: u64) -> bool;
    pub(crate) fn cancel_scroll(&mut self);
    pub(crate) fn publish(&mut self, frame: Frame, announcement: Option<String>) -> Result<u64, String>;
}

pub(crate) fn serialize(payload: &Payload) -> String;
pub(crate) fn subscribe() -> iced::Subscription<ActionRequest>;
pub(crate) fn publish_json(json: &str) -> Result<(), String>;

#[cfg(target_os = "android")]
pub use accessibility::Java_bitshift_qlaunch_MainActivity_onAccessibilityAction;
```

`App` adds `accessibility: accessibility::State` and `Message::{Accessibility(ActionRequest), AccessibilitySettled(u64)}`.

- [ ] **Step 1: Write the failing publisher and action tests**

In `accessibility.rs`:

```rust
#[test] fn identical_frames_are_not_published_twice();
#[test] fn revision_increments_only_on_publish();
#[test] fn changed_bounds_publish_a_new_revision();
#[test] fn screen_change_sets_the_announcement_once();
#[test] fn launcher_page_change_announces_the_new_page();
#[test] fn stale_scroll_generation_does_not_settle(); // scroll_started, scroll_started, settle first => false
#[test] fn payload_serializes_logical_bounds_and_actions();
```

In `lib.rs`:

```rust
#[test] fn item_click_is_dropped_while_paging();             // frame published, pager.begin_drag, click does nothing
#[test] fn stale_revision_cannot_launch_or_open_a_menu();    // request with revision - 1 is ignored
#[test] fn title_action_is_still_allowed_while_paging();     // title:next advances the pager
#[test] fn accessibility_scroll_updates_the_region_offset(); // clamped offset is stored before the operation runs
#[test] fn page_scroll_actions_step_the_pager();             // scroll_left/right => Step(-1/1)
#[test] fn accessibility_search_text_reaches_the_query();   // set_text dispatches Message::Query
#[test] fn overlay_dismiss_uses_the_back_path();
#[test] fn unknown_node_or_action_is_ignored();
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib accessibility`
Expected: FAIL — publisher, requests and `Message::Accessibility` are absent.

- [ ] **Step 3: Implement the publisher and serialization**

`Publisher::prepare` compares a fingerprint containing the screen, announcement key, logical window, and every node's role, label, bounds bits, actions, selected, enabled and value. A matching fingerprint returns `None`; otherwise it returns `revision + 1` with the announcement. `Publisher::commit` stores the revision, fingerprint and announced key only after `publish_json` succeeds. The announcement key is the `screen` key, except that the launcher key includes the committed group id (`launcher:<group-id>`) so a title step announces `"<group>, page <n> of <m>"` once. The publisher remembers the last announced key, not every query/scroll tick.

`publish_json` on Android calls
`qlaunch_core::platform::android::jni_host::JniHost::call_void("publishAccessibility", &[json.to_owned()])`.
On non-Android it returns `Ok(())`. A failed call logs at debug level and leaves the state unpublished so a later update can retry.

- [ ] **Step 4: Wire publishing into `App::update`**

Add `State`; after `update_inner` completes in `App::update`, call `self.publish_accessibility_if_idle()` unless the app is backgrounded, the pager is dragging/animating, or `pending_scroll` is set. The method builds `accessibility::frame(self)`, assigns ids, computes the announcement, and calls `State::publish`. `State::publish` uses `Publisher::prepare`, calls `publish_json`, then `Publisher::commit`; a failed publish consumes no revision and leaves the frame dirty for the next idle update.

`Message::Scrolled(region, offset)` inserts the offset, calls `scroll_started()`, sets `pending_scroll`, and returns a one-shot
`Task::perform(tokio::time::sleep(QUIESCENCE), move |_| Message::AccessibilitySettled(generation))`.
`AccessibilitySettled(gen)` calls `scroll_settled(gen)`; only the current generation clears the flag and publishes. A new touch, page step, screen change or overlay action calls `cancel_scroll()`, which increments `generation` and clears `pending_scroll` so a stale timer cannot publish, and/or marks the tree for the next idle update. On `Foreground(true)`, `Resized`, `Insets`, `Loaded`, `Mutated`, `Results`, `HistoryLoaded` and page commits, the normal idle publish runs.

- [ ] **Step 5: Implement action subscriptions and dispatch**

`subscribe()` returns an always-on `Subscription` over a process-global `tokio::sync::broadcast` channel (`OnceLock<broadcast::Sender<ActionRequest>>`). Each call to the subscription builder obtains a fresh `Sender::subscribe()` receiver, so a restarted subscription still works; the JNI export reads `nodeId`, `action`, `value` and `revision`, parses the action, and calls `send` without blocking. Ignore a send error when no receiver exists.

`App::accessibility_action` looks only at the last successfully published `State::frame` (never rebuilds one), and rejects requests whose revision is not the published revision, whose node id is absent, or whose action is not advertised by that node. While `pager.is_dragging() || pager.is_animating()`, reject item click/long-click, search `set_text`, and all scroll actions; title and overlay actions remain allowed. Dispatch the matched node's `Target`:

- `Target::Item(id)`: click finds the item and calls `App::activate`; long click calls `apply_gesture(Outcome::LongPress(Start::Item(id)))`.
- `Target::Step(delta)`: `self.step(delta)`.
- `Target::Search` / `Target::Back`: existing `Message::Search` / `Message::Back` paths.
- `Target::TextField(TextField::Search)`: click returns `iced::widget::operation::focus("search")`; `SetText` focuses `"search"` and dispatches `Message::Query(value)`.
- `Target::SearchTab(recent)`: `Message::History(recent)`.
- `Target::TextField(AppName|FolderName)`: click returns `operation::focus("overlay-app-name")` / `operation::focus("overlay-folder-name")`; set-text focuses the same id and dispatches the existing rename/folder buffer `overlay::Message`.
- `Action::ScrollLeft` / `Action::ScrollRight`: dispatch `Message::Step(-1)` / `Message::Step(1)` before the target match, so the grid node can advertise both page directions while still carrying a `Target::Scroll` for vertical movement.
- `Target::Scroll`: for `ScrollForward`/`ScrollBackward`, compute `clamp(offset ± 0.8 * viewport, 0, content - viewport)`, store it in `Region`, and call `iced::widget::operation::scroll_to(region.widget_id(), AbsoluteOffset { x: Some(0.0), y: Some(target) })`.
- `Target::Overlay(message)`: `self.overlay_message(message)`.
- `Target::Dismiss`: `overlay::Message::Back` through the existing overlay path.

- [ ] **Step 6: Add the JNI export**

In `accessibility.rs`:

```rust
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_bitshift_qlaunch_MainActivity_onAccessibilityAction(
    mut env: jni::JNIEnv,
    _this: jni::objects::JObject,
    node_id: jni::sys::jint,
    action: jni::objects::JString,
    value: jni::objects::JString,
    revision: jni::sys::jlong,
) { /* read strings, parse action, push request */ }
```

Re-export it under `cfg(target_os = "android")` from `lib.rs` so the cdylib keeps the symbol.

- [ ] **Step 7: Run the tests to verify they pass**

Run:

```sh
cd rust
cargo test -p qlaunch_iced --no-default-features --lib
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
```

Expected: PASS and clean.

---

### Task 5: Kotlin accessibility host view and bridge

**Files:**
- Create: `android/app/src/main/kotlin/bitshift/qlaunch/LauncherAccessibilityView.kt`
- Create: `android/app/src/main/kotlin/bitshift/qlaunch/AccessibilityDiff.kt`
- Create: `android/app/src/test/kotlin/bitshift/qlaunch/AccessibilityDiffTest.kt`
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt`
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt`
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/LauncherActions.kt`
- Modify: `android/app/build.gradle.kts`
- Build: `scripts/build-iced.sh` (debug)

**Interfaces:**
- Consumes: the JSON payload from Task 4; `MainActivity` native action export.
- Produces:

```kotlin
// AccessibilityDiff.kt
data class NodeRect(val left: Int, val top: Int, val right: Int, val bottom: Int)
data class NodeSnapshot(
    val id: Int,
    val role: String,
    val bounds: NodeRect,
    val label: String,
    val value: String?,
    val selected: Boolean,
    val enabled: Boolean,
    val actions: List<String>,
)
data class TreeSnapshot(
    val screen: String,
    val order: List<Int>,
    val nodes: Map<Int, NodeSnapshot>,
)
data class TreeDiff(
    val invalidateRoot: Boolean,
    val invalidated: Set<Int>,
    val textChanged: Set<Int>,
    val focusTarget: Int?,
)
fun diffTrees(old: TreeSnapshot?, new: TreeSnapshot, focusedId: Int?): TreeDiff

// LauncherAccessibilityView.kt
class LauncherAccessibilityView(context: Context) : View(context) {
    var onAction: ((nodeId: Int, action: String, value: String, revision: Long) -> Unit)? = null
    fun publish(json: String)
}

// MainActivity.kt (members)
private lateinit var accessibility: LauncherAccessibilityView
fun publishAccessibility(json: String)
private fun sendAccessibilityAction(nodeId: Int, action: String, value: String, revision: Long)
private external fun onAccessibilityAction(nodeId: Int, action: String, value: String, revision: Long)

// QLaunchBridge.kt
@JvmStatic fun publishAccessibility(json: String) = LauncherActions.publishAccessibility(json)

// LauncherActions.kt
fun publishAccessibility(json: String) // latest-payload coalescing, never FutureTask.get()
fun flushAccessibility(host: MainActivity) // publish the stored latest payload after view creation
```

Gradle adds exactly `implementation("androidx.customview:customview:1.1.0")`.

- [ ] **Step 1: Write the failing accessibility-diff tests**

`AccessibilityDiffTest.kt` exercises the pure focus/invalidation policy without Android classes:

```kotlin
@Test fun screenChangeInvalidatesRoot();
@Test fun orderChangeKeepsFocusAndInvalidatesMovedNodes();
@Test fun boundsChangeKeepsFocusAndInvalidatesThatNode();
@Test fun labelOnlyChangeUsesTextInvalidation();
@Test fun disappearingFocusedNodeMovesToTheNearestSurvivor();
@Test fun disappearingLastNodeFallsBackToTheHost();
@Test fun unchangedTreeInvalidatesNothing();
```

- [ ] **Step 2: Run the Kotlin tests to verify they fail**

```sh
cd /home/bronson/Projects/qlaunch
JAVA_SRC="$(cargo metadata --locked --format-version 1 --filter-platform aarch64-linux-android --no-default-features --manifest-path rust/Cargo.toml | python3 -c 'import json,sys,pathlib; p=next(p for p in json.load(sys.stdin)["packages"] if p["name"]=="iced_android"); print(pathlib.Path(p["manifest_path"]).parent / "java")')"
bash android/gradlew -p android "-PicedAndroidJava=$JAVA_SRC" testDebugUnitTest
```

Expected: FAIL because `diffTrees` and its data classes are absent.

- [ ] **Step 3: Implement `diffTrees`, then re-run the tests**

Rules: an old screen different from the new screen invalidates the root. Otherwise, with the focused id still present, keep it and invalidate only changed nodes; compare bounds, index (from `order`), label, value, selected, enabled and actions. A label/value-only change goes in `textChanged`, everything else in `invalidated`. Do **not** invalidate the root merely because result order changed, or typing would drop accessibility focus on every keystroke. With the focused id gone, choose the nearest survivor at or after its old index, then before it, and send focus there; invalidate the removed node. If none survives, invalidate the root. A null `focusedId` means host focus. Run the command from Step 2; expected: PASS.

- [ ] **Step 4: Add the AndroidX dependency and build the empty view**

Add the dependency, create the transparent `View`, and wire it into `MainActivity.onCreate` after `super.onCreate` and after the existing `window.setFormat`:

```kotlin
accessibility = LauncherAccessibilityView(this)
accessibility.onAction = ::sendAccessibilityAction
addContentView(
    accessibility,
    ViewGroup.LayoutParams(MATCH_PARENT, MATCH_PARENT),
)
LauncherActions.flushAccessibility(this)
```

The flush closes the startup race: iced can publish its first tree before `super.onCreate` returns, and the JNI call would otherwise succeed while there is no view to receive it.

`LauncherAccessibilityView` sets `importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_YES`, `isClickable = false`, installs the helper with `ViewCompat.setAccessibilityDelegate`, and overrides `dispatchHoverEvent` to delegate to the helper before `super`. It does not override `onTouchEvent` to return true. After constructing the helper, clear `isFocusable` and `isFocusableInTouchMode`; never call `requestFocus()`.

- [ ] **Step 5: Implement parsing, node population and bounds**

Parse `revision`, `screen`, `window`, `announcement` and `nodes` with `org.json`. Map the role strings to Android class names: `button` → `android.widget.Button`, `tab` → `android.widget.TabWidget`, `header` → `android.widget.TextView` (with `isHeading`), `text` → `android.widget.TextView`, `text_field` → `android.widget.EditText`, `scroll` → `android.widget.ScrollView`.

Scale bounds by `width / window.width` and `height / window.height` (guard zero dimensions), round to integers, and store `Rect`s. `onPopulateNodeForVirtualView` sets `isVisibleToUser = true`, text/label, `setBoundsInParent`, class, enabled/selected, clickable/long-clickable/scrollable/editable flags, `ACTION_SET_TEXT` for editable nodes, and only the standard actions advertised by the node. `getVirtualViewAt` prefers a non-scroll node over a containing scroll container; `getVisibleVirtualViews` preserves payload order and skips empty bounds.

- [ ] **Step 6: Implement action mapping and focus-preserving publish diff**

Map strings to `ACTION_CLICK`, `ACTION_LONG_CLICK`, `ACTION_SCROLL_FORWARD/BACKWARD/LEFT/RIGHT`, `ACTION_SET_TEXT` and `ACTION_DISMISS`. `onPerformActionForVirtualView` returns true only when the node advertises the action, then calls `onAction(nodeId, action, value, revision)`; for `ACTION_SET_TEXT`, read `ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE`.

On publish, call `diffTrees(previous, next, helper.accessibilityFocusedVirtualViewId.takeUnless { it == ExploreByTouchHelper.HOST_ID })` and apply it:

- `invalidateRoot`: call `invalidateRoot()` and announce.
- `invalidated`: call `invalidateVirtualView(id)`.
- `textChanged`: call `invalidateVirtualView(id, CONTENT_CHANGE_TYPE_TEXT)`.
- `focusTarget`: send `TYPE_VIEW_ACCESSIBILITY_FOCUSED` for that surviving node via `helper.sendEventForVirtualView(focusTarget, AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUSED)`; if the diff instead invalidated the root, TalkBack chooses the first node. Do not leave focus on a dead id.
- announcement: `announceForAccessibility` (or `TYPE_ANNOUNCEMENT`). Do not request input focus.

- [ ] **Step 7: Add the non-blocking bridge path**

`LauncherActions.publishAccessibility` always stores the latest string in an `AtomicReference`, then, if an activity is attached and no UI-thread post is outstanding, posts one with `host.runOnUiThread`. The posted runnable clears the posted flag, takes the latest payload, and calls `host.publishAccessibility`. Never use `onMain`, `FutureTask` or `get()` for publishing. A missing activity keeps the latest payload and does not throw.

`LauncherActions.flushAccessibility(host)` takes the stored latest payload and calls `host.publishAccessibility(json)` on the UI thread; it is called once after the accessibility view is added. `MainActivity.publishAccessibility` returns early unless `::accessibility.isInitialized`, then calls `accessibility.publish(json)` on the UI thread. `sendAccessibilityAction` calls the private native `onAccessibilityAction`, whose JNI signature is `(ILjava/lang/String;Ljava/lang/String;J)V`.

- [ ] **Step 8: Build the APK**

Run:

```sh
cd /home/bronson/Projects/qlaunch
ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build-iced.sh debug
```

Expected: Rust and Kotlin compile; the debug APK is produced.

---

### Task 6: Accessibility go/no-go on the tablet

**Files:**
- Modify: `docs/iced-spike.md` only after a result is known.
- No source changes unless a fixable bounds/action bug is found; fix it in the owning module and re-run that task's tests.

**Interfaces:**
- Consumes: release APK, TalkBack, `uiautomator`, the tablet's saved layout.
- Produces: a go/no-go decision and recorded findings.

- [ ] **Step 1: Build and install the release APK without clearing data**

```sh
cd /home/bronson/Projects/qlaunch
ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build-iced.sh release
adb -s 6bfcc911 install -r build/app/outputs/apk/release/app-release.apk
adb -s 6bfcc911 shell am start -W -n bitshift.qlaunch/.MainActivity
```

Expected: the saved Apps/Home/Games layout and wallpaper still render.

- [ ] **Step 2: Enable TalkBack and record its version**

```sh
adb -s 6bfcc911 shell pm list packages | grep com.google.android.marvin.talkback
adb -s 6bfcc911 shell dumpsys package com.google.android.marvin.talkback | grep -m1 versionName
adb -s 6bfcc911 shell settings get secure enabled_accessibility_services
```

Read the current service list and append `com.google.android.marvin.talkback/com.google.android.marvin.talkback.TalkBackService` rather than replacing it; set `accessibility_enabled` to `1`. If TalkBack is absent, install Android Accessibility Suite before judging the result. Restore the previous service list and enabled flag after the run.

- [ ] **Step 3: Verify the first five acceptance items**

With TalkBack, confirm all of:

1. It announces the page and first focusable item; forward/backward swipes reach title slots, grid container, visible apps and error/warning text in published order.
2. Double-tap launches an app, opens a folder, steps a title side slot, and opens search from the centre slot.
3. Long-click opens the app menu and blank-grid launcher menu; Rename, Move, Uninstall, Cancel/Back work.
4. Search results are reachable; the real keyboard opens; typing updates the query/results without resetting accessibility focus on every keystroke.
5. Vertical scroll/page actions move content; after a fling settles, focus rectangles are on the rendered cells.

If any of these five fail after a focused implementation pass and cannot be explained by a fixable bounds/action bug, stop here. Reinstall the saved Flutter release with `adb -s 6bfcc911 install -r /tmp/opencode/qlaunch-iced-baseline/flutter-release.apk`, record the failure in `docs/iced-spike.md`, and do not start the pager work.

- [ ] **Step 4: Verify the remaining acceptance items**

Check that focus survives scroll/page steps when the node exists; no node exists for a hidden page, closed folder or closed menu; an item action during drag/settle does not launch or open a menu; and `uiautomator dump` contains matching labels and plausible bounds. Sample an app cell, folder header, search row and overlay action and compare each to the drawn screenshot; each must be within 2 dp or be reported with the measured delta and reason.

With TalkBack off, verify ordinary tap, long press, double tap, pull-down, hardware keyboard and the search/rename soft keyboard are unchanged. The transparent view must consume nothing.

- [ ] **Step 5: Record the result**

Add an "Accessibility" section to `docs/iced-spike.md` with the TalkBack version, the five go/no-go results, text-field limits, `uiautomator`/bounds findings, and any remaining gaps. Do not commit screenshots or personal wallpaper images.

---

### Task 7: Custom `Slots` widget

**Files:**
- Create: `rust/qlaunch_iced/src/slots.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod slots;`)
- Test: `rust/qlaunch_iced/src/slots.rs`

**Interfaces:**
- Consumes: `iced::Element`, `iced::advanced::widget::Widget`, `iced::advanced::{layout, mouse, renderer, Clipboard, Shell}`.
- Produces:

```rust
pub(crate) struct Slot<'a> {
    pub key: String,
    pub content: iced::Element<'a, crate::Message>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Interaction { None, Centre, All }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arrangement { Pages, Strip }

/// Pure geometry used by both `layout`/`draw` and the unit tests.
pub(crate) fn child_bounds(
    arrangement: Arrangement,
    index: usize,
    width: f32,
    height: f32,
) -> iced::Rectangle;
pub(crate) fn translation(
    arrangement: Arrangement,
    index: usize,
    offset: f32,
    width: f32,
) -> iced::Vector;

pub(crate) struct Slots<'a> { /* three slots, arrangement, interaction */ }
impl<'a> Widget<crate::Message, iced::Theme, iced::Renderer> for Slots<'a> { /* ... */ }
impl<'a> Slots<'a> {
    pub(crate) fn pages(
        slots: [Slot<'a>; 3],
        offset: f32,
        width: f32,
        interaction: Interaction,
    ) -> Self;
    pub(crate) fn strip(
        slots: [Slot<'a>; 3],
        offset: f32,
        width: f32,
        height: f32,
        interaction: Interaction,
    ) -> Self;
}
```

Geometry: pages use `dx = (index as f32 - 1.0 - offset) * width`, each child at `Rectangle::new(0, 0, width, height)`; strip uses a child width of `width / 3`, child x `index as f32 * width / 3`, and translation `-offset * width / 3`. `child_bounds` returns the untranslated child rectangle; `translation` returns the `Vector`. Both draw inside one `renderer.with_layer(bounds, ...)` with each child in `with_translation`. `overlay` returns `None`.

- [ ] **Step 1: Write the failing slot tests**

```rust
#[test] fn page_translations_follow_the_pager_offset();
#[test] fn strip_children_are_thirds_and_translate_together();
#[test] fn strip_translation_never_changes_child_spacing();
#[test] fn changed_slot_key_marks_only_that_tree_for_recreation();
#[test] fn window_events_reach_the_centre_child_when_input_is_gated();
#[test] fn touch_events_do_not_reach_a_gated_child();
#[test] fn touch_events_reach_the_centre_only_for_pages();
#[test] fn touch_events_reach_all_three_settled_strip_slots();
```

Use a small recording child widget in tests to observe `update`. Slot math tests call `child_bounds` and `translation` directly, not a renderer. The recording widget can be a unit struct whose `size()` is `Fill × Fill` and whose `update` pushes each event into a shared `Rc<RefCell<Vec<Event>>>`; build it as `Element` so the test can construct `Slot`s without a window.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib slots`
Expected: FAIL because the module is missing.

- [ ] **Step 3: Implement the widget and state reconciliation**

`tag()` uses a dedicated marker type. `state()` stores the three semantic keys. `children()` returns one tree per child. `diff()` compares each child's key against the stored key: on a key change replace `tree.children[i]` with `Tree::new(&slot.content)`; otherwise call `tree.children[i].diff(&slot.content)`. This prevents a scrollable's offset from leaking into another group.

`layout()` resolves `size()` (`Fill × Fill` for pages, `Fill × Fixed(height)` for the strip), lays each child out with `layout::Limits::new(Size::ZERO, child_size)`, and positions strip children at their thirds. `draw()` clips once and translates each child. `update()` always forwards `Event::Window` to the centre child, including `RedrawRequested`; all other events go only to children allowed by `Interaction`, with the cursor and viewport shifted by the inverse translation. `mouse_interaction()` and `operate()` forward only when allowed; `operate()` forwards the centre page so `scroll_to("apps")` and `reset_scroll` still reach the scrollable.

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```sh
cd rust
cargo test -p qlaunch_iced --no-default-features --lib slots
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
```

Expected: PASS and clean.

---

### Task 8: Replace `Float` pages and title strip

**Files:**
- Modify: `rust/qlaunch_iced/src/view.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` only if the slot keys need an accessor
- Test: `rust/qlaunch_iced/src/lib.rs` and existing view/pager tests

**Interfaces:**
- Consumes: `slots::{Slot, Slots, Interaction}`, `Pager`, `grid` metrics, `Region`.
- Produces: the same visual behaviour with no `float`/`Stack` overlay path, plus two pure plans the view consumes and tests assert:

```rust
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PageSlotPlan {
    pub key: String,
    pub group: Option<qlaunch_core::model::Id>,
    pub scrollable: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct TitleSlotPlan {
    pub key: String,
    pub label: String,
    pub message: Message,
}
```

`Message` is not `PartialEq`; assert title messages with `matches!` rather than equality.

```rust
pub(crate) fn page_slot_plan(app: &App) -> [PageSlotPlan; 3];
pub(crate) fn title_slot_plan(app: &App) -> [TitleSlotPlan; 3];
```

- [ ] **Step 1: Write the failing integration tests**

```rust
#[test] fn page_and_title_slot_keys_follow_wrapped_groups();
#[test] fn only_the_centre_page_is_scrollable_and_named_apps();
#[test] fn title_buttons_keep_step_and_search_messages();
```

Use the existing test app fixture and assert `page_slot_plan` / `title_slot_plan`; do not require a renderer. `page_slot_plan` marks only index 1 scrollable and names `"apps"`; neighbours are static.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib page_and_title_slot_keys`
Expected: FAIL because the `Slots` view helper does not exist.

- [ ] **Step 3: Replace `pages` and `title_strip`**

Implement `page_slot_plan` and `title_slot_plan` from `pager.slots()`, the committed page ids and `snapshot.groups`; the view consumes them. Page keys are `page:<group-id>` (and `page:empty:<slot>` when a slot has no group); only the centre plan is scrollable with widget id `"apps"` and `Region::Group(group)` on-scroll. Title keys are `title:<group-id>:<slot>`, labels are the wrapped group labels, opacity uses the same `title_button` formula, and messages are `Step(-1)`, `Search`, `Step(1)`.

Build each page element from the plan in order; neighbours remain static clipped containers. Set `Interaction::Centre` when `!pager.is_dragging() && !pager.is_animating()`, otherwise `Interaction::None`.

Build the title strip from its plan. Set `Interaction::All` when settled and `Interaction::None` during motion. Keep the strip above the pager and keep the outer `mouse_area(BlankPressed)` wrapper around pages.

Remove the `float`, `Stack` and `Vector` page/title usage. Do not change `pager.rs`, the animation subscription, the scroll reset or the gesture classifier.

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```sh
cd rust
cargo test -p qlaunch_iced --no-default-features --lib
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
```

Expected: PASS; the existing 76 iced tests plus the new slot/tree tests pass.

---

### Task 9: Device pager verification, measurements and documentation

**Files:**
- Modify: `docs/iced-spike.md`
- No source changes unless verification finds a bug; fix it in the owning module and re-run that task's tests.

- [ ] **Step 1: Build and install release on the tablet and phone**

```sh
cd /home/bronson/Projects/qlaunch
ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build-iced.sh release
adb devices
adb -s 6bfcc911 install -r build/app/outputs/apk/release/app-release.apk
adb -s 6bfcc911 shell am start -W -n bitshift.qlaunch/.MainActivity
```

If the Pixel 5a is connected, read its serial from `adb devices` and repeat the install/start with `-s <that serial>`. If it is not connected, record the phone check as not run rather than inventing a serial.

Expected: each connected device installs over its existing data and shows its saved layout. Compare the tablet home/apps screenshots with `/tmp/opencode/qlaunch-iced-baseline/tablet-flutter.png`; do not commit personal screenshots.

- [ ] **Step 2: Verify mid-drag rendering**

Record a slow drag in both directions and a wrap, then inspect extracted frames:

```sh
adb -s 6bfcc911 shell rm -f /sdcard/qlaunch-pager.mp4
adb -s 6bfcc911 shell screenrecord --time-limit 15 --bit-rate 8000000 /sdcard/qlaunch-pager.mp4 &
SCREENRECORD_PID=$!
sleep 2
adb -s 6bfcc911 shell input touchscreen swipe 1050 1200 150 1200 1200
adb -s 6bfcc911 shell input touchscreen swipe 150 1200 1050 1200 1200
wait "$SCREENRECORD_PID"
adb -s 6bfcc911 pull /sdcard/qlaunch-pager.mp4 /tmp/opencode/qlaunch-iced-pager.mp4
mkdir -p /tmp/opencode/qlaunch-iced-pager-frames
rm -f /tmp/opencode/qlaunch-iced-pager-frames/frame-*.png
ffmpeg -i /tmp/opencode/qlaunch-iced-pager.mp4 -vf fps=5 /tmp/opencode/qlaunch-iced-pager-frames/frame-%03d.png
```

Use screenshots at several offsets. Confirm both neighbours, the current page and the title strip are visible with no overlap, dropout or flicker, in both directions and across a wrap.

- [ ] **Step 3: Verify input routing and fling continuity**

Confirm a drag starting on an icon pages without launching; a tap still launches; only the settled page scrolls or launches; taps during a settle cannot hit a neighbour; and a title step during a vertical fling keeps the fling animating and settling on the same content without a jump. Use `adb shell input` plus logcat `frame-work` lines while doing this.

- [ ] **Step 4: Measure tablet frame pacing**

```sh
SERIAL=6bfcc911 REFRESH=90 RUNS=5 bash scripts/profile-iced.sh
```

Expected: a per-run SurfaceFlinger interval table and median, plus `frame-work` lines for each page motion. Keep the same layout/settings and a cool device. Record the existing tablet baseline separately: iced p50/p95/p99 = 11.14/11.15/11.17 ms, max 11.22 ms, 0.0% over 16.7 ms. Do not compare SurfaceFlinger intervals with app-side `update`/`view` work as if they were the same number.

- [ ] **Step 5: Update `docs/iced-spike.md`**

Add or replace:

- An "Accessibility" section: TalkBack result, bounds check, actions, focus retention, text-field limits, and the go/no-go conclusion.
- A "Page renderer" section: `Slots` replaces `Float`, mid-drag screenshot/video result, input gating and fling finding, and any remaining fork limitation.
- A tablet frame table with the device, 90 Hz refresh, run count, date and the measured p50/p95/p99/max plus over-budget share.
- Update "Deliberate gaps" and "Can Dart/Flutter be removed?" so accessibility and the `Float` overlay limitation are no longer listed as unaddressed if the checks pass.

- [ ] **Step 6: Run the full regression**

```sh
cd rust
cargo test -p qlaunch_core --no-default-features
cargo test -p qlaunch_iced --no-default-features --lib
cargo fmt -p qlaunch_iced --check
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cd ..
python3 -m unittest discover -s scripts -p 'test_*.py'
ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build-iced.sh release
```

Expected: all pass; the saved layouts and existing gesture/menu/search/lifecycle checks still work. Record any remaining gaps and stop.
