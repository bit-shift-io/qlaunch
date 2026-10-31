//! Reusable Material 3 widgets shared by every screen and dialog.
//!
//! Every colour, size and text style comes from `theme`; nothing here invents
//! a value. The pure geometry helpers are tested; the builders are exercised
//! by the screens and dialogs that use them.

use iced::widget::{
    Space, button, checkbox, column, container, row, scrollable, slider, text, toggler,
};
use iced::{Element, Fill, Length};

use crate::theme::{self, Text, TextRole};
use crate::{Message, icons};

pub(crate) fn list_tile_height(has_subtitle: bool) -> f32 {
    if has_subtitle {
        theme::metrics::LIST_TWO_LINE
    } else {
        theme::metrics::LIST_ONE_LINE
    }
}

pub(crate) fn app_bar_height() -> f32 {
    theme::metrics::APP_BAR_HEIGHT
}

pub(crate) fn button_height() -> f32 {
    theme::metrics::ACTION_HEIGHT
}

/// A disabled colour: 38% opacity, as Material disables controls.
fn disabled(color: iced::Color) -> iced::Color {
    iced::Color { a: 0.38, ..color }
}

fn tile_style(_: &iced::Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered => 0.08,
        button::Status::Pressed => 0.12,
        _ => 0.0,
    };
    button::Style {
        background: Some(
            iced::Color {
                a: alpha,
                ..theme::color::ON_SURFACE
            }
            .into(),
        ),
        text_color: theme::color::ON_SURFACE,
        border: iced::border::rounded(0),
        ..Default::default()
    }
}

/// The Material app bar: an optional back button, a title and trailing actions.
pub(crate) fn app_bar<'a>(
    text_settings: &Text,
    title: &str,
    on_back: Option<Message>,
    actions: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let title_style = text_settings.role(TextRole::TitleLarge);
    let mut bar = row![]
        .align_y(iced::Alignment::Center)
        .spacing(4)
        .height(app_bar_height());
    if let Some(message) = on_back {
        bar = bar.push(icon_button(icons::ARROW_BACK, "Back", message, true));
    }
    bar = bar.push(
        text(title.to_string())
            .size(title_style.size)
            .line_height(title_style.line_height())
            .font(title_style.font)
            .color(theme::color::ON_SURFACE),
    );
    bar = bar.push(Space::new().width(Fill));
    for action in actions {
        bar = bar.push(action);
    }
    container(bar)
        .width(Fill)
        .padding([0, 4])
        .style(|_| container::Style {
            background: Some(theme::color::SURFACE.into()),
            text_color: Some(theme::color::ON_SURFACE),
            ..Default::default()
        })
        .into()
}

pub(crate) fn section_header<'a>(text_settings: &Text, label: &str) -> Element<'a, Message> {
    let style = text_settings.role(TextRole::TitleSmall);
    container(
        text(label.to_string())
            .size(style.size)
            .line_height(style.line_height())
            .font(style.font)
            .color(theme::color::ON_SURFACE),
    )
    .width(Fill)
    .padding(iced::Padding {
        top: theme::metrics::SETTINGS_HEADER_TOP,
        bottom: theme::metrics::SETTINGS_HEADER_BOTTOM,
        left: 16.0,
        right: 16.0,
    })
    .into()
}

/// A Material list tile. `leading` is a glyph from `icons`.
pub(crate) fn list_tile<'a>(
    text_settings: &Text,
    title: &str,
    subtitle: Option<&str>,
    leading: Option<char>,
    trailing: Option<Element<'a, Message>>,
    message: Option<Message>,
    enabled: bool,
) -> Element<'a, Message> {
    list_tile_with_height(
        text_settings,
        title,
        subtitle,
        leading,
        trailing,
        message,
        enabled,
        0.0,
    )
}

/// A list tile with extra room for wrapped labels in a narrow settings card.
pub(crate) fn list_tile_with_height<'a>(
    text_settings: &Text,
    title: &str,
    subtitle: Option<&str>,
    leading: Option<char>,
    trailing: Option<Element<'a, Message>>,
    message: Option<Message>,
    enabled: bool,
    min_height: f32,
) -> Element<'a, Message> {
    let title_style = text_settings.role(TextRole::BodyLarge);
    let subtitle_style = text_settings.role(TextRole::BodyMedium);
    let title_color = if enabled {
        theme::color::ON_SURFACE
    } else {
        disabled(theme::color::ON_SURFACE)
    };
    let subtitle_color = if enabled {
        theme::color::ON_SURFACE_VARIANT
    } else {
        disabled(theme::color::ON_SURFACE_VARIANT)
    };

    let mut content = row![].align_y(iced::Alignment::Center).spacing(16);
    if let Some(glyph) = leading {
        content = content.push(
            text(icons::glyph(glyph))
                .font(icons::FONT)
                .size(theme::metrics::ICON_SIZE)
                .color(if enabled {
                    theme::color::ON_SURFACE_VARIANT
                } else {
                    disabled(theme::color::ON_SURFACE_VARIANT)
                }),
        );
    }
    let mut labels = column![
        text(title.to_string())
            .size(title_style.size)
            .line_height(title_style.line_height())
            .font(title_style.font)
            .color(title_color)
    ]
    .spacing(0);
    if let Some(subtitle) = subtitle {
        labels = labels.push(
            text(subtitle.to_string())
                .size(subtitle_style.size)
                .line_height(subtitle_style.line_height())
                .font(subtitle_style.font)
                .color(subtitle_color),
        );
    }
    content = content.push(container(labels).width(Fill));
    if let Some(trailing) = trailing {
        content = content.push(trailing);
    }

    button(
        container(content)
            .width(Fill)
            .height(min_height.max(list_tile_height(subtitle.is_some())).max(
                title_style.line_height
                    + subtitle.map_or(0.0, |_| subtitle_style.line_height)
                    + 16.0,
            ))
            .padding([0, 16])
            .center_y(Length::Fill),
    )
    .width(Fill)
    .padding(0)
    .style(tile_style)
    .on_press_maybe(if enabled { message } else { None })
    .into()
}

