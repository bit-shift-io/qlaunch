pub(crate) mod advanced;
pub(crate) mod group_apps;
pub(crate) mod groups;
pub(crate) mod settings;

use crate::{App, Message, Region, components, theme};
use iced::widget::{column, container, mouse_area, scrollable};
use iced::{Element, Fill};

pub(crate) fn scaffold<'a>(
    app: &'a App,
    title: &str,
    actions: Vec<Element<'a, Message>>,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    let settings = app
        .snapshot
        .as_ref()
        .map(|s| s.settings.clone())
        .unwrap_or_default();
    let typography = theme::Text::from_settings(&settings);
    iced::widget::opaque(
        mouse_area(
            container(column![
                components::app_bar(&typography, title, Some(Message::Back), actions),
                scrollable(content)
                    .id("screen-scroll")
                    .width(Fill)
                    .height(Fill)
                    .style(theme::scrollable)
                    .on_scroll(|viewport| Message::Scrolled(
                        Region::Screen,
                        viewport.absolute_offset().y
                    )),
            ])
            .padding(app.insets)
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(theme::color::SURFACE.into()),
                text_color: Some(theme::color::ON_SURFACE),
                ..Default::default()
            }),
        )
        .on_press(Message::Dialog(crate::dialog::Message::Noop)),
    )
}
