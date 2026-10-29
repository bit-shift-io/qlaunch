# Iced accessibility and page renderer — design

Date: 2026-10-29
Status: proposed for the `dev/iced` comparison spike

## Context

The iced launcher now matches the Flutter launcher's static grid, gestures,
menus and motion on the test devices, and a five-run comparison on the 90 Hz
tablet found no frame-pacing dead end: iced held a clean 90 Hz cadence while
the Flutter release build showed occasional tail jank. The remaining
high-risk areas are the two the spike deliberately left out:

1. **Accessibility.** iced has no semantics tree, and `iced_android` renders a
   native surface with no Android view hierarchy to describe. A launcher that
   cannot be driven by TalkBack is not a candidate replacement, whatever its
   frame times.
2. **Page rendering.** The current pager and title strip are translated
   `Float` overlays. The settled page is correct, but on this `iced_android`
   build the overlays can overlap and the title strip can disappear during a
   drag. That is a rendering-fork limitation, not a state bug.

Both need an app-owned answer that does not depend on an iced fork. This
document records the approach chosen in the design conversation, the contracts
the two sides must keep, and the checks that decide whether the spike
continues.

## Intent

Make the launcher navigable with a real screen reader and make page motion
render correctly, without forking iced, without rebuilding the UI as Android
views, and without moving any launcher rule out of the core.

Accessibility is the hard go/no-go. If TalkBack cannot reliably focus, read and
activate the launcher after a focused implementation pass, stop and report;
do not spend the next pass on the pager.

## Goals

1. A screen reader can traverse the title strip, the current page's grid,
   folders, search and its results, and the app and launcher menus; it can read
   labels and positions and activate the same actions a sighted user can.
2. Blank-space actions remain reachable: long-click on the grid background
   opens the launcher menu, and horizontal/vertical scroll actions page or
   scroll the grid.
3. The accessibility tree is derived from the existing app state and
   geometry. It does not duplicate launcher rules, and actions re-enter the
   existing `Message`/overlay paths.
4. Accessibility publishing costs nothing on idle frames and does not block
   the iced thread on Android's UI thread.
5. The pager and title strip draw three slots in one clipped layer using
   transformations, replacing `Float` overlays. Mid-drag screenshots and video
   show both neighbour pages, the current page and the title strip, with no
   overlap or dropout.
6. Only the settled current page receives input. A drag that starts on an icon
   still pages; a tap still launches.
7. Existing behaviour, gestures, menus and frame pacing do not regress.

## Non-goals

- Full Flutter semantics parity: collection info, custom accessibility
  actions, live regions, character-level text editing announcements, and
  focus-order tuning beyond the order described here.
- A general accessibility backend for iced or upstream `iced_android`.
- Replacing the grid with Android views, a `RecyclerView`, or a mirrored
  view tree.
- Rasterising pages to textures.
- Settings, group-editor or shortcut-editor screens. The existing notice-only
  entries remain out of scope.
- New permissions. The screen-lock accessibility service already exists for
  the pull-down/lock gestures; this work must work with TalkBack, not require
  our own service.

## Constraints

- `qlaunch_iced` keeps building with `--no-default-features`; no Flutter or
  Dart dependency.
- Core calls stay off iced's UI thread. Accessibility publishing is the one
  new Rust→Kotlin call; it must be fire-and-forget, never a blocking
  `FutureTask.get()` on the iced thread.
- Node bounds are published in **iced logical pixels** relative to the window
  origin. Kotlin maps them to physical view pixels using the host view size
  and the published window size. Do not mix dp, physical px and iced logical
  px in the payload.
- Format only `qlaunch_iced`; `cargo fmt --all` touches unrelated generated
  core files.
- Strict Clippy applies to `qlaunch_iced` with `--no-deps`.
- The existing save/layout on the test devices is preserved; do not clear app
  data to switch builds.

## Approaches considered

### Accessibility

1. **App-owned semantics plus an Android virtual view (chosen).** Rust emits a
   small tree from state and geometry; Kotlin adds a transparent
   `ExploreByTouchHelper` view over the native surface and sends actions back
   through JNI. No fork, no duplicated UI. Trade-off: a second representation
   of bounds and actions, and text-field semantics are limited.
2. **Semantics inside iced / `iced_android`.** Add nodes to the renderer or
   window backend. This would give geometry and focus for free, but requires
   maintaining a mobile accessibility backend in a community fork, with no
   upstream support. Rejected: too much fork work before the go/no-go.
