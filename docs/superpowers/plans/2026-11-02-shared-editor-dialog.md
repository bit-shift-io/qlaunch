# Shared Editor Dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the separate app/shortcut and folder editor dialogs with one shared `Dialog::Editor`, and remove the installed app's package-name row.

**Architecture:** `Editor` owns the common name/group/busy/error state, while `EditorKind` carries the app item or folder identity. All rows, layout, rendering, actions and save dispatch go through the shared editor; existing save plans and group-option rules remain intact.

**Tech Stack:** Rust, iced 0.14, qlaunch_core model types.

**Spec:** `docs/superpowers/specs/2026-11-02-shared-editor-dialog-design.md`

## Global Constraints

- Do not change core save/uninstall/remove/delete behaviour.
- App and shortcut group options keep folders; folder group options exclude folders and hidden groups.
- Destructive actions remain left-aligned trash icons.
- No package-name row in the editor.
- Leave changes uncommitted; the user has not asked for a commit.

## Review Focus

- An installed app editor must not show `com.<package>` or an `Uninstalled` row.
- A folder-create editor must not expose a Delete action.
- A shortcut editor must expose Remove, while an app-backed item may expose both Uninstall and Remove.
- Group changes must still use the correct option list for app versus folder editors.
- Accessibility must still map Save/Uninstall/Remove/Delete to their existing messages.

---

### Task 1: Introduce the shared editor model and rendering

**Files:**
- Modify: `rust/qlaunch/src/dialog.rs`
- Modify: `rust/qlaunch/src/lib.rs`
- Modify: `rust/qlaunch/src/accessibility.rs`
- Test: `rust/qlaunch/src/dialog.rs` (unit tests in the same module)

**Interfaces:**
- Consumes: existing `Item`, `Id`, `ModelSnapshot`, `app_save_plan`, and folder change logic.
- Produces:
  - `Dialog::Editor(Editor)`
  - `Editor { pub kind: EditorKind, pub name: String, pub group_id: Id, pub busy: bool, pub error: Option<String> }`
  - `EditorKind::{App { item: Item }, Folder { folder: Option<Id>, label: String }}`
  - `Editor::{app, folder_create, folder_edit, item, can_uninstall, is_shortcut, is_folder_edit, folder_id, can_save, select_field}`
  - `Message::{EditorName, EditorSave}` replacing `AppName`, `AppSave`, `FolderName`, `FolderSave`
  - `editor_save_plan(editor: &Editor, snapshot: &ModelSnapshot) -> Vec<Change>`
  - `folder_save_plan(folder: Option<&Id>, label: &str, name: &str, group_id: &Id, snapshot: &ModelSnapshot) -> Vec<Change>`

- [ ] **Step 1: Write the failing shared-editor tests**

In `rust/qlaunch/src/dialog.rs` tests, replace app/folder constructors with the shared editor and add:

```rust
#[test]
fn editor_select_field_follows_the_kind() {
    let (app, item) = dialog_app();
    let app_editor = Editor::app(item);
    assert!(matches!(app_editor.select_field(), SelectField::AppGroup));
    let group = app.snapshot.as_ref().unwrap().page_ids[0].clone();
    let folder_editor = Editor::folder_create(group);
    assert!(matches!(folder_editor.select_field(), SelectField::FolderGroup));
}

#[test]
fn editor_rows_show_name_group_and_error_but_no_package() {
    let (app, item) = dialog_app();
    let package = item.app.as_ref().unwrap().package.clone();
    let dialog = Dialog::Editor(Editor::app(item));
    let rows = rows(&app, &dialog);
    assert!(rows.iter().any(|row| matches!(row, Row::Field { .. })));
    assert!(rows.iter().any(|row| matches!(row, Row::Select { .. })));
    assert!(!rows.iter().any(|row| matches!(row, Row::Text { label, .. } if *label == package)));
    assert!(!rows.iter().any(|row| matches!(row, Row::Text { label, .. } if label == "Uninstalled")));
}

#[test]
fn folder_create_has_no_delete_action() {
    let (app, _) = dialog_app();
    let group = app.snapshot.as_ref().unwrap().page_ids[0].clone();
    let dialog = Dialog::Editor(Editor::folder_create(group));
    assert_eq!(action_labels(&dialog), vec!["Cancel", "Save"]);
}
```

Update existing app/folder tests to use `Dialog::Editor(Editor::app(...))` and `Dialog::Editor(Editor::folder_edit(...))`. Update the folder save-plan test to call:

```rust
folder_save_plan(None, "Folder", "Folder", &snapshot.page_ids[1], snapshot)
```

- [ ] **Step 2: Run the dialog tests to verify they fail**

Run: `cargo test -p qlaunch dialog::tests`

