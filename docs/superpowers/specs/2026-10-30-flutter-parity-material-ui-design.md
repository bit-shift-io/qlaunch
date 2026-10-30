# Flutter-parity Material UI for the Iced port — design

Date: 2026-10-30
Status: proposed for the `dev/iced` comparison spike

## Context

The Iced launcher matches the Flutter launcher's grid, paging, gestures,
folders, search and accessibility, but its secondary UI is still a placeholder.
App and folder long-press open a tall action list with Rename/Move sub-pages,
the launcher menu is a list of the same shape, and the Settings, Advanced and
Group Editor entries say "Not yet available in the Iced port".

Flutter has a complete Material 3 UI for all of these: one compact form dialog
for an app, one for a folder, a five-row launcher menu, a settings screen with
every §8.6 control, an advanced screen, a group editor with its own dialogs,
and a Choose Apps checklist. This document defines how the Iced port gets the
same screens, the same visual language and the same behaviour, from one shared
theme rather than per-screen styling.

## Intent

Every popup, dialog and secondary screen in the Iced port should look and
behave like its Flutter counterpart, while continuing to call the existing core
API for every model or settings change.

The theme is the foundation: a small set of Material 3 tokens and reusable
components that every screen and dialog consumes. No screen defines its own
colours, text sizes, paddings, button shapes or icon sizes.

## Goals

1. A reusable dark Material 3 theme: colour tokens, typography, spacing,
   shapes, elevation and widget styles, all traceable to Flutter's defaults.
2. Reusable components for dialogs, app bars, list tiles, section headers,
   sliders, switches, choice chips, text fields, dropdowns, buttons, checkboxes,
   snackbars and icons.
3. A unified compact dialog model used by every popup, with the same geometry
   as Flutter's `AlertDialog`: 28 dp corners, `surfaceContainerHigh`, title,
   content and a right-aligned action row.
4. App long-press becomes Flutter's `AppSettingsDialog`: Name, package,
   Group dropdown, Uninstall/Remove/Cancel/Save.
5. Folder-tile long-press becomes Flutter's `FolderDialog.edit`: Name, Group,
   Delete/Cancel/Save. Creating a folder uses the same form.
6. The launcher menu, Add Shortcut, Command Shortcut, delete/restore/factory
   reset confirmations and the restore picker match Flutter.
7. Settings, Advanced, Groups and Choose Apps are full screens with working
   controls, not visual shells.
8. A screen stack and a single back order cover dialogs, dropdowns, screens,
   overlays, search and folders.
9. Accessibility frames describe every new screen, dialog and dropdown with
   the same labels and bounds as the drawing.
10. Existing launcher behaviour, gestures, performance and tests do not
    regress.

## Non-goals

- Pixel-identical rendering. Flutter's Impeller and Iced's renderer differ in
  antialiasing, font shaping and shadows; the target is the same layout,
  dimensions, colours and interaction.
- Rebuilding the grid, pager, title strip, folder contents popup or search
  results. They keep their behaviour and geometry; they adopt theme tokens
  where they already use `material.rs`.
- Desktop-specific keyboard navigation beyond what Iced already provides
  (Tab focus, Enter submit). The product target is Android touch and TalkBack.
- Android runtime permissions beyond what the core already exposes. Flutter's
  "All files access" prompt for restore is a Flutter platform detail; the
  Iced port calls `list_backups` and reports its error or empty result.
- New dependencies, `iced_material`, or a Flutter/Dart dependency in the Rust
  build.
- Changes to core model rules, §8.4 role rules or §8.6 ranges. The UI sends
  requests; the core decides.

## Constraints

- Work stays on `dev/iced`; the user owns Git operations.
- `qlaunch_iced` keeps building with `--no-default-features`.
- Core calls stay off Iced's UI thread; use the existing `work` helper and
  `Task`.
- Format only `qlaunch_iced`; `cargo fmt --all` touches unrelated core files.
- Strict Clippy applies to `qlaunch_iced` with `--no-deps`.
- Text sizes and typeface follow the launcher's Text settings, as Flutter's
  `LauncherFontScope` does for its screens and dialogs.
- The Material Icons font is bundled with its Apache 2.0 licence. It is a font
  asset, not a UI dependency.

## Approaches considered

### Theme location

1. **`theme.rs` + `components.rs` + `icons.rs` (chosen).** `theme` owns tokens
   and iced widget style functions; `components` owns composed widgets;
   `icons` owns the bundled font and glyph constants. `material.rs` is replaced.
   Clear ownership, easy to test token values and typography scaling.
2. **Grow `material.rs` into one large module.** Fewer files, but mixes tokens,
   widget composition and icon data. Rejected.
3. **A separate crate.** Extra build and versioning for no reuse outside this
   binary. Rejected.

### Screen navigation

1. **A `Vec<Screen>` stack in `App` (chosen).** Screens are app state, so the
   existing `Message`/`update`/`view` loop, back handling and accessibility
   publishing all see them without a router dependency. Matches Flutter's
   `Navigator.push`, including a dialog staying open underneath a pushed
   Choose Apps screen.
2. **An overlay variant per screen.** Conflates modal menus with pages, makes
   back order and accessibility selection harder. Rejected.