3. **Mirror the UI as Android views.** Native accessibility, but duplicates
   the grid and menus and gives up iced's drawing and motion. Rejected: a
   second launcher.
4. **Rasterise pages.** Does not provide semantics, and loses text quality.
   Rejected.

### Page rendering

1. **Custom widget drawing the slots directly (chosen).** One clipped layer,
   child widgets translated with `renderer.with_translation`, no overlay path.
   Trade-off: the widget must own child layout, input gating and tree
   reconciliation.
2. **Fix `Float` overlays in the fork.** Correct in principle, but is upstream
   work in the same community dependency and does not help accessibility.
   Rejected for this pass.
3. **One texture per page.** Avoids overlay ordering but degrades text and
   adds a raster path. Rejected.

## Behaviour decisions

### Accessibility tree

The tree is a flat, ordered list of virtual nodes under one host view. Rust
publishes only the screen that is active: launcher, folder, search or overlay.
Nodes belonging to a hidden screen are not sent, so TalkBack cannot reach
behind a menu.

Payload (JSON, one object per publish):

```json
{
  "revision": 17,
  "window": { "width": 393.0, "height": 851.0 },
  "announcement": "App menu: Calculator",
  "nodes": [
    {
      "id": 12,
      "role": "button",
      "label": "Calculator",
      "bounds": [12.0, 402.0, 96.0, 110.0],
      "actions": ["click", "long_click"],
      "selected": false,
      "enabled": true,
      "value": null
    }
  ]
}
```

- `revision` increases on every published tree. Actions carry the revision
  they were performed against; a stale revision is ignored.
- `window` is the iced logical size used for the bounds.
- `announcement` is optional and only changes when the screen or step changes,
  not on every query keystroke or scroll tick. Kotlin sends it as
  `TYPE_ANNOUNCEMENT`.
- `id` is a stable virtual-view id for the lifetime of the process, kept in a
  `NodeKey -> i32` map. Keys are semantic (`item:<id>`, `title:prev`,
  `overlay:action:rename`, `overlay:move:<group>`), so focus survives a
  republish when the node still exists. Ids are never reused.
- `role` is one of `button`, `tab`, `header`, `text`, `text_field`, `scroll`.
  Kotlin maps roles to Android class names and flags; the exact mapping is a
  builder call.
- `bounds` is `[x, y, width, height]` in logical px, clipped to the window.
  Nodes with no visible intersection are omitted.
- `actions` uses symbolic names, not Android action constants. Kotlin maps
  them to standard `AccessibilityAction` ids.
- `selected` is used for the current page tab; `enabled` disables actions that
  are unavailable in the current state. `value` is the text of a
  `text_field`, otherwise null.

#### Nodes by screen

**Launcher**

- `title:prev` / `title:current` / `title:next`: one per slot, using the same
  wrapped group labels as the title strip. Side slots are `tab` nodes labelled
  `"Previous page, <label>"` and `"Next page, <label>"`, action `click`
  (`Step(-1)` / `Step(1)`). The centre slot is a `tab` labelled
  `"<label>, search apps"`, action `click` (the existing search affordance),
  selected when it is the current page.
- `grid:container`: a `scroll` node over the visible grid viewport, label
  `"<group>, page <n> of <m>, <count> apps"`. Actions:
  `scroll_forward` / `scroll_backward` when the content can move in that
  direction, `scroll_left` / `scroll_right` when there is more than one page,
  and `long_click` to open the launcher menu from blank space.
- `item:<id>`: one `button` per visible item, in row-major order, label from
  the item. Actions `click` (launch or open folder) and `long_click` (the app
  menu). Items that intersect the viewport are included; fully off-screen rows
  are omitted.
- Warnings and the transient error line are `text` nodes after the grid.

**Folder**

- `folder:back` (`button`, click = `Back`), `folder:title` (`header`).
- The same `grid:container` and `item:<id>` contract for the folder group.

**Search**

- `search:field` (`text_field`, value = the query, actions `click` to focus
  the real iced field, `set_text`), `search:close` (`button`, click closes
  search).
- `search:frequent` / `search:recent`: `tab` nodes, click switches the list.
- A `scroll` container and item nodes for the visible result rows. The search
  results' scroll offset is tracked in the same per-region map as the grid so
  the bounds and scroll actions match what is drawn.

**Overlay**

