# Iced Shared Styling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Match Flutter's field and dropdown appearance and add reusable touch edge-stretch feedback to Iced vertical scrolling.

**Architecture:** Keep styling in the shared theme/components and introduce focused advanced-widget modules for underline decoration and scroll feedback. Retain Iced's existing scrollable as the source of logical offsets, inertia, notifications, and programmatic operations. Validate the edge-effect rendering and event-forwarding feasibility before migrating consumers.

**Tech Stack:** Rust, existing pinned Iced/iced_android, advanced widgets, existing headless-renderer tests.

**Spec:** `docs/superpowers/specs/2026-10-31-iced-fields-design.md`

## Execution status

- [x] Task 1: shared underlined fields and search styling.
- [x] Task 2: dropdown spacing, folder indentation, measured ellipsis, and accessibility geometry.
- [x] Task 3: translation-only elastic scroll model and decorators in `elastic_scroll.rs`. Ruling: the pinned renderer cannot vertically stretch glyphs, so the approved replacement uses a 75% content translation plus 25% row spread, bounded at `min(48 dp, 10% viewport)`, with a critically damped spring (omega 15/s), 32 ms capped tick, 18 dp axis lock, and exact rest at 0.05 dp / 0.5 dp/s. A first feedback round found the effect barely visible; fixes (each proven with a failing test first): the axis-locking move now feeds the bounce, smoothed finger velocity keeps momentum on release, and a fingerless fling reaching an end is detected and sprung back.
- [x] Task 4: every migrated vertical scroller now uses the shared wrapper and decorators; launcher grids reserve the top edge for pull-down search (`Policy::LauncherTop`), folder/search/settings/dialog/menu/overlay use `Policy::Normal`. Gesture ownership is tracked per drag so a reorder grip or other move-capturing child keeps its gesture instead of tripping the bounce.

Work remains uncommitted on the existing branch, as requested. Full Rust workspace tests pass; all 29 headless Iced tests pass explicitly. Android visual/touch verification and the requested release install remain outstanding.

Final verification: 282 regular Iced tests and all 29 headless tests pass (`cargo test -p qlaunch_iced`, then `-- --ignored`). `cargo fmt --all -- --check` clean; `cargo clippy -p qlaunch_iced --all-targets --no-deps` with the three recorded baseline lint categories allowed passes with no other warnings. Debug APK built with `JAVA_HOME=/opt/jbr-21 bash scripts/build-iced.sh debug` at `build/app/outputs/apk/debug/app-debug.apk` (the default system Java lacks compiler capability). Review's dropdown touch-target regression was reproduced and fixed; the full 56 dp field remains clickable. Task 3/4 findings fixed with failing-then-passing tests: the outward residual sign (a bounce could fire while the list was actually scrolling, found by a new inward-from-bottom test), stale displacement after a programmatic scroll, the discarded single-move flick, missing fling momentum, and `Motion::clear` clobbering gesture ownership on the first press (caught by the four groups reorder tests). Deferred review polish: selected-option hover/press layers and additional full-menu narrow-layout/accessibility integration tests.

## Global Constraints

- No changes to selection, editing, persistence, destination filtering, or the Flutter implementation.
- Use the existing Iced stack and Material theme tokens; avoid dependencies and screen-specific copies.
- Dropdowns use 48 dp rows, 8 dp vertical padding, 20 dp folder indentation, 18 dp folder icons, and 8 dp icon gaps.
- Search keeps an underline; horizontal paging stays unchanged.
- Stretch is bounded and clipped, preserves clamped logical offsets, and must not interfere with pull-down search, horizontal paging, taps, or accessibility.
- Mouse-wheel scrolling and programmatic scrolling retain existing behavior; the edge stretch targets touch interaction.

## Review Focus

- Dragging from a dropdown option must scroll without selecting it; a stationary tap must still select it (Task 3).
- Top-edge pull-down search and horizontal page swipes must retain ownership instead of producing competing scroll animations (Task 4).
- Empty, short, and resized viewports must not produce invalid transforms or stuck animation (Task 3).
- Long destination labels and folder indentation must fit narrow menus without expanding their bounds (Task 2).
- Focus and programmatic scroll operations must cross wrappers unchanged; menu accessibility must include padding but exclude decoration (Tasks 1–4).

