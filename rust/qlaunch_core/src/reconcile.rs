use std::collections::{HashMap, HashSet};

use crate::model::{AppCategory, AppEntry, AppRef, GroupRole, Id, Model};

/// What a reconciliation changed. Every list holds item ids.
#[derive(Default, PartialEq, Debug)]
pub struct ReconcileOutcome {
    /// New items, in the order of the entries they were made from.
    pub added: Vec<Id>,
    /// Items whose app is no longer listed and that were not already marked.
    pub marked_uninstalled: Vec<Id>,
    /// Items that were marked uninstalled and whose app is listed again.
    pub restored: Vec<Id>,
}

/// Brings `model` up to date with the applications the platform lists.
///
/// This only adds items, sets or clears the `uninstalled` flag, and nothing
/// else: no item is moved, renamed or removed, and groups are not touched, so
/// the user's arrangement survives. An app that is already in the model stays
/// where it is whatever its category now is. Items without an app (shortcuts
/// and empty items) are not looked at.
///
/// `entries` is the complete list of what is installed: an empty list marks
/// every app item uninstalled, so it must come from a successful enumeration.
pub fn reconcile(model: &mut Model, entries: &[AppEntry]) -> ReconcileOutcome {
    let mut out = ReconcileOutcome::default();

    let mut known: HashMap<AppRef, Id> = HashMap::new();
    for item in &model.items {
        if let Some(app) = &item.app {
            known.entry(app.clone()).or_insert_with(|| item.id.clone());
        }
    }

    let mut live: HashSet<AppRef> = HashSet::new();
    for entry in entries {
        let app = AppRef {
            package: entry.package.clone(),
            activity: entry.activity.clone(),
        };
        live.insert(app.clone());

        if let Some(id) = known.get(&app) {
            if let Some(item) = model.items.iter_mut().find(|i| i.id == *id)
                && item.uninstalled
            {
                item.uninstalled = false;
                out.restored.push(item.id.clone());
            }
            continue;
        }

        let role = match entry.category {
            AppCategory::Game => GroupRole::Games,
            AppCategory::App | AppCategory::Unknown => GroupRole::Apps,
        };
        // A model without the role holder is invalid; the root makes
        // `add_item_to` fall back to the `Apps` group rather than panic.
        let group = model
            .role_holder(role)
            .map_or_else(|| model.root_group_id.clone(), |g| g.id.clone());
        let id = model.add_item_to(&group, &entry.label);
        if let Some(item) = model.items.iter_mut().find(|i| i.id == id) {
            item.app = Some(app.clone());
        }
        known.insert(app, id.clone());
        out.added.push(id);
    }

    for item in model.items.iter_mut() {
        if let Some(app) = &item.app
            && !item.uninstalled
            && !live.contains(app)
        {
            item.uninstalled = true;
            out.marked_uninstalled.push(item.id.clone());
        }
    }

    out
}
