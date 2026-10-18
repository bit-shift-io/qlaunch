mod error;
mod ids;
mod roles;

use std::collections::HashSet;

pub use error::ModelError;
pub use ids::Id;
pub use roles::GroupRole;

use ids::new_id;

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub id: Id,
    pub label: String,
    pub parent_id: Option<Id>,
    pub role: Option<GroupRole>,
    pub order: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppRef {
    pub package: String,
    pub activity: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtraKind {
    String,
    Bool,
    Int,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extra {
    pub key: String,
    pub kind: ExtraKind,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShortcutPayload {
    Android {
        action: Option<String>,
        data: Option<String>,
        mime_type: Option<String>,
        categories: Vec<String>,
        package: Option<String>,
        component: Option<String>,
        flags: i32,
        extras: Vec<Extra>,
    },
    Command {
        command: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppEntry {
    pub package: String,
    pub activity: String,
    pub label: String,
    pub category: AppCategory,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
        };
        let child = |label: &str, role: GroupRole, order: i32| Group {
            id: new_id(),
            label: label.into(),
            parent_id: Some(root.id.clone()),
            role: Some(role),
            order,
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

        let group_ids: HashSet<&str> = self.groups.iter().map(|g| g.id.as_str()).collect();
        for parent in self.groups.iter().filter_map(|g| g.parent_id.as_ref()) {
            if !group_ids.contains(parent.as_str()) {
                return Err(ModelError::UnknownGroup(parent.clone()));
            }
        }
        for item in &self.items {
            if !group_ids.contains(item.group_id.as_str()) {
                return Err(ModelError::UnknownGroup(item.group_id.clone()));
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