3. **A third-party router.** New dependency and an abstraction the app does not
   need. Rejected.

### Dialog model

1. **One `Dialog` enum with per-kind state and one shared card renderer
   (chosen).** All popups get the same scrim, card, title, content and action
   row; each kind only supplies its rows and actions.
2. **Keep the current entry-list overlay and add forms as new steps.** Retains
   the tall list look and the Rename/Move sub-pages the user asked to remove.
   Rejected.
3. **One widget per dialog with duplicated chrome.** Guarantees drift. Rejected.

### Dropdown

1. **Custom `Select` component with an app-owned open state (chosen).** It can
   be styled like Flutter's `DropdownButtonFormField`, opened by accessibility
   actions, and rendered above the dialog without clipping.
2. **Iced `pick_list`.** Its menu cannot be opened programmatically, its menu
   style is global, and its closed field is not a Material form field.
   Rejected: it cannot meet the accessibility and visual contracts.
3. **Inline expanding list.** Simple, but Flutter overlays the menu instead of
   growing the dialog. Rejected.

### Icons

1. **Bundle `MaterialIcons-Regular.otf` (chosen).** Exact glyphs, no runtime
   font discovery, works on Android and Linux. 1.6 MB asset.
2. **Draw icons as SVG/PNG.** More assets and no colour inheritance. Rejected.
3. **Use text characters or emoji.** Wrong glyphs and metrics. Rejected.

## Design

### Theme tokens

`theme.rs` exports the Flutter `_colorSchemeDarkM3` values used by the UI. The
full scheme is defined so later screens do not invent colours; only the used
subset needs a named constant.

| Token | Value | Used for |
| --- | --- | --- |
| `SURFACE` | `#141218` | screen background |
| `ON_SURFACE` | `#E6E0E9` | primary text, app bar title |
| `SURFACE_CONTAINER_LOWEST` | `#0F0D13` | pressed surface fallback |
| `SURFACE_CONTAINER_LOW` | `#1D1B20` | menu surface |
| `SURFACE_CONTAINER` | `#211F26` | dropdown menu |
| `SURFACE_CONTAINER_HIGH` | `#2B2930` | dialog and folder popup |
| `SURFACE_CONTAINER_HIGHEST` | `#36343B` | slider inactive rail, switch off track |
| `ON_SURFACE_VARIANT` | `#CAC4D0` | subtitles, labels, icons |
| `OUTLINE` | `#938F99` | field underline, chip border |
| `OUTLINE_VARIANT` | `#49454F` | dividers if needed |
| `PRIMARY` | `#D0BCFF` | accent, active slider, selected controls |
| `ON_PRIMARY` | `#381E72` | filled button text, selected check |
| `PRIMARY_CONTAINER` | `#4F378B` | selected container fallback |
| `ON_PRIMARY_CONTAINER` | `#EADDFF` | on selected container |
| `SECONDARY_CONTAINER` | `#4A4458` | selected chip and dropdown option |
| `ON_SECONDARY_CONTAINER` | `#E8DEF8` | selected chip text |
| `ERROR` | `#F2B8B5` | destructive text, errors |
| `ON_ERROR` | `#601410` | destructive filled text |
| `INVERSE_SURFACE` | `#E6E0E9` | snackbar background |
| `ON_INVERSE_SURFACE` | `#322F35` | snackbar text |
| `SCRIM` | black 54% | dialog barrier |

Shapes: dialog 28, menu 4, chip 8, field underline 4 (bottom only), button 20,
snackbar 4, icon-button circle.

Elevation: dialog shadow offset `(0, 6)`, blur 12, black 30%; dropdown and
snackbar use the same shadow; app bars have no shadow.

### Typography and user font settings

`theme.rs` defines the Material 3 dark text styles:

| Style | Size / line | Weight |
| --- | --- | --- |
| `headline_small` | 24 / 32 | 400 |
| `title_large` | 22 / 28 | 400 |
| `title_medium` | 16 / 24 | 500 |
| `title_small` | 14 / 20 | 600 (sections) |
| `body_large` | 16 / 24 | 400 |
| `body_medium` | 14 / 20 | 400 |
| `body_small` | 12 / 16 | 400 |
| `label_large` | 14 / 20 | 500 |
| `label_medium` | 12 / 16 | 500 |

Every screen and dialog text style is passed through
`theme::scaled(style, settings)`:

- Multiply the size and line height by `settings.label_size / 16`, matching
  Flutter's `fontSizeFactor`.
- Family: `Normal` → platform default (`Roboto` on Android, the Iced default on
  Linux), `Sans` → `sans-serif`, `Serif` → `serif`, `Monospace` → `monospace`.
  If a family is unavailable, the renderer's default is used.
- Colour is supplied by the component, not the user's label colour; Flutter's
  `LauncherFontScope` leaves colour to the theme for lists and dialogs.

Launcher labels keep `label_style` semantics (colour, shadow, size) and are not
affected by this scaling.

### Components

`components.rs` provides the following. Each returns an `Element` and takes the
settings only where text must scale.

- `app_bar(title, on_back, actions)` — 64 dp, surface background, 56 dp leading
  slot, title at 16 dp after it, 24 dp icons, 48 dp touch targets.
