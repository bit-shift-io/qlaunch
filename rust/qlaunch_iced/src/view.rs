//! Launcher rendering. State lives in `App`; this module only reads it.

use iced::widget::{
    button, column, container, image, mouse_area, row, scrollable, space, text, text_input,
};
use iced::{Color, Element, Fill};
use qlaunch_core::model::{Id, Item};
use qlaunch_core::settings::{LabelColour, LayoutStyle};

use crate::slots::{Interaction, Slot, Slots};
use crate::{App, Message, Region, grid, placeholder};

pub fn launcher(app: &App) -> Element<'_, Message> {
    let Some(snapshot) = &app.snapshot else {
        return container(text(app.error.as_deref().unwrap_or("Loading apps…")))
            .center(Fill)
            .into();
    };
    let settings = &snapshot.settings;
    let colour = match settings.label_colour {
        LabelColour::White => Color::WHITE,
        LabelColour::Black => Color::BLACK,
        LabelColour::Grey => Color::from_rgb8(158, 158, 158),
    };
    let content_width = grid::content_width(app.size, app.insets);
    let mut content = column![].spacing(0);
    if app.searching {
        content = content.push(
            row![
                crate::components::icon_button(
                    crate::icons::SETTINGS,
                    "Launcher Settings",
                    Message::Push(crate::Screen::Settings),
                    true
                ),
                container(
                    text_input("Search apps", &app.query)
                        .id("search")
                        .on_input(Message::Query)
                        .padding(10),
                )
                .width(Fill)
                .height(grid::SEARCH_CONTENT_HEIGHT)
                .center_y(Fill),
                crate::components::icon_button(
                    crate::icons::CLOSE,
                    "Close search",
                    Message::Search,
                    true
                )
            ]
            .spacing(grid::SEARCH_GAP)
            .padding([2.0, grid::SEARCH_HORIZONTAL_PADDING])
            .height(grid::SEARCH_ROW_HEIGHT),
        );
        if app.query.is_empty() {
            content = content.push(
                row![
                    title_button(
                        app,
                        "Frequent".to_string(),
                        colour,
                        if app.show_recent { 0.4 } else { 1.0 },
                        Message::History(false)
                    ),
                    title_button(
                        app,
                        "Recent".to_string(),
                        colour,
                        if app.show_recent { 1.0 } else { 0.4 },
                        Message::History(true)
                    )
                ]
                .height(grid::SEARCH_TABS_HEIGHT),
            );
        }
        content = content.push(single_grid(app, app.visible_items(), colour, true));
    } else {
        if grid::tabs_visible(settings, app.size, snapshot.page_ids.len()) {
            content = content.push(title_strip(app, colour, content_width));
        }
        content = content.push(pages(app, colour, content_width));
    }
    if let Some(error) = &app.error {
        content = content.push(
            container(text(error).size(14).color(Color::from_rgb8(255, 140, 140)))
                .width(Fill)
                .height(grid::ERROR_LINE_HEIGHT),
        );
    }
    for warning in &snapshot.warnings {
        content = content.push(
            container(text(warning).size(12))
                .width(Fill)
                .height(grid::WARNING_LINE_HEIGHT),
        );
    }
    let base: Element<'_, Message> = container(
        container(content)
            .padding(app.insets)
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(move |_| container::Style {
        background: Some(
            Color {
                a: settings.wallpaper_alpha as f32 / 100.0,
                ..Color::BLACK
            }
            .into(),
        ),
        ..Default::default()
    })
    .into();
    let folder: Element<'_, Message> = if app.folder.is_some() {
        crate::folder::view(app)
    } else {
        space().into()
    };
    let overlay: Element<'_, Message> = if let Some(overlay) = &app.overlay {
        crate::overlay::view(app, overlay)
    } else {
        space().into()
    };
    let screen: Element<'_, Message> = if app.screens.last() == Some(&crate::Screen::Settings) {
        crate::screens::settings::view(app)
    } else if app.screens.last() == Some(&crate::Screen::Advanced) {
        crate::screens::advanced::view(app)
    } else if app.screens.last() == Some(&crate::Screen::Groups) {
        crate::screens::groups::view(app)
    } else if let Some(crate::Screen::GroupApps(group)) = app.screens.last() {
        crate::screens::group_apps::view(app, group)
    } else if let Some(screen) = app.screens.last() {
        let title = match screen {
            crate::Screen::Settings => "Launcher Settings",
            crate::Screen::Advanced => "Advanced",
            crate::Screen::Groups => "Groups",
            crate::Screen::GroupApps(_) => "Choose apps",
        };
        let typography = crate::theme::Text::new();
        iced::widget::opaque(
            mouse_area(
                container(column![crate::components::app_bar(
                    &typography,
                    title,
                    Some(Message::Back),
                    vec![],
                )])
                .padding(app.insets)
                .width(Fill)
                .height(Fill)
                .style(|_| container::Style {
                    background: Some(crate::theme::color::SURFACE.into()),
                    ..Default::default()
                }),
            )
            .on_press(Message::Dialog(crate::dialog::Message::Noop)),
        )
    } else {
        space().into()
    };
    let snackbar: Element<'_, Message> = if let Some((_, message)) = &app.snackbar {
        let typography = crate::theme::Text::new();
        container(crate::components::snackbar(&typography, message))
            .width(Fill)
            .height(Fill)
            .align_y(iced::alignment::Vertical::Bottom)
            .padding(app.insets)
            .into()
    } else {
        space().into()
    };
    let dialog: Element<'_, Message> = if let Some(dialog) = &app.dialog {
        crate::dialog::view(app, dialog)
    } else {
        space().into()
    };
    let select: Element<'_, Message> = if let Some(select) = &app.select {
        crate::dialog::select_view(app, select)
    } else {
        space().into()
    };
    // Keep the base and folder at stable child indices so modal transitions
    // never reconcile a scrollable against a different tree hierarchy.
    iced::widget::stack![base, folder, overlay, screen, snackbar, dialog, select].into()
}

