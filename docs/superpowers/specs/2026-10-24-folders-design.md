# Folder tiles — design

Date: 2026-10-24
Status: approved in conversation (F1); F2 (multi-group pages) is out of scope.

## 1. Intent

The launcher has many groups and each group is a whole page. Groups with only
three to six apps waste a page, and finding the right group means scrolling
through many pages. A **folder** collapses a small set of items into a single
tile on a page, like the traditional Android launcher folder: tapping the tile
opens a small floating popup of its members.

This is deliberately *not* the same feature as multiple groups per page. That
second feature (F2, nesting groups on a page) is deferred and this design is
built so it does not get in F2's way.

## 2. What a folder is, in the user's words

- A folder behaves like a regular single app icon on a page.
- Clicking it opens a hovering popup with a small number of apps.
- It is **not a group** from the user's point of view, and folders never
  appear in the tab strip, the pager, or Edit Groups.
- Folders may hold **apps and shortcuts**.
- A folder has a **name** and its own icon, derived from a member.

## 3. Goals

- Create a folder from the empty-space long-press menu and give it a name and
  a containing group.
- Show it as one tile with a derived icon and its name.
- Open a popup listing its members; tap to launch. Long-press a member inside
  the popup opens that member's ordinary app popup.
- Move an app or shortcut into a folder from the app popup's existing Group
  dropdown, now shown as a tree (groups with their folders nested beneath).
- Move, rename and delete a folder from the same folder dialog.
- Keep member apps visible to search.
- Preserve the model's existing two-level invariant and persistence schema.

## 4. Non-goals

- Folders inside folders (no nesting). F2 may change this later.
- Multiple groups on one page (F2).
- Custom folder icons or folder colours.
- Drag-and-drop.
- Long-press gestures on the folder popup itself.
- Icon/colour pickers.

## 5. Anti-goals

- A folder feature that forces a schema version bump or breaks old saved
  layouts.
- Folders that leak into Edit Groups, the tab strip, or the pager.
- A folder that can strand its members with no way to remove them.
- Reworking F1 into F2 later (the data shape should carry over).

## 6. Constraints and existing architecture

- `Group.parent_id` is currently restricted: every non-root group must be a
  direct child of the root (`Model::validate`, `NotChildOfRoot`). We do **not**
  relax this in F1.
- `Model::pages()` = children of root that are not `hidden` and do not hold
  the `Hidden` role. The pager's `pageIds` come from this.
- The group editor lists every non-root group, including hidden ones, so users
  can unhide them. Folder groups must be excluded there.
- `Item` already holds either an app or a shortcut. `Item.group_id` is the
  only location rule.
- `Model::move_item` already moves any item between groups.
- Usage (`Usage.apps`, count + last-used) is already stored and exposed for
  Frequent/Recent. It is app-only; shortcuts have no usage.
- The snapshot gives Dart all groups, all items in display order, and
  `pageIds`. It is computed in Rust (`snapshot_of`).
- Mudations go through `state::mutate_model` → `commit_model`, which persists
  and emits `ModelChanged`.

## 7. Decisions

Each item is marked **User** (decided in conversation) or **Call** (a decision
made while writing this design, open to review).

### 7.1 Creation

- **User:** A new **Create Folder** entry is added to the *empty-space*
  long-press menu (`LauncherMenu`), not to the app popup.
- **User:** Choosing it opens the folder dialog (same dialog as editing a
  folder), with a name, the containing group, and Cancel/Save.
- **Call:** The dialog defaults the name to `Folder` and the group to the page
  the long-press happened on (the `groupId` already passed to `LauncherMenu`).
- **Call:** On create, the dialog shows Name, Group, Cancel, Save. Delete is
  only shown when editing an existing folder (nothing to delete yet).
- **Call:** Creating a folder is atomic: the folder group and its tile item are
  created together.

### 7.2 The folder dialog (create and edit)

- **User:** Long-pressing an existing folder opens the same dialog.
- **User:** It has Name, Group, Delete, Cancel, Save, just like the app popup.
- **User:** Save renames the folder and/or moves it to another group; Delete
  deletes it.
- **Call:** The Group dropdown lists ordinary, non-root destinations only
  (including the Apps/Games pages), and **excludes** the group holding the
  `Hidden` role. Reason: folders are excluded from Edit Groups, so a folder
  moved into the Hidden group would be unreachable. This can be revisited if
  folders ever become visible in Edit Groups.