- `section_header(label)` — 14 dp/600, 16 dp horizontal, 24 dp top.
- `list_tile(title, subtitle, leading_icon, trailing, on_press, enabled)` —
  56 dp one-line / 72 dp two-line, 16 dp padding, 24 dp leading, 16 dp gap,
  body-large title, body-medium on-surface-variant subtitle, 8% state layer,
  38% opacity when disabled.
- `action_tile(title, subtitle, on_press)` — the two-line list tile used by
  Settings and Advanced.
- `switch_row(label, value, on_toggle, enabled)` — 56 dp, label left, 52×32
  switch right; selected track primary, off track
  `surfaceContainerHighest`, thumb `onPrimary`/`outline`, 38% disabled.
- `slider_row(label, value, unit, min, max, on_change, on_release)` — label row
  (body large + body small value, 16 dp sides, 12 dp top), then a 48 dp slider
  with 4 dp rail, primary active rail, `surfaceContainerHighest` inactive rail,
  10 dp radius thumb and a 40 dp state layer.
- `choice_row(label, options, selected, on_select)` — label row, then a wrapping
  row of chips with 8 dp gaps and 16 dp side padding. Chip 32 dp high, 8 dp
  radius, 8 dp horizontal padding, label-large; unselected outline border and
  on-surface-variant text, selected `secondaryContainer` with
  `onSecondaryContainer` text.
- `text_field(label, value, placeholder, on_input, on_submit, widget_id)` —
  56 dp; 12 dp floated label at the top in `onSurfaceVariant`; 16 dp value;
  1 dp `outline` bottom border, 2 dp `primary` when focused; primary cursor and
  35% primary selection.
- `select(label, value, is_open, on_open, widget_id)` — the same 56 dp field
  with a 24 dp chevron. The open menu is a separate layer, not part of the
  field: `surfaceContainer`, 4 dp radius, elevation shadow, 48 dp options,
  selected option `secondaryContainer`, folder options indented 20 dp with an
  18 dp folder glyph, max height 280 dp, clamped inside the window.
- `text_button(label, on_press, enabled)`, `filled_button(...)`,
  `destructive_button(...)` — 40 dp high, 20 dp radius, 12 dp horizontal
  padding, label-large; disabled 38% opacity.
- `icon_button(glyph, label, on_press, enabled)` — 48 dp square, 24 dp glyph,
  circular 8% state layer.
- `checkbox_row(label, checked, on_toggle, enabled)` — 56 dp, 18 dp box with
  2 dp outline, primary fill and `onPrimary` check when selected.
- `snackbar(message)` — bottom-anchored, 8 dp sides and bottom plus the window
  inset, min 48 dp, 16/14 dp padding, 4 dp radius, `inverseSurface` background,
  `onInverseSurface` body-medium, elevation shadow.
- `dialog_scrim(on_dismiss)` — full-window black 54%, closes on press.
- `dialog_card(title, content, actions)` — the shared AlertDialog chrome.

All components take their colours and metrics from `theme`, never literals.

### Icons

`icons.rs`:

- `FONT: Font = Font::with_name("Material Icons")`.
- The font is loaded in `App::new`/`main` with
  `app.font(include_bytes!("../assets/MaterialIcons-Regular.otf").as_slice())`
  next to the existing platform fonts.
- Glyph constants, copied from Flutter's `icons.dart`:

| Name | Code point |
| --- | --- |
| `SETTINGS` | `e57f` |
| `ADD_LINK` | `e053` |
| `CREATE_NEW_FOLDER` | `ef8c` |
| `DASHBOARD_CUSTOMIZE` | `e1b2` |
| `WALLPAPER` | `e6ca` |
| `LOCK_OUTLINE` | `e3b1` |
| `NOTIFICATIONS_NONE` | `e451` |
| `LINK` | `e380` |
| `CHECKLIST` | `e15b` |
| `ARROW_UPWARD` | `e0a0` |
| `ARROW_DOWNWARD` | `e097` |
| `ARROW_BACK` | `e092` |
| `FOLDER` | `e2a3` |
| `KEYBOARD_ARROW_DOWN` | `e353` |
| `ADD` | `e047` |
| `CHECK` | `e156` |
| `CLOSE` | `e16a` |
| `ERROR_OUTLINE` | `e238` |

The asset lives at `rust/qlaunch_iced/assets/MaterialIcons-Regular.otf`, with
the Apache 2.0 licence beside it. Icons are `text(glyph).font(icons::FONT)`;
the glyph colour is set by the component.

### App state and navigation

`App` gains:

```rust
screens: Vec<Screen>,
dialog: Option<Dialog>,
select: Option<SelectState>,
settings_drag: Option<(SettingField, f64)>,
snackbar: Option<Snackbar>,
```

```rust
enum Screen {
    Settings,
    Advanced,
    Groups,
    GroupApps(Id),
}
```

- The launcher is the base layer; screens render full-window above it with the
  `surface` background, so the grid is not visible behind them.
- `Settings` pushes `Advanced` and `Groups`; `Groups` pushes `GroupApps`.
- A dialog may sit above a screen. Pushing `GroupApps` from the Group Editor
  dialog leaves that dialog in place underneath, matching Flutter's Navigator.