---

## File structure

### Approved amendment: spacing plus bounce (supersedes Tasks 3–4 stretch details)

Continue native execution in this branch without git writes, as requested. The user approved the revised spec and explicitly requested implementation. Keep completed Tasks 1–2 intact.

**Task 3 interfaces:** create `elastic_scroll.rs` with `Motion` (cloneable shared visual state), `Policy` (normal, launcher-top-search, disabled), `wrap(scroller: Element<'a, Message>, motion: Motion, policy: Policy) -> Element<'a, Message>`, `content(child: Element<'a, Message>, motion: &Motion) -> Element<'a, Message>`, and `row(child: Element<'a, Message>, motion: &Motion) -> Element<'a, Message>`. The outer wrapper owns touch and spring state in its Iced tree and observes the direct child scrollable through operations. Content translates for bounce, row decorators translate to increase spacing without altering layout extent. Translation-only drawing keeps glyphs and controls unchanged; scrollbars remain outside the content decoration.

- [x] Task 3 RED: deterministic model tests for top/bottom outward residual movement, inward unwinding, axis lock, ignored second finger, cancellation, finite bounded resistance (32 dp or 6% of viewport, whichever is smaller), and exact settling. Layout/extent/resize and input tests use real Iced widgets on the headless adapter. Expected: new assertions fail against inert model/wrappers.
- [x] Task 3 GREEN: implement the model and decorators. Use an approximately critically damped spring (omega 18/s), a maximum 32 ms tick, 18 dp axis-lock slop, and zero rest threshold at 0.05 dp/0.5 dp/s. Visual bounce is 70% of displacement; row spread is the remaining 30% distributed according to row position in the viewport, expanding away from the touched edge. Clamp logical offsets and never invalidate layout for animation. Clear stale motion on resize/content extent change and programmatic scroll. Preserve overlays, focus and scroll operations. Suppress presses on moving content while allowing the underlying scroller to receive drag events; do not change existing tap/drag arbitration when at rest.
- [x] Task 3 verify: `cargo test -p qlaunch_iced elastic_scroll` and explicit headless wrapper tests; all pass, including stationary tap versus scroll drag, wheel/programmatic behavior, interruption and no idle redraw loop.

**Task 4 integration:** add optional motion to `view::grid_rows`; decorate actual grid rows, not virtualization spacers. Pass motion through dialog body row construction and wrap option/overlay rows. Let screen builders supply motion to the shared scaffold, decorating card/list rows while preserving reorder-grip event routing. Use the same wrapper and physics for every vertical scroller, and disable top-edge feedback in launcher grids so pull-down search keeps priority; folder and search grids use normal policy. Disabled preview pages must not retain motion.

- [x] Task 4 RED: assert a downward touch at the bottom of a long menu produces no bounce, an upward outward drag does, logical offset remains clamped, and every migrated viewport has one scroller. Preserve existing page/pull-down/long-press/reorder regression tests.
- [x] Task 4 GREEN: migrate `components.rs`, `dialog.rs`, `screens/{mod,settings,groups,group_apps,advanced}.rs`, `view.rs`, `folder.rs`, and `overlay.rs` using only shared motion/decorators. Existing geometry/accessibility stays unchanged because offsets are visual-only.
- [x] Task 4 verify: full Rust workspace tests, all ignored headless tests, format check, Clippy (report existing baseline lint failures separately), Android debug build using `/opt/jbr-21`. Request one fresh final review of the new effects and fix important findings with RED→GREEN tests. No installation on the user's device or git writes.

The original stretch feasibility gate below is resolved by this approved translation-only design; no renderer fork, scaling, or glyph deformation is involved.