pub(crate) fn action_tile<'a>(
    text_settings: &Text,
    title: &str,
    subtitle: &str,
    message: Message,
) -> Element<'a, Message> {
    list_tile(
        text_settings,
        title,
        Some(subtitle),
        None,
        None,
        Some(message),
        true,
    )
}

fn text_button_with<'a>(
    text_settings: &Text,
    label: &str,
    message: Option<Message>,
    style: fn(&iced::Theme, button::Status) -> button::Style,
) -> Element<'a, Message> {
    let label_style = text_settings.role(TextRole::LabelLarge);
    button(
        container(
            text(label.to_string())
                .size(label_style.size)
                .line_height(label_style.line_height())
                .font(label_style.font),
        )
        .padding([0, 12])
        .center_y(Length::Fill),
    )
    .height(button_height())
    .padding(0)
    .style(style)
    .on_press_maybe(message)
    .into()
}

pub(crate) fn text_button<'a>(
    text_settings: &Text,
    label: &str,
    message: Message,
    enabled: bool,
) -> Element<'a, Message> {
    text_button_with(
        text_settings,
        label,
        enabled.then_some(message),
        theme::text_button,
    )
}

pub(crate) fn filled_button<'a>(
    text_settings: &Text,
    label: &str,
    message: Message,
    enabled: bool,
) -> Element<'a, Message> {
    text_button_with(
        text_settings,
        label,
        enabled.then_some(message),
        theme::filled_button,
    )
}

pub(crate) fn destructive_button<'a>(
    text_settings: &Text,
    label: &str,
    message: Message,
    enabled: bool,
) -> Element<'a, Message> {
    text_button_with(
        text_settings,
        label,
        enabled.then_some(message),
        theme::destructive_button,
    )
}

pub(crate) fn filled_destructive_button<'a>(
    text_settings: &Text,
    label: &str,
    message: Message,
    enabled: bool,
) -> Element<'a, Message> {
    text_button_with(
        text_settings,
        label,
        enabled.then_some(message),
        theme::filled_destructive_button,
    )
}

pub(crate) fn icon_button<'a>(
    glyph: char,
    _label: &str,
    message: Message,
    enabled: bool,
) -> Element<'a, Message> {
    button(
        container(
            text(icons::glyph(glyph))
                .font(icons::FONT)
                .size(theme::metrics::ICON_SIZE),
        )
        .center(Length::Fill),
    )
    .width(theme::metrics::TOUCH_TARGET)
    .height(theme::metrics::TOUCH_TARGET)
    .padding(0)
    .style(theme::icon_button)
    .on_press_maybe(enabled.then_some(message))
    .into()
}

pub(crate) fn checkbox_row<'a>(
    text_settings: &Text,
    label: &str,
    checked: bool,
    message: Message,
    enabled: bool,
) -> Element<'a, Message> {
    let label_style = text_settings.role(TextRole::BodyLarge);
    let checkbox = checkbox(checked)
        .label(label.to_string())
        .size(theme::metrics::CHECKBOX_SIZE)
        .text_size(label_style.size)
        .font(label_style.font)
        .style(theme::checkbox);
    let checkbox = if enabled {
        checkbox.on_toggle(move |_| message.clone())
    } else {
        checkbox
    };
    container(checkbox)
        .width(Fill)
        .height(theme::metrics::LIST_ONE_LINE.max(label_style.line_height + 16.0))
        .padding([0, 16])
        .center_y(Length::Fill)
        .into()
}

pub(crate) fn slider_height() -> f32 {
    theme::metrics::SLIDER_HEIGHT
}