- `Screen::Groups` and `Screen::GroupApps` carry no snapshot copy; they read
  `app.snapshot`, so a core change is reflected immediately.
- The screen stack is cleared on Home press, like the other launcher state.

Back order, highest first:

1. Open dropdown menu → close it.
2. Dialog → close it (or step back inside it; the unified dialogs have no
   sub-pages, so this closes).
3. Screen → pop one screen.
4. Overlay (launcher/app menu) → existing overlay back.
5. Search → close.
6. Folder → close.
7. Otherwise, leave the existing Android-back behaviour unchanged.

`Back` and `CoreEvent::BackPressed` use this order. A scrim press closes the
dialog or dropdown; it does not pop a screen.

Every launcher input path must also stop at a screen or dialog, because a
screen is a plain container and does not consume events by itself:

- `Message::ItemPressed` and `Message::BlankPressed` return early when a screen
  or dialog is open, before the gesture classifier runs.
- `Message::Search`, `Message::FolderDismiss`, the double-tap lock gesture and
  the pull-down (`Outcome::PullDown`) are no-ops while a screen or dialog is
  open.
- `HomePressed` closes the dropdown, the dialog and the screen stack, then
  follows the existing overlay/search/folder behaviour.
- The screen layer and the dialog scrim each start with a full-window
  `mouse_area` no-op barrier so a press cannot reach the launcher underneath.
  `opaque` only affects drawing, not hit-testing.

The existing `ActionDone(Ok)` handler closes the overlay; the new screen and
dialog completions must use their own message so they do not close an unrelated
overlay.

`Region` gains a `Screen` variant whose widget id is `screen-scroll`, shared by
Settings, Advanced, Groups and Choose Apps (only one screen is ever active).

### Dialog model and geometry

`dialog.rs` replaces `overlay.rs`.

```rust
enum Dialog {
    App(AppEditor),
    Folder(FolderEditor),
    Launcher,
    AddShortcut(AddShortcutState),
    CommandShortcut(CommandShortcutState),
    AddGroup(AddGroupState),
    GroupEditor(GroupEditorState),
    Confirm(ConfirmState),
    RestorePicker(RestorePickerState),
}
```

Shared geometry, used by the view and the accessibility tree:

- `width = (window.width - 80).clamp(280, 560)`, so phone layouts match
  Flutter's 40 dp inset and desktop forms stay readable.
- `max_height = window.height - 48`.
- Centred horizontally and vertically; if content exceeds `max_height`, the
  content area scrolls and the title/actions stay fixed.
- Corner radius 28; background `surfaceContainerHigh`; scrim 54%.
- Title: headline-small, 24 dp sides and top, 0 bottom when content follows.
- Content: 16 dp top, 24 dp sides, 24 dp bottom.
- Actions: right-aligned row, 8 dp gap, 24 dp sides and bottom, no top padding;
  buttons 40 dp high. A dialog with no actions omits the row.
- Content rows are 56 dp fields, 16 dp between form controls, 12 dp between a
  field and its helper text, 8 dp between list rows.
- The dialog's rows and total height are computed by a pure
  `dialog::layout(app, dialog)` returning the card, title, content viewport,
  each row's rect and the action rects. `view` builds the same rows in the same
  order; accessibility reads the layout. No widget is measured.

`SelectState` records the dialog field that is open and its option list. The
open menu is drawn in a top-level layer at the field's computed rect, below the
field, clamped to the window; a transparent full-window barrier closes it.
`Group` option lists are built by the same helper as the form so the closed
label and the menu agree.

### Dialogs

#### App editor (long-press an app)

Matches Flutter's `AppSettingsDialog`:

- No title.
- Content, 24 dp all round: Name text field; if the item has an app, a 12 dp
  gap then the package in body-small (and "Uninstalled" in body-small when
  applicable); 16 dp gap; Group select.
- Group options: every non-root, non-folder group in snapshot order, each
  followed by its folders (folder options indented with a folder glyph). The
  item's current group is appended if the list would not contain it.
- Actions: `Uninstall` (only when `item.app.is_some() && !item.uninstalled`),
  `Remove` (only when `item.shortcut.is_some()`), `Cancel`, `Save`.
- Save: trim the name; if changed call `rename_item`; if the group changed call
  `move_item` with `append_order` for the destination. Close only after both
  succeed; on error keep the dialog open and show the error in body-small
  `error`.
- Uninstall: `uninstall_app(package)`; Remove: `remove_item(id)`. Both close on
  success, show the error on failure.
- Save is disabled when the trimmed name is empty. Uninstall/Remove are text
  buttons; Save is filled.

#### Folder editor (long-press a folder tile; Create Folder)

Matches Flutter's `FolderDialog`:

- Title: the folder label, or `New Folder` when creating.
- Content: Name field (default `Folder` for a new folder), 16 dp gap, Group
  select.
- Group options: non-root, non-folder groups excluding the Hidden role; the
  folder's current group is added back if it is excluded, so the initial value
  is always valid.
- Actions: `Delete` (edit only), `Cancel`, `Save`.
- Save (create): `add_folder(containing_group, name)`.
- Save (edit): `rename_group` if the name changed, then `move_item` of the
  folder's tile if the group changed, using `append_order`.
