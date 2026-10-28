# Iced Gestures and Page Transitions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add long press, double tap, pull-down and finger-following page transitions to the Flutter-free iced launcher, then measure its frame work on the Pixel 5a against the saved Flutter baseline.

**Architecture:** Raw iced touch events feed a pure gesture classifier; a pure pager state machine owns the committed page, signed fractional offset and settle animation. The view renders three wrapped page slots in translated, clipped `Float` layers, with menus as an in-app overlay. Core calls stay behind the existing async worker boundary.

**Tech Stack:** Rust 2024, `qlaunch_iced` on the `iced_android` v0.1.1 fork, the existing `qlaunch_core` model/API, Kotlin/JNI OS adapter, Android SDK/NDK, Python 3 and `adb` for measurement.

**Spec:** `docs/superpowers/specs/2026-10-28-iced-gestures-and-page-transition-design.md`

## Global Constraints

- `qlaunch_iced` must build with `--no-default-features` and must not depend on Flutter, Dart or `flutter_rust_bridge`.
- Core/JNI calls run in `work(...)` (`tokio::task::spawn_blocking`), never on iced's UI thread.
- Gesture constants: long press 500 ms, touch slop 18 logical px, double-tap window 300 ms, double-tap slop 100 logical px, pull-down trigger 48 logical px, pager fling threshold 300 logical px/s.
- Page animation: `easeOutCubic` over `settings.page_transition_ms` clamped 100–800 ms (default 300 ms).
- Format only `qlaunch_iced` with `cargo fmt -p qlaunch_iced`; never run workspace-wide `cargo fmt`.
- Strict Clippy applies to this crate: `cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings`.
- No new Android permissions, no settings/group-editor/shortcut-creation screens, no production hardening.
- Menus call existing core APIs; no model rule is reimplemented in the UI.
- Animation frames come from `window::frames()` only while the pager is animating; no permanent frame subscription.

## Review Focus

- A swipe that starts on an icon must never launch it, while a tap with a small drift must launch exactly once.
- A long press must not fire if the finger is released or drifts beyond slop before 500 ms.
- One-page and empty groups, plus snapshot changes mid-gesture, must not panic, jump or lose the current group.
- While an overlay is open, the pager, lock and pull-down must not react underneath; Back closes the overlay first, and an invalid rename must keep the menu open with an error.
- A resize/orientation change during an animation must not corrupt the committed page or leave the pager stuck animating.

---

### Task 1: Checkpoint the existing spike

**Files:**
- Commit only; no source edits.

**Interfaces:**
- Consumes: the current uncommitted `dev/iced` spike.
- Produces: a clean working tree so every later task is an isolated commit.

- [ ] **Step 1: Verify the baseline builds and passes**

```sh
cd rust
cargo test -p qlaunch_core --no-default-features
cargo test -p qlaunch_iced --no-default-features --lib
cargo fmt -p qlaunch_iced --check
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
```

Expected: all pass. If Clippy fails, fix the iced crate only.

- [ ] **Step 2: Review what is being committed**

```sh
cd /home/bronson/Projects/qlaunch
git status --short
git diff --stat
```

Expected: only the known spike work (Android adapter/configuration, `qlaunch_core`, `rust/qlaunch_iced/`, `scripts/`, `docs/iced-spike.md`, this spec and plan, Rust manifests/lockfile, README/.gitignore). Do not add `build/`, `target/`, `android/app/src/main/jniLibs/`, or the saved Flutter artifacts under `/tmp`.

- [ ] **Step 3: Commit the checkpoint**

```sh
git add -A
git diff --cached --stat
git commit -m "spike: baseline Flutter-free iced launcher"
```

- [ ] **Step 4: Confirm the tree is clean**

Run: `git status --short`
Expected: no output.

---

### Task 2: Pager state machine

**Files:**
- Create: `rust/qlaunch_iced/src/pager.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod pager;` next to `mod placeholder;`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `Pager::new(count: usize, page: usize) -> Pager`
  - `Pager::{count, committed, offset, position, slots, is_animating, is_dragging}`
  - `Pager::{set_count, begin_drag, drag, end_drag, cancel_drag, step, settle_to, tick}`
  - `offset()` is signed and stays in `(-1.0, 1.0)`; `slots()` is `[prev, current, next]` with wrapping; count 0 yields `[0, 0, 0]`.

**Pinned behavior:**
- Dragging maps total finger travel to `raw = -total_dx / width`. The committed page is `wrap(drag_origin + raw.trunc())` and the offset is `raw.fract()`, so a long drag commits pages as it crosses each full width.
- Release bias: if `|velocity_x| >= 300.0`, `bias = (-velocity_x / width * 0.5).clamp(-0.5, 0.5)`, else 0. `target = (offset + bias).round()` clamped to `-1..=1`.
- Animation easing: `1.0 - (1.0 - t).powi(3)`. At `t >= 1.0`, commit `target` (`committed = wrap(committed + target)`) and set offset 0.
- `step(delta)`: 0 is a no-op; `|delta| == 1` animates offset 0 to delta; any other value commits immediately with no animation.
- `settle_to(page)`: same page is a no-op; forward distance 1 animates `+1`; backward distance 1 animates `-1`; anything else is immediate.
- Count 0 makes every mutator a no-op and keeps committed 0.

