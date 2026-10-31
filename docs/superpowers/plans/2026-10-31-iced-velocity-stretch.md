# Iced Velocity Stretch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a velocity-proportional elastic stretch to vertical scrolling and to launcher page swipes, using one shared translate/spring mechanism, with source and target pages stretched identically so the cross-fade stays smooth.

**Architecture:** A new `stretch` module owns the pure lag curve and a layout-transparent `Stretch` element that translates a wrapped subtree along one axis from the shared `Motion`. The vertical scroller merges a speed term into the same `Motion` its content already translates by; the pager drives a `Region::Pager` `Motion` and wraps the three-page slot row with `Stretch(Horizontal)`, so every slot gets one identical transform while `page_opacity` does the fade.

**Tech Stack:** Rust, pinned Iced / iced_android advanced widgets, existing headless-renderer tests.

**Spec:** `docs/superpowers/specs/2026-10-31-iced-elastic-stretch-design.md`

## Global Constraints

- No new dependencies; do not fork Iced; uniform scale only — stretch is translation plus interior spread, never glyph deformation.
- No layout extent change, no logical offset change, no selection/persistence changes.
- Do not change paging commit thresholds, pull-down search ownership, tap/drag/long-press/reorder arbitration.
- One stretch mechanism: the `Motion` values and the `stretch` lag curve are shared by both axes; no second copy of the physics.
- Cap: `min(48 dp, 10% viewport)` — reuse `elastic_scroll::displacement_limit`.
- Shape: 75% content translation / 25% row spread for vertical (existing); horizontal is rigid translation only in this plan.
- Work stays uncommitted on the existing branch unless the user asks for commits.

## Review Focus

- A tap or drag that starts while content is mid-stretch must still reach the intended target, not a stale one. Test: pressed-while-moving is ignored on both axes (vertical already covered; add horizontal page-slot gating test).
- Stretch must be exactly zero at rest: no residue after a slow drag, no idle redraw loop. Test: `stretch` returns 0 below the speed floor and the wrapper requests no frame once settled.
- A pager swipe that settles without committing (cancel drag) must still decay to zero. Test: pure driver over a cancel/settle sequence ends at 0.
- Narrow or empty viewports (0 width, 1 page) must not produce NaN or stuck stretch. Test: driver with width 0 and count 1 yields 0.
- Source and target pages must share one transform. Test: the driver output is a single scalar independent of slot index, and the row is wrapped exactly once.

---

### Task 1: Shared stretch model and element

**Files:**
- Create: `rust/qlaunch_iced/src/stretch.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (module declaration only)
- Test: `rust/qlaunch_iced/src/stretch.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: `crate::elastic_scroll::{Motion, Values, Edge}`. (Task 3 also needs `elastic_scroll::displacement_limit`, so that function becomes `pub(crate)`.)
- Produces:
  - `pub(crate) enum Axis { Vertical, Horizontal }`
  - `pub(crate) fn lag(speed: f32, limit: f32) -> f32` — signed, bounded, `0.0` at or below a small speed floor.
  - `pub(crate) fn vector(axis: Axis, total: f32) -> iced::Vector`
  - `pub(crate) fn wrap<'a>(child: Element<'a, Message>, motion: &Motion, axis: Axis) -> Element<'a, Message>`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn lag_is_signed_bounded_and_monotonic() {
    let limit = 40.0;
    assert!(lag(0.0, limit) == 0.0);
    assert!(lag(50.0, limit) == 0.0);           // below the speed floor
    let slow = lag(600.0, limit);
    let fast = lag(2400.0, limit);
    assert!(slow > 0.0 && slow < fast);
    assert!(fast < limit);
    assert_eq!(lag(-600.0, limit), -slow);
    assert!(lag(f32::NAN, limit) == 0.0);
    assert!(lag(1000.0, 0.0) == 0.0);
}

#[test]
fn vector_moves_only_along_its_axis() {
    assert_eq!(vector(Axis::Vertical, 5.0), iced::Vector::new(0.0, 5.0));
    assert_eq!(vector(Axis::Horizontal, -3.0), iced::Vector::new(-3.0, 0.0));
}
```

Add one headless test (`#[ignore]` + run with `--ignored`) proving `wrap` is layout-transparent and forwards a scroll operation, reusing the `elastic_scroll` test harness patterns:

```rust
#[test]
#[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
fn wrap_is_layout_transparent_and_forwards_operations() {
    // layout(wrap(child)) size == layout(child) size
    // a scroll_to operation on the wrapped scrollable reaches the child
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qlaunch_iced stretch -- --include-ignored`
Expected: FAIL — `lag` / `vector` / `wrap` not defined.

- [ ] **Step 3: Implement `stretch.rs`**

`lag`: `let floor = 0.04 * limit.max(1.0);` return `0.0` when `!speed.is_finite() || limit <= 0.0 || speed.abs() < floor`; otherwise `sign * limit * (1.0 - (-speed.abs() / limit).exp())` (mirror `elastic_scroll::resistance`, reusing its shape but its own floor).

`vector`: place `total` on the axis, `0.0` on the other.