- The underlying launcher nodes are omitted.
- `overlay:title` (`header`), `overlay:error` (`text`) when present.
- One node per action button, in the order the card draws them. `click`
  dispatches the existing `overlay::Message`; `dismiss` closes one level
  through the same `Back` path.
- Rename and folder-name fields are `text_field` nodes. `click` focuses the
  real iced field (give both inputs a stable widget id); `set_text` sends the
  full replacement text through the existing overlay buffer message.
- Move-target and shortcut-provider lists are a `scroll` node plus item
  nodes. Their offset is tracked like the grid, and `on_scroll` is wired for
  those lists so accessibility and sighted scrolling share one offset.

`dismiss` maps to the existing Back behaviour: close a sub-step, then the
overlay, then search/folder. It is a standard action on the overlay/list nodes
where it makes sense.

#### Actions

Kotlin's helper maps standard actions to these strings:

| Android action | String | Rust handling |
| --- | --- | --- |
| `ACTION_CLICK` | `click` | Node-specific, always through an existing `Message` or gesture outcome |
| `ACTION_LONG_CLICK` | `long_click` | Item menu or launcher menu, through `apply_gesture(Outcome::LongPress(...))` |
| `ACTION_SCROLL_FORWARD` | `scroll_forward` | `scroll_to` the region id by one viewport, clamped |
| `ACTION_SCROLL_BACKWARD` | `scroll_backward` | `scroll_to` the region id by minus one viewport, clamped |
| `ACTION_SCROLL_LEFT` | `scroll_left` | `Message::Step(-1)` |
| `ACTION_SCROLL_RIGHT` | `scroll_right` | `Message::Step(1)` |
| `ACTION_SET_TEXT` | `set_text` (+ value) | Search query or the active overlay buffer |
| `ACTION_DISMISS` | `dismiss` | `Message::Back` |

Actions re-enter the same paths as touch:

- Item click resolves the item from the snapshot and calls the existing
  `activate`.
- Item long-click uses `apply_gesture(Outcome::LongPress(Start::Item(id)))`.
- Title clicks use `Message::Step`, `Message::Search`.
- Overlay actions use `Message::Overlay(...)`.
- Scroll uses `iced::widget::operation::scroll_to` with the region's widget id
  and a clamped absolute offset.

No menu, move, rename or launch rule is reimplemented in the accessibility
module.

Goal 6 applies to accessibility actions on page content. While the pager is
dragging or animating, Rust drops item `click`/`long_click` actions and
`set_text` for page-level fields; it does not let a late double-tap launch an
item on a departing page. Title-strip clicks and overlay actions remain
available because they are chrome, not page content, and title clicks keep the
existing touch behaviour of restarting the settle from the current offset.
Scroll actions are also dropped while the pager moves, so a screen reader
cannot scroll a page that is sliding away. The revision check additionally
drops any action whose published screen no longer matches the current state.

#### Geometry

Bounds must come from one geometry source used by both the view and the tree.
Do not copy padding and row-height numbers into `accessibility.rs`.

Use explicit shared metrics, not a measured-layout path. The tree is built in
`update`, before view layout; adding a measurement pass that publishes after
layout would make publishing asynchronous and couple it to the renderer. The
view is instead made deterministic:

- Title strip: insets plus `grid::title_height`, split into thirds.
- Folder header: the same explicit height as the title strip
  (`grid::title_height`). The current button/label row is about 45 px at
  defaults and would miss the 2 dp acceptance; give it the shared height.
- Search: the search row and the optional history row get explicit heights in
  the shared metrics. `App::body_top` keeps its gesture meaning (search is
  chrome and never a pull-down/long-press target), while a separate
  `body_top_for_layout` returns the real top used by the tree.
- Grid: `content_width`, `columns`, `grid::row_height`, the layout body top,
  the error/warning block height, and the current region's scroll offset. Cell
  width is `content_width / columns`. The scroll viewport's bottom is the
  window bottom inset minus the explicit error/warning block, because those
  lines are pushed after the `Fill` pager today.
- Overlay card: fixed width, centred; the card title, error line, every action
  row and every input get explicit heights so the card's vertical cursor and
  total height are deterministic. Long labels are clipped to the fixed title
  height rather than allowed to reflow. Keep these heights next to the view
  code that uses them.
- Overlay lists and search results use the same explicit viewport height and
  scroll-offset tracking as the grid; their row heights come from the same
  helpers as the view.