- [ ] **Step 1: Write the failing pager tests**

Add `#[cfg(test)] mod tests` in `pager.rs` with a `const D: Duration = Duration::from_millis(300);` and these tests:

```rust
#[test] fn new_starts_at_page_and_wraps_slots();       // new(3,1): committed 1, offset 0, slots [0,1,2]
#[test] fn slots_wrap_at_both_ends();                  // new(3,0).slots()==[2,0,1]; new(3,2).slots()==[1,2,0]
#[test] fn short_drag_settles_back();                  // drag(-30, 300) => offset ~ +0.1; end v=0; tick(D) => (0, 0)
#[test] fn drag_past_half_settles_next();              // drag(-180, 300) => offset ~ 0.6; end v=0; tick(D) => committed 1
#[test] fn backwards_fling_goes_to_previous();         // drag(+90, 300); end v=+400 => prev, tick(D)
#[test] fn forward_fling_adds_half_page_bias();        // drag(-90, 300); end v=-400 => next, tick(D)
#[test] fn long_drag_commits_each_crossed_page();      // drag(-640, 320) from 0 => committed 2, slots [1,2,0]
#[test] fn step_animates_one_page_and_wraps();         // new(3,2).step(1) animates; tick(D) => committed 0
#[test] fn multi_step_is_immediate();                  // step(-3) => committed wraps immediately, !is_animating()
#[test] fn settle_to_adjacent_animates_and_far_is_immediate();
#[test] fn single_page_always_stays_on_page_zero();    // new(1,0): drag/end/tick and step leave committed 0
#[test] fn empty_pager_is_inert();                     // new(0,5): committed 0, slots [0,0,0], no animation
#[test] fn set_count_clamps_and_keeps_the_requested_page();
#[test] fn resize_mid_animation_does_not_change_the_target(); // animation is time-based, tick(D) commits target
```

Assertions use `(committed(), offset())` tuples and an epsilon of `1e-4` for fractions.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib pager`
Expected: FAIL — `Pager` is undefined/module missing.

- [ ] **Step 3: Implement `Pager`**

State: `committed: usize`, `count: usize`, `offset: f32`, `drag_origin: usize`, `dragging: bool`, `animation: Option<Animation>`, where `Animation { from: f32, to: f32, started: Instant, duration: Duration }`. Implement `wrap(index: i32, count: usize) -> usize` with `rem_euclid`. `tick` replaces the animation when it completes; `step`/`settle_to` call a private `animate(to, now, duration)` and set `from = self.offset`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib pager`
Expected: PASS.

- [ ] **Step 5: Format, lint and commit**

```sh
cd rust
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cd ..
git add rust/qlaunch_iced/src/pager.rs rust/qlaunch_iced/src/lib.rs
git commit -m "feat(iced): add pager state machine"
```

---

### Task 3: Gesture classifier

**Files:**
- Create: `rust/qlaunch_iced/src/gesture.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod gesture;`)

**Interfaces:**
- Consumes: `qlaunch_core::model::Id` and `iced::{Point, touch}`.
- Produces:

```rust
pub const LONG_PRESS: Duration = Duration::from_millis(500);
pub const TOUCH_SLOP: f32 = 18.0;
pub const DOUBLE_TAP: Duration = Duration::from_millis(300);
pub const DOUBLE_TAP_SLOP: f32 = 100.0;
pub const PULL_DOWN: f32 = 48.0;
pub const FLING: f32 = 300.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start { Item(Id), Blank }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase { Idle, Pressed, Dragging, Pulled, LongPressed }

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    None,
    DragStart,
    Drag,
    DragEnd { velocity: f32 },
    PullDown,
    LongPress(Start),
    Tap(Start),
    DoubleTap,
    Cancelled,
}

impl Gesture {
    pub fn new() -> Self;
    pub fn phase(&self) -> Phase;
    pub fn target(&self) -> Option<&Start>;
    pub fn is_tap_candidate(&self) -> bool;
    pub fn sequence(&self) -> u64;
    pub fn pressed(&mut self, finger: touch::Finger, position: Point, at: Instant) -> u64;
    pub fn identify(&mut self, target: Start) -> Outcome;
    pub fn moved(&mut self, finger: touch::Finger, position: Point, at: Instant, at_top: bool) -> Outcome;
    pub fn released(&mut self, finger: touch::Finger, position: Point, at: Instant) -> Outcome;
    pub fn lost(&mut self, finger: touch::Finger) -> Outcome;
    pub fn tick(&mut self, sequence: u64, now: Instant) -> Outcome;
    pub fn drag_dx(&self) -> f32;
}
```