Expected: compile failure because `Editor`, `EditorKind`, `Editor::app`, `Editor::folder_create`, `Editor::folder_edit`, or the new `folder_save_plan` signature does not exist.

- [ ] **Step 3: Implement `Editor` and collapse the dialog rendering**

In `rust/qlaunch/src/dialog.rs`:

- Remove `AppEditor` and `FolderEditor`.
- Add `Editor`/`EditorKind` with the constructors and helpers from the Interfaces block.
- Replace `Dialog::App` and `Dialog::Folder` with `Dialog::Editor`.
- `heading`: `None` for every editor; `accessible_title` names the kind for screen readers.
- `can_save`: `!busy && !name.trim().is_empty() && !group_id.is_empty()`.
- `rows`: name field, group select, error only.
- `is_icon_action`: true for `Uninstall` when `can_uninstall`, `Remove` when `is_shortcut`, `Delete` when `is_folder_edit`.
- `action_labels`: destructive labels as above, then `Cancel`, `Save`.
- `body`: shared name field using `EditorName`/`EditorSave`, shared group select, optional error. No package or uninstalled row.
- `actions`: trash icons for the destructive actions, then Cancel/Save.
- Replace `folder_save_plan` with the signature above; add `editor_save_plan` dispatching to `app_save_plan` or `folder_save_plan`.
- Remove `Message::{AppName, AppSave, FolderName, FolderSave}`; add `EditorName` and `EditorSave`.

- [ ] **Step 4: Update the update handlers and accessibility mapping**

In `rust/qlaunch/src/lib.rs`:

- `dialog_error`: match `Dialog::Editor(editor)` and clear `busy`, set `error`.
- Busy check: `Some(Dialog::Editor(editor)) => editor.busy`.
- `OpenGroupSelect`: `Some(Dialog::Editor(editor)) => editor.select_field()`.
- `GroupChosen` and `EditorName`: write `editor.group_id` / `editor.name`.
- `EditorSave`: call `dialog::editor_save_plan(editor, snapshot)`.
- `AppUninstall`: get the item via `editor.item()`.
- `AppRemove`: get the item id via `editor.item()`.
- `FolderDelete`: get `editor.folder_id()`.
- Dialog creation:
  - app/shortcut: `Dialog::Editor(Editor::app(item))`
  - folder edit: `Dialog::Editor(Editor::folder_edit(folder, label, group))`
  - folder create: `Dialog::Editor(Editor::folder_create(group))`
- `open_select`: get the selected group from `Dialog::Editor`.
- Update test matches from `Dialog::App`/`Dialog::Folder` to `Dialog::Editor`.

In `rust/qlaunch/src/accessibility.rs`, map:

```rust
(Dialog::Editor(e), "Save") => (dialog::Message::EditorSave, e.can_save()),
(Dialog::Editor(e), "Uninstall") => (dialog::Message::AppUninstall, !e.busy),
(Dialog::Editor(e), "Remove") => (dialog::Message::AppRemove, !e.busy),
(Dialog::Editor(e), "Delete") => (dialog::Message::FolderDelete, !e.busy),
```

- [ ] **Step 5: Run the full test suite**

Run: `cargo test -p qlaunch`

Expected: all tests pass, 0 failures.

- [ ] **Step 6: Check formatting**

Run: `cargo fmt --all -- --check`

Expected: no new formatting diffs in the files touched by this task. The pre-existing `dashboard.rs` diff is unrelated.

- [ ] **Step 7: Leave the changes uncommitted for review**

Run: `git status --short`

Expected: only the intended source and design/plan files are modified.

---

### Task 2: Verify the shared editor on the Pixel 8

**Files:** none.

- [ ] **Step 1: Build the release APK**

Run:

```bash
JAVA_HOME=/opt/jbr-21 bash scripts/build.sh release
```

Expected: `BUILD SUCCESSFUL`.

- [ ] **Step 2: Install and launch**

Run:

```bash
adb install -r build/app/outputs/apk/release/app-release.apk
adb shell am start -W -n bitshift.qlaunch/.MainActivity
```

Expected: install success and the launcher becomes the focused window.

- [ ] **Step 3: Check the installed-app editor**

Long-press an installed app icon and capture a screenshot. Confirm:

- Name and Group only; no package name row.
- Trash icon on the left.
- Cancel and Save on the right in one row.

- [ ] **Step 4: Check the folder editor**

Long-press a folder tile. Confirm the same layout, with the trash icon performing folder delete and no package row.

- [ ] **Step 5: Check the shortcut editor**

Open a shortcut's editor. Confirm Remove is a left-aligned trash icon and the rest of the layout matches the app editor.

- [ ] **Step 6: Check the group dropdown**

Open the Group dropdown and scroll. Confirm it still lists the expected groups and keeps the slim scrollbar.