- Clip every node to the window and to its scroll viewport before publishing.

The acceptance's 2 dp applies to every interactive node, including folder
headers, search chrome and overlay rows. If a widget cannot be made
deterministic without a visual regression, say so and relax that node's
tolerance in the report rather than adding a hidden measurement path.

Rust publishes logical coordinates and the logical window size. Kotlin scales
by `view.width / window.width` and `view.height / window.height` and rounds to
integers. This avoids guessing density or system-font scale on the Kotlin
side.

#### Publishing

- Publishing is driven by app state, not by a permanent frame subscription.
- Do not publish while the pager is dragging or animating. Mark the tree dirty
  and publish when motion settles. Idle launcher frames remain subscription-free.
- **Settled scroll** means the fling has stopped, not that the finger lifted.
  On this `iced_android` build the scrollable continues a fling on each
  `RedrawRequested` and calls `on_scroll` until its velocity falls below 20.
  Each `Scrolled` message bumps a scroll generation and schedules a one-shot
  quiescence timer (approximately 150 ms). Only the latest generation publishes
  when it fires; a new touch cancels the pending publish by bumping the
  generation. This uses a task, not a frame subscription, so idle frames stay
  free. Publishing at finger-up would leave focus rectangles on last frame's
  content.
- Build and serialize only when the semantic state may have changed. Compare
  the new payload with the last published one and skip the JNI call when they
  are identical.
- Accessibility-driven `scroll_to` actions publish through the same
  quiescence path, so one scroll does not produce one JNI call per frame.
- Rust calls `QLaunchBridge.publishAccessibility(json)` through the existing
  `JniHost::call_void` static-call bridge. Kotlin posts to the UI thread and
  coalesces pending payloads, keeping only the latest. A failed call is logged
  at debug level and never shown as a launcher error; the host preview and
  non-Android builds are no-ops.
- When the app returns to the foreground, or the window is recreated, mark the
  tree dirty and publish after the first suitable update.

### Kotlin virtual view

Add `LauncherAccessibilityView`, a transparent full-screen `View` added with
`addContentView` in `MainActivity.onCreate` after `super.onCreate`, alongside
the existing `ImeView` pattern. It must not consume ordinary touches: it does
not override `onTouchEvent` to return true and has no click listener.

- Its `ExploreByTouchHelper` subclass holds the latest parsed tree on the UI
  thread.
- `getVirtualViewAt(x, y)` returns the node under the point, preferring item
  nodes over a scroll container when both contain it.
- `getVisibleVirtualViews` returns ids in published order, skipping empty
  bounds.
- `onPopulateNodeForVirtualView` sets label/text, class name, bounds, flags
  (clickable, long-clickable, scrollable, editable, selected, enabled),
  `ACTION_SET_TEXT` for editable nodes, and the standard actions from the
  node's action list.
- `onPerformActionForVirtualView` maps the action to its string and calls
  `MainActivity.onAccessibilityAction(nodeId, action, value, revision)`,
  returning true only when the node advertises that action.
- `dispatchHoverEvent` delegates to the helper, then to the view.
- On publish, diff against the previous tree:
  - **Active screen or modal change** (launcher, folder, search, overlay kind
    or overlay step): `invalidateRoot()` and announce; resetting focus to the
    new screen is intended.
  - **Bounds, visible-set or ordering change with the focused node still
    present** (scroll, page step, new results): keep the focused virtual id,
    invalidate the nodes whose bounds or labels changed, and do not call
    `invalidateRoot()`. TalkBack re-queries bounds and `getVisibleVirtualViews`
    as it moves. On a page step the activated title tab survives, so focus
    stays on it; the grid items are new nodes and need no invalidation.
  - **Label/value-only change**: `invalidateVirtualView(id,
    CONTENT_CHANGE_TYPE_TEXT)` so typing does not reset focus.
  - **Focused node disappeared** (for example the last result in a query):
    move focus to the nearest surviving node in the same screen, or to the
    host, and announce the new context. Do not leave TalkBack focused on a
    virtual id that no longer exists.
  Send `TYPE_ANNOUNCEMENT` when the payload carries one. Do not request input
  focus; the iced IME view keeps that.
- The helper's constructor makes the host focusable. Do not call
  `requestFocus()` from the accessibility view. Verify that the soft keyboard
  still opens for search and rename; if the full-screen host becomes the
  default focus and swallows key events, clear focusability after constructing
  the helper. Touch exploration and virtual-node actions do not require input
  focus, and virtual keyboard focus is out of scope.