**Pinned behavior:**
- `pressed` starts a new sequence only when idle; a second finger while active leaves state untouched. `identify` sets the target while `Pressed`; if the press is already ≥500 ms old with no slop drift, it immediately returns `LongPress(target)` (late message delivery). When the phase is `Idle`, `identify` is a no-op returning `None`; the app keeps the hint in `pending_target` and retries after the press message.
- Track `slop_exceeded` for any movement whose distance from the down point exceeds 18 px. A tap requires the target to be known, slop not exceeded, and no drag/pull/long-press phase. `is_tap_candidate()` is exactly that predicate while `Pressed`.
- Precedence on move: pull-down when target known, `at_top`, `dy > 48`, and `dy > 1.5 * |dx|`; otherwise drag-start when `|dx| > 18` and `|dx| > 1.5 * |dy|`; afterwards drag updates. Phase `Pulled`/`LongPressed` ignores the rest of the gesture.
- Velocity is an exponential average (0.4 of the instantaneous `dx/dt`) over moves, starting at 0.
- Double tap only from `Start::Blank`: second release within 300 ms and 100 px of the previous blank tap returns `DoubleTap`. Item taps and any drag/pull clear the pending blank tap.
- `tick` fires only for the current sequence, in `Pressed`, without slop drift, at least 500 ms after down, with a known target. Early release, drift, or a stale sequence returns `None`.
- `lost` returns `Cancelled` only from `Dragging`; otherwise `None`. It always resets to idle.

- [ ] **Step 1: Write the failing gesture tests**

Add a helper that presses at `Instant::now()` and identifies `Start::Item("a")` (or `Blank`), then these tests:

```rust
#[test] fn tap_on_item_is_a_tap();
#[test] fn tap_with_drift_under_slop_is_still_a_tap();
#[test] fn drag_starting_on_item_is_never_a_tap();      // DragStart, Drag, DragEnd, no Tap
#[test] fn horizontal_drag_must_dominate_vertical();
#[test] fn long_press_fires_at_500ms();                  // tick(+499)=None, tick(+500)=LongPress(Item)
#[test] fn long_press_cancelled_by_slop_drift();
#[test] fn long_press_cancelled_by_early_release();
#[test] fn stale_long_press_timer_is_ignored();
#[test] fn long_press_on_blank_reports_blank();
#[test] fn double_tap_on_blank_within_window();         // Tap(Blank) then DoubleTap
#[test] fn slow_second_blank_tap_is_not_a_double();
#[test] fn distant_second_blank_tap_is_not_a_double();
#[test] fn double_tap_never_fires_from_an_item();       // Tap, Tap; never DoubleTap
#[test] fn pull_down_requires_top_and_48px();
#[test] fn pull_down_fires_once_per_gesture();
#[test] fn lost_mid_drag_cancels();
#[test] fn tap_candidate_clears_after_slop_drift();   // is_tap_candidate() true after press, false after an 18+ px move
#[test] fn press_on_chrome_without_target_never_taps();
#[test] fn drag_velocity_is_reported();
#[test] fn second_finger_does_not_restart_the_gesture();
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib gesture`
Expected: FAIL — `Gesture` is undefined.

- [ ] **Step 3: Implement `Gesture`**

Use a single `Option<Finger>`, `down: Point`, `last: Point`, `last_move: Option<(Instant, Point)>`, `velocity`, `have_velocity_sample`, `sequence: u64`, `phase`, `target: Option<Start>`, `slop_exceeded`, and `last_blank_tap: Option<(Instant, Point)>`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib gesture`
Expected: PASS.

- [ ] **Step 5: Format, lint and commit**

```sh
cd rust
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cd ..
git add rust/qlaunch_iced/src/gesture.rs rust/qlaunch_iced/src/lib.rs
git commit -m "feat(iced): add gesture classifier"
```

---

### Task 4: Grid geometry and view extraction

**Files:**
- Create: `rust/qlaunch_iced/src/grid.rs`
- Create: `rust/qlaunch_iced/src/view.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (module list; move `view`/`cell`/`label`; add scroll tracking)

**Interfaces:**
- Consumes: `qlaunch_core::model::{Id, Item}`, `qlaunch_core::api::api::ModelSnapshot`, `qlaunch_core::settings::{LayoutStyle, Settings}`, `iced_android::Insets`.
- Produces:
  - `grid::title_height(settings: &Settings) -> f32` = `(label_size * 1.4 + 20.0).max(48.0)`.
  - `grid::row_height(settings: &Settings) -> f32` = the exact `.height(...)` currently computed in `App::cell` (list: `max(icon_size, label_size*2) + 10`; tiles: icon height if visible + label height + `max(label_height*0.3, 8)`, label height `label_size*2.34`).
  - `grid::content_width(size: iced::Size, insets: Insets) -> f32` = `(size.width - insets.left - insets.right).max(1.0)`.
  - `view::launcher(app: &App) -> Element<'_, Message>`.
  - `App::scroll: HashMap<Id, f32>`, `Message::Scrolled(Id, f32)`, `App::current_group_id(&self) -> Option<&Id>`.

**Pinned behavior:** No visual change in this task. The scrollable keeps one stable id `"apps"`; its `on_scroll` closure captures the currently displayed group id and sends `Message::Scrolled(group, viewport.absolute_offset().y)`. Search results use id `"search-results"` and do not report. `App::update` inserts the offset into `scroll`; the map is the source for "at top" later.

- [ ] **Step 1: Write failing grid and scroll tests**

In `grid.rs`:

```rust
#[test] fn title_height_is_at_least_48();      // label_size 10 => 48; label_size 40 => > 48
#[test] fn row_height_matches_the_cell_body(); // default Settings => tiles 54 + 44.12 + 12.6 (approx)
#[test] fn content_width_subtracts_insets();   // 1080 wide, insets 10/30 => 1040
```

In `lib.rs` tests:

```rust
#[test] fn scrolled_tracks_offsets_per_group(); // update(Scrolled("g", 42.0)); scroll["g"] == 42.0
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib grid`
Expected: FAIL — module missing.

- [ ] **Step 3: Create `grid.rs` and move the view**

Move `App::view`, `App::cell` and `App::label` into `view.rs` as free functions taking `app: &App`; `App::view` in `lib.rs` becomes `view::launcher(self)`. Keep `App::columns`, `home_index`, `visible_items` and `warm_icons` on `App`, marking any fields `view.rs` reads as `pub(crate)` (`snapshot`, `icons`, `folder`, `searching`, `query`, `results`, `frequent`, `recent`, `show_recent`, `error`, `insets`, `size`, `scale`, `density`). Use `grid::row_height` from the cell and `grid::title_height` for the tab row. Add `.on_scroll(...)` to `scrollable(...)` outside search.

- [ ] **Step 4: Run tests, format, lint and commit**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib`
Expected: PASS (both new and existing tests).

```sh
cd rust
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cd ..
git add rust/qlaunch_iced/src/grid.rs rust/qlaunch_iced/src/view.rs rust/qlaunch_iced/src/lib.rs
git commit -m "refactor(iced): extract grid geometry and view"
```

---

### Task 5: Animated pages and title strip

**Files:**
- Modify: `rust/qlaunch_iced/src/lib.rs`
- Modify: `rust/qlaunch_iced/src/view.rs`

**Interfaces:**
- Consumes: `Pager` from Task 2, `grid::{content_width, title_height}`.
- Produces:
  - `App::pager: Pager` replaces `App::page: usize`; all `self.page` uses become `self.pager.committed()`.
  - `Message::Frame(Instant)`; `subscription` adds `window::frames().map(Message::Frame)` only while `pager.is_animating()`.
  - `App::transition(&self) -> Duration` from clamped `settings.page_transition_ms`.
  - `App::reset_scroll() -> Task<Message>` inserts `0.0` for the current group and scrolls id `"apps"` to `AbsoluteOffset { x: Some(0.0), y: Some(0.0) }`.

**Pinned behavior:**
- `step(delta)` calls `pager.step(delta, now, transition)`; Home calls `pager.settle_to(home, now, transition)` when not searching/folder.
- On `Loaded` snapshot replacement: first load goes to the home page; later replacements keep the current group id if it still exists, otherwise use index 0 via `pager.set_count(count, index)`. Task 6 reuses this logic for mutation results.
- View: `let [prev, current, next] = app.pager.slots();` and `let content_width = grid::content_width(app.size, app.insets);`. Build each slot as a full-width/height container and push it into a `stack!` as `float(slot).translate(move |_, _| Vector::new((i as f32 - 1.0 - app.pager.offset()) * content_width, 0.0))` for `i = 0..3`; wrap the stack in `container(...).width(Fill).height(Fill).clip(true)`.
- Only the middle/current slot is a `scrollable` with id `"apps"`; neighbour slots are static `container(grid).height(Fill).clip(true)` (no independent scrolling). This keeps one scroll state, not three. Group scroll position resets to top on every committed page change; document this as a spike gap.
- Title strip: three fixed `pitch = content_width / 3.0` buttons in a row, translated by `-app.pager.offset() * pitch` inside a clipped container of `grid::title_height`. Slot opacities are `1 - (slot - offset).abs() * 0.6` with slot `-1, 0, 1`.
- `warm_icons` loads all `pager.slots()` groups.
- `Frame(now)` ticks the pager; if the committed page changed, reset the scroll and clear the pressed item. Animation completion must leave `is_animating()` false so the next subscription drops `window::frames()`.

- [ ] **Step 1: Write failing app tests**

```rust
#[test] fn step_animates_then_commits();          // Step(1) is animating; Frame(now+1s) => committed 2 (wraps)
#[test] fn home_from_a_neighbour_animates();      // from page 2 with home 1: Home animates to 1
#[test] fn home_from_a_far_page_is_immediate();   // snapshot with 4 pages and home at 1; from page 3, Home commits at once
#[test] fn resize_mid_animation_keeps_the_target(); // Step(1), Resized(new), Frame(+1s) => committed target
#[test] fn snapshot_reload_keeps_the_current_group(); // move current group in page_ids, update Loaded, committed follows id
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib step_animates_then_commits`
Expected: FAIL — no `pager`/`Frame`.

- [ ] **Step 3: Implement pager integration and animated rendering**

Replace `page` with `pager` everywhere (`home_index` callers, `visible_items`, `Loaded`, `HomePressed*`, tab labels, `warm_icons`, tests). Add `Frame` handling. Update the view as above; import `iced::Vector` and `iced::widget::{float, stack}`. Use `iced::widget::operation::scroll_to` for `reset_scroll`.

- [ ] **Step 4: Run tests, format, lint and commit**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib`
Expected: PASS.

```sh
cd rust
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cd ..
git add rust/qlaunch_iced/src/lib.rs rust/qlaunch_iced/src/view.rs
git commit -m "feat(iced): animate paging and the title strip"
```

---

### Task 6: Overlay menus and core actions

**Files:**
- Create: `rust/qlaunch_iced/src/overlay.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs`
- Modify: `rust/qlaunch_iced/src/view.rs`

