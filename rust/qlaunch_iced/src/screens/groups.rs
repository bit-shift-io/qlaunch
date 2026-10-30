//! Top-level groups, including hidden groups, in model order.

use crate::{App, Message, components, icons, theme};
use iced::widget::{column, row, toggler};
use iced::{Alignment, Element};
use qlaunch_core::api::api::ModelSnapshot;
use qlaunch_core::model::{GroupRole, Id};

pub(crate) struct GroupRow {
    pub id: Id,
    pub label: String,
    pub role: Option<GroupRole>,
    pub hidden: bool,
    pub always_hidden: bool,
    pub page_index: Option<usize>,
    pub page_count: usize,
}

pub(crate) fn rows(snapshot: &ModelSnapshot) -> Vec<GroupRow> {
    let mut groups: Vec<_> = snapshot
        .groups
        .iter()
        .filter(|g| g.id != snapshot.root_group_id && !g.folder)
        .collect();
    groups.sort_by_key(|g| g.order);
    groups
        .into_iter()
        .map(|g| GroupRow {
            id: g.id.clone(),
            label: g.label.clone(),
            role: g.role,
            hidden: g.hidden || g.role == Some(GroupRole::Hidden),
            always_hidden: g.role == Some(GroupRole::Hidden),
            page_index: snapshot.page_ids.iter().position(|id| id == &g.id),
            page_count: snapshot.page_ids.len(),
        })
        .collect()
}

pub(crate) fn role_name(role: GroupRole) -> &'static str {
    match role {
        GroupRole::Root => "Root",
        GroupRole::Apps => "Apps",
        GroupRole::Games => "Games",
        GroupRole::Home => "Home",
        GroupRole::Hidden => "Hidden",
    }
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let Some(snapshot) = &app.snapshot else {
        return column![].into();
    };
    let typography = theme::Text::new();
    let mut content = column![];
    for group in rows(snapshot) {
        let id = group.id.clone();
        let toggle = toggler(group.hidden)
            .size(theme::metrics::SWITCH_HEIGHT)
            .style(theme::toggler);
        let toggle = if group.always_hidden {
            toggle
        } else {
            toggle.on_toggle(move |hidden| Message::GroupHidden(id.clone(), hidden))
        };
        let mut trailing = row![toggle].align_y(Alignment::Center);
        if let Some(index) = group.page_index {
            trailing = trailing.push(components::icon_button(
                icons::ARROW_UPWARD,
                "Move up",
                Message::GroupMove(group.id.clone(), index.saturating_sub(1) as i32),
                index > 0,
            ));
            trailing = trailing.push(components::icon_button(
                icons::ARROW_DOWNWARD,
                "Move down",
                Message::GroupMove(group.id.clone(), (index + 1) as i32),
                index + 1 < group.page_count,
            ));
        }
        content = content.push(components::list_tile(
            &typography,
            &group.label,
            group.role.map(role_name),
            None,
            Some(trailing.into()),
            Some(Message::GroupEdit(group.id)),
            true,
        ));
    }
    super::scaffold(
        app,
        "Groups",
        vec![components::icon_button(
            icons::ADD,
            "Add group",
            Message::GroupAdd,
            true,
        )],
        content.into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_exclude_root_and_folders_and_follow_order() {
        let mut snapshot = crate::tests::app().snapshot.unwrap();
        let mut folder = snapshot.groups[1].clone();
        folder.id = "folder".into();
        folder.folder = true;
        snapshot.groups.push(folder);
        snapshot.groups.reverse();
        let rows = rows(&snapshot);
        assert!(
            rows.iter()
                .all(|row| row.id != snapshot.root_group_id && row.id != "folder")
        );
        let orders: Vec<_> = rows
            .iter()
            .map(|row| {
                snapshot
                    .groups
                    .iter()
                    .find(|g| g.id == row.id)
                    .unwrap()
                    .order
            })
            .collect();
        assert!(orders.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn hidden_role_is_always_hidden() {
        let snapshot = crate::tests::app().snapshot.unwrap();
        let rows = rows(&snapshot);
        let hidden = rows
            .iter()
            .find(|row| row.role == Some(GroupRole::Hidden))
            .unwrap();
        assert!(hidden.always_hidden && hidden.hidden);
    }

    #[test]
    fn page_groups_get_an_index_and_others_do_not() {
        let snapshot = crate::tests::app().snapshot.unwrap();
        for row in rows(&snapshot) {
            assert_eq!(
                row.page_index.is_some(),
                snapshot.page_ids.contains(&row.id)
            );
            assert_eq!(row.page_count, snapshot.page_ids.len());
        }
    }
}