- **Call:** Folder names need not be unique. An empty name cannot be saved, as
  with the app dialog.

### 7.3 Appearance

- **User:** No custom icon or colour.
- **User:** The folder icon is the **most-frequently-used member's** icon; if
  no member has usage data, the **first member's** icon.
- **User:** An **empty** folder is allowed and shows a **generic folder icon**.
- **User:** The folder displays its **name**.
- **Call:** "First member" means first in the folder's display order (the same
  order the grid shows). The tile item's label mirrors the folder name so the
  existing label renderer draws it without special-casing.
- **Call:** Because usage is app-only, a shortcut can win the icon only by the
  first-member fallback.
- **Call:** If the chosen icon member has no usable image (a shortcut without a
  stored icon, or an uninstalled app), the folder tile shows the **generic
  folder icon**. It does not borrow the member's placeholder glyph (lock/link/
  terminal), because the tile is a folder, not that item.

### 7.4 Opening a folder

- **User:** Tapping the folder opens a floating popup of its members.
- **User:** Tapping a member launches it.
- **User:** Long-pressing a member inside the popup opens that member's normal
  app popup (the same `AppSettingsDialog`, which already handles shortcuts).
- **User:** No other long-press behaviour inside the popup.
- **Call:** The popup is modal, dismisses on outside tap/back, and shows an
  empty message (or the generic folder icon with the name) when the folder has
  no members.
- **Call:** The app popup's title is removed. The Name field already shows the
  name, and the folder popup reuses the same dialog-free presentation.

### 7.5 Putting items into a folder

- **User:** The app popup's **Group** dropdown becomes a tree: ordinary groups,
  with each group's folders nested/indented beneath it. Selecting a folder
  moves the item into that folder.
- **User:** This is the only way to add an item to a folder. There is no drag
  and no separate "add to folder" field.
- **Call:** Choosing a regular group moves the item to that group as today;
  choosing a folder moves it to the folder's backing group. Both are the same
  `moveItem` call with a different destination group id.
- **Call:** Folders are not valid destinations for other folders (no nesting),
  so a folder tile never opens the app popup and never appears in that tree as
  a folder inside a folder.

### 7.6 Removing items and deleting folders

- **User:** Deleting a folder returns its icons to the folder's **parent
  group**.
- **User:** Empty folders are allowed; they are never auto-deleted.
- **User:** Deleting an empty folder is a manual action from the dialog.
- **Call:** "Parent group" is the group that contains the folder's tile item at
  the moment of deletion. Members are appended after that group's existing
  items, preserving their relative order within the folder.

### 7.7 Search

- **User:** Folder members should show in search.
- **Call:** Member apps are ordinary items in the snapshot, so the existing
  app search already finds them with no change. Search is app-only today;
  shortcut members are not searched, and this design does not change that.
  Folder tile items have no app and are naturally excluded.

## 8. Data model

Keep the two-level group invariant. A folder is represented by **two**
cooperating objects:

1. **A folder group** — a normal root-child `Group` that is never a page.
2. **A folder tile item** — an ordinary `Item` living in a real page/group,
   which points at the folder group and renders as the tile.

Membership is just `Item.group_id = folderGroupId`; movement is the existing
`move_item`. The folder's *containing* group is the tile item's `group_id`, not
a new parent relationship. This is the shape F2 can later generalise.

### 8.1 `Group`

Add:

```rust
#[serde(default)]
pub folder: bool,
```

- `folder = true` marks a folder group.
- Folder groups have `parent_id = Some(root)`, `role = None`,
  `hidden = false`. They are root children so the invariant is untouched.
- They are excluded from `pages()` and from Edit Groups by the flag, not by
  `hidden`.

### 8.2 `Item`

Add:

```rust
#[serde(default)]
pub folder_group_id: Option<Id>,
```

- Set only on a folder tile item. Such an item has `app = None` and
  `shortcut = None`.
- The tile's `group_id` is the folder's containing group, and its `label`
  mirrors the folder group's label.
- Existing items load with `None`.

### 8.3 Invariants (extend `Model::validate`)

- A folder group must:
  - have `parent_id == root`, `role == None`, `folder == true`;
  - have exactly one tile item whose `group_id` is a non-root, non-folder
    group and whose `folder_group_id == this group's id`.
