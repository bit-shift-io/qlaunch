//! Launcher rendering. State lives in `App`; this module only reads it.

use iced::widget::{
    Stack, button, column, container, float, image, mouse_area, row, scrollable, space, text,
    text_input,
};
use iced::{Color, Element, Fill, Font, Vector};
use iced_android as platform;
use qlaunch_core::model::Item;
use qlaunch_core::settings::{LabelColour, LayoutStyle, TabVisibility};

use crate::{App, Message, grid, placeholder};

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
                text_input("Search apps", &app.query)
                    .id("search")
                    .on_input(Message::Query)
                    .padding(10),
                button("×").on_press(Message::Search).style(button::text)
            ]
            .spacing(6)
            .padding([2, 8]),
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
                .height(48),
            );
        }
        content = content.push(single_grid(app, app.visible_items(), colour, true));
    } else if let Some(folder) = &app.folder {
        let label = snapshot
            .groups
            .iter()
            .find(|g| &g.id == folder)
            .map(|g| g.label.as_str())
            .unwrap_or("Folder");
        content = content.push(
            row![
                button("‹").on_press(Message::Back).style(button::text),
                text(label).size(settings.label_size as f32)
            ]
            .padding(8),
        );
        content = content.push(
            mouse_area(single_grid(app, app.visible_items(), colour, false))
                .on_press(Message::BlankPressed),
        );
    } else {
        let landscape = app.size.width > app.size.height;
        let tabs = match settings.tab_visibility {
            TabVisibility::Always => true,
            TabVisibility::Never => false,
            TabVisibility::LandscapeOnly => landscape,
            TabVisibility::PortraitOnly => !landscape,
        };
        if tabs && !snapshot.page_ids.is_empty() {
            content = content.push(title_strip(app, colour, content_width));
        }
        content = content.push(pages(app, colour, content_width));
    }
    if let Some(error) = &app.error {
        content = content.push(text(error).size(14).color(Color::from_rgb8(255, 140, 140)));
    }
    for warning in &snapshot.warnings {
        content = content.push(text(warning).size(12));
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
    if let Some(overlay) = &app.overlay {
        iced::widget::stack![base, crate::overlay::view(app, overlay)].into()
    } else {
        base
    }
}

/// Three wrapped page slots in translated layers. Only the current slot is
/// scrollable; the neighbours are static previews of what a drag reveals.
fn pages<'a>(app: &'a App, colour: Color, content_width: f32) -> Element<'a, Message> {
    let snapshot = app.snapshot.as_ref().unwrap();
    let columns = app.columns();
    let slots = app.pager.slots();
    let offset = app.pager.offset();
    let mut layers = Vec::with_capacity(3);
    for (index, slot) in slots.iter().enumerate() {
        let group = snapshot.page_ids.get(*slot);
        let items: Vec<&Item> = snapshot
            .items
            .iter()
            .filter(|item| Some(&item.group_id) == group)
            .collect();
        let rows = grid_rows(app, items, columns, colour);
        let content: Element<'a, Message> = if index == 1 {
            let mut apps = scrollable(rows).id("apps").height(Fill);
            if let Some(group) = group.cloned() {
                apps = apps.on_scroll(move |viewport| {
                    Message::Scrolled(group.clone(), viewport.absolute_offset().y)
                });
            }
            apps.into()
        } else {
            container(rows).width(Fill).height(Fill).clip(true).into()
        };
        // The pages render through `float` overlays, which receive input before
        // the base tree; target the blank press inside the layer, or presses
        // between icons never reach the launcher's gesture classifier.
        let content = mouse_area(content).on_press(Message::BlankPressed);
        let dx = (index as f32 - 1.0 - offset) * content_width;
        layers.push(
            float(container(content).width(Fill).height(Fill))
                .translate(move |_, _| Vector::new(dx, 0.0))
                .into(),
        );
    }
    mouse_area(
        container(Stack::with_children(layers))
            .width(Fill)
            .height(Fill)
            .clip(true),
    )
    .on_press(Message::BlankPressed)
    .into()
}

fn title_strip<'a>(app: &'a App, colour: Color, content_width: f32) -> Element<'a, Message> {
    let snapshot = app.snapshot.as_ref().unwrap();
    let settings = &snapshot.settings;
    let slots = app.pager.slots();
    let offset = app.pager.offset();
    let mut labels = row![].spacing(0).height(Fill);
    for (index, slot) in slots.iter().enumerate() {
        let position = index as f32 - 1.0;
        let opacity = (1.0 - (position - offset).abs() * 0.6).max(0.0);
        let label = snapshot
            .groups
            .iter()
            .find(|g| g.id == snapshot.page_ids[*slot])
            .map(|g| g.label.clone())
            .unwrap_or_default();
        let message = match index {
            0 => Message::Step(-1),
            1 => Message::Search,
            _ => Message::Step(1),
        };
        labels = labels.push(title_button(app, label, colour, opacity, message));
    }
    let dx = -offset * content_width / 3.0;
    container(float(labels).translate(move |_, _| Vector::new(dx, 0.0)))
        .width(Fill)
        .height(grid::title_height(settings))
        .clip(true)
        .into()
}

fn single_grid<'a>(
    app: &'a App,
    items: Vec<&'a Item>,
    colour: Color,
    searching: bool,
) -> Element<'a, Message> {
    let columns = app.columns();
    let rows = grid_rows(app, items, columns, colour);
    let mut apps = scrollable(rows).height(Fill);
    if searching {
        apps = apps.id("search-results");
        apps.into()
    } else {
        apps = apps.id("apps");
        if let Some(group) = app.current_group_id().cloned() {
            apps = apps.on_scroll(move |viewport| {
                Message::Scrolled(group.clone(), viewport.absolute_offset().y)
            });
        }
        apps.into()
    }
}

fn grid_rows<'a>(
    app: &'a App,
    items: Vec<&'a Item>,
    columns: usize,
    colour: Color,
) -> Element<'a, Message> {
    let mut rows = column![].spacing(0);
    for chunk in items.chunks(columns) {
        let mut line = row![].spacing(0);
        for item in chunk {
            line = line.push(cell(app, item, colour));
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

fn cell<'a>(app: &'a App, item: &'a Item, colour: Color) -> Element<'a, Message> {
    let settings = &app.snapshot.as_ref().unwrap().settings;
    let icon: Element<'a, Message> = if let Some(handle) = item
        .app
        .as_ref()
        .and_then(|reference| app.icons.get(reference))
    {
        image(handle.clone())
            .width(settings.icon_size as f32)
            .height(settings.icon_size as f32)
            .opacity(settings.icon_alpha as f32 / 100.0)
            .into()
    } else {
        image(placeholder::handle(item.folder_group_id.is_some()))
            .width(settings.icon_size as f32)
            .height(settings.icon_size as f32)
            .opacity(settings.icon_alpha as f32 / 100.0)
            .into()
    };
    let label = self::label(
        app,
        item.label.clone(),
        colour,
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
                    Some(Color::from_rgba(1.0, 1.0, 1.0, 0.10).into())
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
            .font(Font {
                family: match settings.label_typeface {
                    qlaunch_core::settings::Typeface::Monospace => iced::font::Family::Monospace,
                    qlaunch_core::settings::Typeface::Serif => iced::font::Family::Serif,
                    _ => platform::DEFAULT_FONT.family,
                },
                ..platform::DEFAULT_FONT
            })
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