/// Horizontal padding inside a choice chip, per side.
pub(crate) const CHIP_PADDING_X: f32 = 12.0;
/// Gap between chips on one line, and between lines.
pub(crate) const CHIP_SPACING: f32 = 8.0;
/// Side padding of the chip row within the screen.
const CHIP_ROW_PADDING: f32 = 16.0;
/// Width the leading tick adds to a selected chip: an 18 dp glyph plus a 6 dp
/// gap before the label.
const CHIP_CHECK_WIDTH: f32 = 24.0;
/// Rough advance per character as a fraction of the text size. Only used to
/// decide where a chip row wraps; the chips themselves lay out at their true
/// width, so a slight overestimate just wraps a touch early.
const CHIP_CHAR_WIDTH: f32 = 0.6;

/// Width left for chips after the row's 16 dp side padding.
pub(crate) fn chip_available(content_width: f32) -> f32 {
    (content_width - 2.0 * CHIP_ROW_PADDING).max(0.0)
}

/// The horizontal space one chip needs: its label, its padding and, when
/// selected, the leading tick.
pub(crate) fn chip_width(label: &str, text_size: f32, selected: bool) -> f32 {
    let label_width = label.chars().count() as f32 * text_size * CHIP_CHAR_WIDTH;
    label_width + 2.0 * CHIP_PADDING_X + if selected { CHIP_CHECK_WIDTH } else { 0.0 }
}

/// Greedy line breaks for a wrapping chip row: how many chips sit on each line
/// within `available` width. Each chip keeps its own width and always gets a
/// line, even when it is wider than the row.
pub(crate) fn chip_rows(options: &[(String, bool)], available: f32, text_size: f32) -> Vec<usize> {
    let mut rows = Vec::new();
    let mut count = 0usize;
    let mut used = 0.0f32;
    for (label, selected) in options {
        let width = chip_width(label, text_size, *selected);
        let next = if count == 0 {
            width
        } else {
            used + CHIP_SPACING + width
        };
        if count > 0 && next > available {
            rows.push(count);
            count = 1;
            used = width;
        } else {
            count += 1;
            used = next;
        }
    }
    if count > 0 {
        rows.push(count);
    }
    rows
}

pub(crate) fn switch_row<'a>(
    text_settings: &Text,
    label: &str,
    value: bool,
    message: Message,
    enabled: bool,
) -> Element<'a, Message> {
    let label_style = text_settings.role(TextRole::BodyLarge);
    let toggle = toggler(value)
        .size(theme::metrics::SWITCH_HEIGHT)
        .font(label_style.font)
        .text_size(label_style.size)
        .style(theme::toggler);
    let toggle = if enabled {
        toggle.on_toggle(move |_| message.clone())
    } else {
        toggle
    };
    container(
        row![
            text(label.to_string())
                .size(label_style.size)
                .line_height(label_style.line_height())
                .font(label_style.font)
                .color(theme::color::ON_SURFACE)
                .width(Fill),
            toggle
        ]
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .height(theme::metrics::LIST_ONE_LINE.max(label_style.line_height + 16.0))
    .padding([0, 16])
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

pub(crate) fn slider_row<'a>(
    text_settings: &Text,
    label: &str,
    value: f32,
    unit: &str,
    range: std::ops::RangeInclusive<f32>,
    on_change: impl Fn(f32) -> Message + 'a,
    on_release: Message,
) -> Element<'a, Message> {
    let label_style = text_settings.role(TextRole::BodyLarge);
    let value_style = text_settings.role(TextRole::BodySmall);
    let shown = format!("{}{unit}", value.round() as i64);
    column![
        container(
            row![
                text(label.to_string())
                    .size(label_style.size)
                    .line_height(label_style.line_height())
                    .font(label_style.font)
                    .color(theme::color::ON_SURFACE)
                    .width(Fill),
                text(shown)
                    .size(value_style.size)
                    .line_height(value_style.line_height())
                    .font(value_style.font)
                    .color(theme::color::ON_SURFACE_VARIANT),
            ]
            .padding(iced::Padding {
                top: theme::metrics::SETTINGS_ROW_PADDING,
                left: 16.0,
                right: 16.0,
                ..Default::default()
            }),
        )
        .width(Fill),
        container(
            slider(range, value, on_change)
                .step(1.0_f32)
                .on_release(on_release)
                .style(theme::slider),
        )
        .width(Fill)
        .height(slider_height())
        .padding([0, 16]),
    ]
    .into()
}

fn chip_style_selected(_: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(theme::color::SECONDARY_CONTAINER.into()),
        text_color: theme::color::ON_SECONDARY_CONTAINER,
        border: iced::border::rounded(8),
        ..Default::default()
    }
}

