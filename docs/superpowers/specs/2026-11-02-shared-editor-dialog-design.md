# Shared editor dialog design

## Intent

The installed-app, shortcut and folder editors are the same dialog with
different headings and destructive actions. Collapse them into one editor
model so their rows, layout, actions and focus/accessibility behaviour have a
single implementation.

## Goals

- One `Dialog::Editor(Editor)` variant for all three flows.
- One `rows`, `body`, `action_labels`, `actions` and `layout` path.
- App, shortcut and folder differences live in an `EditorKind`.
- Remove the installed app's package-name row; it carries no useful
  information for editing.
- Keep the current visual result: name field, group dropdown, trash action on
  the left when present, Cancel/Save on the right.

## Non-goals

- Changing save/uninstall/remove/delete behaviour in the core.
- Changing group choices: apps and shortcuts can move into folders; folders
  move between page groups only.
- Changing the launcher menu, confirmation dialog or group editor.

## Shape

```rust
enum Dialog {
    Editor(Editor),
    // ...
}

struct Editor {
    kind: EditorKind,
    name: String,
    group_id: Id,
    busy: bool,
    error: Option<String>,
}

enum EditorKind {
    App { item: Item },          // installed app or shortcut
    Folder { folder: Option<Id>, label: String }, // None = create
}
```

- A shortcut is `EditorKind::App` with `item.shortcut.is_some()`; it does not
  need a separate kind because only its destructive action differs.
- Folder create uses `folder: None`; folder edit stores the folder id and its
  original label for rename comparison and the heading.
- No editor dialog draws a heading. Screen readers use `Edit app`,
  `Edit shortcut`, `New folder` or `Edit folder` as the accessible title.
- `can_save` requires a non-empty trimmed name and a group id.
- The shared name/save messages become `EditorName` and `EditorSave`.
  Destructive messages stay specific (`AppUninstall`, `AppRemove`,
  `FolderDelete`) because each calls a different core operation.

## Rendering differences

- Rows: name field, group select, optional error. No package row.
- Actions: `Uninstall` when `item.app` exists and is not uninstalled;
  `Remove` when `item.shortcut` exists; `Delete` for folder edit; then
  `Cancel` and `Save`.
- Destructive actions use the left-aligned trash icon already introduced.
- Group select uses `SelectField::AppGroup` for `EditorKind::App` and
  `SelectField::FolderGroup` for `EditorKind::Folder`.

## Code changes

- `dialog.rs`: replace `AppEditor`/`FolderEditor` and the two `Dialog`
  variants with `Editor`/`EditorKind`; collapse their row, body, action and
  layout matches.
- `lib.rs`: update dialog creation, busy/error helpers, name/save/group
  handlers, select open/choose handling, and tests.
- `accessibility.rs`: map shared editor actions to the existing messages.
- Keep `app_save_plan` and `folder_save_plan`; dispatch from the shared save
  handler by `EditorKind`.

## Verification

- Update the existing dialog tests for the shared editor.
- Keep regression coverage for:
  - one-row actions at the Pixel 8 logical width,
  - left-aligned trash icon for app, shortcut and folder edits,
  - folder create has no delete action,
  - app save and folder save plans still produce the same changes,
  - the package row is absent.
- Run `cargo test -p qlaunch`.
- Build the release APK and check the app, shortcut and folder dialogs on the
  Pixel 8.