- Delete: `delete_folder(folder_group_id)`.
- Save disabled when the name is empty or no group is selected; all actions
  disabled while a request is in flight.

Long-pressing a folder tile now opens this dialog. Tapping a folder tile still
opens the folder contents popup, and long-pressing an icon inside that popup
still opens the app editor for that member.

#### Launcher menu (long-press blank space)

Matches Flutter's launcher menu:

- Title `Launcher`; content padding 8 dp vertical, 0 horizontal.
- Five list tiles with 24 dp leading icons: Launcher Settings, Add Shortcut,
  Create Folder, Edit Groups, Choose Wallpaper.
- No action row. Scrim/back closes.
- Settings pushes the Settings screen; Add Shortcut opens that dialog; Create
  Folder opens the folder create dialog for the current group; Edit Groups
  pushes the Groups screen; Choose Wallpaper calls `choose_wallpaper` and
  reports an error in a snackbar (on Linux the core returns the
  Android-only error).

#### Add Shortcut

Matches Flutter's `AddShortcutDialog`:

- Title `Add Shortcut`.
- Content: while providers load, a 64 dp centred progress area; then list tiles:
  Lock Screen, Notifications (Android only), then one per provider, with the
  command provider using the link icon. Busy disables every row.
- Actions: `Cancel`.
- Choosing lock/notifications/provider runs the existing core call and closes
  on success; an error keeps the dialog open. Choosing the command provider
  opens the Command Shortcut dialog.

#### Command Shortcut

Matches Flutter's `_CommandShortcutDialog`:

- Title `Command Shortcut`.
- Content: Name field; Command field with the 12 dp helper `No shell is
  involved.`
- Actions: `Cancel`, `Add` (filled). `Add` is disabled until both trimmed
  values are non-empty.
- Add calls `add_command_shortcut(target_group, name, command)`, then closes
  both this dialog and the Add Shortcut dialog.

#### Add group

Matches Flutter's `_AddGroupDialog`:

- Title `Add group`; Name field; `Cancel`, `Add`.
- Add is disabled until the trimmed name is non-empty; it calls `add_group` and
  closes on success.

#### Group editor

Matches Flutter's `_GroupEditDialog`:

- Title: the group label.
- Content: Name field; 16 dp gap; `Role` label-large; 4 dp gap; choice chips
  `Home`, `Apps`, `Games`, `None`; then a `Choose apps` list tile with a
  checklist icon.
- Role chips send `set_group_role` immediately, as Flutter does; the dialog's
  local role updates on success. When the current role is `Apps`, `Games` or
  `Hidden`, the chips are disabled and Delete is disabled (the core will not
  release a one-holder role). `Hidden` is not offered as a chip; it is shown by
  the Groups screen switch.
- `Choose apps` pushes `GroupApps` and keeps the dialog underneath.
- Actions: `Delete` (text, destructive; disabled unless role is `None`),
  `Cancel`, `Save` (filled; disabled when the name is empty).
- Save renames when the trimmed name differs, then closes.
- Delete opens a confirmation dialog: title `Delete <label>?`, body `The apps in
  this group move to the Apps group.`, actions `Cancel`, `Delete` (filled
  destructive). On confirm, `delete_group` runs and both dialogs close.

#### Confirmation dialog

- Title, body text (body-medium, on-surface), `Cancel`, confirm button.
- Confirm is filled, or filled with `onError`/`error` colours for destructive
  actions (`Delete`, `Reset`).
- Used by group delete, restore and factory reset. App/folder delete stay
  immediate, matching Flutter.

#### Restore picker

Matches Flutter's `SimpleDialog`:

- Title `Restore config`.
- Scrollable list of backup file names, 48 dp rows, 24 dp horizontal padding;
  no action row. Choosing one opens the restore confirmation.
- Empty list is reported by snackbar (`No backups found in Downloads.`) instead
  of opening the dialog.

### Settings screen

Matches `settings_screen.dart`, top to bottom:

- App bar: `Launcher Settings`, back.
- `Display`:
  - Continuous scrolling — switch.
  - Page transition duration — slider 100–800, unit ` ms`.
  - Portrait columns — slider 1–12.
  - Landscape columns — slider 1–12.
  - Cell layout — Tiles / List / Centred.
  - Tab strip — Always / Landscape / Portrait / Never.
  - Scroll fade — None / Gradient / Tile.
- `Window`:
  - Choose wallpaper — action tile with subtitle
    `Pick one with the system wallpaper chooser.`
  - Wallpaper dimming — slider 0–100.
  - Show the status bar — switch.
  - Requested orientation — Auto / Landscape / Portrait.
- `Icon`:
  - Show icons — switch.
  - Icon size — slider 0–100.
  - Icon opacity — slider 0–100.
- `Text`:
  - Show labels — switch.
  - Font size — slider 0–100.
  - Typeface — Normal / Sans / Serif / Mono.
  - Colour — White / Black / Grey.
  - Label shadow — switch.
  - Shadow blur — slider 0–10.
- `Groups` — action tile `Groups`, subtitle
  `Add, rename, reorder and fill the tabs.`, pushes the Groups screen.
