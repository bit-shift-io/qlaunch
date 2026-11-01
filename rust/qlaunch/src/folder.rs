//! Content-sized folder modal. One geometry source for drawing, touch and TalkBack.
use crate::{App, Message, Region, grid, theme, view};
use iced::widget::{container, mouse_area, opaque, scrollable, space, text};
use iced::{Color, Element, Fill, Rectangle, Size};
use iced_android::Insets;
use qlaunch_core::settings::{LayoutStyle, Settings};

pub(crate) struct FolderGeometry {
    pub bounds: Rectangle,
    pub viewport: Rectangle,
    pub columns: usize,
}

pub(crate) fn geometry(
    size: Size,
    insets: Insets,
    settings: &Settings,
    item_count: usize,
) -> FolderGeometry {
    let usable_width = (size.width - insets.left - insets.right).max(0.0);
    let usable_height = (size.height - insets.top - insets.bottom).max(0.0);
    let configured = if size.width > size.height {
        settings.grid_columns_landscape
    } else {
        settings.grid_columns_portrait
    };
    let columns = (configured as usize)
        .max(1)
        .min((item_count as f32).sqrt().ceil().max(1.0) as usize);
    let padding = 16.0_f32.min(usable_width / 4.0).min(usable_height / 4.0);
    let margin = 24.0_f32.min(usable_width / 8.0).min(usable_height / 8.0);
    let available = (usable_width - 2.0 * margin - 2.0 * padding).max(0.0);
    let natural = if settings.layout == LayoutStyle::List {
        (settings.icon_size as f32 + 100.0).max(180.0)
    } else {
        (settings.icon_size as f32 + 60.0).max(96.0)
    };
    let width = if item_count == 0 {
        140.0_f32.min(available)
    } else {
        (columns as f32 * natural).min(available)
    };
    let natural_height = if item_count == 0 {
        64.0
    } else {
        item_count.div_ceil(columns) as f32 * grid::row_height(settings)
    };
    let height = natural_height.min((usable_height * 0.7 - padding * 2.0).max(0.0));
    let bounds = Rectangle {
        x: insets.left + (usable_width - width - 2.0 * padding) / 2.0,
        y: insets.top + (usable_height - height - 2.0 * padding) / 2.0,
        width: width + 2.0 * padding,
        height: height + 2.0 * padding,
    };
    FolderGeometry {
        bounds,
        viewport: Rectangle {
            x: bounds.x + padding,
            y: bounds.y + padding,
            width,
            height,
        },
        columns,
    }
}

pub(crate) fn for_app(app: &App) -> Option<FolderGeometry> {
    let snapshot = app.snapshot.as_ref()?;
    app.folder.as_ref()?;
    Some(geometry(
        app.size,
        app.insets,
        &snapshot.settings,
        app.visible_items().len(),
    ))
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let layout = for_app(app).expect("folder view requires an open folder");
    let settings = &app.snapshot.as_ref().unwrap().settings;
    let colour = match settings.label_colour {
        qlaunch_core::settings::LabelColour::White => Color::WHITE,
        qlaunch_core::settings::LabelColour::Black => Color::BLACK,
        qlaunch_core::settings::LabelColour::Grey => Color::from_rgb8(158, 158, 158),
    };
    let items = app.visible_items();
    let content: Element<'_, Message> = if items.is_empty() {
        container(
            text("Empty folder")
                .size(14)
                .color(theme::color::ON_SURFACE),
        )
        .center(Fill)
        .into()
    } else {
        let group = app.folder.as_ref().unwrap().clone();
        let offset = app
            .scroll
            .get(&Region::Group(group.clone()))
            .copied()
            .unwrap_or(0.0);
        let window = grid::RowWindow::new(
            items.len(),
            layout.columns,
            grid::row_height(settings),
            layout.viewport.height,
            offset,
        );
        let motion = app.motion(Region::Group(group.clone()));
        let rows = view::grid_rows(
            app,
            items,
            layout.columns,
            colour,
            1.0,
            Some(&window),
            Some(&motion),
        );
        crate::elastic_scroll::wrap(
            scrollable(crate::elastic_scroll::content(rows, &motion))
                .id("folder-apps")
                .direction(theme::vertical_scrollbar(app.scrollbars_visible()))
                .style(theme::scrollable)
                .on_scroll(move |viewport| {
                    Message::Scrolled(Region::Group(group.clone()), viewport.absolute_offset().y)
                })
                .width(Fill)
                .height(Fill)
                .into(),
            motion,
            crate::elastic_scroll::Policy::Normal,
        )
    };
    let padding = layout.viewport.x - layout.bounds.x;
    let card = opaque(
        mouse_area(
            container(
                container(content)
                    .width(layout.viewport.width)
                    .height(layout.viewport.height)
                    .clip(true),
            )
            .padding(padding)
            .width(layout.bounds.width)
            .height(layout.bounds.height)
            .style(theme::card),
        )
        .on_press(Message::FolderCardPressed),
    );
    let scrim =
        mouse_area(
            container(space())
                .width(Fill)
                .height(Fill)
                .style(|_| container::Style {
                    background: Some(theme::color::SCRIM.into()),
                    ..Default::default()
                }),
        )
        .on_press(Message::FolderDismiss);
    opaque(iced::widget::stack![
        scrim,
        iced::widget::pin(card)
            .x(layout.bounds.x)
            .y(layout.bounds.y)
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_folders_wrap_to_compact_columns() {
        let settings = Settings::default();
        let one = geometry(Size::new(420.0, 900.0), Insets::default(), &settings, 1);
        assert_eq!(one.columns, 1);
        assert_eq!(one.viewport.width, 114.0);
        let three = geometry(Size::new(420.0, 900.0), Insets::default(), &settings, 3);
        assert_eq!(three.columns, 2);
        assert_eq!(three.viewport.width, 228.0);
        assert!((three.viewport.height - 221.512).abs() < 0.01);
        let empty = geometry(Size::new(420.0, 900.0), Insets::default(), &settings, 0);
        assert_eq!(empty.viewport.size(), Size::new(140.0, 64.0));
    }

    #[test]
    fn folders_fit_insets_tiny_windows_and_large_lists() {
        for size in [
            Size::new(420.0, 900.0),
            Size::new(900.0, 420.0),
            Size::new(80.0, 100.0),
        ] {
            let insets = Insets {
                left: 10.0,
                right: 15.0,
                top: 20.0,
                bottom: 25.0,
            };
            for count in [0, 1, 2, 9, 100] {
                for mode in [Settings::default().layout, LayoutStyle::List] {
                    let settings = Settings {
                        layout: mode,
                        ..Settings::default()
                    };
                    let layout = geometry(size, insets, &settings, count);
                    assert!(layout.bounds.x >= insets.left);
                    assert!(layout.bounds.y >= insets.top);
                    assert!(
                        layout.bounds.x + layout.bounds.width <= size.width - insets.right + 0.01
                    );
                    assert!(
                        layout.bounds.y + layout.bounds.height
                            <= size.height - insets.bottom + 0.01
                    );
                    assert!(layout.columns >= 1);
                }
            }
        }
    }
}
