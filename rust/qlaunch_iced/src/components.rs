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
        container(
            iced::widget::text_input(placeholder, value)
                .id(widget_id)
                .on_input(on_input)
                .on_submit(on_submit)
                .size(value_style.size)
                .font(value_style.font)
                .padding(8)
                .style(theme::input),
        )
        .height(theme::metrics::FIELD_HEIGHT - 16.0)
        .center_y(Length::Fill),
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
        border: iced::Border {
            color: theme::color::OUTLINE,
            width: 1.0,
            radius: 4.0.into(),
        },
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
    button(
        container(
            row![
                column![
                    text(label.to_string())
                        .size(label_style.size)
                        .line_height(label_style.line_height())
                        .font(label_style.font)
                        .color(theme::color::ON_SURFACE_VARIANT),
                    text(value.to_string())
                        .size(value_style.size)
                        .line_height(value_style.line_height())
                        .font(value_style.font)
                        .color(theme::color::ON_SURFACE),
                ]
                .width(Fill),
                text(icons::glyph(icons::KEYBOARD_ARROW_DOWN))
                    .font(icons::FONT)
                    .size(theme::metrics::ICON_SIZE)
                    .color(theme::color::ON_SURFACE_VARIANT),
            ]
            .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .padding([4, 12]),
    )
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
    let natural = option_count as f32 * theme::metrics::SELECT_OPTION_HEIGHT;
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
    let x = anchor.x.min(window.width - anchor.width).max(0.0);
    iced::Rectangle {
        x,
        y,
        width: anchor.width,
        height,
    }
}

fn option_style(_: &iced::Theme, status: button::Status) -> button::Style {
    let _ = status;
    button::Style {
        background: Some(iced::Color::TRANSPARENT.into()),
        text_color: theme::color::ON_SURFACE,
        border: iced::border::rounded(0),
        ..Default::default()
    }
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
) -> Element<'a, Message> {
    let option_style_text = text_settings.role(TextRole::BodyLarge);
    let mut menu = column![].spacing(0);
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
        content = content.push(
            text(label.to_string())
                .size(option_style_text.size)
                .line_height(option_style_text.line_height())
                .font(option_style_text.font),
        );
        menu = menu.push(
            button(
                container(content)
                    .width(Fill)
                    .height(theme::metrics::SELECT_OPTION_HEIGHT)
                    .padding([0, 16])
                    .center_y(Length::Fill),
            )
            .width(Fill)
            .padding(0)
            .style(if is_selected {
                option_style_selected
            } else {
                option_style
            })
            .on_press(choose(index)),
        );
    }
    container(
        scrollable(menu)
            .id("select-scroll")
            .on_scroll(|viewport| {
                Message::Scrolled(crate::Region::Select, viewport.absolute_offset().y)
            })
            .style(theme::scrollable),
    )
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