fn chip_style_unselected(_: &iced::Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered => 0.08,
        button::Status::Pressed => 0.12,
        _ => 0.0,
    };
    button::Style {
        background: Some(
            iced::Color {
                a: alpha,
                ..theme::color::ON_SURFACE
            }
            .into(),
        ),
        text_color: theme::color::ON_SURFACE_VARIANT,
        border: iced::Border {
            color: theme::color::OUTLINE,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn chip<'a>(
    text_settings: &Text,
    label: &str,
    selected: bool,
    enabled: bool,
    message: Message,
) -> Element<'a, Message> {
    let style = text_settings.role(TextRole::LabelLarge);
    let mut content = row![].spacing(6).align_y(iced::Alignment::Center);
    if selected {
        content = content.push(
            text(icons::glyph(icons::CHECK))
                .font(icons::FONT)
                .size(18.0),
        );
    }
    content = content.push(
        text(label.to_string())
            .size(style.size)
            .line_height(style.line_height())
            .font(style.font),
    );
    button(container(content).center_y(Length::Fill))
        .height(theme::metrics::CHIP_HEIGHT.max(style.line_height))
        .padding([0.0, CHIP_PADDING_X])
        .style(if selected {
            chip_style_selected
        } else {
            chip_style_unselected
        })
        .on_press_maybe(enabled.then_some(message))
        .into()
}

pub(crate) fn choice_row<'a>(
    text_settings: &Text,
    label: &str,
    options: &[(String, bool)],
    available: f32,
    choose: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    let label_style = text_settings.role(TextRole::BodyLarge);
    let text_size = text_settings.role(TextRole::LabelLarge).size;
    let mut chips = column![].spacing(CHIP_SPACING);
    let mut index = 0usize;
    for count in chip_rows(options, available, text_size) {
        let mut line = row![].spacing(CHIP_SPACING);
        for _ in 0..count {
            let (option, selected) = &options[index];
            line = line.push(chip(text_settings, option, *selected, true, choose(index)));
            index += 1;
        }
        chips = chips.push(line);
    }
    column![
        container(
            text(label.to_string())
                .size(label_style.size)
                .line_height(label_style.line_height())
                .font(label_style.font)
                .color(theme::color::ON_SURFACE)
        )
        .width(Fill)
        .padding(iced::Padding {
            top: theme::metrics::SETTINGS_ROW_PADDING,
            bottom: theme::metrics::SETTINGS_LABEL_GAP,
            left: 16.0,
            right: 16.0,
        }),
        container(chips).width(Fill).padding(iced::Padding {
            bottom: theme::metrics::SETTINGS_ROW_PADDING,
            left: 16.0,
            right: 16.0,
            ..Default::default()
        }),
    ]
    .into()
}

pub(crate) fn text_field<'a>(
    text_settings: &Text,
    label: &str,
    value: &str,
    placeholder: &str,
    widget_id: &'static str,
    on_input: impl Fn(String) -> Message + 'a,
    on_submit: Message,
) -> Element<'a, Message> {
    let label_style = text_settings.role(TextRole::LabelMedium);
    let value_style = text_settings.role(TextRole::BodyLarge);
    column![
        text(label.to_string())
            .size(label_style.size)
            .line_height(label_style.line_height())
            .font(label_style.font)
            .color(theme::color::ON_SURFACE_VARIANT),
        container(crate::field::input(
            iced::widget::text_input(placeholder, value)
                .id(widget_id)
                .on_input(on_input)
                .on_submit(on_submit)
                .size(value_style.size)
                .line_height(value_style.line_height())
                .font(value_style.font)
                .padding([8, 0])
                .style(theme::input),
        ))
        .center_y(theme::metrics::FIELD_HEIGHT - label_style.line_height),
    ]
    .into()
}

fn select_style(_: &iced::Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            iced::Color {
                a: if hovered { 0.06 } else { 0.0 },
                ..theme::color::ON_SURFACE
            }
            .into(),
        ),
        text_color: theme::color::ON_SURFACE,
        border: iced::Border::default(),
        ..Default::default()
    }
}

pub(crate) fn select_field<'a>(
    text_settings: &Text,
    label: &str,
    value: &str,
    on_open: Message,
) -> Element<'a, Message> {
    let label_style = text_settings.role(TextRole::LabelMedium);
    let value_style = text_settings.role(TextRole::BodyLarge);
    button(column![
        text(label.to_string())
            .size(label_style.size)
            .line_height(label_style.line_height())
            .font(label_style.font)
            .color(theme::color::ON_SURFACE_VARIANT),
        crate::field::underline(
            container(
                row![
                    crate::label::ellipsized(value, value_style),
                    text(icons::glyph(icons::KEYBOARD_ARROW_DOWN))
                        .font(icons::FONT)
                        .size(theme::metrics::ICON_SIZE)
                        .color(theme::color::ON_SURFACE_VARIANT),
                ]
                .align_y(iced::Alignment::Center),
            )
            .width(Fill)
            .center_y(theme::metrics::FIELD_HEIGHT - label_style.line_height)
            .into()
        )
    ])
    .width(Fill)
    .height(theme::metrics::FIELD_HEIGHT)
    .padding(0)
    .style(select_style)
    .on_press(on_open)
    .into()
}

