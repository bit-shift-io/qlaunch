# Shared Iced fields and dropdown styling

## Intent
Match Flutter's app and folder editor controls using shared Iced components, improving consistency rather than introducing dialog-specific styling.

## Goals
- Replace boxed text inputs with transparent, underline-only fields, preserving the primary-colour focused underline.
- Give dropdown fields a separate small label above the value and arrow, with an underline instead of an enclosing box.
- Match Flutter dropdown spacing: 48 dp option rows, 8 dp vertical menu padding, and folder entries indented 20 dp with an 18 dp folder icon and 8 dp icon gap.
- Style launcher search with an underline (an intentional difference from Flutter's borderless search).
- Keep shared geometry and accessibility bounds aligned, including padded menu height and option positions.
- Add a shared vertical scroll component with Flutter Android-style edge stretch: content deforms subtly when pulled against an end, then springs back on release. Retain clamped logical scroll offsets rather than translating the whole list beyond its boundary.
- Reuse the scroll component across dropdowns, dialogs, settings, app lists, and folder contents. Keep horizontal paging unchanged.

## Non-goals and constraints
No changes to selection, editing, persistence, destination filtering, or the Flutter implementation. Preserve keyboard input, focus, touch targets, ordinary scrolling, gesture arbitration, and accessibility. Use the existing Iced stack and Material theme tokens; avoid dependencies and screen-specific copies. The stretch must be bounded, clipped to the scroll viewport, and settle fully; it must not move surrounding controls, trigger list-item actions during a drag, interfere with pull-down search or horizontal paging, or change logical accessibility scroll positions. Mouse-wheel scrolling and programmatic scrolling retain existing behavior; the edge stretch targets touch interaction.

## Approaches
1. **Shared components (chosen):** fix shared fields, theme styles, and dropdown layout, then reuse them for search where appropriate. Consistent styling across consumers with slightly broader visual impact.
2. **Dialog-only styling:** smaller scope but duplicates styling and leaves other controls inconsistent. Rejected in favour of shared components by the user.

### Scroll edge effect
1. **Shared scroll component (chosen):** add reusable edge-stretch animation around the existing Iced scrolling behavior and migrate vertical scrollers to it. Keeps dependency maintenance unchanged and centralizes gesture/animation logic.
2. **Modify the Iced dependency:** implement the effect directly in its scrollable widget. More invasive, with an ongoing fork-maintenance cost; not chosen.

## Implementation discretion
Use an appropriate shared underline rendering primitive because Iced's standard border applies to all four sides. Preserve focus-aware input appearance. Match Flutter menu typography and spacing without introducing a new dropdown framework. Add regression tests for styles and geometry and run the Iced test suite; perform rendering checks if supported by the environment.

For the shared scroller, choose modest stretch limits and spring constants, centralize them, and test edge detection, resistance, release/settling, and interruption. Validate touch event forwarding, clipping, and compatibility with the existing scroll-fade and gesture wrappers. If investigation shows a wrapper cannot preserve the existing behavior, stop and revisit the approach rather than silently forking Iced or degrading interactions.