/// What each of the three page slots shows, before it becomes a widget. The
/// key is per group, so a slot that changes group gets a fresh widget tree
/// instead of inheriting the previous group's scroll offset.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PageSlotPlan {
    pub key: String,
    pub group: Option<Id>,
    pub scrollable: bool,
}

/// What each of the three title slots shows.
#[derive(Debug, Clone)]
pub(crate) struct TitleSlotPlan {
    pub key: String,
    pub label: String,
    pub message: Message,
}

pub(crate) fn page_slot_plan(app: &App) -> [PageSlotPlan; 3] {
    let slots = app.pager.slots();
    let page_ids = app
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.page_ids.as_slice())
        .unwrap_or_default();

    std::array::from_fn(|index| {
        let group = page_ids.get(slots[index]).cloned();
        PageSlotPlan {
            key: group
                .as_ref()
                .map_or_else(|| format!("page:empty:{index}"), |id| format!("page:{id}")),
            group,
            // Only the committed page scrolls; its neighbours are previews of
            // what a drag reveals and must not answer scroll or tap actions.
            scrollable: index == 1,
        }
    })
}

pub(crate) fn title_slot_plan(app: &App) -> [TitleSlotPlan; 3] {
    let slots = app.pager.slots();
    let snapshot = app.snapshot.as_ref();
    let page_ids = snapshot
        .map(|snapshot| snapshot.page_ids.as_slice())
        .unwrap_or_default();

    std::array::from_fn(|index| {
        let group = page_ids.get(slots[index]).cloned();
        let label = group
            .as_ref()
            .and_then(|id| {
                snapshot?
                    .groups
                    .iter()
                    .find(|candidate| &candidate.id == id)
            })
            .map(|group| group.label.clone())
            .unwrap_or_default();
        TitleSlotPlan {
            key: group.as_ref().map_or_else(
                || format!("title:empty:{index}"),
                |id| format!("title:{id}:{index}"),
            ),
            label,
            message: match index {
                0 => Message::Step(-1),
                1 => Message::Search,
                _ => Message::Step(1),
            },
        }
    })
}