- `Advanced` — action tile `Advanced`, subtitle
  `Restart, backup, restore and factory reset.`, pushes the Advanced screen.
- 32 dp bottom space.

Behaviour:

- Every displayed value comes from the snapshot; after a patch the core's
  clamped value is what the screen shows.
- A slider's drag value is transient in `settings_drag`; `on_change` only
  updates that value, and `on_release` sends one `update_settings` patch. The
  drag value is cleared when the request completes, success or error.
- Switches and chips send one patch on press.
- Errors are shown in the snackbar; the screen stays open.
- The screen is a single scrollable column with a tracked `Region::Screen`
  offset, so accessibility bounds follow scrolling.

### Advanced screen

Matches `advanced_screen.dart`:

- App bar `Advanced`, back.
- Four action tiles: Restart Launcher, Backup config, Restore config, Factory
  Reset, with Flutter's subtitles.
- Restart: run `core::init(None)` through the existing `Loaded` path, keeping
  the screen open.
- Backup: `backup_config`; snackbar `Backup saved to Downloads/<name>.` on
  success, `Backup failed: <error>` on failure.
- Restore: `list_backups`; empty → snackbar `No backups found in Downloads.`;
  otherwise open the restore picker, then the confirmation
  `This replaces your current layout and settings with <name>.`; on confirm call
  `restore_config` and snackbar `Config restored from <name>.` or
  `Could not restore: <error>`.
- Factory Reset: confirmation `This wipes every group, item and setting. It
  cannot be undone.`; on confirm call `factory_reset` and snackbar
  `The launcher was reset.`.
- Snackbar duration 4 s; a new message replaces the current one; a generation
  id ignores a stale dismissal timer.

### Groups screen

Matches `group_editor_screen.dart`:

- App bar `Groups`, back, `+` action.
- Rows: every non-root, non-folder group sorted by `order`. Each row shows the
  label, the role name as subtitle when set, a hide switch, and up/down icon
  buttons when the group is a page.
- Row height 72 with a role, 56 without.
- The Hidden role's switch is on and disabled; other switches call
  `set_group_hidden`.
- Arrows call `move_group(id, page_index ± 1)` and are disabled at the ends.
- Tapping a row opens the Group editor dialog.
- `+` opens Add group.

### Choose Apps screen

Matches `group_apps_screen.dart`:

- App bar: the group label, back.
- Every installed, non-uninstalled app sorted case-insensitively by label, as a
  56 dp checkbox row.
- Checked when `item.group_id == group_id`.
- Ticking moves the app into this group at the append order; unticking moves it
  to the Apps group at the append order.
- In the Apps group itself every checkbox is disabled (there is nowhere to
  move an app out to).
- The list is scrollable with a tracked `Region::Screen` offset.

### Core integration

- Every mutation uses the existing `qlaunch_core::api::api` functions; no model
  or settings rule is reimplemented.
- Add one completion message, e.g.
  `Message::ModelDone { result: Result<ModelSnapshot, String>, then: Completion }`
  where `Completion` is `Keep`, `CloseDialog`, `CloseDialogAndParent`,
  `CloseDialogAndSnackbar(String)`, or `Snackbar(String)`. On `Ok` the snapshot
  is applied immediately (then the `ModelChanged` event is harmless); on `Err`
  the error goes to the active dialog or the snackbar.
- Pure `plan` helpers (`app_editor_plan`, `folder_editor_plan`,
  `group_editor_plan`) return the ordered core calls for a Save. This keeps
  the “rename then move” order testable without IO.
- Dialog state is only closed on a successful request; an error leaves the
  fields intact so the user can retry.
- Existing `ActionDone` for lock/notifications/wallpaper remains, or is folded
  into the new completion message; behaviour is unchanged.

### Accessibility

`accessibility.rs` gains frames for:

- `settings`, `advanced`, `groups`, `group-apps:<id>` screens.
- `dialog:<kind>` dialogs.
- `select:<field>` when a dropdown menu is open.

Selection is top-most layer only: dropdown → dialog → screen → overlay →
search → folder → launcher. Hidden layers publish no nodes.

Common nodes:

- App bar back: `button`, label `Back`, click = Back.
- App bar title: `header`.
- Screen scroll container: `scroll`, with `scroll_forward`/`scroll_backward`
  when content can move and the tracked offset.
- Dialog title: `header`; dialog error: `text`.
- Every action button: `button`, click dispatches the same message as touch.
- Dismiss/back closes one layer through the same Back path.

Settings:

- Section headers are `header`.
- Switch rows are `button` nodes labelled `<label>, on/off`, selected when on,
  click toggles.
- Slider rows are `button` nodes labelled `<label>, <value><unit>`, advertising
  the existing `scroll_forward`/`scroll_backward` actions, which step the value
  by one unit. This reuses Kotlin's current action mapping
  (`LauncherAccessibilityView.kt` maps Android's
  `ACTION_SCROLL_FORWARD`/`ACTION_SCROLL_BACKWARD` to those strings); no Kotlin
  change is needed, at the cost of TalkBack wording that says "scroll". A
  RangeInfo / `ACTION_SET_PROGRESS` mapping with "increase/decrease" wording is
  a later improvement and would require Kotlin work.