- A tile item (`folder_group_id.is_some()`) must:
  - have `app == None` and `shortcut == None`;
  - point at an existing group with `folder == true`;
  - live in a non-root, non-folder group.
- No item inside a folder group may itself be a tile item (no nesting).
- A non-folder group must not have `folder = true` implied; `folder = false`.
- `is_page` becomes `parent == root && role != Hidden && !hidden && !folder`.

Add `ModelError` variants for the new failures (e.g. `FolderTileHasAppOrShortcut`,
`FolderInFolder`, `FolderTileInFolder`, `FolderGroupHasNoTile`,
`NotAFolderGroup`, `NotAFolderDestination`). Exact names are the builder's.

### 8.4 No schema bump

Follow the `hidden` precedent: `#[serde(default)]` on both new fields, keep the
store schema at 1. Old documents load with `folder = false` and
`folder_group_id = None`. New documents write both keys.

The store test that asserts the exact key sets
(`rust/qlaunch_core/tests/store.rs`, group and item key lists) must be updated.

## 9. Core operations

Add to `Model` (in `model/mutate.rs`), all routed like existing mutations:

- `add_folder(&mut self, containing_group: &Id, label: &str) -> Result<Id, ModelError>`
  - Creates the folder group (root child, `folder = true`), creates the tile
    item in `containing_group` (fallback to the Apps group if the containing
    group is root/unknown/folder, matching `add_item_to`), sets the tile's
    `folder_group_id` and label. Returns the folder group id.
  - `containing_group` must not be a folder or the root.
- `delete_folder(&mut self, folder_group_id: &Id) -> Result<(), ModelError>`
  - Resolve the tile item; `parent = tile.group_id`.
  - Move every member item of the folder group to `parent`, appended in order.
  - Remove the tile item and the folder group; renumber items.
  - Refuse a non-folder group.
- `rename_group` already exists; when the group is a folder, also update the
  tile item's label so the tile renders the new name.
- `move_item`: when the item is a folder tile, reject a destination that is a
  folder group or the root (`FolderInFolder` / `ItemInRoot`). Moving a
  non-folder item into a folder group is allowed (that is membership).
- `delete_group` must refuse a folder group (use `delete_folder` instead), so
  a mistaken call cannot silently move its members to Apps.
- `add_group` unaffected. `set_group_hidden` on a folder group returns
  `NotAPage` (folders are not pages), which is acceptable.

## 10. Snapshot and bridge

### 10.1 Snapshot

`snapshot_of` (Rust) gains the resolved folder icon member:

```rust
pub struct FolderIcon {
    pub folder_group_id: Id,
    pub item_id: Option<Id>, // None => empty folder
}
pub struct ModelSnapshot {
    ...
    pub folder_icons: Vec<FolderIcon>,
}
```

Resolution rule:

1. Among the folder group's member items, choose the one that refers to the
   app with the highest `Usage` count.
2. Ties: most recent `last_used_ms`, then display order.
3. If no member has usage data, choose the first member in display order
   (which may be a shortcut).
4. If the folder is empty, `item_id = None`.

`page_ids` stays `pages()`, so folder groups are absent. `groups` still
contains folder groups so Dart can read labels and flags. `items` still contains
folder tiles and folder members.

### 10.2 FFI surface

Add to `api/api.rs`, mirroring `add_group`/`delete_group`:

- `add_folder(containing_group: Id, label: String) -> Result<ModelSnapshot, String>`
- `delete_folder(folder_group_id: Id) -> Result<ModelSnapshot, String>`

Use `state::mutate_model`. Regenerate the bridge:

```
flutter_rust_bridge_codegen generate
```

This updates `lib/src/rust/**` (`model.dart`, `api/api.dart`,
`frb_generated.*`) and the Rust generated file. Do not hand-edit generated
files.

## 11. Flutter changes

### 11.1 `LauncherState` / `LauncherBackend`

- Add `addFolder(String containingGroupId, String label)` and
  `deleteFolder(String folderGroupId)`.
- Add convenience getters used by the UI:
  - `Group? folderGroup(String id)` (or reuse `groups` lookup).
  - `List<Group> foldersIn(String groupId)`: folder groups whose tile item's
    `group_id` equals `groupId`.
  - `Item? folderTile(String folderGroupId)`: the tile item.
  - `List<Item> folderMembers(String folderGroupId)` =
    `itemsIn(folderGroupId)`.
  - `Id? folderIconItem(String folderGroupId)` from `snapshot.folderIcons`.