/// Three wrapped page slots. Only the committed slot scrolls; the neighbours
/// are static previews of what a drag reveals.
fn pages<'a>(app: &'a App, colour: Color, content_width: f32) -> Element<'a, Message> {
    let snapshot = app.snapshot.as_ref().unwrap();
    let columns = app.columns();
    let plan = page_slot_plan(app);
    let slots = std::array::from_fn(|index| {
        let planned = &plan[index];
        let items: Vec<&Item> = snapshot
            .items
            .iter()
            .filter(|item| Some(&item.group_id) == planned.group.as_ref())
            .collect();
        let alpha = crate::slots::page_opacity(index as f32 - 1.0 - app.pager.offset());
        let rows = grid_rows(app, items, columns, colour, alpha);
        let content: Element<'a, Message> = if planned.scrollable {
            let mut apps = scrollable(rows)
                .id("apps")
                .direction(crate::theme::vertical_scrollbar(app.scrollbars_visible()))
                .style(crate::theme::scrollable)
                .height(Fill);
            if let Some(group) = planned.group.clone() {
                apps = apps.on_scroll(move |viewport| {
                    Message::Scrolled(Region::Group(group.clone()), viewport.absolute_offset().y)
                });
            }
            apps.into()
        } else {
            container(rows).width(Fill).height(Fill).clip(true).into()
        };
        Slot {
            key: planned.key.clone(),
            content: mouse_area(content).on_press(Message::BlankPressed).into(),
        }
    });

    let interaction = if settled(app) {
        Interaction::Centre
    } else {
        Interaction::None
    };

    mouse_area(Slots::pages(
        slots,
        app.pager.offset(),
        content_width,
        columns,
        interaction,
    ))
    .on_press(Message::BlankPressed)
    .into()
}

/// The title strip: the previous, current and next group names.
fn title_strip<'a>(app: &'a App, colour: Color, content_width: f32) -> Element<'a, Message> {
    let settings = &app.snapshot.as_ref().unwrap().settings;
    let offset = app.pager.offset();
    let plan = title_slot_plan(app);
    let height = grid::title_height(settings);
    let slots = std::array::from_fn(|index| {
        let planned = &plan[index];
        let position = index as f32 - 1.0;
        let opacity = (1.0 - (position - offset).abs() * 0.6).max(0.0);
        Slot {
            key: planned.key.clone(),
            content: title_button(
                app,
                planned.label.clone(),
                colour,
                opacity,
                planned.message.clone(),
            ),
        }
    });

    let interaction = if settled(app) {
        Interaction::All
    } else {
        Interaction::None
    };

    container(Slots::strip(
        slots,
        offset,
        content_width,
        height,
        interaction,
    ))
    .width(Fill)
    .height(height)
    .clip(true)
    .into()
}

/// A settled pager is not dragging or animating; only then may its slots take
/// input, so a tap during a drag or a settle cannot hit a moving page.
fn settled(app: &App) -> bool {
    app.folder.is_none()
        && app.overlay.is_none()
        && !app.pager.is_dragging()
        && !app.pager.is_animating()
}

fn single_grid<'a>(
    app: &'a App,
    items: Vec<&'a Item>,
    colour: Color,
    searching: bool,
) -> Element<'a, Message> {
    let columns = app.columns();
    let rows = grid_rows(app, items, columns, colour, 1.0);
    let mut apps = scrollable(rows)
        .direction(crate::theme::vertical_scrollbar(app.scrollbars_visible()))
        .style(crate::theme::scrollable)
        .height(Fill);
    if searching {
        apps = apps.id("search-results");
        apps.into()
    } else {
        apps = apps.id("apps");
        if let Some(group) = app.current_group_id().cloned() {
            apps = apps.on_scroll(move |viewport| {
                Message::Scrolled(Region::Group(group.clone()), viewport.absolute_offset().y)
            });
        }
        apps.into()
    }
}

pub(crate) fn grid_rows<'a>(
    app: &'a App,
    items: Vec<&'a Item>,
    columns: usize,
    colour: Color,
    alpha: f32,
) -> Element<'a, Message> {
    let mut rows = column![].spacing(0);
    let settings = &app.snapshot.as_ref().unwrap().settings;
    let row_count = items.len().div_ceil(columns);
    for (index, chunk) in items.chunks(columns).enumerate() {
        let mut line = row![].spacing(0);
        for &item in chunk {
            line = line.push(crate::scroll_fade::cell(
                settings.scroll_fade,
                index,
                row_count,
                grid::row_height(settings),
                move |fade| cell(app, item, colour, alpha * fade),
            ));
        }
        for _ in chunk.len()..columns {
            line = line.push(space().width(Fill));
        }
        rows = rows.push(line);
    }
    rows.into()
}