- Choice chips are `button` nodes with `selected`.
- Action tiles are `button` nodes.
- Text fields (dialog forms) are `text_field` nodes with their value,
  `click` to focus the real iced field and `set_text` for a whole replacement.
- Select fields are `button` nodes labelled `<label>, <current>`; click opens
  the menu. Menu options are `button` nodes; selecting one sends the same
  message as touch; `dismiss` closes.
- Groups rows are `button` nodes labelled `<label>, <role>`; the hide switch,
  up and down are separate nodes with their own labels and enabled state.
- Choose Apps rows are `button` nodes labelled `<app>, checked/unchecked`,
  selected when checked, click toggles; disabled in the Apps group.
- Snackbar text is a `text` node and an announcement when it appears.

Node keys stay semantic (`settings:control:<field>`, `dialog:action:save`,
`group:<id>:up`, `select:<field>:option:<id>`), so focus survives republishes.
Bounds come from the same layout functions as the view: dialog layout, screen
row heights, select popup rect, scroll offsets. Every node is clipped to its
scroll viewport and the window, as today.

Two new `Target` variants carry the controls without duplicating dispatch:

- `Target::Message(Message)` for buttons, chips, switches, fields and rows. It
  reuses the widget's own `AppMessage`, so an accessibility press and a touch
  press take the identical path. `Message` is already `Clone + Debug`.
- `Target::Slider { step_up: Message, step_down: Message }` so
  `scroll_forward`/`scroll_backward` step the slider through the same messages
  the widget uses. `Action::ScrollLeft`/`ScrollRight` are never advertised on a
  slider or screen node: they are intercepted globally as page steps.
- `Target::Dialog(dialog::Message)` and `Target::Screen(Message)` are only
  needed if carrying a full `Message` proves awkward; either shape is a builder
  call.

### Rendering and geometry

`view.rs` keeps stable top-level slots so modal transitions never reconcile a
scrollable against a different hierarchy:

```
launcher (base, includes search)
folder
overlay
screen
snackbar
dialog
select menu
```

Each absent layer is `space()`. Modal layers are `opaque` for drawing and
carry their own full-window press barrier for hit-testing (see the back-order
section); the launcher's gesture classifier only receives events when no
screen/dialog is above it. Screens are full-window `container`s with the
surface background behind that barrier. The dialog and select layers use
`stack!` + `pin` at computed rects.

Explicit metrics live in `theme::metrics` (app bar height, row heights, field
height, dialog paddings, action height, snackbar height, icon sizes) and are
shared by `components`, `dialog`, the screen modules and `accessibility`. No
screen copies numbers from another module.

### Existing chrome

The grid, pager, title strip, folder contents popup and search results keep
their layout. Two small parity changes:

- Search gains the Settings gear Flutter's `AppSearch` has
  (`appSearch.gear`), pushing the Settings screen; its close control uses the
  Material `close` glyph instead of the `×` text button.
