use std::cmp::Ordering;
use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use super::{AppRef, Group, GroupRole, Id, Item, Model, ModelError, new_id};

// `order` is dense and explicit: after any insertion, removal, move or sort the
// affected list (the pages, or the items of one group) is rewritten to `0..n` in
// list order. Ties and gaps in loaded data are resolved by position in the
// backing `Vec`, so the resulting order is deterministic.
impl Model {
    /// The visible top-level groups: children of the root that are not the
    /// `Hidden` group, by `order`.
    pub fn pages(&self) -> Vec<&Group> {
        let mut pages: Vec<&Group> = self.groups.iter().filter(|g| self.is_page(g)).collect();
        pages.sort_by_key(|g| g.order);
        pages
    }

    /// The items of `group`, by `order`.
    pub fn items_in(&self, group: &Id) -> Vec<&Item> {
        let mut items: Vec<&Item> = self.items.iter().filter(|i| i.group_id == *group).collect();
        items.sort_by_key(|i| i.order);
        items
    }

    pub fn item(&self, id: &Id) -> Option<&Item> {
        self.items.iter().find(|i| i.id == *id)
    }

    /// The id of the group holding the `Home` role, if there is one.
    pub fn home_group_id(&self) -> Option<Id> {
        self.role_holder(GroupRole::Home).map(|g| g.id.clone())
    }

    /// Adds an ordinary group (no role) as the last page.
    pub fn add_group(&mut self, label: &str) -> Id {
        let id = new_id();
        let order = self.pages().last().map_or(0, |g| g.order.saturating_add(1));
        self.groups.push(Group {
            id: id.clone(),
            label: label.into(),
            parent_id: Some(self.root_group_id.clone()),
            role: None,
            order,
        });
        self.renumber_pages();
        id
    }

    pub fn rename_group(&mut self, id: &Id, label: &str) -> Result<(), ModelError> {
        let group = self
            .groups
            .iter_mut()
            .find(|g| g.id == *id)
            .ok_or_else(|| ModelError::UnknownGroup(id.clone()))?;
        group.label = label.into();
        Ok(())
    }

    /// Moves the page `id` so that it ends up at position `new_order` (a
    /// zero-based index) in the list of pages, shifting the pages in between.
    /// An index below zero is treated as zero and one past the end as the end,
    /// so every index is valid. Afterwards the pages are numbered `0..n`.
    ///
    /// The root and the `Hidden` group are not pages: `NotAPage`.
    pub fn move_group(&mut self, id: &Id, new_order: i32) -> Result<(), ModelError> {
        let group = self.group(id)?;
        if !self.is_page(group) {
            return Err(ModelError::NotAPage(id.clone()));
        }
        let mut ids = self.page_ids();
        ids.retain(|p| p != id);
        ids.insert(clamp_index(new_order, ids.len()), id.clone());
        self.set_page_orders(&ids);
        Ok(())
    }

