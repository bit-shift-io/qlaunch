# Java parity audit

Date: 2026-10-21. Status: source comparison complete; proposed scope approved and implemented, with device verification recorded in `../plans/2026-10-21-java-parity-progress.md`.

## Reference and intent

Compare the current Flutter/Rust launcher with `origin/java`, restoring relevant
working functionality and the original home-screen feel while preserving the
later documented search and group-management decisions. This table records the
initial source audit; the resulting implementation and device checks are in the
progress ledger.

Authority: the 2026-10-18 rewrite design, the 2026-10-19 organisation plan,
search design, hidden-group/wallpaper plan and the 2026-10-20 icon-performance
design. Later decisions override the original design where they disagree.

## Findings

| Area | Java reference | Current implementation | Proposed disposition |
| --- | --- | --- | --- |
| Group titles | `PagerTabStripNoInd` inherits Android's previous/current/next title layout; current title centred, adjacent titles dimmed; titles move with pager position | `TabStrip` displays every group in a separately scrollable row, adds a dark background, bold current label and underline | Restore transparent, pager-linked previous/current/next titles with no underline; preserve orientation visibility setting |
| Tab indicator | `PagerTabStripNoInd.onDraw` intentionally does not call `super`, suppressing the indicator | Explicit underline in `TabStrip._tab` | Remove underline for Java parity |
| Swipe transition | Standard Android ViewPager slide in checked-in Java; `SectionsPagerAdapter.transformPage` body is completely commented out | Standard Flutter PageView slide | Confirm whether the desired reference is the running plain slide or historical fade/reduced-slide effect |
| Historical transition | Commented code specifies alpha `clamp(1 - abs(position) * 1.75, 0, 1)` and translation `-position * width * 0.5`; also incorrectly hides views | No equivalent | If selected, implement the opacity/translation mathematics without visibility toggling, avoiding Java's disappearing-page defect |
| Tab/Home navigation | `setCurrentItem` animates navigation | `GroupPager._syncPage` uses `jumpToPage`; controller targets a canonical cycle index rather than a nearby page | Animate explicit navigation through a nearby copy; preserve immediate reconciliation when pages change |
| Grid row geometry | Native GridView measures content-height rows | `AppCells` forces square cells for all layouts | Use layout-appropriate row extents from icon, two-line text, padding and text scale; avoid excessive vertical gaps and list rows that are square |
| Labels | Both tile and list XML layouts reserve two lines | `AppCell._label` explicitly forces one line and disables wrapping | Restore two-line labels and ellipsis |
| Size fidelity | Icon/font dimensions are independent | `AppCell` wraps the complete cell in `FittedBox.scaleDown`, allowing text to shrink with the icon | Keep text readable at configured size; calculate row height and constrain icons to available width |
| Centred cells | Java centred icon and label overlap | Rewrite uses a vertical stack | Retain the fixed stack |
| Orientation | `MainActivity.updateSettings` applies requested orientation | Setting persists, but no native or Flutter code applies it | Apply on Android at startup and when settings change |
| Status bar visibility | Java applies fullscreen window flags | Setting persists, but host has no implementation | Apply on Android while retaining correct content insets |
| Factory reset | Working confirmation followed by data wipe | `SettingsScreen._factoryReset` explicitly reports unavailable; bridge has no reset command | Implement core-owned reset of model/settings/usage/icons and update the active snapshot |
| Restart | Java exits the process | Settings reloads stored model and discovery | Preserve safe in-process reload; verify refresh and native settings reapplication |
| Shortcuts | Provider chooser, activity result, launch, rename/move/delete exist; original destination calculation is faulty | Payload/model support only; no creation API, provider flow, launch API or usable UI; cells reject items without `app` | Port complete consuming flow with target group captured before launching provider; support cancel/error, persistence, launch and edit/delete |
| Shortcut icons | Provider icons are saved but rendering incorrectly always asks PackageManager | Core can store icon overrides, but `AppCells` explicitly cannot read them | Add override reading/rendering for shortcut icons; do not reproduce saved-but-unused icons |
| Custom app icon picker | Starts picker without a result-handling path; documented as dead/out of scope in organisation plan | Not exposed | Keep previous scope decision; supporting supplied shortcut icons does not require adding this picker |
| Left drawer | Shortcut, groups, wallpaper and settings destinations | Groups/wallpaper are in Settings; settings reachable by blank-space long press and search gear; no shortcut entry | Add shortcut entry in existing Settings flow rather than reinstating obsolete navigation |
| Right drawer | All-app list/search | Deliberately replaced by Home-button search with Frequent/Recent and no empty-query all-app dump | Preserve newer approved search behavior |
| Search long press | Old all-app drawer opens per-app settings | New search design explicitly specifies inert long press | Preserve newer search decision |
| Groups | Add/rename/reorder/delete/roles and visibility | Present, including hidden-group management and app membership checklist | Retain and regression-check |
| App management | Rename/move/hide/uninstall | Present; hidden group selectable | Retain and regression-check |
| Discovery and package changes | Polling; remembered groups on updates/reinstalls | Discovery/reconciliation plus Android package receiver and Linux watcher implemented | Retain; verify update/reinstall behavior rather than adding polling |
| Wallpaper | System wallpaper with adjustable black overlay and chooser | Implemented using transparent Android window plus overlay; chooser in Settings | Retain; device-check transparency, lifecycle and dimming |
| Deferred integrations | Notification expansion, proximity/PIN/device admin, QS tile, backup, widget scaffolding | Explicitly deferred in rewrite design | Preserve prior deferrals |
| Documentation | Original Java reference plus later rewrite designs | README claims wallpaper/package receiver missing; top-level roadmap has stale checkboxes; old design status headers still say awaiting approval | Update current status and link this audit and execution results |

## Proposed delivery order

1. **Home-screen fidelity:** pager position notifications, transparent three-title
   strip, continuous title movement, animated tab/Home navigation and confirmed
   page transition; tests for wraparound, interrupted gestures, model updates and
   navigation during animation.
2. **Rendering:** two-line labels and content-driven row heights across tiles,
   list and centred layouts; validate large fonts, narrow cells, hidden icons or
   labels, rotation and search's reuse of the same cells.
3. **Working native settings and reset:** orientation/status-bar application,
   persistence/restart reapplication, real confirmed factory reset, failure
   reporting and cache invalidation.
4. **Shortcuts end to end:** provider discovery, explicit destination, lifecycle
   and result handling, typed payloads, supplied icons, launch, edit/move/remove;
   Linux command shortcuts through the same model where relevant.
5. **Verification and documentation:** Flutter analysis/widget tests, Rust tests,
   builds and device inspection where available. Record what was actually run,
   distinguish source-based conclusions from visual verification, update status
   docs and leave an explicit residual-gap list.

## Implementation constraints

- Rust owns persistent state and mutations; Kotlin performs Android operations;
  Dart owns layout, gestures and ephemeral animation.
- Retain schema 1 compatibility and regenerate bindings for public Rust API changes.
- No automatic migration of the Java app's private data.
- No commits or branch changes without an explicit request.
- Device tests must use a freshly built/installed APK; inspect current device and
  Home app before testing and restore the original Home app afterward.

## Follow-up user decisions

The user confirmed the historical crossing fade/slide is a key part of the
snappy feel and must be grid aligned. The user also requested a long-press menu
containing Settings, Shortcuts and related launcher actions, replacing the old
slide-out drawer. See `2026-10-21-java-parity-design.md` for the proposed behavior.