- `material.rs` call sites move to `theme`, so the launcher, folder popup and
  search use the same colours and scrollbar styling as the new UI. Folder
  membership for a group (used by the app dialog's folder options and the
  folder dialog's current group) is derived read-only from the snapshot:
  a folder group `F` belongs to group `G` when an item in `G` has
  `folder_group_id == F`, mirroring Flutter's `foldersIn`.

### Known visual deviations

- Flutter lets a desktop dialog grow to the available width; the Iced dialog is
  capped at 560 dp for readability. On a phone-width window, the comparison
  target, both are `window - 80`.
- Search chrome is restyled but not relaid out.

### Text field label

Iced's `text_input` exposes no focus event, so the Material floating label
cannot animate from the in-field position to the floated position. The label is
always drawn in its floated position inside the 56 dp field. Every prefilled
form (app name, folder name, group, command name) is identical to Flutter; only
an empty, unfocused field shows the label slightly higher. This is recorded
rather than hidden; if a custom focus-tracking field becomes worthwhile, it is
a builder call.

## Acceptance

### Visual and behavioural parity

- On the desktop preview, each of these is compared side by side with the
  Flutter build at the same window size:
  - app long-press, folder long-press, create folder;
  - launcher menu, Add Shortcut, Command Shortcut;
  - group delete, restore and factory-reset confirmations;
  - Settings, Advanced, Groups, Choose Apps.
- Card corner radius, background, scrim, title size, field height, action
  order, button emphasis (text/filled/destructive) and icon choice match.
- Every control works against the real core: values persist, clamps show up,
  dialogs close on success, errors stay in the dialog, snackbars report
  backup/restore/reset results.
- A slider sends one request per gesture, not one per pixel.
- App Save renames and moves in that order; folder Save renames and moves the
  tile; app dialog shows Remove only for shortcuts and Uninstall only for
  installed apps.
- Back order is: dropdown, dialog, screen, overlay, search, folder.
- The launcher is not interactive behind a screen or dialog.
- Home press clears dialogs and screens and returns to the launcher.

### Accessibility

- TalkBack can reach every control on Settings, Advanced, Groups, Choose Apps
  and every dialog; labels, values, checked/selected state and enabled state are
  read correctly.
- A slider's scroll-forward/backward action changes the value by one step and
  announces the new value.
- Opening a dropdown exposes its options; choosing one updates the field;
  dismiss closes the menu.
- Text fields focus the real iced field and accept a whole-value set.
- No node is exposed for a layer that is not on top; bounds match the drawing
  within the existing tolerance.

### Build and regression

- `cargo test -p qlaunch_iced --no-default-features --lib` passes, including
  new layout, navigation, plan and accessibility tests.
- `cargo test -p qlaunch_core --no-default-features` still passes.
- `cargo fmt -p qlaunch_iced --check` passes.
- `cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps
  -- -D warnings` passes.
- The desktop preview runs with no errors and the existing grid, pager,
  folders, search and gestures are unchanged.
- On Android, the release build still compiles; on-device visual/IME/TalkBack
  comparison remains pending a connected device and is reported as such.

## Risks and mitigations

- **Custom switch/slider/checkbox look approximate.** Iced's toggler and slider
  are styled as closely as their geometry allows; if the M3 shape is
  unacceptable, replace that one component with a small custom widget. The
  theme API keeps the call sites unchanged.
- **Dropdown popup clipping or layering.** It is drawn in its own top-level
  layer at a computed rect, not inside the dialog's scrollable; test with the
  dialog near the window edges and with a long option list.
- **Text scaling extremes.** At `label_size` near 0 text becomes unreadable,
  matching Flutter's factor; at large sizes rows may need to grow. Row heights
  are computed from the scaled line height where a fixed Flutter height would
  overflow.
- **Font family availability.** Serif/mono names may not resolve on Android;
  fall back to the renderer default rather than failing to shape text.
- **Icon font not loaded.** Verify the family name `Material Icons` and the
  glyphs on both platforms; a missing font shows tofu, which the desktop
  comparison catches immediately.
- **Async mutation closes the wrong dialog.** Completions carry the dialog kind
  or an identity; a stale completion after the user pressed Back is ignored.
- **Snapshot event races.** Apply the returned snapshot immediately on success
  and let the `ModelChanged` event be idempotent.
- **Accessibility drift.** Layout is a pure function shared by view and tree;
  add a test per screen/dialog that every interactive node is inside the
  window and its row's rect.
- **Scope creep into launcher rendering.** The grid, pager, title strip, folder
  contents popup and search results keep their current layout; only their
  shared colours move to `theme`.
- **No device available.** Pixel-level parity, Android GPU text and TalkBack
  remain unverified until a device is connected; the design does not claim
  otherwise.

## Builder-check findings

Verified against the repository rather than assumed:

- All named core calls exist with the assumed signatures
  (`rust/qlaunch_core/src/api/api.rs`), and `SettingsPatch`/`Settings::clamp`
  cover every control and range in the design.
- `Id` is `String`; `GroupRole` is `Root | Apps | Games | Home | Hidden`;
  folders are found through `Item::folder_group_id`, and `ModelSnapshot` exposes
  `root_group_id`, `home_group_id`, `page_ids`, `groups`, `items`.
- Mutations emit `CoreEvent::ModelChanged`, and the existing subscription
  reloads the snapshot, so the UI can rely on events; applying the returned
  snapshot immediately avoids a flicker.
- `.font(bytes)` exists on the Iced application builder (`application.rs:245`)
  and is already used for platform fonts, so the icon font loads the same way.
- Iced's `text_input` has no `on_focus`/`on_blur`; the floating-label deviation
  is real and now recorded.
- Kotlin's `LauncherAccessibilityView.kt` maps exactly eight action strings
  (`click`, `long_click`, `scroll_forward`, `scroll_backward`, `scroll_left`,
  `scroll_right`, `set_text`, `dismiss`); the slider plan reuses them so no
  Kotlin change is required.
- On Linux, backup/list-backups are supported through the Downloads folder;
  `choose_wallpaper` and `uninstall_app` return an Android-only error, which the
  snackbar and inline dialog error already handle.
- The `opaque` layer does not block hit-testing, so the design now requires
  explicit press barriers and early returns on the launcher messages; without
  them a screen would leak presses to the grid.
- Renaming `overlay.rs` to `dialog.rs` will touch the overlay tests in
  `lib.rs`; keeping the module name is an acceptable alternative.

## Left to the builder

- Exact module split: whether screens live in `screens/settings.rs` etc. or one
  `screens.rs`, and whether `dialog.rs` keeps `overlay.rs`'s helper names.
- The precise component signatures, builder-style APIs and whether components
  take `&Settings` or a precomputed `TextScale`.
- Pixel tuning of the toggler, slider, checkbox and state layers against
  Flutter screenshots.
- Whether to implement a custom focus-tracking text field for the floating
  label, or keep the recorded deviation.
- Whether to add RangeInfo/`ACTION_SET_PROGRESS` for sliders (with a Kotlin
  change) or keep the scroll-action wording.
- Snackbar animation (Flutter slides/fades in); a static 4 s appearance is
  acceptable for the first pass.
- Restore picker scrolling and whether to show the file date/size.
- Test decomposition and the exact fake-snapshot helpers.
- The final Android build check and the device comparison report.
