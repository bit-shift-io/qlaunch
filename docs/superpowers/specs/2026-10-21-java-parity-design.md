# QLaunch — home-screen fidelity and missing-feature port

Date: 2026-10-21. Status: approved by user, including one-column motion.

## Intent and authority

Make the rewrite feel like QLaunch and complete the relevant missing working
features identified in `2026-10-21-java-parity-audit.md`. Later documented product
decisions take precedence over old Java scaffolding. Preserve the Flutter UI,
Rust-owned model/storage and thin Kotlin Android host.

The user explicitly requested:

- Comparison with Java and existing plans, followed by porting relevant gaps.
- The snappy crossing fade/slide transition, aligned to the grid.
- A long-press menu with Settings, Shortcuts and related launcher actions,
  replacing the old slide-out drawer.

The details below labelled implementation choices are proposed builder calls,
not additional requirements attributed to the user.

## 1. Page motion

One continuous page position drives grid transforms and titles. Gestures track
the finger; release settles to an exact page. At rest, the visible grid is fully
opaque and untransformed. Adjacent grids share identical column origins, row
origins and padding, and never shift vertically during a transition.

Proposed interpretation of **grid aligned** (implementation choice): reduced
travel is one column pitch, rather than an arbitrary half-screen offset. With
viewport width W and C configured columns, each grid's visible translation is
`position * W / C`; counteract PageView's ordinary travel by
`-position * W * (1 - 1/C)`. This leaves a one-column horizontal crossing, with
icons landing exactly on the existing grid. For a single column, use half-width
travel so the effect remains a reduced slide. Row heights remain fixed during a
gesture and use the same layout settings on both pages.

Crossfade (implementation): clamp `1 - 1.5 * abs(position)` to 0–1, so outgoing
and incoming pages both remain visible mid-transition but a heavy page stops
compositing before the incoming page is fully centred. The historical Java
multiplier 1.75 makes both pages invisible around the midpoint; it is not copied
because the user asked for a crossing fade. Never hide/detach a page based on a
fractional position, avoiding the original disappearing-page bug.

Explicit tab/Home navigation animates rather than jumping. Use the nearest
virtual copy of a requested group; title neighbour taps always go one page in
the indicated direction. Settle duration is 150 ms with an
ease-out curve, respecting reduced-motion preferences. Structural model updates
retain the current group where possible and reconcile immediately without
animating through unrelated groups. Preserve group identity mapping in the one
pager controller. One-page groups do not page to duplicate copies of themselves.

Each group preserves its vertical scroll offset while visiting neighbouring
pages; transforms never change scroll position or trigger model mutations.

## 2. Group title strip

Restore the transparent Java-style title strip: current group centred, previous
and next groups at the sides and dimmer. No background panel, bold active tab,
underline or independently scrolling all-groups row. The labels track continuous
pager movement and cross to the new positions as the grid settles. Honour text
style settings and the existing tab-visibility/orientation setting.

Implementation choices: ellipsize long titles before they overlap; expose
neighbour titles as accessible tap targets; draw only the current title for one
group, and allow the same neighbouring group on both sides for two groups.
Remove the overflow-driven pips introduced for the all-tabs strip: that overflow
concept no longer exists. Empty page sets remain a usable blank launcher with
the long-press action available.

## 3. Long-press launcher menu

Long-press blank home-screen space opens a compact menu at the press location,
clamped within safe screen bounds. This replaces the current blank-space action
that opens Settings directly. App/shortcut long presses still open that item's
editor and must not also trigger the launcher menu.

Proposed menu entries (implementation choice, based on the old destinations):

1. Launcher Settings.
2. Add Shortcut — targets the group on which the menu was opened.
3. Edit Groups.
4. Choose Wallpaper — where supported.

Close the menu before opening its destination. Home, Back, an outside tap or
Escape dismisses it. Home dismissal consumes that press; it does not also open
search or change group. Existing Home-button search and search gear behavior
remain. Keep the menu reachable on an empty group. For desktop usability, a
secondary click may open the same menu (implementation choice).

## 4. Grid rendering

Use content-driven row heights rather than square cells in all layouts. Tiles
and centred layouts use an icon above a two-line label without overlap; list
layout uses a left icon and up to two lines of text. Keep columns and label/icon
settings shared between home and search. Tile padding starts from Java's 6 dp
top, 2 dp bottom and 2 dp label gap; list layout starts from 5 dp outer padding
and 10 dp icon/text gap (implementation choices).