/// Where the dropped-down menu for a select field is drawn.
pub(crate) fn select_menu_bounds(
    anchor: iced::Rectangle,
    option_count: usize,
    window: iced::Size,
) -> iced::Rectangle {
    let natural = select_content_height(option_count);
    let height = natural
        .min(theme::metrics::SELECT_MENU_MAX_HEIGHT)
        .min(window.height.max(0.0));
    let below = anchor.y + anchor.height;
    let y = if below + height <= window.height {
        below
    } else {
        (anchor.y - height).max(0.0)
    }
    .min((window.height - height).max(0.0));
    let width = anchor.width.min(window.width.max(0.0)).max(0.0);
    let x = anchor.x.min(window.width - width).max(0.0);
    iced::Rectangle {
        x,
        y,
        width,
        height,
    }
}

pub(crate) fn select_content_height(option_count: usize) -> f32 {
    option_count as f32 * theme::metrics::SELECT_OPTION_HEIGHT
        + 2.0 * theme::metrics::MENU_VERTICAL_PADDING
}

pub(crate) fn select_option_bounds(
    menu: iced::Rectangle,
    index: usize,
    offset: f32,
) -> iced::Rectangle {
    iced::Rectangle {
        y: menu.y
            + theme::metrics::MENU_VERTICAL_PADDING
            + index as f32 * theme::metrics::SELECT_OPTION_HEIGHT
            - offset,
        height: theme::metrics::SELECT_OPTION_HEIGHT,
        ..menu
    }
}

fn option_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    tile_style(theme, status)
}

fn option_style_selected(_: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(theme::color::SECONDARY_CONTAINER.into()),
        text_color: theme::color::ON_SECONDARY_CONTAINER,
        border: iced::border::rounded(0),
        ..Default::default()
    }
}

pub(crate) fn select_menu<'a>(
    text_settings: &Text,
    options: Vec<(String, String, bool)>,
    selected: Option<&str>,
    choose: impl Fn(usize) -> Message + 'a,
    motion: &crate::elastic_scroll::Motion,
) -> Element<'a, Message> {
    let option_style_text = text_settings.role(TextRole::BodyLarge);
    let mut menu = column![]
        .spacing(0)
        .padding([theme::metrics::MENU_VERTICAL_PADDING, 0.0]);
    for (index, (id, label, is_folder)) in options.iter().enumerate() {
        let is_selected = selected == Some(id.as_str());
        let mut content = row![].align_y(iced::Alignment::Center).spacing(8);
        if *is_folder {
            content = content.push(
                text(icons::glyph(icons::FOLDER))
                    .font(icons::FONT)
                    .size(18.0)
                    .color(theme::color::ON_SURFACE_VARIANT),
            );
        }
        content = content.push(crate::label::ellipsized(label, option_style_text));
        menu = menu.push(crate::elastic_scroll::row(
            button(
                container(content)
                    .width(Fill)
                    .padding(iced::Padding {
                        left: 16.0 + if *is_folder { 20.0 } else { 0.0 },
                        right: 16.0,
                        ..Default::default()
                    })
                    .center_y(theme::metrics::SELECT_OPTION_HEIGHT),
            )
            .width(Fill)
            .padding(0)
            .style(if is_selected {
                option_style_selected
            } else {
                option_style
            })
            .on_press(choose(index))
            .into(),
            motion,
        ));
    }
    container(crate::elastic_scroll::wrap(
        scrollable(crate::elastic_scroll::content(menu.into(), motion))
            .id("select-scroll")
            .on_scroll(|viewport| {
                Message::Scrolled(crate::Region::Select, viewport.absolute_offset().y)
            })
            .style(theme::scrollable)
            .into(),
        motion.clone(),
        crate::elastic_scroll::Policy::Normal,
    ))
    .width(Fill)
    .style(theme::menu)
    .into()
}