The view is added to Gradle as `androidx.customview:customview:1.1.0`
(`ExploreByTouchHelper`). The artifact is already in the local Gradle cache.
No other AndroidX dependency is introduced.

### Kotlin/Rust direction

- **Rust → Kotlin:** `QLaunchBridge.publishAccessibility(json: String)` as
  above. Fire-and-forget; no blocking wait.
- **Kotlin → Rust:** `MainActivity.onAccessibilityAction(nodeId: Int,
  action: String, value: String, revision: Long)` (JNI signature
  `(ILjava/lang/String;Ljava/lang/String;J)V`). The export lives in
  `qlaunch_iced::accessibility`, is re-exported under `cfg(target_os =
  "android")` so the cdylib keeps it, and pushes a request into a
  process-global channel. An always-on subscription forwards requests to
  `Message::Accessibility(ActionRequest)`.
- Stale revisions, unknown node ids and unknown actions are ignored. Actions
  are handled on iced's UI thread through the normal update path.

### Page renderer

Replace the translated `Float` pages and title strip with one custom `Slots`
widget configured two ways:

- **Pages:** three children, each laid out at the full content size and
  translated by `(index - 1 - offset) * content_width`. Only slot 1 (the
  committed page) receives events, and only when motion is settled. The
  neighbours are static previews.
- **Title strip:** three children laid out at one third of the strip width,
  arranged left to right and translated by `-offset * strip_width / 3`. All
  three receive events when settled; none do while motion is in progress.

Both modes draw inside one `renderer.with_layer(bounds, ...)` and translate
each child with `renderer.with_translation(Vector::new(dx, 0.0), ...)`. The
widget's own `overlay` returns `None`; there is no overlay path and no
per-page layer. The title strip remains a separate widget above the pager, so
the pager's clip cannot cover it.

- `size` is `Fill × Fill` for pages and `Fill × title_height` for the strip.
  Children are laid out at the resolved bounds; the implementation may share
  a layout helper.
- `draw` passes the slot's translated cursor to the child; only the centre
  page is drawn with the current page's cursor. The layer clip is the pager
  or strip bounds.
- `update` gates **input** events only (touch, mouse, keyboard, IME) when the
  pager is not settled. It must always forward `Event::Window`, especially
  `RedrawRequested`, to the centre child: the scrollable advances its fling on
  redraw, and freezing redraws would stop a fling mid-flight and then jump it
  with a large elapsed-time delta when motion ends. `mouse_interaction` and
  `operate` forward only when settled; `operate` forwards the centre page so
  the existing `scroll_to("apps")` and `reset_scroll` operations still reach
  the real scrollable. An in-flight fling is allowed to continue through a
  page transition; it settles on the same page content.
- `children` returns the three child trees. `diff` reconciles by **slot
  identity**, not by slot index: store the three group/region keys in the
  widget state and drop a child tree when its key changes. This stops a
  scrollable's offset from leaking into a different group and replaces the
  current index-based reuse. The app's existing reset-on-commit stays as a
  backstop.
- The outer `mouse_area(BlankPressed)` around the pager stays, because
  `scrollable` still captures presses and the classifier infers blank presses
  positionally. The custom widget must not capture raw touches itself.
- Animation ticks remain `window::frames()` only while `pager.is_animating()`.

The existing `pager.rs` state machine (committed page, offset, drag, settle) is
unchanged. This is a rendering and input-routing change, not a paging-rule
change.

## Acceptance

### Accessibility go/no-go

On the tablet, with TalkBack enabled:

- TalkBack announces the page and the first focusable item; swiping forward
  and backward reaches the title slots, the grid container, every visible app,
  and the error/warning text, in the published order.
- Double-tap on an app launches it; double-tap on a folder opens it;
  double-tap on a title side slot steps a page; double-tap on the centre slot
  opens search.
- Long-click on an app opens the app menu; long-click on blank grid space
  opens the launcher menu. Menu actions work, including Rename, Move, Uninstall
  and Cancel/Back.
- Search results are reachable; the real keyboard opens; typing updates the
  field and results without losing accessibility focus on every keystroke.
- Vertical scroll and page actions move the visible content; after a fling
  settles, the focus rectangles are on the rendered cells (the quiescence
  publish, not the finger-up position).