- Create `rust/qlaunch_iced/src/field.rs`: shared underline decorator forwarding advanced widget lifecycle and operations; focus-aware input underline.
- Modify `rust/qlaunch_iced/src/theme.rs`: field styles and centralized menu/animation geometry tokens.
- Modify `rust/qlaunch_iced/src/components.rs`: shared field builders, dropdown rows and menu geometry.
- Create `rust/qlaunch_iced/src/elastic_scroll.rs`: pure spring/edge model and wrapper for existing vertical scrollables.
- Modify `rust/qlaunch_iced/src/lib.rs`: module declarations only, except regression tests if needed.
- Modify `rust/qlaunch_iced/src/view.rs`, `folder.rs`, `dialog.rs`, `overlay.rs`, `screens/mod.rs`: use shared styled fields/scrollers without changing business logic.
- Modify `rust/qlaunch_iced/src/accessibility.rs`: padded menu option and scroll bounds.

### Task 1: Underline-only shared fields

**Interfaces:** Preserve `components::text_field` and `components::select_field` signatures. Add `field::underline<'a>(content: Element<'a, Message>) -> Element<'a, Message>` and a focus-aware input decorator in the same module; both forward child operations, overlays, updates, and layout. Keep input caret/focus in the existing text input, not application state.

- [ ] Add failing style tests: `input_has_no_enclosing_border` asserts active/focused input borders have width zero; underline resolution asserts 1 dp outline idle and 2 dp primary focused. Add a headless layout test asserting both labelled fields remain 56 dp tall, value starts below the label, and dropdown has no enclosing border.
- [ ] Run `cargo test -p qlaunch_iced input_` from `rust/`; confirm new assertions fail for the current boxed style.
- [ ] Implement the shared underline drawing and lifecycle forwarding in `field.rs`, using the child text-input state for focus. Set text input padding to zero horizontally; keep a separate small label above the value. Draw the dropdown underline under its value/arrow area and use a borderless button.
- [ ] Update search in `view.rs` and the legacy overlay input in `overlay.rs` to use shared underline decoration and styles, preserving widget IDs, callbacks, and current search geometry. Search remains unlabelled.
- [ ] Run field/style tests and add a focus-operation regression proving `text_input::focus` still reaches wrapped inputs. Run headless layout tests explicitly when an adapter is available.
- [ ] Record verified changes; commit only if the user requests commits.

### Task 2: Flutter-style dropdown menu layout

**Interfaces:** Preserve `components::select_menu` and `select_menu_bounds`. Add `components::select_content_height(option_count: usize) -> f32` and `components::select_option_bounds(menu: iced::Rectangle, index: usize, offset: f32) -> iced::Rectangle` so rendering and accessibility use the same geometry.

- [ ] Add failing tests: one option has natural height 64 dp; five have 256 dp; large menus cap at 280 dp; first option begins 8 dp below menu top and is 48 dp high; scrolling subtracts logical offset from option y. Cover windows narrower than the anchor and shorter than the menu, including zero dimensions.
- [ ] Run `cargo test -p qlaunch_iced select_` from `rust/`; confirm padding tests fail before implementation.
- [ ] Implement shared menu geometry and 8 dp top/bottom content padding, retaining 48 dp rows and 16 dp horizontal row padding. Add 20 dp indentation for folder options before the 18 dp icon and 8 dp gap. Constrain long text with single-line ellipsis instead of allowing it to expand the menu. Preserve selected-state contrast and add existing Material hover/press feedback.
- [ ] Update `accessibility::select_frame` to consume content height and option bounds helpers, retaining viewport clipping and logical scroll positions. Remove redundant menu surface decoration in `dialog::select_view` if it would duplicate shadows.
- [ ] Run select, dialog, and accessibility tests. Add a headless narrow-menu test with a long folder label asserting width stays within its viewport and icon/indent spacing matches the spec.
- [ ] Record verified changes; commit only if requested.

### Task 3: Shared edge-stretch scroller

**Interfaces:** Add `elastic_scroll::wrap<'a>(scrollable: Element<'a, Message>) -> Element<'a, Message>` around an already-configured Iced scrollable. Store animation/touch state in the widget tree, not `App`; retain existing scroll IDs and callbacks. Pure internal `Spring` exposes pull, release, tick, and reset methods for deterministic tests.

