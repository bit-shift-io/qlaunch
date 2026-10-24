mod error;
pub mod icon;
mod ids;
pub(crate) mod mutate;
pub mod roles;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub use error::ModelError;
pub use icon::{IconFormat, IconImage};
pub use ids::Id;
pub use roles::GroupRole;

use ids::new_id;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: Id,
    pub label: String,
    pub parent_id: Option<Id>,
    pub role: Option<GroupRole>,
    pub order: i32,
    /// Whether the group is hidden from the pager and the tab strip. Ordinary
    /// groups can be hidden and shown again from the group editor; the group
    /// holding the `Hidden` *role* is never a page regardless of this, and the
    /// root can never be hidden. Defaulted so a document written before this
    /// field still loads (the schema stays at 1).
    #[serde(default)]
    pub hidden: bool,
    /// Whether this group exists only to hold a folder's members. Folder
    /// groups are root children but are never pages, never appear in the group
    /// editor, and cannot hold a role. A tile item in a real group points at
    /// one through `Item::folder_group_id`. Defaulted so a document written
    /// before this field still loads (the schema stays at 1).
    #[serde(default)]
    pub folder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppRef {
    pub package: String,
    pub activity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtraKind {
    String,
    Bool,
    Int,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extra {
    pub key: String,
    pub kind: ExtraKind,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShortcutPayload {
    LockScreen,
    Notifications,
    Android {
        action: Option<String>,
        data: Option<String>,
        mime_type: Option<String>,
        #[serde(default)]
        categories: Vec<String>,
        package: Option<String>,
        component: Option<String>,
        #[serde(default)]
        flags: i32,
        #[serde(default)]
        extras: Vec<Extra>,
    },
    Command {
        command: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: Id,
    pub group_id: Id,
    pub label: String,
    pub label_overridden: bool,
    pub app: Option<AppRef>,
    pub shortcut: Option<ShortcutPayload>,
    pub uninstalled: bool,
    pub order: i32,
    pub icon_override: Option<String>,
    /// Set only on a folder tile item: the folder group whose members this tile
    /// represents. Such an item has neither an app nor a shortcut and lives in
    /// a real group (never in a folder). Defaulted so a document written before
    /// this field still loads (the schema stays at 1).
    #[serde(default)]
    pub folder_group_id: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppEntry {
    pub package: String,
    pub activity: String,
    pub label: String,
    pub category: AppCategory,
    /// When the platform last updated this app, or `0` when it does not say.
    /// It is not persisted; it only distinguishes one version's icon from the
    /// next in the icon cache.
    #[serde(default)]
    pub last_update_time: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    Game,
    App,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub root_group_id: Id,
    pub groups: Vec<Group>,
    pub items: Vec<Item>,
}

impl Model {
    pub fn new_default() -> Model {
        let root = Group {
            id: new_id(),
            label: "_root_".into(),
            parent_id: None,
            role: Some(GroupRole::Root),
            order: 0,
            hidden: false,
            folder: false,
        };
        let child = |label: &str, role: GroupRole, order: i32| Group {
            id: new_id(),
            label: label.into(),
            parent_id: Some(root.id.clone()),
            role: Some(role),
            order,
            hidden: false,
            folder: false,
        };
        let children = [
            child("Apps", GroupRole::Apps, 0),
            child("Home", GroupRole::Home, 1),
            child("Games", GroupRole::Games, 2),
            child("Hidden", GroupRole::Hidden, 999),
        ];
        let root_group_id = root.id.clone();
        let mut groups = vec![root];
        groups.extend(children);
        Model {
            root_group_id,
            groups,
            items: Vec::new(),
        }
    }

    pub fn role_holder(&self, role: GroupRole) -> Option<&Group> {
        self.groups.iter().find(|g| g.role == Some(role))
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        let root_holds_role = self
            .groups
            .iter()
            .any(|g| g.id == self.root_group_id && g.role == Some(GroupRole::Root));
        if !root_holds_role {
            return Err(ModelError::MissingRoot);
        }

        let mut seen_groups: HashSet<&str> = HashSet::new();
        for group in &self.groups {
            if !seen_groups.insert(group.id.as_str()) {
                return Err(ModelError::DuplicateGroupId(group.id.clone()));
            }
        }
        let mut seen_items: HashSet<&str> = HashSet::new();
        for item in &self.items {
            if !seen_items.insert(item.id.as_str()) {
                return Err(ModelError::DuplicateItemId(item.id.clone()));
            }
        }

        for role in [
            GroupRole::Root,
            GroupRole::Hidden,
            GroupRole::Apps,
            GroupRole::Games,
            GroupRole::Home,
        ] {
            let held = self.groups.iter().filter(|g| g.role == Some(role)).count();
            if held > 1 {
                return Err(ModelError::DuplicateRole(role));
            }
            if held == 0 && role != GroupRole::Home {
                return Err(ModelError::MissingRole(role));
            }
        }

        for group in &self.groups {
            match (&group.parent_id, group.id == self.root_group_id) {
                (Some(_), true) => return Err(ModelError::RootHasParent(group.id.clone())),
                (None, true) => {}
                (None, false) => return Err(ModelError::SecondRoot(group.id.clone())),
                (Some(parent), false) => {
                    if !seen_groups.contains(parent.as_str()) {
                        return Err(ModelError::UnknownGroup(parent.clone()));
                    }
                    if *parent != self.root_group_id {
                        return Err(ModelError::NotChildOfRoot(group.id.clone()));
                    }
                }
            }
        }
        for group in &self.groups {
            if group.id == self.root_group_id && group.folder {
                return Err(ModelError::RootIsFolder(group.id.clone()));
            }
            if !group.folder {
                continue;
            }
            if group.role.is_some() {
                return Err(ModelError::FolderHoldsRole(group.id.clone()));
            }
            let tiles: Vec<&Item> = self
                .items
                .iter()
                .filter(|i| i.folder_group_id.as_deref() == Some(group.id.as_str()))
                .collect();
            match tiles.len() {
                0 => return Err(ModelError::FolderGroupHasNoTile(group.id.clone())),
                1 => {}
                _ => return Err(ModelError::FolderGroupHasManyTiles(group.id.clone())),
            }
            let tile = tiles[0];
            if tile.app.is_some() || tile.shortcut.is_some() {
                return Err(ModelError::FolderTileHasContent(tile.id.clone()));
            }
            if tile.group_id == self.root_group_id {
                return Err(ModelError::ItemInRoot(tile.id.clone()));
            }
            match self.groups.iter().find(|g| g.id == tile.group_id) {
                None => return Err(ModelError::UnknownGroup(tile.group_id.clone())),
                Some(container) if container.folder => {
                    return Err(ModelError::FolderTileInFolder(tile.id.clone()));
                }
                Some(_) => {}
            }
        }
        for item in &self.items {
            if item.group_id == self.root_group_id {
                return Err(ModelError::ItemInRoot(item.id.clone()));
            }
            if !seen_groups.contains(item.group_id.as_str()) {
                return Err(ModelError::UnknownGroup(item.group_id.clone()));
            }
            if item.app.is_some() && item.shortcut.is_some() {
                return Err(ModelError::ItemHasAppAndShortcut(item.id.clone()));
            }
            if let Some(folder_id) = &item.folder_group_id {
                match self.groups.iter().find(|g| g.id == *folder_id) {
                    Some(g) if g.folder => {}
                    _ => return Err(ModelError::NotAFolderGroup(folder_id.clone())),
                }
                if item.app.is_some() || item.shortcut.is_some() {
                    return Err(ModelError::FolderTileHasContent(item.id.clone()));
                }
            }
        }

        let mut seen: HashSet<(&str, &str)> = HashSet::new();
        for app in self.items.iter().filter_map(|i| i.app.as_ref()) {
            if !seen.insert((app.package.as_str(), app.activity.as_str())) {
                return Err(ModelError::DuplicateAppRef(app.clone()));
            }
        }

        Ok(())
    }
}