    /// Gives `role` to the group `id`, or with `None` takes its role away.
    ///
    /// A role has at most one holder, so assigning a role moves it from its
    /// previous holder, who is left with no role. `Root` can neither be assigned
    /// nor changed, and a group holding `Hidden`, `Apps` or `Games` cannot give
    /// the role up except by another group taking it over, so the model keeps
    /// validating. All of these are `RoleNotAssignable`. `Home` is optional and
    /// can be unset.
    pub fn set_group_role(&mut self, id: &Id, role: Option<GroupRole>) -> Result<(), ModelError> {
        let current = self.group(id)?.role;
        if current == role {
            return Ok(());
        }
        let required = |r: Option<GroupRole>| {
            matches!(
                r,
                Some(GroupRole::Root | GroupRole::Hidden | GroupRole::Apps | GroupRole::Games)
            )
        };
        if required(current) || role == Some(GroupRole::Root) {
            return Err(ModelError::RoleNotAssignable);
        }

        if let Some(role) = role {
            let displaced_order = self.pages().last().map_or(0, |g| g.order.saturating_add(1));
            if let Some(previous) = self.groups.iter_mut().find(|g| g.role == Some(role)) {
                previous.role = None;
                if role == GroupRole::Hidden {
                    previous.order = displaced_order;
                }
            }
        }
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == *id) {
            group.role = role;
        }
        self.renumber_pages();
        Ok(())
    }

    /// Deletes an ordinary group. Its items are appended, in their order, to
    /// the `Apps` group. A group holding any role (this includes the root) is
    /// refused with `GroupHoldsRole`.
    pub fn delete_group(&mut self, id: &Id) -> Result<(), ModelError> {
        if self.group(id)?.role.is_some() {
            return Err(ModelError::GroupHoldsRole(id.clone()));
        }
        let apps = self
            .role_holder(GroupRole::Apps)
            .ok_or(ModelError::MissingRole(GroupRole::Apps))?
            .id
            .clone();

        let mut ids = self.item_ids(&apps);
        ids.extend(self.item_ids(id));
        for item in self.items.iter_mut().filter(|i| i.group_id == *id) {
            item.group_id = apps.clone();
        }
        self.set_item_orders(&ids);
        self.groups.retain(|g| g.id != *id);
        self.renumber_pages();
        Ok(())
    }

    /// Moves the item `id` into `to_group` so that it ends up at position
    /// `new_order` (a zero-based index) among that group's items, shifting the
    /// others. `to_group` may be the item's current group, which reorders it.
    /// An index below zero is treated as zero and one past the end as the end.
    /// Afterwards the orders of the target group, and of the group the item
    /// left, are numbered `0..n`.
    ///
    /// Items cannot live in the root group: `ItemInRoot`.
    pub fn move_item(&mut self, id: &Id, to_group: &Id, new_order: i32) -> Result<(), ModelError> {
        let from_group = self.item_or_err(id)?.group_id.clone();
        self.group(to_group)?;
        if *to_group == self.root_group_id {
            return Err(ModelError::ItemInRoot(id.clone()));
        }

        let mut ids = self.item_ids(to_group);
        ids.retain(|i| i != id);
        ids.insert(clamp_index(new_order, ids.len()), id.clone());
        if let Some(item) = self.items.iter_mut().find(|i| i.id == *id) {
            item.group_id = to_group.clone();
        }
        self.set_item_orders(&ids);
        if from_group != *to_group {
            self.renumber_items(&from_group);
        }
        Ok(())
    }

    /// Sets the label and marks it as overridden, so that it is no longer
    /// replaced by the application's own label.
    pub fn rename_item(&mut self, id: &Id, label: &str) -> Result<(), ModelError> {
        self.item_or_err(id)?;
        if let Some(item) = self.items.iter_mut().find(|i| i.id == *id) {
            item.label = label.into();
            item.label_overridden = true;
        }
        Ok(())
    }

    pub fn remove_item(&mut self, id: &Id) -> Result<(), ModelError> {
        let group = self.item_or_err(id)?.group_id.clone();
        self.items.retain(|i| i.id != *id);
        self.renumber_items(&group);
        Ok(())
    }

    /// Orders the items of a group by label: case-insensitively and ignoring
    /// accents, then by lowercase label, then by the exact label, so the result
    /// is a total order that does not depend on the previous arrangement.
    pub fn sort_group(&mut self, id: &Id) -> Result<(), ModelError> {
        self.group(id)?;
        let mut keyed: Vec<(SortKey, Id)> = self
            .items
            .iter()
            .filter(|i| i.group_id == *id)
            .map(|i| (SortKey::new(&i.label), i.id.clone()))
            .collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        let ids: Vec<Id> = keyed.into_iter().map(|(_, id)| id).collect();
        self.set_item_orders(&ids);
        Ok(())
    }

    /// Adds an item for `app` to the `Apps` group, after its other items. An
    /// application can only be in the launcher once: when an item already refers
    /// to `app`, its id is returned and nothing changes.
    pub fn add_app(&mut self, app: AppRef, label: &str) -> Id {
        if let Some(existing) = self.items.iter().find(|i| i.app.as_ref() == Some(&app)) {
            return existing.id.clone();
        }
        let id = self.add_item_to(&self.apps_group_id(), label);
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.app = Some(app);
        }
        id
    }

    /// Adds an item with no application or shortcut as the last item of
    /// `group`. Items cannot live in the root, so when `group` is the root or
    /// unknown the item goes to the `Apps` group instead.
    pub fn add_item_to(&mut self, group: &Id, label: &str) -> Id {
        let group = match self.groups.iter().find(|g| g.id == *group) {
            Some(g) if g.id != self.root_group_id => g.id.clone(),
            _ => self.apps_group_id(),
        };
        let id = new_id();
        let order = self
            .items_in(&group)
            .last()
            .map_or(0, |i| i.order.saturating_add(1));
        self.items.push(Item {
            id: id.clone(),
            group_id: group.clone(),
            label: label.into(),
            label_overridden: false,
            app: None,
            shortcut: None,
            uninstalled: false,
            order,
            icon_override: None,
        });
        self.renumber_items(&group);
        id
    }

    fn apps_group_id(&self) -> Id {
        self.role_holder(GroupRole::Apps)
            .expect("a valid model has an Apps group")
            .id
            .clone()
    }

    fn is_page(&self, group: &Group) -> bool {
        group.parent_id.as_ref() == Some(&self.root_group_id)
            && group.role != Some(GroupRole::Hidden)
    }

    fn group(&self, id: &Id) -> Result<&Group, ModelError> {
        self.groups
            .iter()
            .find(|g| g.id == *id)
            .ok_or_else(|| ModelError::UnknownGroup(id.clone()))
    }

    fn item_or_err(&self, id: &Id) -> Result<&Item, ModelError> {
        self.item(id)
            .ok_or_else(|| ModelError::UnknownItem(id.clone()))
    }

    fn page_ids(&self) -> Vec<Id> {
        self.pages().into_iter().map(|g| g.id.clone()).collect()
    }

    fn item_ids(&self, group: &Id) -> Vec<Id> {
        self.items_in(group)
            .into_iter()
            .map(|i| i.id.clone())
            .collect()
    }

    fn renumber_pages(&mut self) {
        let ids = self.page_ids();
        self.set_page_orders(&ids);
    }

    fn renumber_items(&mut self, group: &Id) {
        let ids = self.item_ids(group);
        self.set_item_orders(&ids);
    }

    fn set_page_orders(&mut self, ids: &[Id]) {
        let orders = positions(ids);
        for group in self.groups.iter_mut() {
            if let Some(&order) = orders.get(group.id.as_str()) {
                group.order = order;
            }
        }
    }

    fn set_item_orders(&mut self, ids: &[Id]) {
        let orders = positions(ids);
        for item in self.items.iter_mut() {
            if let Some(&order) = orders.get(item.id.as_str()) {
                item.order = order;
            }
        }
    }
}

fn positions(ids: &[Id]) -> HashMap<&str, i32> {
    ids.iter()
        .enumerate()
        .map(|(n, id)| (id.as_str(), n as i32))
        .collect()
}

fn clamp_index(index: i32, len: usize) -> usize {
    usize::try_from(index).unwrap_or(0).min(len)
}

#[derive(PartialEq, Eq)]
struct SortKey {
    folded: String,
    lower: String,
    raw: String,
}

impl SortKey {
    fn new(label: &str) -> Self {
        let lower = label.to_lowercase();
        let folded = lower.nfd().filter(|c| !is_combining_mark(*c)).collect();
        SortKey {
            folded,
            lower,
            raw: label.to_string(),
        }
    }
}

impl Ord for SortKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.folded
            .cmp(&other.folded)
            .then_with(|| self.lower.cmp(&other.lower))
            .then_with(|| self.raw.cmp(&other.raw))
    }
}

impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
