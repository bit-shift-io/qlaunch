//! Top-level groups, including hidden groups, in model order.

use crate::{
    App, Message, components, icons,
    reorder_list::{Entry, ReorderList},
    theme,
};
use iced::widget::{column, container, row, text, toggler};
use iced::{Alignment, Element, Fill};
use qlaunch_core::api::api::ModelSnapshot;
use qlaunch_core::model::{GroupRole, Id};

pub(crate) struct GroupRow {
    pub id: Id,
    pub label: String,
    pub role: Option<GroupRole>,
    pub hidden: bool,
    pub always_hidden: bool,
}

pub(crate) fn rows(snapshot: &ModelSnapshot) -> Vec<GroupRow> {
    let mut groups: Vec<_> = snapshot
        .groups
        .iter()
        .filter(|g| g.parent_id.as_ref() == Some(&snapshot.root_group_id) && !g.folder)
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
    let mut entries = Vec::new();
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
        let handle = container(
            text(icons::glyph(icons::DRAG_INDICATOR))
                .font(icons::FONT)
                .size(theme::metrics::ICON_SIZE)
                .color(theme::color::ON_SURFACE_VARIANT),
        )
        .width(theme::metrics::TOUCH_TARGET)
        .height(theme::metrics::GROUP_CARD_HEIGHT)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);
        let tile = components::list_tile_with_height(
            &typography,
            &group.label,
            group.role.map(role_name),
            None,
            Some(toggle.into()),
            Some(Message::GroupEdit(group.id.clone())),
            true,
            theme::metrics::GROUP_CARD_HEIGHT,
        );
        entries.push(Entry {
            id: group.id,
            content: container(row![handle, tile].align_y(Alignment::Center))
                .width(Fill)
                .height(theme::metrics::GROUP_CARD_HEIGHT)
                .style(theme::settings_card)
                .clip(true)
                .into(),
        });
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
        container(ReorderList::new(entries))
            .padding(iced::Padding {
                top: theme::metrics::GROUP_LIST_PADDING_Y,
                bottom: theme::metrics::GROUP_LIST_PADDING_Y,
                left: theme::metrics::SETTINGS_SIDE_MARGIN,
                right: theme::metrics::SETTINGS_SIDE_MARGIN
                    + theme::metrics::SETTINGS_SCROLLBAR_GUTTER,
            })
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{Event, Point, mouse, touch};

    fn run_events(app: &App, events: &[Event]) -> Vec<Message> {
        run_events_at_offset(app, events, 0.0)
    }

    fn run_events_at_offset(app: &App, events: &[Event], offset: f32) -> Vec<Message> {
        use iced::advanced::{Layout, Shell, clipboard, layout, renderer::Headless, widget::Tree};
        use iced::{Rectangle, Size};
        let renderer = futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            16.0.into(),
            None,
        ))
        .expect("headless wgpu adapter");
        let mut page = view(app);
        let mut tree = Tree::new(&page);
        let node = page.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, app.size),
        );
        let mut operation = iced::advanced::widget::operation::scrollable::scroll_to::<()>(
            iced::advanced::widget::Id::new("screen-scroll"),
            iced::widget::scrollable::AbsoluteOffset { x: None, y: Some(offset) },
        );
        page.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut operation);
        let mut messages = Vec::new();
        let mut cursor = mouse::Cursor::Unavailable;
        for event in events {
            let previous_messages = messages.len();
            match event {
                Event::Touch(
                    touch::Event::FingerPressed { position, .. }
                    | touch::Event::FingerMoved { position, .. }
                    | touch::Event::FingerLifted { position, .. }
                    | touch::Event::FingerLost { position, .. },
                )
                | Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    cursor = mouse::Cursor::Available(*position);
                }
                _ => {}
            }
            page.as_widget_mut().update(
                &mut tree,
                event,
                Layout::new(&node),
                cursor,
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &Rectangle::with_size(app.size),
            );
            // Apply actual scroll operations so later frames exercise the
            // shifted viewport, not just the first scroll-request message.
            for message in &messages[previous_messages..] {
                if let Message::GroupDragScroll(offset) = message {
                    let mut operation = iced::advanced::widget::operation::scrollable::scroll_to::<()>(
                        iced::advanced::widget::Id::new("screen-scroll"),
                        iced::widget::scrollable::AbsoluteOffset { x: None, y: Some(*offset) },
                    );
                    page.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut operation);
                }
            }
        }
        messages
    }

    fn touch_drag(start: Point, end: Point) -> Vec<Event> {
        vec![
            Event::Touch(touch::Event::FingerPressed {
                id: touch::Finger(1),
                position: start,
            }),
            Event::Touch(touch::Event::FingerMoved {
                id: touch::Finger(1),
                position: end,
            }),
            Event::Touch(touch::Event::FingerLifted {
                id: touch::Finger(1),
                position: end,
            }),
        ]
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn dragging_a_grip_reorders_without_opening_the_editor() {
        let app = crate::tests::app();
        let first = rows(app.snapshot.as_ref().unwrap())[0].id.clone();
        let start = Point::new(40.0, 108.0);
        let end = Point::new(40.0, 268.0);
        let messages = run_events(&app, &touch_drag(start, end));
        assert!(
            matches!(messages.as_slice(), [Message::GroupMove(id, 2)] if *id == first),
            "dragging the grip should emit exactly one reorder, got {messages:?}"
        );
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn grip_dragging_uses_the_scrolled_rows_not_screen_coordinates() {
        let mut app = crate::tests::app();
        app.size.height = 260.0;
        let group = rows(app.snapshot.as_ref().unwrap())[1].id.clone();
        let messages = run_events_at_offset(&app,
            &touch_drag(Point::new(40.0, 108.0), Point::new(40.0, 188.0)), 80.0);
        assert!(matches!(messages.as_slice(), [Message::GroupMove(id, 2)] if *id == group), "{messages:?}");
        let messages = run_events_at_offset(&app, &[
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(40.0, 108.0) }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(40.0, 188.0) }),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ], 80.0);
        assert!(matches!(messages.as_slice(), [Message::GroupMove(id, 2)] if *id == group), "{messages:?}");
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn holding_a_grip_at_an_edge_reaches_both_ends_of_a_long_list() {
        let mut app = crate::tests::app();
        app.size.height = 260.0;
        let snapshot = app.snapshot.as_mut().unwrap();
        for index in 0..14 {
            let mut group = snapshot.groups.iter().find(|g| g.role == Some(GroupRole::Home)).unwrap().clone();
            group.id = format!("extra-{index}");
            group.label = format!("Extra {index}");
            group.role = None;
            group.order = index + 10;
            snapshot.groups.push(group);
        }
        let groups = rows(app.snapshot.as_ref().unwrap());
        for (offset, start_y, end_y, source, target) in [
            (0.0, 108.0, 245.0, 0, groups.len() - 1),
            (10000.0, 216.0, 90.0, groups.len() - 1, 0),
        ] {
            let start = Point::new(40.0, start_y);
            let end = Point::new(40.0, end_y);
            let mut events = touch_drag(start, end);
            let release = events.pop().unwrap();
            let now = std::time::Instant::now();
            for frame in 0..400 {
                events.push(Event::Window(iced::window::Event::RedrawRequested(now + std::time::Duration::from_millis(frame * 16))));
            }
            events.push(release);
            let messages = run_events_at_offset(&app, &events, offset);
            let moves: Vec<_> = messages.iter().filter(|message| matches!(message, Message::GroupMove(..))).collect();
            assert!(matches!(moves.as_slice(), [Message::GroupMove(id, index)] if *id == groups[source].id && *index == target as i32), "{moves:?}");
            let offsets: Vec<_> = messages.iter().filter_map(|message| match message {
                Message::GroupDragScroll(offset) => Some(*offset), _ => None,
            }).collect();
            assert!(offsets.len() > 100, "the stationary grip must keep scrolling: {offsets:?}");
            if target == 0 { assert_eq!(offsets.last(), Some(&0.0)); }
            else { assert!(offsets.last().is_some_and(|offset| *offset > 1000.0)); }
        }
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn hidden_group_grips_can_move_to_the_first_position() {
        let mut app = crate::tests::app();
        let snapshot = app.snapshot.as_mut().unwrap();
        let home = snapshot
            .groups
            .iter_mut()
            .find(|g| g.role == Some(GroupRole::Home))
            .unwrap();
        home.hidden = true;
        let home_id = home.id.clone();
        snapshot.page_ids.retain(|id| *id != home_id);
        let groups = rows(app.snapshot.as_ref().unwrap());
        for (index, group) in groups.iter().enumerate().filter(|(_, g)| g.hidden) {
            let messages = run_events(
                &app,
                &touch_drag(
                    Point::new(40.0, 108.0 + index as f32 * 80.0),
                    Point::new(40.0, 108.0),
                ),
            );
            assert!(
                matches!(messages.as_slice(), [Message::GroupMove(id, 0)] if *id == group.id),
                "{messages:?}"
            );
        }
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn cancelling_or_tapping_a_grip_never_saves_or_opens_an_editor() {
        let app = crate::tests::app();
        let start = Point::new(40.0, 108.0);
        assert!(run_events(&app, &touch_drag(start, start)).is_empty());
        let mut events = touch_drag(start, Point::new(40.0, 268.0));
        events[2] = Event::Touch(touch::Event::FingerLost {
            id: touch::Finger(1),
            position: Point::new(40.0, 268.0),
        });
        assert!(run_events(&app, &events).is_empty());
        events[2] = Event::Window(iced::window::Event::Unfocused);
        assert!(run_events(&app, &events).is_empty());
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn labels_and_visibility_switches_keep_their_actions() {
        let app = crate::tests::app();
        let id = rows(app.snapshot.as_ref().unwrap())[0].id.clone();
        let label = Point::new(130.0, 108.0);
        let messages = run_events(&app, &touch_drag(label, label));
        assert!(
            matches!(messages.as_slice(), [Message::GroupEdit(group)] if *group == id),
            "{messages:?}"
        );
        let toggle = Point::new(340.0, 108.0);
        let messages = run_events(&app, &touch_drag(toggle, toggle));
        assert!(
            matches!(messages.as_slice(), [Message::GroupHidden(group, true)] if *group == id),
            "{messages:?}"
        );
        let messages = run_events(&app, &touch_drag(label, Point::new(130.0, 268.0)));
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::GroupMove(..)))
        );
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn mouse_dragging_and_touch_edge_scrolling_are_supported() {
        let mut app = crate::tests::app();
        app.screens.push(crate::Screen::Groups);
        let messages = run_events(
            &app,
            &[
                Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(40.0, 108.0),
                }),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(40.0, 268.0),
                }),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
        );
        assert!(
            matches!(messages.as_slice(), [Message::GroupMove(_, 2)]),
            "{messages:?}"
        );
        app.size.height = 260.0;
        let mut events = touch_drag(Point::new(40.0, 108.0), Point::new(40.0, 245.0));
        events[2] = Event::Window(iced::window::Event::RedrawRequested(
            std::time::Instant::now(),
        ));
        let messages = run_events(&app, &events);
        assert!(
            messages.iter().any(
                |message| matches!(message, Message::GroupDragScroll(offset) if *offset > 0.0)
            ),
            "{messages:?}"
        );
    }

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
}