Respect the configured font size and system text scaling without shrinking the
whole cell in a FittedBox. Calculate sufficient height for scaled two-line text;
cap icons to available width. Hidden icons/labels reserve no unnecessary space.
Changing configuration or orientation recalculates geometry before another
gesture starts. Extreme supported settings must not overflow or overlap.

## 5. Native display settings and reset

Apply persisted orientation and status-bar visibility on Android after startup,
after settings changes and after in-process reload. Kotlin performs window
operations on the main thread, through the existing Rust/JNI boundary. Retain
correct system insets and transparent wallpaper compositing. Linux treats
Android-only window actions as unsupported/no-op capability, not app failures.

Implement confirmed Factory Reset in Rust. Reset model, settings, usage and
owned icon overrides/cache, rediscover installed apps into the default groups
and return the new snapshot. Persist the replacement atomically before exposing
success; a persistence failure must preserve usable prior state and be reported.
Do not delete unrelated app/Flutter files. Invalidate Dart caches and return the
home UI to its default Home group. Existing restart remains a safe reload.

## 6. Shortcuts

Android: list activities handling ACTION_CREATE_SHORTCUT, show a provider picker
and start the selected activity for result. Capture target group identity before
leaving QLaunch; never infer it from a pager index after returning. A single
pending request is sufficient; cancellation or invalid result clears it and is
reported appropriately. If the captured group was removed, refuse the result
with a clear error rather than silently filing it elsewhere. If the process
dies during provider selection, discard the pending creation on return rather
than fabricate a destination (implementation choice).

Store the label, typed intent payload and supplied icon using Rust storage.
Preserve action, data and MIME type together, categories, component/package,
flags and supported typed extras. Reject unsupported payload data explicitly
rather than silently dropping extras. Filter launcher-category app duplicates
as specified by the original rewrite design. Include package visibility queries
needed for provider discovery on current Android.

Shortcut cells render supplied icon overrides, launch their stored payload and
open an editor on long press. Support rename, move and remove; do not show an
app-uninstall action for a shortcut. Reuse the item editor where practical.
Errors travel through the existing event/state error path; cancellation does not
create an empty item. A missing/corrupt supplied icon uses the normal placeholder.

Linux: Add Shortcut opens a name/command form using the existing Command payload;
launch through the Rust platform layer. Command parsing/launching follows the
existing platform launch conventions, with explicit error reporting. A generic
placeholder is sufficient without adding a file/icon picker.

## 7. Boundaries retained from updated plans

### Additional approved lock actions

Double-tap blank home space locks the screen; app gestures must not trigger it.
The Add Shortcut flow includes a built-in Lock Screen action, stored as a
dedicated launcher-action shortcut, editable/movable/removable like other
shortcuts. Android uses DevicePolicyManager.lockNow with a minimal force-lock
device-admin receiver; when not enabled, open Android's enable-admin prompt.
Do not lock automatically after permission is granted: the next explicit lock
gesture/action locks. Linux uses loginctl lock-session and reports unsupported
session/environment failures. No PIN, sensor or other device-admin features are
implied. This explicitly supersedes the earlier device-lock deferral.

Preserve Frequent/Recent search, hidden apps searchable, Settings-based group
management, group roles, persistence, package receivers/watchers and transparent
wallpaper. Keep deferred notification expansion, device admin, sensors/PIN,
widgets, cloud backup and Java-data migration deferred. Do not introduce the old
broken custom-app-icon picker or scraped category field. Supplying and reading
shortcut icons completes a real ported flow without requiring that picker.

## 8. Verification and delivery

Meaningful widget regressions cover continuous animation, exact resting
alignment, wraparound in both directions, interrupted drags, animated navigation,
model changes, title positioning, scaled two-line labels, menu/item gesture
ownership and shortcut editor behavior. Rust tests cover reset persistence and
failure behavior, shortcut validation/mutations and launch payload handling.
Regenerate the bridge for public API changes and rebuild the native library.

Run Flutter analysis and tests, Rust tests, Linux/Android builds as supported,
then inspect a freshly built installation on an available Android device for
wallpaper/transparency, actual motion, system settings and shortcut results.
Record any unverified device behavior honestly. Preserve device user data during
verification; exercise reset against isolated test configuration. Restore the
original Home app after testing.

Update README, roadmap and the audit with actual delivery status and remaining
platform differences. Leave changes uncommitted unless explicitly requested.

## Builder discretion

Exact widget decomposition, animation plumbing, native request/result transport,
typed-extra representation details compatible with the existing model, menu
styling and test fixtures remain implementation decisions. Keep generated code
generated and all persistent rules in Rust.