**Interfaces:**
- Consumes: `qlaunch_core::model::{Group, Id, Item}`, `qlaunch_core::api::api::ModelSnapshot`, `qlaunch_core::api::shortcuts::ShortcutProvider`, existing `work` helper.
- Produces:

```rust
pub enum Overlay { App(AppMenu), Launcher(LauncherMenu) }
pub struct AppMenu { pub item: Item, pub step: AppStep, pub error: Option<String> }
pub enum AppStep { Actions, Rename(String), Move }
pub struct LauncherMenu {
    pub step: LauncherStep,
    pub providers: Vec<ShortcutProvider>,
    pub providers_loading: bool,
    pub error: Option<String>,
}
pub enum LauncherStep { Actions, FolderName(String), Shortcuts }
pub enum Message {
    Close, Noop, Back,
    AppRename(String), AppRenameSubmit, AppMove, AppMoveTo(Id), AppUninstall, AppRemove,
    LauncherSettings, LauncherAddShortcut, LauncherCreateFolder,
    LauncherFolderName(String), LauncherFolderSubmit, LauncherEditGroups, LauncherWallpaper,
    ShortcutLock, ShortcutNotifications, ShortcutProvider(String),
    Providers(Result<Vec<ShortcutProvider>, String>),
}
pub fn move_targets<'a>(snapshot: &'a ModelSnapshot, item: &Item) -> Vec<&'a Group>;
pub fn append_order(snapshot: &ModelSnapshot, group: &Id) -> i32;
pub fn view<'a>(app: &'a App, overlay: &'a Overlay) -> Element<'a, crate::Message>;

impl Overlay {
    /// Steps a sub-menu back to its action list; returns true when the
    /// overlay itself should close.
    pub fn back(&mut self) -> bool;
}
```

  App additions:
  - `overlay: Option<overlay::Overlay>`
  - `Message::{Overlay(overlay::Message), Mutated(Result<ModelSnapshot, String>), ActionDone(Result<(), String>)}`

**Pinned behavior:**
- `move_targets` keeps page groups only: exclude `snapshot.root_group_id`, `group.folder == true`, and `group.id == item.group_id`, preserving `snapshot.groups` order. `append_order` is the destination group's item count.
- App actions: Rename/Move/Remove/Create Folder/Lock-Shortcut/Notifications-Shortcut call `rename_item`/`move_item`/`remove_item`/`add_folder`/`add_lock_shortcut`/`add_notifications_shortcut` and deliver `Mutated`; Uninstall, Wallpaper and provider shortcut start deliver `ActionDone`. All run through `work`. Successful mutation/action closes the overlay; a mutation error or a menu action error is stored in the open overlay's `error` and the overlay stays open. `ActionDone` with no overlay (double-tap lock, pull-down notifications) stores failures in `self.error` instead.
- The containing group for Create Folder, Lock Shortcut and Notifications Shortcut is always the page group `snapshot.page_ids[pager.committed()]`, never a folder group, even if a folder is currently open.
- Empty rename or folder name (trimmed) sets `error = Some("Name cannot be empty")` without a core call.
- Add Shortcut sets `LauncherStep::Shortcuts`, `providers_loading = true` and loads `shortcut_providers()`.
- Settings and Edit Groups set `error = Some("Not ported in this spike")`.
- Back: `Overlay::back()` returns `true` when the current step is `Actions` (close the overlay) and `false` after stepping a sub-menu back to `Actions`. In `App`, Back checks the overlay before search/folder.
- Rendering: base launcher, then a full-screen scrim (`mouse_area(...).on_press(Overlay(Close))`), then a centered card wrapped in `mouse_area(...).on_press(Overlay(Noop))` so taps on card padding do not close it. Card buttons publish `Message::Overlay(...)`; sub-menu cards include a Back row publishing `Overlay(Back)`. `overlay::view` returns one full-screen stack of scrim + centered card.
- Home while an overlay is open closes the overlay, cancels search/folder and goes to the home page; it does not open search. `HomePressedFromOverlay` also clears the overlay.

- [ ] **Step 1: Write failing overlay/app tests**

In `overlay.rs`:

```rust
#[test] fn move_targets_exclude_root_folders_and_current_group();
#[test] fn append_order_counts_items_in_the_destination();
```

In `lib.rs` tests:

```rust
#[test] fn back_closes_overlay_before_search();        // searching + overlay => Back keeps searching, clears overlay
#[test] fn empty_rename_keeps_menu_open_with_error();  // set app.overlay = App menu in Rename(""); submit empty
#[test] fn overlay_home_goes_home_and_closes();        // Home while overlay open
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib overlay`
Expected: FAIL — module missing.

- [ ] **Step 3: Implement `overlay.rs`**

Define the state/message/view, including `Overlay::back`. Reuse `app.label(...)` styles and the launcher palette: card background `Color::from_rgba(20,20,24,0.96)`, rows as `button::text` actions, `text_input` for rename/folder name with `on_submit`, and `on_input` updating the step buffer. Keep the card width at `320.0` and let long lists use a bounded `scrollable` with id `"overlay-list"`.

- [ ] **Step 4: Wire App actions and Back/Home precedence**

Add the match arms to `update_inner`; refactor `Message::Loaded` snapshot replacement into `fn apply_snapshot(&mut self, snapshot: ModelSnapshot, first: bool)` so `Mutated` reuses it (icon-revision clearing, page retention). On `Mutated(Ok)`, call `apply_snapshot(..., false)` and close the overlay.