- Update `FakeBackend` / snapshot builders used by tests.

### 11.2 `LauncherMenu`

- Add a **Create Folder** `ListTile` and action alongside Add Shortcut / Edit
  Groups / Choose Wallpaper.
- On tap, open `FolderDialog` in create mode with `groupId` (the current page)
  as the default containing group.

### 11.3 New `FolderDialog` (`lib/src/ui/folder_dialog.dart`)

Model it on `AppSettingsDialog` (including `LauncherFontScope`):

- Name `TextField`, required, default `Folder` on create.
- Group `DropdownButtonFormField` listing ordinary, non-root, non-Hidden-role
  destinations.
- Create: Cancel / Save.
- Edit: Delete / Cancel / Save.
- Save (create): `addFolder`.
- Save (edit): rename the folder group, and if the group changed,
  `moveItem(tile.id, destination, end)`.
- Delete: `deleteFolder`.
- Use `ValueKey`s for tests, in the same style as `AppSettingsDialog`.

### 11.4 Grid rendering (`AppCells` / `AppCell`)

A folder tile is an `Item` with `folderGroupId != null`. It needs the derived
icon and the folder name (already mirrored into `item.label`).

- In `AppCells.buildTile`, resolve the icon for a folder tile from
  `state.folderIconItem(item.folderGroupId)`:
  - If it points at an app: use that app's icon (existing `cachedIcon` /
    `iconFor` path).
  - If it points at a shortcut with `iconOverride`: use `cachedItemIcon` /
    `itemIcon`.
  - If it points at a member with **no usable image** (shortcut without a
    stored icon, uninstalled app), or `None` (empty folder): draw the generic
    folder placeholder icon.
- The tap handler opens the folder popup instead of launching.
- The long-press handler opens `FolderDialog` instead of `AppSettingsDialog`.
- `AppCell` itself can stay mostly unchanged; prefer passing a resolved icon
  provider and keeping label handling as-is. A small `isFolder` flag for the
  generic placeholder is acceptable.
- The folder tile's raster key uses `item.label` (folder name) and the resolved
  provider, so it invalidates when the chosen member or its icon changes.

### 11.5 New folder popup (`lib/src/ui/folder_popup.dart`)