- [ ] Establish feasibility with focused failing tests for edge detection using the child scrollable operation's bounds/content bounds/translation. Assert outward motion at either clamped edge activates stretch, inward motion does not, and a touch starting outside the viewport has no effect. Inspect only the direct child scroller rather than accidentally selecting nested scrollers.
- [ ] Prove a clipped content deformation can be rendered with this pinned Iced version while keeping surrounding chrome and scrollbar undeformed. Iced exposes only uniform scale through its public transformation constructors; do not silently substitute whole-list bounce or unsupported unsafe matrix construction. If a faithful, maintainable stretch cannot be implemented with the approved stack, stop and present the concrete limitation before migrating callers.
- [ ] Add failing pure spring tests: resistance increases with distance; amplitude is capped at 8% of viewport height and at 48 dp; release settles exactly to zero; interruption is continuous; zero-height and nonfinite inputs stay finite; finger cancellation, resize, and content shrink cannot leave a stuck stretch. Use a bounded damped spring with capped frame timestep and explicit rest threshold.
- [ ] Run `cargo test -p qlaunch_iced elastic_scroll` from `rust/`; establish RED before implementing each behavior.
- [ ] Implement wrapper lifecycle forwarding and touch tracking. Forward the original event once to Iced, inspect logical scrolling, and derive edge feedback only from outward unconsumed movement after vertical touch slop. Tick animation on redraw and request frames only while settling. Keep wheel/programmatic operations unchanged. Preserve child overlays and clipping; map pointer coordinates consistently with any active deformation or suppress incidental activation during a scroll drag.
- [ ] Add event-sequence tests: option press/move/release never selects; tap still selects; second finger does not steal the first; lost finger settles; wheel emits the same logical offset without activating stretch; programmatic scroll and focus operations cross the wrapper; idle redraw requests stop. Cover fling-to-edge feedback if the existing scrollable exposes enough state without replacing its inertia; document any limitation rather than inventing private-state coupling.
- [ ] Run spring and wrapper tests, plus a headless rendering/event check with a partially scrolled list. Confirm clipping and no logical overscroll.
- [ ] Record verified changes; commit only if requested.

### Task 4: Reuse the scroller and verify interactions

**Interfaces:** Consume `elastic_scroll::wrap` around each existing fully configured vertical scrollable, leaving their IDs, sizing, scrollbars, notifications, and content unchanged.

- [ ] Add regression sequences using existing gesture/pager fixtures: a horizontal swipe does not start edge stretch; a top-edge pull still opens search; a vertical drag does not launch an app; long press still opens its editor. Assert unchanged logical offsets and targets.
- [ ] Run the regression tests before migration to capture baseline behavior; add wrapper-specific assertions that fail until integration is present.
- [ ] Wrap vertical scrollers in `components.rs`, `dialog.rs`, `screens/mod.rs`, `view.rs`, `folder.rs`, and `overlay.rs`. Keep existing scroll-fade wrappers in place. Ensure unrelated horizontal or reorder behavior is not changed. If the wrapper needs gesture arbitration, pass a minimal explicit enable/disable flag rather than reading global application state inside it.
- [ ] Run `cargo fmt --all -- --check`, `cargo test -p qlaunch_iced`, and `cargo clippy -p qlaunch_iced --all-targets -- -D warnings` from `rust/`. Distinguish pre-existing/environment failures from regressions.
- [ ] Run ignored headless field/menu/wrapper tests where a GPU adapter is available. Manually validate app editor, folder editor, search, long dropdown, settings, and folder/app lists on Android when available; otherwise report touch/visual verification as outstanding rather than claiming parity.
- [ ] Request final code review, address verified findings, and summarize test evidence and remaining device checks. Commit only if requested.

## Self-review

Spec coverage includes shared field appearance, search's intentional underline, dropdown indentation/padding and accessibility, shared vertical-scroller reuse, gesture preservation, clipping, interruption, and settling. The rendering feasibility gate is intentional: this pinned Iced API lacks a public nonuniform scale constructor, so an executor must prove the effect instead of disguising a bounce or adding a dependency. No dependency or Flutter changes are planned.
