# Shared Iced fields and dropdown styling

## Intent
Match Flutter's app and folder editor controls using shared Iced components, improving consistency rather than introducing dialog-specific styling.

## Goals
- Replace boxed text inputs with transparent, underline-only fields, preserving the primary-colour focused underline.
- Give dropdown fields a separate small label above the value and arrow, with an underline instead of an enclosing box.
- Match Flutter dropdown spacing: 48 dp option rows, 8 dp vertical menu padding, and folder entries indented 20 dp with an 18 dp folder icon and 8 dp icon gap.
- Style launcher search with an underline (an intentional difference from Flutter's borderless search).
- Keep shared geometry and accessibility bounds aligned, including padded menu height and option positions.
- Add a shared vertical scroll component with a small resisted overscroll bounce plus increased visual spacing between rows when pulled against either end. Spring both effects back on release, without scaling or deforming text, icons, or controls. Logical scroll offsets remain clamped; overscroll is a temporary visual translation, not additional scrollable content.
- Reuse the scroll component across dropdowns, dialogs, settings, app lists, and folder contents. Keep horizontal paging unchanged.

## Non-goals and constraints
No changes to selection, editing, persistence, destination filtering, or the Flutter implementation. Preserve keyboard input, focus, touch targets, ordinary scrolling, gesture arbitration, and accessibility. Use the existing Iced stack and Material theme tokens; avoid dependencies and screen-specific copies. Bounce and row displacement must be bounded, clipped to the scroll viewport, and settle fully; they must not move surrounding controls or scrollbars, trigger list-item actions during a drag, interfere with pull-down search or horizontal paging, or change logical accessibility scroll positions. Mouse-wheel scrolling and programmatic scrolling retain existing behavior; the edge effect targets touch interaction. Animated visual offsets must not alter layout extent or cause scroll-range jumps. After settling, hit targets and visuals must align exactly; during animation, input must be mapped consistently or activation suppressed safely.

## Approaches
1. **Shared components (chosen):** fix shared fields, theme styles, and dropdown layout, then reuse them for search where appropriate. Consistent styling across consumers with slightly broader visual impact.
2. **Dialog-only styling:** smaller scope but duplicates styling and leaves other controls inconsistent. Rejected in favour of shared components by the user.

### Scroll edge effect
1. **Shared spacing-and-bounce components (approved revision):** keep Iced's scrolling and add one shared edge-motion model, a content translation wrapper, and row decorators that spread rows visually without resizing them. Reuse the same motion state and tuning across menus, dialogs, settings, grids, and folder contents; screen adapters identify row boundaries but do not implement separate spring physics. This follows the user's requested spacing and small bounce and does not need renderer changes.
2. **True glyph/content stretch (previous approach):** blocked by the pinned renderer's uniform-only glyph scaling. Extending or forking the renderer is more invasive and not required for spacing plus bounce.
3. **Animate actual layout spacing:** rejected as an implementation choice because changing content extent during overscroll can cause scroll-range and anchoring jumps. Use per-row visual translations instead.

## Implementation discretion
Use an appropriate shared underline rendering primitive because Iced's standard border applies to all four sides. Preserve focus-aware input appearance. Match Flutter menu typography and spacing without introducing a new dropdown framework. Add regression tests for styles and geometry and run the Iced test suite; perform rendering checks if supported by the environment.

For the shared scroller, choose modest bounce/displacement limits and spring constants, centralize them, and test edge detection, resistance, release/settling, and interruption. Use shared row decorations for spacing, including rows in a grid, and retain the existing virtualization spacers and fixed dialog geometry. Validate touch event forwarding, clipping, and compatibility with the existing scroll-fade and gesture wrappers. Preserve pull-down search priority at the launcher top edge. Do not fork Iced or substitute zoom/glyph deformation. If a particular content structure needs an adapter, keep it to row identification and shared-wrapper composition rather than duplicating effect logic.