- Accessibility focus survives a scroll and a page step when the focused node
  still exists; it is not reset to the root on every content change.
- An item action sent while the pager is dragging or settling does not launch
  or open a menu on a departing page.
- No node is exposed for a hidden page, a closed folder, or a closed menu.
- `uiautomator dump` contains the same labels and plausible bounds; a sampled
  app cell, folder header, search row and overlay action are each within 2 dp
  of what is drawn.
- TalkBack off: ordinary touch, long press, double tap, pull-down, keyboard and
  IME behaviour are unchanged; the transparent view consumes nothing.

If any of the first five fail after a focused implementation pass and cannot be
explained by a fixable bounds/action bug, stop and report; do not proceed to
the pager.

### Page renderer

- A slow horizontal drag captured with screenshots and `screenrecord` shows
  both neighbours, the current page and the title strip without overlap,
  dropout or flicker, at several offsets and in both directions.
- Wrapping, multi-page drags, fling bias, settle duration and `easeOutCubic`
  are unchanged.
- A drag starting on an icon pages and does not launch; a tap still launches.
- Only the settled current page scrolls or launches; taps during a settle
  cannot hit a neighbour.
- A vertical fling that is still decaying when a page step starts keeps
  receiving redraws, finishes without a jump, and does not freeze mid-flight.
- Mid-drag frame pacing is no worse than the existing tablet baseline; idle
  still has no frame subscription.

### Build and regression

- `cargo test -p qlaunch_core --no-default-features`,
  `cargo test -p qlaunch_iced --no-default-features --lib`,
  `cargo fmt -p qlaunch_iced --check`, and
  `cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps
  -- -D warnings` pass.
- The release APK builds and installs on both the phone and the tablet without
  clearing data, and the saved layouts still render identically.
- Existing gesture, menu, search and lifecycle checks still pass.
- `docs/iced-spike.md` records the tablet frame results, the accessibility
  findings (including text-field limits), the pager result, and any remaining
  gaps.

## Risks and mitigations

- **Bounds drift from rendering.** Geometry is shared with the view, overlay
  rows get explicit heights, and on-device focus rectangles are checked
  against screenshots. A small drift is a usability bug, not a data-model bug.
- **Virtual view blocks touches or steals the keyboard.** It is not clickable
  and never requests input focus; test search/rename keyboard paths and normal
  touches with TalkBack off.
- **TalkBack caches stale nodes.** Publish revisions, invalidate the root on
  structural change, invalidate individual nodes on value change, and ignore
  stale actions.
- **JNI publishing stalls the renderer.** Fire-and-forget posting plus latest-
  payload coalescing; never use the blocking `onMain` path for publishing.
- **Text-field semantics are limited.** Focus, reading the value and setting
  the whole value are in scope; character-level editing announcements and
  selection actions are not. A screen-reader user can still type through the
  real keyboard.
- **`ExploreByTouchHelper` and OEM screen readers differ.** Test on the tablet
  first; record the TalkBack version. If the tablet's TalkBack is missing,
  install Android Accessibility Suite before judging the result.
- **Scroll offsets lag.** Accessibility scroll goes through the same
  `scroll_to` and `on_scroll` path as sighted scrolling; bounds are refreshed
  after motion, not on every frame.
- **The custom widget hits another fork limitation.** If one-layer clipping
  and translation still misbehave, capture the failure and treat it as a
  rendering-fork finding rather than papering over it with overlays again.
- **Accessibility tree build cost.** Only semantic changes publish; motion is
  suppressed; the tree is small and the cost is measured in the same app-side
  `update` numbers as the rest of the UI.

## Left to the builder

- Exact module split beyond `accessibility.rs`, `slots.rs`, `view.rs`,
  `grid.rs` and the Kotlin view file; whether the tree builder is a free
  function or an `App` method.
- JSON field names beyond the contract above, and whether to add a debug
  logcat dump of the published tree.
- Android class-name and flag mapping for roles; exact announcement wording.
- The visibility cutoff for partially clipped rows and whether to include one
  extra row as a focus target.
- The scroll increment (a sensible default is 0.8 of the viewport) and the
  exact quiescence timeout (about 150 ms), to be tuned on device.
- Whether `set_text` is enabled for every text field in the first pass or only
  search and rename.
- Widget names and the exact `Slots` configuration API; the shared layout
  helper's location.
- Test decomposition, including the headless smoke test for slot math and
  tree reconciliation.