pub(crate) fn snackbar<'a>(text_settings: &Text, message: &str) -> Element<'a, Message> {
    let style = text_settings.role(TextRole::BodyMedium);
    container(
        text(message.to_string())
            .size(style.size)
            .line_height(style.line_height())
            .font(style.font)
            .color(theme::color::ON_INVERSE_SURFACE),
    )
    .width(Fill)
    .height(theme::metrics::SNACKBAR_HEIGHT)
    .padding([0, 16])
    .center_y(Length::Fill)
    .style(|_| container::Style {
        background: Some(theme::color::INVERSE_SURFACE.into()),
        text_color: Some(theme::color::ON_INVERSE_SURFACE),
        border: iced::border::rounded(4),
        ..Default::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn control_layout(mut control: Element<'_, Message>) -> iced::advanced::layout::Node {
        use iced::advanced::{layout, renderer::Headless, widget::Tree};
        let renderer = futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            16.0.into(),
            None,
        ))
        .expect("headless wgpu adapter");
        let mut tree = Tree::new(&control);
        control.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(iced::Size::ZERO, iced::Size::new(360.0, 1000.0)),
        )
    }

    #[test]
    fn select_field_has_no_enclosing_border() {
        assert_eq!(
            select_style(&iced::Theme::Dark, button::Status::Active)
                .border
                .width,
            0.0
        );
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn select_field_label_and_value_share_the_full_touch_target() {
        use iced::advanced::{Layout, Shell, clipboard, layout, renderer::Headless, widget::Tree};
        use iced::{Event, Point, Rectangle, Size, mouse, touch};
        let renderer = futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            16.0.into(),
            None,
        ))
        .expect("headless wgpu adapter");
        for y in [8.0, 36.0, 54.0] {
            let mut field = select_field(&Text::new(), "Group", "Apps", Message::Back);
            let mut tree = Tree::new(&field);
            let node = field.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, Size::new(300.0, 56.0)),
            );
            let position = Point::new(100.0, y);
            let mut messages = Vec::new();
            for event in [
                touch::Event::FingerPressed {
                    id: touch::Finger(1),
                    position,
                },
                touch::Event::FingerLifted {
                    id: touch::Finger(1),
                    position,
                },
            ] {
                field.as_widget_mut().update(
                    &mut tree,
                    &Event::Touch(event),
                    Layout::new(&node),
                    mouse::Cursor::Available(position),
                    &renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &Rectangle::with_size(Size::new(300.0, 56.0)),
                );
            }
            assert!(
                matches!(messages.as_slice(), [Message::Back]),
                "tap at y={y} must open the dropdown: {messages:?}"
            );
        }
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn fields_layout_keeps_label_above_value_in_56_dp() {
        let input = control_layout(text_field(
            &Text::new(),
            "Name",
            "Example",
            "",
            "test-name",
            |_| Message::Back,
            Message::Back,
        ));
        let select = control_layout(select_field(&Text::new(), "Group", "Apps", Message::Back));
        assert_eq!(input.size().height, 56.0);
        assert_eq!(select.size().height, 56.0);
        // Both fields must expose the label as the first row, not cram it
        // inside the value button's horizontal padding.
        let select_rows = select.children()[0].children();
        assert_eq!(select_rows.len(), 2);
        assert!(select_rows[1].bounds().y >= 16.0);
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn settings_layout_centers_switch_content_in_the_tile() {
        let node = control_layout(switch_row(
            &Text::new(),
            "Show icons",
            true,
            Message::Back,
            true,
        ));
        let row = &node.children()[0];
        for child in row.children() {
            let center = row.bounds().y + child.bounds().center().y;
            assert!(
                (center - node.size().height / 2.0).abs() < 0.1,
                "switch text and control must center within the entire tile: {center}"
            );
        }
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn settings_layout_keeps_headings_compact() {
        let node = control_layout(section_header(&Text::new(), "Display"));
        assert!(
            node.size().height <= 44.0,
            "heading has excessive vertical space"
        );
        assert_eq!(node.children()[0].bounds().y, 16.0);
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn settings_layout_slider_does_not_double_pad_above_the_track() {
        let node = control_layout(slider_row(
            &Text::new(),
            "Icon size",
            50.0,
            "",
            0.0..=100.0,
            |_| Message::Back,
            Message::Back,
        ));
        assert_eq!(node.children()[1].bounds().y, 36.0);
        assert_eq!(
            node.children()[1].size().height,
            48.0,
            "preserve the slider touch target"
        );
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn settings_layout_choice_chips_have_bottom_clearance() {
        let node = control_layout(choice_row(
            &Text::new(),
            "Layout",
            &[("Tiles".into(), true)],
            328.0,
            |_| Message::Back,
        ));
        let chips = &node.children()[1];
        let content_bottom =
            chips.bounds().y + chips.children()[0].bounds().y + chips.children()[0].size().height;
        assert_eq!(
            node.size().height - content_bottom,
            12.0,
            "last chips must not hug the card edge"
        );
        assert_eq!(chips.bounds().y, 44.0);
    }

    #[test]
    fn list_tile_height_by_subtitle() {
        assert_eq!(list_tile_height(false), 56.0);
        assert_eq!(list_tile_height(true), 72.0);
    }

    #[test]
    fn app_bar_height_is_64() {
        assert_eq!(app_bar_height(), 64.0);
    }

    #[test]
    fn buttons_are_40_high() {
        assert_eq!(button_height(), 40.0);
    }

    #[test]
    fn select_menu_clamps_below_the_field() {
        let window = iced::Size::new(400.0, 300.0);
        let anchor = iced::Rectangle {
            x: 40.0,
            y: 240.0,
            width: 300.0,
            height: 56.0,
        };
        let bounds = select_menu_bounds(anchor, 5, window);
        assert!(bounds.y + bounds.height <= window.height);
        assert_eq!(bounds.width, anchor.width);
    }

    #[test]
    fn select_menu_reserves_padding_around_option_rows() {
        let anchor = iced::Rectangle {
            x: 40.0,
            y: 100.0,
            width: 300.0,
            height: 56.0,
        };
        let window = iced::Size::new(400.0, 900.0);
        assert_eq!(select_menu_bounds(anchor, 1, window).height, 64.0);
        assert_eq!(select_menu_bounds(anchor, 5, window).height, 256.0);
        assert_eq!(select_content_height(1), 64.0);
        assert_eq!(select_content_height(5), 256.0);
        let menu = iced::Rectangle {
            x: 40.0,
            y: 156.0,
            width: 300.0,
            height: 256.0,
        };
        assert_eq!(
            select_option_bounds(menu, 0, 0.0),
            iced::Rectangle {
                x: 40.0,
                y: 164.0,
                width: 300.0,
                height: 48.0
            }
        );
        assert_eq!(select_option_bounds(menu, 2, 40.0).y, 220.0);
    }

    #[test]
    fn select_menu_fits_a_window_narrower_than_its_anchor() {
        let anchor = iced::Rectangle {
            x: 40.0,
            y: 100.0,
            width: 300.0,
            height: 56.0,
        };
        let bounds = select_menu_bounds(anchor, 5, iced::Size::new(200.0, 100.0));
        assert_eq!(bounds.x, 0.0);
        assert_eq!(bounds.width, 200.0);
        let empty = select_menu_bounds(anchor, 5, iced::Size::ZERO);
        assert_eq!(empty, iced::Rectangle::default());
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn select_menu_layout_pads_and_indents_folder_rows() {
        let node = control_layout(select_menu(
            &Text::new(),
            vec![
                ("apps".into(), "Apps".into(), false),
                (
                    "folder".into(),
                    "An unusually long folder name that must not expand the dropdown menu".into(),
                    true,
                ),
            ],
            None,
            |_| Message::Back,
            &crate::elastic_scroll::Motion::new(),
        ));
        // Surface -> wrapper -> scrollable/content -> padded column -> rows.
        let list = node.children()[0].children()[0].children();
        assert_eq!(node.size().width, 360.0);
        assert_eq!(node.size().height, 112.0);
        assert_eq!(list[0].bounds().y, 8.0);
        let folder = &list[1].children()[0].children()[0];
        assert_eq!(folder.bounds().x, 36.0);
        assert_eq!(folder.size().height, 24.0);
    }

    fn headless() -> iced::Renderer {
        use iced::advanced::renderer::Headless;
        futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            16.0.into(),
            None,
        ))
        .expect("headless wgpu adapter")
    }

    fn long_menu(
        motion: &crate::elastic_scroll::Motion,
    ) -> (
        Element<'static, Message>,
        iced::advanced::widget::Tree,
        iced::advanced::layout::Node,
    ) {
        use iced::advanced::{layout, widget::Tree};
        let options: Vec<(String, String, bool)> = (0..30)
            .map(|index| (format!("id-{index}"), format!("Option {index}"), false))
            .collect();
        let mut menu = select_menu(&Text::new(), options, None, |_| Message::Back, motion);
        let mut tree = Tree::new(&menu);
        let node = menu.as_widget_mut().layout(
            &mut tree,
            &headless(),
            &layout::Limits::new(iced::Size::ZERO, iced::Size::new(300.0, 240.0)),
        );
        (menu, tree, node)
    }

    fn position(event: &iced::Event) -> Option<iced::Point> {
        match event {
            iced::Event::Touch(
                iced::touch::Event::FingerPressed { position, .. }
                | iced::touch::Event::FingerMoved { position, .. }
                | iced::touch::Event::FingerLifted { position, .. },
            ) => Some(*position),
            _ => None,
        }
    }

    fn send(
        element: &mut Element<'_, Message>,
        tree: &mut iced::advanced::widget::Tree,
        node: &iced::advanced::layout::Node,
        renderer: &iced::Renderer,
        event: iced::Event,
        messages: &mut Vec<Message>,
    ) {
        use iced::advanced::{Shell, layout, mouse};
        let cursor = position(&event).map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available);
        let mut shell = Shell::new(messages);
        element.as_widget_mut().update(
            tree,
            &event,
            layout::Layout::new(node),
            cursor,
            renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &iced::Rectangle::with_size(iced::Size::new(300.0, 240.0)),
        );
    }

    fn pressed(id: u64, position: iced::Point) -> iced::Event {
        iced::Event::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(id),
            position,
        })
    }

    fn moved(id: u64, position: iced::Point) -> iced::Event {
        iced::Event::Touch(iced::touch::Event::FingerMoved {
            id: iced::touch::Finger(id),
            position,
        })
    }

    fn lifted(id: u64, position: iced::Point) -> iced::Event {
        iced::Event::Touch(iced::touch::Event::FingerLifted {
            id: iced::touch::Finger(id),
            position,
        })
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn dragging_a_long_select_menu_scrolls_without_choosing() {
        use iced::Point;
        let (mut menu, mut tree, node) = long_menu(&crate::elastic_scroll::Motion::new());
        let renderer = headless();

        let mut messages = Vec::new();
        // A stationary tap on an option still chooses it.
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            pressed(1, Point::new(100.0, 32.0)),
            &mut messages,
        );
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            lifted(1, Point::new(100.0, 32.0)),
            &mut messages,
        );
        assert_eq!(
            messages
                .iter()
                .filter(|message| matches!(message, Message::Back))
                .count(),
            1,
            "{messages:?}"
        );

        messages.clear();
        // Dragging across options scrolls the menu instead of choosing the
        // option under the release point.
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            pressed(2, Point::new(100.0, 150.0)),
            &mut messages,
        );
        for y in [130.0, 110.0, 90.0, 70.0, 50.0] {
            send(
                &mut menu,
                &mut tree,
                &node,
                &renderer,
                moved(2, Point::new(100.0, y)),
                &mut messages,
            );
        }
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            lifted(2, Point::new(100.0, 50.0)),
            &mut messages,
        );
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::Back)),
            "{messages:?}"
        );
        assert!(
            messages.iter().any(|message| matches!(
                message,
                Message::Scrolled(crate::Region::Select, y) if *y > 0.0
            )),
            "the drag must scroll the menu: {messages:?}"
        );
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn select_menu_bounces_only_when_dragged_outward_past_the_bottom() {
        use iced::Point;
        use iced::advanced::Layout;
        use iced::advanced::widget::operation::scrollable::scroll_to;

        fn bottom(
            motion: &crate::elastic_scroll::Motion,
        ) -> (
            Element<'static, Message>,
            iced::advanced::widget::Tree,
            iced::advanced::layout::Node,
            iced::Renderer,
        ) {
            let (mut menu, mut tree, node) = long_menu(motion);
            let renderer = headless();
            let mut scroll = scroll_to::<()>(
                iced::widget::Id::new("select-scroll"),
                iced::widget::scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: 100_000.0,
                }
                .into(),
            );
            menu.as_widget_mut()
                .operate(&mut tree, Layout::new(&node), &renderer, &mut scroll);
            (menu, tree, node, renderer)
        }

        let outward = crate::elastic_scroll::Motion::new();
        let (mut menu, mut tree, node, renderer) = bottom(&outward);
        let mut messages = Vec::new();
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            pressed(1, Point::new(100.0, 160.0)),
            &mut messages,
        );
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            moved(1, Point::new(100.0, 130.0)),
            &mut messages,
        );
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            moved(1, Point::new(100.0, 100.0)),
            &mut messages,
        );
        assert!(outward.values().total < 0.0, "{:?}", outward.values());
        assert_eq!(outward.values().edge, crate::elastic_scroll::Edge::Bottom);

        let inward = crate::elastic_scroll::Motion::new();
        let (mut menu, mut tree, node, renderer) = bottom(&inward);
        let mut messages = Vec::new();
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            pressed(1, Point::new(100.0, 160.0)),
            &mut messages,
        );
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            moved(1, Point::new(100.0, 190.0)),
            &mut messages,
        );
        send(
            &mut menu,
            &mut tree,
            &node,
            &renderer,
            moved(1, Point::new(100.0, 220.0)),
            &mut messages,
        );
        assert_eq!(inward.values().total, 0.0);
    }

    #[test]
    fn select_menu_never_exceeds_max_height() {
        let window = iced::Size::new(400.0, 900.0);
        let anchor = iced::Rectangle {
            x: 40.0,
            y: 100.0,
            width: 300.0,
            height: 56.0,
        };
        let bounds = select_menu_bounds(anchor, 50, window);
        assert_eq!(bounds.height, theme::metrics::SELECT_MENU_MAX_HEIGHT);
    }

    #[test]
    fn select_menu_fits_a_short_window() {
        let window = iced::Size::new(400.0, 100.0);
        let anchor = iced::Rectangle {
            x: 40.0,
            y: 40.0,
            width: 300.0,
            height: 56.0,
        };
        let bounds = select_menu_bounds(anchor, 20, window);
        assert!(bounds.height <= window.height);
        assert!(bounds.y >= 0.0 && bounds.y + bounds.height <= window.height);
    }

    #[test]
    fn slider_track_is_48_high() {
        assert_eq!(slider_height(), 48.0);
    }

    #[test]
    fn chip_rows_wrap_only_when_a_line_is_full() {
        let options: Vec<(String, bool)> = ["White", "Black", "Grey"]
            .iter()
            .map(|label| (label.to_string(), false))
            .collect();
        assert_eq!(chip_rows(&options, 400.0, 14.0), vec![3]);
        let two =
            chip_width("White", 14.0, false) + CHIP_SPACING + chip_width("Black", 14.0, false);
        assert_eq!(chip_rows(&options, two, 14.0), vec![2, 1]);
    }

    #[test]
    fn a_selected_chip_reserves_room_for_its_tick() {
        assert!(chip_width("Grey", 14.0, true) > chip_width("Grey", 14.0, false));
        // A chip wider than the row still gets a line of its own.
        assert_eq!(chip_rows(&[("Grey".into(), true)], 1.0, 14.0), vec![1]);
        assert!(chip_rows(&[], 400.0, 14.0).is_empty());
    }
}