- [ ] **Step 5: Render the overlay and run tests**

In `view::launcher`, wrap the existing container in `stack![base, overlay::view(app, overlay)]` when `app.overlay.is_some()`, with the scrim covering `Fill` and the card centered.

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib`
Expected: PASS.

- [ ] **Step 6: Format, lint and commit**

```sh
cd rust
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cd ..
git add rust/qlaunch_iced/src/overlay.rs rust/qlaunch_iced/src/lib.rs rust/qlaunch_iced/src/view.rs
git commit -m "feat(iced): add launcher and app menus"
```

---

### Task 7: Wire gestures into the launcher

**Files:**
- Modify: `rust/qlaunch_iced/src/lib.rs`
- Modify: `rust/qlaunch_iced/src/view.rs`
- Modify: `rust/qlaunch_iced/Cargo.toml` (add `time` to the `tokio` features, so the one-shot long-press timer does not rely on a transitive feature)

**Interfaces:**
- Consumes: `Gesture`, `Pager`, `Overlay`, `Message::Scrolled`.
- Produces:
  - `App::gesture: Gesture`, `App::pending_target: Option<Start>`, `App::pressed: Option<Id>`, `App::cursor: Point`, `App::apply_gesture(&mut self, outcome: Outcome) -> Task<Message>`, `App::open_app_menu(&mut self, item: Item)`, `App::open_launcher_menu(&mut self)`.
  - `Message::{ItemPressed(Item), BlankPressed, LongPressTimer(u64), Mouse(iced::mouse::Event)}`; `Message::Launch`, the old `App::gesture` tuple and `suppress_tap_until` are removed.
  - Inputs from the view: each cell is `mouse_area(container(...)).on_press(ItemPressed(item.clone()))`; the pager area and the folder grid are wrapped in `mouse_area(...).on_press(BlankPressed)`, while search results are not. The cell container draws its pressed background when `app.pressed == Some(item.id.clone())`.

**Pinned behavior:**
- `FingerPressed`: `let seq = gesture.pressed(...)`; clear `pressed`; apply `pending_target.take()` through `gesture.identify`; schedule `LongPressTimer(seq)` with `Task::perform(tokio::time::sleep(gesture::LONG_PRESS), ...)` for the returned sequence (a duplicate timer for a rejected second finger is harmless because `tick` checks the sequence).
- `ItemPressed`/`BlankPressed`: set `pressed` for items; store the target in `pending_target`; call `gesture.identify` and apply any returned outcome (handles late long press). Clear `pending_target` on lift/lost.
- `FingerMoved`: `gesture.moved(..., at_top)`, where `at_top = scroll.get(current_group).copied().unwrap_or(0.0) <= 0.5`; apply the outcome. After every move, clear `pressed` when `!gesture.is_tap_candidate()` so a scroll or drag does not leave a cell highlighted.
- `FingerLifted`/`FingerLost`: clear `pressed` and `pending_target`; apply the outcome.
- `apply_gesture`: `DragStart` → `pager.begin_drag()`; `Drag` → `pager.drag(gesture.drag_dx(), content_width)`; `DragEnd` → `pager.end_drag(velocity, content_width, now, transition)`; `Cancelled` → `pager.cancel_drag(now, transition)`; `PullDown` → `work(open_notifications, ActionDone)`; `LongPress(Item(id))` → look up the item and `self.open_app_menu(item)`; `LongPress(Blank)` → `self.open_launcher_menu()`; `Tap(Item(id))` → `activate`; `Tap(Blank)`/`None` → nothing; `DoubleTap` → `work(lock_screen, ActionDone)`. Always clear `pressed` when a menu opens.
- `activate(item)`: folder tile sets `folder = Some(group)`; app runs `launch_app`; shortcut runs `launch_shortcut(item.id)`; errors go to `self.error`. Keep this as the only launch path.
- Content width for pager math is `grid::content_width(size, insets)`.
- If `overlay.is_some()`, the raw touch handler returns immediately (after clearing pressed) and no gesture target is recorded.
- `LongPressTimer(seq)` calls `gesture.tick(seq, Instant::now())` and applies the outcome.
- Desktop preview: subscribe to `iced::mouse::Event` as well. `CursorMoved` stores `cursor` and, when the gesture is active, drives `gesture.moved(Finger(0), cursor, now, at_top)`; `ButtonPressed(Left)` drives `gesture.pressed(Finger(0), cursor, now)`; `ButtonReleased(Left)` drives `gesture.released(...)`. The `ItemPressed`/`BlankPressed` messages from `mouse_area` identify the target on desktop exactly as they do for touch.

- [ ] **Step 1: Write failing app tests**

```rust
#[test] fn overlay_blocks_touch_and_paging();        // overlay open + press/move/release leaves pager committed and overlay open
#[test] fn losing_a_finger_mid_drag_settles_back();  // FingerPressed, ItemPressed, FingerMoved > slop, FingerLost, Frame(+1s) => same page
#[test] fn tapping_a_folder_opens_it();              // apply_gesture(Tap(Item(folder))) sets folder
#[test] fn tapping_an_app_does_not_open_a_folder_or_menu(); // apply_gesture(Tap(Item(app))) leaves folder/overlay empty
#[test] fn a_press_on_chrome_never_locks_or_pages();
#[test] fn mouse_events_use_the_same_classifier();   // CursorMoved, ButtonPressed, ItemPressed, ButtonReleased => folder opens
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib overlay_blocks_touch_and_paging`
Expected: FAIL — gesture wiring absent.

- [ ] **Step 3: Implement the wiring and cell targeting**

Replace the old `(Finger, Point, Point)` handler. In `view.rs`, remove the cell `button(...).on_press(Launch(...))`; use `mouse_area(container(body)...)` with the pressed style and `ItemPressed`. Wrap the pager content in the blank `mouse_area`. Keep the search/folder layouts working; folder cells also use `ItemPressed`. Add mouse-event handling and the tokio `time` feature.

- [ ] **Step 4: Run tests, format, lint and commit**

Run: `cd rust && cargo test -p qlaunch_iced --no-default-features --lib`
Expected: PASS.

```sh
cd rust
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cd ..
git add rust/qlaunch_iced/src/lib.rs rust/qlaunch_iced/src/view.rs
git commit -m "feat(iced): wire long press, double tap, pull-down and paging gestures"
```

---

### Task 8: Frame-work metrics and profiling scripts

**Files:**
- Create: `rust/qlaunch_iced/src/metrics.rs`
- Create: `scripts/latency.py`
- Create: `scripts/test_latency.py`
- Create: `scripts/profile-iced.sh` (executable)
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod metrics;` and the wrappers)

