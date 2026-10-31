# Iced Elastic Stretch Design

**Status:** awaiting user approval (superpowers:brainstorming gate).
**Related:** `docs/superpowers/specs/2026-10-31-iced-fields-design.md`,
`docs/superpowers/plans/2026-10-31-iced-shared-styling.md` (Tasks 3–4 shipped the
edge bounce this builds on).

## Intent

The current elastic effect only appears when a list is already clamped at an end,
and only vertically. The user wants the stretch to be present *while moving*: the
faster the scroll, the larger the stretch. It should work along whichever axis the
gesture travels, so horizontal launcher page swipes become elastic too. For the
horizontal case the source and target page must receive the **same** stretch so the
existing cross-fade reads as one smooth transition rather than two different
transforms.

## Goals

- A single reusable `Stretch` element (approach B) that wraps a subtree and applies
  draw-time displacement plus interior spread along one axis, driven by a shared
  per-region value and spring.
- Vertical driver: the existing `elastic_scroll` wrapper feeds smoothed scroll
  velocity into the stretch while content moves, on top of the existing edge bounce.
- Horizontal driver: the launcher `Pager` drag/settle feeds the same value; both the
  outgoing and incoming page get an identical horizontal stretch, with the present
  `page_opacity` cross-fade unchanged.
- Strength grows with speed and is bounded; it decays to exactly zero when motion
  stops, with no idle redraw loop.
- No layout extent change, no logical offset change, no glyph deformation, and no
  change to taps, drags, long-press, reorder, pull-down search, or paging thresholds.

## Non-goals

- Changing the Flutter implementation, selection, or persistence.
- Adding dependencies or forking Iced.
- True non-uniform scaling (unavailable in the pinned renderer).
- Reworking the pager's commit thresholds or the pull-down search ownership.

## Anti-goals (would make this a failure even if it compiles)

- A second, divergent copy of the spring/physics for horizontal.
- Per-screen bespoke effects instead of the shared element.
- Stretch that moves content while leaving hit-testing behind, so a tap lands on the
  wrong target.
- Stretch that fires on plain taps, on a settled idle screen, or during a drag owned
  by another widget (reorder grip, slider).

## Constraints

- The pinned Iced exposes only uniform scale, so "stretch" is faked by translating
  the drawn subtree along the gesture axis and spreading its interior rows/columns
  away from the leading edge. Translation-only keeps text and icons intact.
- Iced can only transform a subtree at draw time by wrapping it; hence a decorator
  element rather than a post-process.
- The pager is pure state (`Pager`: committed page, signed fractional `offset`,
  `velocity_x` on release); page slots are drawn by the `slots::Slots` widget with
  per-slot `page_opacity(i - 1 - offset)` as the cross-fade.
- The shared `elastic_scroll::Motion` (one per `Region`) and its critically damped
  spring are reused for both axes.

## Approaches considered

- **A — grow the existing wrapper only.** Add a velocity term to `Motion` for the
  vertical scroller. Cheapest, but horizontal page swipes get nothing.
- **B — a dedicated `Stretch` element (chosen).** One widget wraps any subtree and
  reads axis + strength from `Motion`. The vertical scroller content uses it as the
  edge effect does today; the launcher page stack gets its own instance driven by the
  pager drag. One visual mechanism, two drivers.
- **C — fold it into the pager only.** Simplest horizontal win, but vertical scroll
  and horizontal paging end up with two implementations of the same idea.

The user chose **B**, with stretch on **any** page swipe and **identical** treatment
of source and target pages.

## Parts

### 1. `stretch.rs` (new)

- `Axis { Vertical, Horizontal }`.
- `Stretch<'a>` wraps `child: Element<'a, Message>`, `motion: Motion`, `axis: Axis`.
  Transparent to layout, operations, overlays, focus, and mouse interaction; only
  `draw` changes.
- On draw, if `motion.values().total == 0`, draw unchanged. Otherwise clip to the
  viewport (as `elastic_scroll::Content` does) and draw the child inside a
  `with_translation` whose vector is the axis component of the displacement, and let
  interior decorators (`stretch::line`) add the spread.
- `stretch::line(child, motion, axis)` decorates a real row (vertical) or column of
  cells (horizontal) and spreads it by position within the viewport, reusing the
  existing `elastic_scroll::row_offset` math generalized to an axis and to a supplied
  reference edge.
- For the horizontal page case the reference is the viewport, not the page, and the
  magnitude is the shared strength, so every page in the row gets the same spread.

### 2. Vertical driver (`elastic_scroll.rs`)

- Reuse the smoothed finger/scroll speed already tracked. While the scroller is
  scrolling (translation changing) or engaged, set `Motion` total from a
  velocity-scaled term in addition to the edge residual, clamped to the same cap.
- On stop, spring to zero; no request-redraw when settled.
- The edge bounce behaviour and all current tests stay.

### 3. Horizontal driver (pager + view)

- Add a `Region::Pager` `Motion`. `Pager` (or a thin wrapper in `view.rs`) exposes the
  signed drag speed; the view converts it to a strength and axis Horizontal.
- `view::pages` wraps the three-slot row in `stretch::Stretch` (Horizontal) so every
  slot, including source and target, receives the identical transform.
- The cross-fade stays `page_opacity` per slot; because the transform is shared, the
  two visible pages blend instead of diverging.
- Settle reuses the spring; when the pager settles the strength returns to zero.

## Cross-fade rule (the user's requirement)

The per-page opacity may differ (that is the fade). The stretch transform must not:
it is computed once from the shared drag speed and applied identically to all slots
in the row. No per-page sign flip based on `index - 1 - offset`.

## Testing

- Pure: strength from speed is monotonic, bounded, finite, zero at rest; axis
  selection; symmetric horizontal transform identical for both visible slots.
- Headless: `Stretch` is layout-transparent and forwards operations/focus; draw at
  zero displacement is identical to no decorator; vertical stretch appears during a
  drag and returns to zero; pager swipe yields equal displacement for source and
  target slots; no idle redraw; reorder-grip and long-press regressions stay green.
- Full workspace suite, ignored headless suite, `cargo fmt --check`, Clippy with the
  recorded baseline allowances, and an Android release build.

## Open items / builder calls

- Exact strength curve and cap for the velocity term (proposed: same `min(48 dp, 10%
  viewport)` cap as the edge bounce, reached asymptotically with speed).
- Whether the horizontal interior spread is needed at all, or whether a rigid
  translation of each page is enough plus the fade. Proposed: rigid translation first;
  add spread only if a device check shows it reads better.
- Whether the pager strength is driven by finger speed during drag, settle speed after
  release, or both. Proposed: both, capped, with release momentum decaying through the
  spring.

## Outstanding

- Android release APK build + install on the user's tablet, and on-device visual/touch
  validation.