`wrap`: `Element::new(Stretch { child, motion: motion.clone(), axis })`. `Stretch` implements `Widget` forwarding `size`, `size_hint`, `children`, `diff`, `layout`, `operate`, `update`, `mouse_interaction`, `overlay` unchanged, and `draw` exactly like `elastic_scroll::Content::draw` but: translation vector is `vector(self.axis, values.total)` (full total, not the 0.7 bounce share), the clip is the same `layout.bounds().intersection(viewport)` layer, and there is no press suppression (the owning scroller/pager already gates input).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qlaunch_iced stretch` then `cargo test -p qlaunch_iced stretch -- --ignored`
Expected: PASS.

- [ ] **Step 5: Commit** (only if the user asks)

---

### Task 2: Vertical velocity stretch

**Files:**
- Modify: `rust/qlaunch_iced/src/elastic_scroll.rs`
- Test: `rust/qlaunch_iced/src/elastic_scroll.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: `stretch::lag`; existing `State.velocity`, `State.translation_delta`, `State.viewport`, `Spring`.
- Produces: no new public signature; `Motion::values().total` now includes a velocity term while the scroller is moving.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
#[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
fn a_fast_drag_stretches_more_than_a_slow_one_and_settles_to_zero() {
    // same total travel, different per-move distance
    // fast drag: |motion.total| strictly greater than slow drag
    // after release + frames: total == 0.0
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p qlaunch_iced a_fast_drag_stretches -- --ignored`
Expected: FAIL — equal magnitudes or zero.

- [ ] **Step 3: Implement the velocity term**

In `Wrapper::moved`, after the existing residual/spring computation, compute a signed speed from the already-tracked `state.velocity` (dp/s, sign = finger direction) plus the scroll's own `state.translation_delta`, take `stretch::lag(speed_scaled, displacement_limit(state.viewport.height))`, and add it to the spring position before `self.motion.set(...)`, clamped to `±limit`. While a stretch term is present, set `state.released = false` and `state.last_tick = None`. On release, or when movement stops, the existing spring settles the combined value to zero.

- [ ] **Step 4: Run tests to verify they pass, plus the whole module**

Run: `cargo test -p qlaunch_iced elastic_scroll -- --ignored`
Expected: PASS, including all existing bounce/fling/ownership tests.

- [ ] **Step 5: Commit** (only if the user asks)

---

### Task 3: Horizontal pager stretch

**Files:**
- Modify: `rust/qlaunch_iced/src/lib.rs` (`Region::Pager`, driver in `update`)
- Modify: `rust/qlaunch_iced/src/view.rs` (`pages()` wraps the slot row)
- Modify: `rust/qlaunch_iced/src/pager.rs` (expose the last offset delta or velocity; no behaviour change)
- Test: `rust/qlaunch_iced/src/pager.rs` or `lib.rs` tests

**Interfaces:**
- Consumes: `stretch::{Axis, lag, wrap}`, `Region::Pager`, `app.motion`.
- Produces:
  - `Region::Pager` (`widget_id` `"page-slots"`).
  - `Pager::offset_velocity(&self) -> f32` or `Pager::last_offset(&self) -> f32` — the value the driver differences.
  - `pub(crate) fn pager_lag(offset_speed_pages_per_sec: f32, content_width: f32, limit: f32) -> f32` in `stretch.rs` (pure; independent of slot index).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn pager_lag_is_one_scalar_for_every_slot() {
    // same inputs -> same output; no index parameter exists
    let a = pager_lag(2.0, 400.0, 40.0);
    let b = pager_lag(2.0, 400.0, 40.0);
    assert_eq!(a, b);
    assert!(a > 0.0 && a < 40.0);
    assert_eq!(pager_lag(2.0, 0.0, 40.0), 0.0);   // zero width
    assert_eq!(pager_lag(f32::NAN, 400.0, 40.0), 0.0);
}

#[test]
fn a_cancelled_pager_drag_settles_the_stretch_to_zero() {
    // drive the pure lag with a drag then cancel: the smoothed speed reaches 0
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qlaunch_iced pager_lag`
Expected: FAIL — not defined.

- [ ] **Step 3: Implement**

`pager_lag` maps page-offset speed to `lag(speed * content_width, limit)`.

In `App::update`, wherever the pager is dragged or ticked, compute the offset delta for the event (store the previous offset), smooth it into a per-`Region::Pager` speed the same way the wrapper smooths finger speed, and set that `Motion` total to `pager_lag(...)`, zeroing it when the pager is settled. `settled()` already gates slot input; do not change it.

In `view::pages`, wrap the `Slots::pages(...)` element with `crate::stretch::wrap(..., &app.motion(Region::Pager), Axis::Horizontal)` before the `mouse_area`.

- [ ] **Step 4: Run tests and the whole crate**

Run: `cargo test -p qlaunch_iced` then `cargo test -p qlaunch_iced -- --ignored`
Expected: PASS; existing slot gating and pager tests unchanged.

- [ ] **Step 5: Commit** (only if the user asks)

---

### Task 4: Verification and device build

**Files:** none.

- [ ] **Step 1:** `cargo test --workspace`
- [ ] **Step 2:** `cargo test -p qlaunch_iced -- --ignored`
- [ ] **Step 3:** `cargo fmt --all -- --check`
- [ ] **Step 4:** `cargo clippy -p qlaunch_iced --all-targets --no-deps -- -D warnings -A clippy::too_many_arguments -A clippy::neg_cmp_op_on_partial_ord -A clippy::bool_comparison`
- [ ] **Step 5:** `JAVA_HOME=/opt/jbr-21 bash scripts/build-iced.sh release` and `adb install -r build/app/outputs/apk/release/app-release.apk`
- [ ] **Step 6:** One fresh review of the new effects; fix important findings with RED→GREEN tests.
- [ ] **Step 7:** Record evidence in the ledger; no commits unless asked.