**Interfaces:**
- Produces:

```rust
// metrics.rs
pub struct Samples { /* active, update: Vec<f32>, view: Vec<f32> */ }
pub struct Summary {
    pub frames: usize,
    pub update_p50: f32, pub update_p95: f32, pub update_max: f32,
    pub view_p50: f32, pub view_p95: f32, pub view_max: f32,
}
impl Samples {
    pub fn start(&mut self);
    pub fn is_active(&self) -> bool;
    pub fn record_update(&mut self, ms: f32);
    pub fn record_view(&mut self, ms: f32);
    pub fn finish(&mut self) -> Option<Summary>;
}
pub fn percentile(sorted: &[f32], q: f32) -> f32;
impl Summary { pub fn log_line(&self) -> String; } // starts with "frame-work"
```

  App: `metrics: std::sync::Mutex<Samples>`; `fn update` becomes a wrapper around `update_inner` that records elapsed update time; `App::view` records view time. After each update, if `pager.is_animating() || pager.is_dragging()` call `metrics.start()`, else if active call `finish()` and `log::info!("{}", summary.log_line())`.

- [ ] **Step 1: Write failing metrics and parser tests**

`metrics.rs`:

```rust
#[test] fn percentile_picks_nearest_rank();   // percentile(&[1.,2.,3.,4.], 0.5) == 2.0
#[test] fn empty_percentile_is_zero();
#[test] fn records_only_while_active();
#[test] fn finish_returns_a_summary_and_deactivates();
#[test] fn log_line_prefixes_frame_work();
```

`scripts/test_latency.py` (stdlib `unittest`, imports `latency`):

```python
def test_percentile_nearest_rank(self): ...          # percentile([10,20,30], .95) == 30
def test_parse_ignores_zero_and_sentinel(self): ...  # 0 and 2**63-1 rows skipped
def test_parse_computes_intervals_and_budget(self): ... # synthetic 90 Hz fixture: frames=3, one over 1.5 periods
```

- [ ] **Step 2: Run them to verify they fail**

```sh
cd rust && cargo test -p qlaunch_iced --no-default-features --lib metrics
python3 -m unittest discover -s scripts -p 'test_*.py'
```

Expected: FAIL — metrics module and `latency.py` missing.

- [ ] **Step 3: Implement metrics and latency parser**

`Samples::finish` returns `None` unless active, clears the vectors and deactivates. Percentile uses nearest rank on a sorted copy; guard empty. `latency.py` reads SurfaceFlinger `--latency` text: skip the first refresh-period line and malformed rows, take the **second** integer (actual present time), ignore `0` and `2**63-1`, sort timestamps, diff in ms, discard `<= 0` or `> 1000`, and report frames, p50/p95/p99/max, and the share of intervals greater than `1.5 * 1000 / refresh_hz`. `main()` takes `--refresh` and one or more files, prints one line per file plus a `median` line computed with `statistics.median`.

- [ ] **Step 4: Add the on-device script**

`scripts/profile-iced.sh`:
- `set -euo pipefail`; vars `SERIAL` (optional), `LAYER` (optional), `REFRESH=90`, `RUNS=5`.
- `adb ${SERIAL:+-s "$SERIAL"} get-state`; find the layer with `adb shell dumpsys SurfaceFlinger --list | tr -d '\r' | grep -i 'bitshift.qlaunch' | grep -vi 'Background' | head -1` unless `LAYER` is set.
- Wake/launch with `input keyevent KEYCODE_WAKEUP`, `wm dismiss-keyguard`, `am start -W -n bitshift.qlaunch/.MainActivity`, then sleep 2.
- For each run: `dumpsys SurfaceFlinger --latency-clear "$LAYER"`, run 8 horizontal swipes (`input touchscreen swipe 850 1200 230 1200 350`, alternating direction), 4 vertical scrolls (`1700 → 900`), with 0.4 s pauses; dump latency to a temp file.
- Call `python3 "$ROOT/scripts/latency.py" --refresh "$REFRESH" "$TMP"/run*.txt`; then print the last 20 `frame-work` lines from `adb logcat -d -s qlaunch-iced`.
- Never install or swap APKs; clean the temp directory in a `trap`.