fn title_button<'a>(
    app: &'a App,
    label: String,
    colour: Color,
    opacity: f32,
    message: Message,
) -> Element<'a, Message> {
    button(
        container(self::label(
            app,
            label,
            Color {
                a: opacity,
                ..colour
            },
            true,
        ))
        .center_y(Fill),
    )
    .padding(8)
    .width(Fill)
    .height(Fill)
    .style(button::text)
    .on_press(message)
    .into()
}

fn cell<'a>(app: &'a App, item: &'a Item, colour: Color, alpha: f32) -> Element<'a, Message> {
    let settings = &app.snapshot.as_ref().unwrap().settings;
    let icon: Element<'a, Message> = if let Some(handle) = item
        .app
        .as_ref()
        .and_then(|reference| app.icons.get(reference))
    {
        image(handle.clone())
            .width(settings.icon_size as f32)
            .height(settings.icon_size as f32)
            .opacity(settings.icon_alpha as f32 / 100.0 * alpha)
            .into()
    } else {
        image(placeholder::handle(item.folder_group_id.is_some()))
            .width(settings.icon_size as f32)
            .height(settings.icon_size as f32)
            .opacity(settings.icon_alpha as f32 / 100.0 * alpha)
            .into()
    };
    let label = self::label(
        app,
        item.label.clone(),
        Color {
            a: colour.a * alpha,
            ..colour
        },
        settings.layout != LayoutStyle::List,
    );
    let label_height = settings.label_size as f32 * 2.34;
    let body: Element<'a, Message> = if settings.layout == LayoutStyle::List {
        let mut body = row![].spacing(10).align_y(iced::Center);
        if settings.icons_visible {
            body = body.push(icon);
        }
        if settings.labels_visible {
            body = body.push(label);
        }
        body.into()
    } else {
        let mut body = column![].spacing(2).align_x(iced::Center);
        if settings.icons_visible {
            body = body.push(icon);
        }
        if settings.labels_visible {
            body = body.push(container(label).height(label_height).clip(true));
        }
        body.into()
    };
    let pressed = app.pressed.as_ref() == Some(&item.id);
    mouse_area(
        container(body)
            .width(Fill)
            .height(grid::row_height(settings))
            .padding([6, 5])
            .style(move |_| container::Style {
                background: if pressed {
                    Some(Color::from_rgba(1.0, 1.0, 1.0, 0.10 * alpha).into())
                } else {
                    None
                },
                ..Default::default()
            }),
    )
    .on_press(Message::ItemPressed(item.clone()))
    .into()
}

/// Sharp 1 dp contrasting shadow, matching Flutter's default. Blurred
/// shadows remain outside the spike; no whole-tile raster cache is needed.
pub(crate) fn label<'a>(
    app: &'a App,
    value: String,
    colour: Color,
    centered: bool,
) -> Element<'a, Message> {
    let settings = &app.snapshot.as_ref().unwrap().settings;
    let make = |value: String, colour| {
        text(value)
            .size(settings.label_size as f32)
            .line_height(1.17)
            .color(colour)
            .font(crate::fonts::label_font(settings.label_typeface))
            .align_x(if centered {
                iced::alignment::Horizontal::Center
            } else {
                iced::alignment::Horizontal::Left
            })
            .width(Fill)
    };
    let foreground = make(value.clone(), colour);
    if settings.label_shadow {
        let contrast = if settings.label_colour == LabelColour::Black {
            Color::WHITE
        } else {
            Color::BLACK
        };
        iced::widget::stack![
            iced::widget::pin(make(
                value,
                Color {
                    a: colour.a,
                    ..contrast
                }
            ))
            .height(iced::Shrink)
            .x(if centered { 2.0 } else { 1.0 })
            .y(1.0),
            foreground
        ]
        .into()
    } else {
        foreground.into()
    }
}
