//! Installed-app membership checklist for one group.

use crate::{
    App, Message, components,
    dialog::{self, Change},
    theme,
};
use iced::Element;
use iced::widget::column;
use qlaunch_core::api::api::ModelSnapshot;
use qlaunch_core::model::{GroupRole, Id, Item};

pub(crate) struct AppRow {
    pub id: Id,
    pub label: String,
    pub checked: bool,
    pub enabled: bool,
}

pub(crate) fn rows(snapshot: &ModelSnapshot, group_id: &Id) -> Vec<AppRow> {
    let apps_id = snapshot
        .groups
        .iter()
        .find(|g| g.role == Some(GroupRole::Apps))
        .map(|g| &g.id);
    let mut items: Vec<_> = snapshot
        .items
        .iter()
        .filter(|i| i.app.is_some() && !i.uninstalled)
        .collect();
    items.sort_by_key(|i| i.label.to_lowercase());
    items
        .into_iter()
        .map(|i| AppRow {
            id: i.id.clone(),
            label: i.label.clone(),
            checked: &i.group_id == group_id,
            enabled: apps_id.is_some() && apps_id != Some(group_id),
        })
        .collect()
}

pub(crate) fn move_plan(
    snapshot: &ModelSnapshot,
    item: &Item,
    group_id: &Id,
    checked: bool,
) -> Option<Change> {
    let apps = snapshot
        .groups
        .iter()
        .find(|g| g.role == Some(GroupRole::Apps))?;
    if group_id == &apps.id {
        return None;
    }
    let target = if checked { group_id } else { &apps.id };
    if &item.group_id == target {
        return None;
    }
    Some(Change::MoveItem(
        item.id.clone(),
        target.clone(),
        dialog::append_order(snapshot, target),
    ))
}

pub(crate) fn view<'a>(app: &'a App, group_id: &Id) -> Element<'a, Message> {
    let Some(snapshot) = &app.snapshot else {
        return column![].into();
    };
    let title = snapshot
        .groups
        .iter()
        .find(|g| &g.id == group_id)
        .map_or("", |g| g.label.as_str());
    let typography = theme::Text::new();
    let mut content = column![];
    for row in rows(snapshot, group_id) {
        content = content.push(components::checkbox_row(
            &typography,
            &row.label,
            row.checked,
            Message::GroupAppToggle(row.id, group_id.clone(), !row.checked),
            row.enabled,
        ));
    }
    super::scaffold(app, title, vec![], content.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ModelSnapshot {
        let app = crate::tests::app();
        let mut snapshot = app.snapshot.clone().unwrap();
        let mut zebra = crate::tests::item(&app, "zebra");
        zebra.label = "Zebra".into();
        let mut alpha = crate::tests::item(&app, "alpha");
        alpha.label = "alpha".into();
        alpha.group_id = snapshot.page_ids[1].clone();
        let mut gone = crate::tests::item(&app, "gone");
        gone.uninstalled = true;
        snapshot.items = vec![zebra, gone, alpha];
        snapshot
    }

    #[test]
    fn rows_list_installed_apps_sorted_and_checked() {
        let snapshot = fixture();
        let group = &snapshot.page_ids[1];
        let rows = rows(&snapshot, group);
        assert_eq!(
            rows.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
            ["alpha", "Zebra"]
        );
        assert!(rows[0].checked);
        assert!(!rows[1].checked);
    }

    #[test]
    fn apps_group_checklist_is_disabled() {
        let snapshot = fixture();
        let apps = snapshot
            .groups
            .iter()
            .find(|g| g.role == Some(GroupRole::Apps))
            .unwrap();
        assert!(rows(&snapshot, &apps.id).iter().all(|r| !r.enabled));
    }

    #[test]
    fn check_moves_to_group_and_uncheck_to_apps_at_append_order() {
        let snapshot = fixture();
        let target = &snapshot.page_ids[1];
        let item = &snapshot.items[0];
        let apps = snapshot
            .groups
            .iter()
            .find(|g| g.role == Some(GroupRole::Apps))
            .unwrap();
        assert!(
            matches!(move_plan(&snapshot, item, target, true), Some(Change::MoveItem(_, ref g, 1)) if g == target)
        );
        let member = &snapshot.items[2];
        assert!(
            matches!(move_plan(&snapshot, member, target, false), Some(Change::MoveItem(_, ref g, _)) if g == &apps.id)
        );
    }
}