- [ ] **Step 5: Wire the metrics wrappers**

Rename the existing `fn update` body to `fn update_inner`; the new `fn update` times `update_inner`, calls `record_update`, then starts/finishes the window. Time `view::launcher(self)` in `App::view` and call `record_view`. Follow the exact log format from `Summary::log_line`.

- [ ] **Step 6: Run tests, format, lint and commit**

```sh
cd rust
cargo test -p qlaunch_iced --no-default-features --lib
cargo fmt -p qlaunch_iced
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cd ..
python3 -m unittest discover -s scripts -p 'test_*.py'
chmod +x scripts/profile-iced.sh
git add rust/qlaunch_iced/src/metrics.rs rust/qlaunch_iced/src/lib.rs scripts/latency.py scripts/test_latency.py scripts/profile-iced.sh
git commit -m "chore(iced): add frame-work metrics and profiling scripts"
```

---

### Task 9: Device verification, measurements and documentation

**Files:**
- Modify: `docs/iced-spike.md`

**Interfaces:**
- Consumes: the release APK, `scripts/build-iced.sh`, `scripts/profile-iced.sh`, the saved Flutter APK in `/tmp/opencode/qlaunch-iced-baseline/`.
- Produces: verified gestures and recorded measurements; no source changes unless verification finds a bug (fix in the owning module and re-run that task's tests).

- [ ] **Step 1: Build and install the release APK**

```sh
cd /home/bronson/Projects/qlaunch
ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build-iced.sh release
adb install -r build/app/outputs/apk/release/app-release.apk
adb shell am start -W -n bitshift.qlaunch/.MainActivity
```

Expected: build succeeds; launcher shows the saved layout.

- [ ] **Step 2: Verify the acceptance checklist on the Pixel 5a**

In a portrait layout with several groups, a folder, and an empty group, confirm:
- Long press an app opens Rename/Move/Uninstall (or Remove); rename and move succeed and close the menu; uninstall opens the system confirmation.
- Long press blank opens the launcher menu; Create Folder works; Settings/Edit Groups show the notice; Choose Wallpaper opens the picker.
- Double tap blank locks the screen; double tap on an icon does not; a double tap on the title strip does not.
- Pull down from the top opens the notification shade once; pulling down on a scrolled page scrolls instead.
- A swipe starting on an icon never launches it; a tap still launches.
- Swiping wraps in both directions, follows the finger, settles with the configured duration, and the title strip moves/fades with it.
- Back closes overlay → search → folder, then leaves the launcher visible.
- Rotate or resize during an animation and confirm the page lands correctly.

- [ ] **Step 3: Measure iced**

```sh
RUNS=5 bash scripts/profile-iced.sh
```

Expected: a per-run SurfaceFlinger interval table plus a median line, and one or more `frame-work` log lines per page motion. Run on a cool device after it has settled; keep the same layout/settings for every run.

- [ ] **Step 4: Measure the saved Flutter artifact manually (optional but preferred)**

```sh
adb install -r /tmp/opencode/qlaunch-iced-baseline/flutter.apk
# launch it, then run the same script with its layer:
LAYER="<flutter layer from dumpsys --list>" RUNS=5 bash scripts/profile-iced.sh
adb install -r build/app/outputs/apk/release/app-release.apk
```

Restore the iced APK afterwards. Do not let the script swap APKs. If the Flutter artifact cannot be installed without disturbing the shared configuration, skip and say so.

- [ ] **Step 5: Update `docs/iced-spike.md`**

Replace the "immediate swipe" gap with the new gesture behaviour. Add:
- A "Gestures and transitions" section listing the Flutter-matching constants and the menu actions, plus the known spike gaps (settings/group editors, custom shortcut creation beyond the system provider picker, group scroll position resets to top, no animation for multi-page jumps).
- A "Frame measurement" section documenting the two numbers separately: SurfaceFlinger presentation intervals p50/p95/p99/max and over-1.5-period share; app-side update+view p50/p95/max from `frame-work`. State explicitly that Flutter's 9–12 ms DevTools figure is frame build/raster work, not presentation cadence, so it is not directly comparable without the same collection method; put the measured numbers in a table with the device, refresh rate, run count and date.
- A short "Can Dart/Flutter be removed?" assessment: the iced APK proves the Rust core + Kotlin/JNI adapter is sufficient for the exercised flows; list what still depends on Flutter/Dart (the original UI, tests/tooling) and what production removal would still require (feature parity, accessibility, settings/editors, packaging, maintenance of the iced_android fork).

- [ ] **Step 6: Re-run the full verification and commit**

```sh
cd rust
cargo test -p qlaunch_core --no-default-features
cargo test -p qlaunch_iced --no-default-features --lib
cargo fmt -p qlaunch_iced --check
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cd ..
python3 -m unittest discover -s scripts -p 'test_*.py'
git add docs/iced-spike.md
git commit -m "docs(iced): record gesture and frame measurement results"
```