- Modal dialog showing the folder name and a grid of members.
- Members are `state.itemsIn(folderGroupId)`; reuse `AppCell`/`AppCells` (an
  `embedded` row mutates layout, so a compact `GridView`/`Wrap` of `AppCell`
  may be simpler; builder's call).
- Tap launches (same launch logic as `AppCells._launch`).
- Long-press opens `AppSettingsDialog` for that member.
- Empty state: folder name plus a short "Empty folder" message.
- No other long-press.

### 11.6 `AppSettingsDialog`

- Remove the `title` (the Name field already shows the name).
- Replace the flat Group dropdown items with a tree:
  - For each ordinary destination group, a group entry.
  - Beneath it, that group's folders, visually indented and with a folder
    leading icon.
  - The selected value is the group id (ordinary) or the folder group id
    (folder). Save still calls `moveItem(item.id, destination, end)`.
- The dropdown may need to render both groups and folders; use a flat list of
  `DropdownMenuItem`s with indentation/presentation rather than a true nested
  widget, to keep it simple.
- If the item is already in a folder, the dropdown's initial value is that
  folder group id.

### 11.7 Group editor

- Filter out `group.folder` groups everywhere it lists groups (they are not
  pages and must not be renamed/moved/hidden/deleted there).
- Deleting a page still moves its items (including folder tiles) to Apps; the
  folders follow their tiles and remain valid.

## 12. Icons, warming and raster cache

- `HomeScreen._warmIcons` currently walks `pageIds` only. Folder members are
  not pages, so add the resolved folder icon member for every folder tile on a
  warmed page to the warm queue. (Do not warm every member, only the icon the
  tile actually shows; the popup will warm its own members when opened, or the
  builder may choose to warm members of folders on the current page.)
- `_syncOwners` already includes all snapshot items and groups as raster
  owners, so folder tiles/members are not purged. Confirm folder groups are in
  `snapshot.groups` (they are).
- Because folder groups are **not** `hidden`, they are not caught by whatever
  path the user observed for hidden groups' icons; verify on device that the
  folder tile icon caches. If a hidden-check exists downstream keyed on
  `hidden`, it must key on `folder` semantics instead.
- Usage-driven icon changes: after a launch, usage changes and the snapshot is
  refreshed (confirm the model-change path covers usage updates; if not, the
  folder icon may lag until the next refresh — acceptable but note it).

## 13. Edge cases

- Delete a folder whose tile was moved: members return to the tile's *current*
  group.
- Delete a folder with no members: tile and group removed.
- Remove the last member from a folder: folder stays, generic icon.
- A folder tile in a hidden page: stays valid; its members are not warmed while
  the page is hidden.
- Deleting a page containing a folder: the folder tile moves to Apps, members
  stay in the folder.
- Moving a folder into the Hidden role group is prevented by the dialog's
  destination filter. The model op should also reject a folder destination that
  holds the Hidden role (call).
- **Hidden-role reassignment cannot strand folders in this build:** the group
  editor only offers `Home`, `Apps`, `Games` and `None` as roles
  (`group_editor_screen.dart`), and the app dialog's Group list is
  `stop`-free. `set_group_role` in Rust can technically move `Hidden`, but no
  UI path does, so validation does not need to reject it. If a future build
  exposes the Hidden role, revisit: either move folder tiles to Apps when a
  group becomes Hidden-role, or include folder groups in Edit Groups.
- The group-apps screen (`group_apps_screen.dart`) only lists items with an
  `app`, so folder tiles are not managed there. This is consistent with F1
  having no separate "add apps to folder" checklist; membership is via the app
  popup's Group tree.
- Moving an app into a folder, then deleting the folder: the app returns to the
  folder tile's group, not its original group.
- An app can still exist only once: folder membership moves the existing item;
  it does not duplicate it.
- Search finds a folder member app; launching from search works as today and
  does not need to open the folder.

## 14. Persistence and compatibility

- Schema stays at 1.
- `Group.folder` and `Item.folder_group_id` are serde-defaulted.
- Old backups and configs load unchanged.
- New backups contain the two fields; older builds would reject unknown keys
  only if they deny unknown fields (they do not) — still, do not test downgrade.
- Update the exact-key store test and any snapshot fixtures.

## 15. Testing

Rust (`rust/qlaunch_core/tests`):

- `add_folder` creates a folder group + tile with consistent labels and
  `folder_group_id`; defaults to Apps when the containing group is invalid.
- `delete_folder` returns members to the tile's current group in order and
  removes the tile and group.
- Moving a folder tile to another group updates its containing group; moving it
  into a folder/root is refused.
- Moving an app into and out of a folder via `move_item`.
- Validation rejects: folder with no tile, tile with app/shortcut, tile in a
  folder, tile pointing at a non-folder, folder with a role.
- Renaming a folder renames the tile label.
- Persistence round-trip and serde defaults for both new fields.
- Folder icon resolution: most-used, tie by recency, fallback first member,
  empty => None.

Dart widget tests:

- Launcher menu shows Create Folder; it opens the folder dialog with the
  current page preselected.
- Create dialog: Save creates; the folder tile appears with the name and the
  chosen icon.
- Edit dialog: rename, move, delete (members return to the parent).
- Folder popup: shows members, tap launches, long-press opens app popup, empty
  state.
- App popup: has no title; the Group dropdown shows groups with folders nested
  and moving an item into a folder works.
- Folder tile uses generic icon when empty.
- Group editor does not list folder groups.
- Search finds an app that lives in a folder.
- Existing app popup tests updated for the removed title and the new dropdown.

## 16. Left to the builder

- Exact `ModelError` variant names and messages.
- Whether `add_folder` falls back to Apps when the containing group is invalid
  (recommended, matching `add_item_to`).
- Popup presentation details (Dialog vs bottom sheet, grid sizing, scroll).
- Whether to warm all members of on-page folders or only the displayed icon.
- Whether the app-popup dropdown is a flat indented list or a true grouped
  widget.
- Test-file organisation and fixture updates.

## 17. Deliberately deferred (F2)

Multiple groups per page / general nesting. The folder group + tile item shape
is chosen so F2 can generalise `parent_id` and the pager without discarding this
work.
