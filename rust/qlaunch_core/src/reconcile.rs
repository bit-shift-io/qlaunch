use std::collections::{HashMap, HashSet};

use crate::model::{AppCategory, AppEntry, AppRef, GroupRole, Id, Model, ShortcutPayload};

/// What a reconciliation changed. Every list holds item ids.
#[derive(Default, PartialEq, Debug)]
pub struct ReconcileOutcome {
    /// New items, in the order of the entries they were made from.
    pub added: Vec<Id>,
    /// Items for apps that are no longer listed, and shortcuts that targeted
    /// those apps, in the model's order.
    pub removed: Vec<Id>,
}

/// Brings `model` up to date with the applications the platform lists.
///
/// New apps are added to their category's role holder. An item that refers to
/// an app no longer in `entries` is removed, together with any Android
/// shortcut whose intent targets a package that no longer has a launcher
/// entry. Shortcuts to other packages, commands, Lock Screen shortcuts and
/// plain items are left alone. An app that is still listed stays where it is
/// whatever its category now is, and a legacy `uninstalled` flag on it is
/// cleared.
///
/// `entries` is the complete list of what is installed: an empty list removes
/// every app item, so it must come from a successful enumeration.
pub fn reconcile(model: &mut Model, entries: &[AppEntry]) -> ReconcileOutcome {
    let mut out = ReconcileOutcome::default();

    let mut known: HashMap<AppRef, Id> = HashMap::new();
    for item in &model.items {
        if let Some(app) = &item.app {
            known.entry(app.clone()).or_insert_with(|| item.id.clone());
        }
    }

    let mut live: HashSet<AppRef> = HashSet::new();
    let mut live_packages: HashSet<&str> = HashSet::new();
    for entry in entries {
        let app = AppRef {
            package: entry.package.clone(),
            activity: entry.activity.clone(),
        };
        live.insert(app.clone());
        live_packages.insert(entry.package.as_str());

        if let Some(id) = known.get(&app) {
            if let Some(item) = model.items.iter_mut().find(|i| i.id == *id) {
                item.uninstalled = false;
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

    // A package is gone when the model still has an app item for it but the
    // platform no longer lists any launcher activity for it. Shortcuts that
    // target that package would fail to launch, so they go with it.
    let gone_packages: HashSet<&str> = model
        .items
        .iter()
        .filter_map(|item| item.app.as_ref())
        .map(|app| app.package.as_str())
        .filter(|package| !live_packages.contains(package))
        .collect();

    let removed: Vec<Id> = model
        .items
        .iter()
        .filter(|item| {
            item.app.as_ref().is_some_and(|app| !live.contains(app))
                || item
                    .shortcut
                    .as_ref()
                    .is_some_and(|shortcut| targets_package(shortcut, &gone_packages))
        })
        .map(|item| item.id.clone())
        .collect();

    for id in &removed {
        // The id came from the model a moment ago, so this cannot fail.
        let _ = model.remove_item(id);
    }
    out.removed = removed;

    out
}

/// The package an Android shortcut's intent targets, whether it was stored as
/// an explicit package or only as a flattened component name.
fn targets_package(shortcut: &ShortcutPayload, packages: &HashSet<&str>) -> bool {
    let ShortcutPayload::Android {
        package, component, ..
    } = shortcut
    else {
        return false;
    };

    let component_package = component
        .as_deref()
        .and_then(|component| component.split_once('/'))
        .map(|(package, _)| package);

    package
        .as_deref()
        .is_some_and(|package| packages.contains(package))
        || component_package.is_some_and(|package| packages.contains(package))
}
