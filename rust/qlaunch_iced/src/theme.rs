//! Shared dark Material 3 tokens, typography and widget styles.
//!
//! Every colour, size and text style the dialogs and screens use comes from
//! here, so no screen invents its own. Values trace to Flutter's default dark
//! scheme (`theme_data.dart`, `_colorSchemeDarkM3`).

use iced::font::Weight;
use iced::{Color, Font};
use iced_android as platform;

/// Flutter's dark Material 3 colour scheme, the subset the UI uses.
pub(crate) mod color {
    use iced::Color;

    pub(crate) const SURFACE: Color = Color::from_rgb8(0x14, 0x12, 0x18);
    pub(crate) const ON_SURFACE: Color = Color::from_rgb8(0xe6, 0xe0, 0xe9);
    pub(crate) const SURFACE_CONTAINER: Color = Color::from_rgb8(0x21, 0x1f, 0x26);
    pub(crate) const SURFACE_CONTAINER_HIGH: Color = Color::from_rgb8(0x2b, 0x29, 0x30);
    pub(crate) const SURFACE_CONTAINER_HIGHEST: Color = Color::from_rgb8(0x36, 0x34, 0x3b);
    pub(crate) const ON_SURFACE_VARIANT: Color = Color::from_rgb8(0xca, 0xc4, 0xd0);
    pub(crate) const OUTLINE: Color = Color::from_rgb8(0x93, 0x8f, 0x99);
    pub(crate) const PRIMARY: Color = Color::from_rgb8(0xd0, 0xbc, 0xff);
    pub(crate) const ON_PRIMARY: Color = Color::from_rgb8(0x38, 0x1e, 0x72);
    pub(crate) const SECONDARY_CONTAINER: Color = Color::from_rgb8(0x4a, 0x44, 0x58);
    pub(crate) const ON_SECONDARY_CONTAINER: Color = Color::from_rgb8(0xe8, 0xde, 0xf8);
    pub(crate) const ERROR: Color = Color::from_rgb8(0xf2, 0xb8, 0xb5);
    pub(crate) const ON_ERROR: Color = Color::from_rgb8(0x60, 0x14, 0x10);
    pub(crate) const INVERSE_SURFACE: Color = Color::from_rgb8(0xe6, 0xe0, 0xe9);
    pub(crate) const ON_INVERSE_SURFACE: Color = Color::from_rgb8(0x32, 0x2f, 0x35);
    pub(crate) const SCRIM: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.54);
}

/// Fixed geometry shared by the view and the accessibility tree.
pub(crate) mod metrics {
    pub(crate) const APP_BAR_HEIGHT: f32 = 64.0;
    pub(crate) const LIST_ONE_LINE: f32 = 56.0;
    pub(crate) const LIST_TWO_LINE: f32 = 72.0;
    pub(crate) const FIELD_HEIGHT: f32 = 56.0;
    pub(crate) const ACTION_HEIGHT: f32 = 40.0;
    pub(crate) const DIALOG_RADIUS: f32 = 28.0;
    pub(crate) const DIALOG_SIDE_MARGIN: f32 = 40.0;
    pub(crate) const DIALOG_VERTICAL_MARGIN: f32 = 24.0;
    pub(crate) const DIALOG_MIN_WIDTH: f32 = 280.0;
    pub(crate) const DIALOG_MAX_WIDTH: f32 = 560.0;
    pub(crate) const DIALOG_PADDING: f32 = 24.0;
    pub(crate) const MENU_VERTICAL_PADDING: f32 = 8.0;
    pub(crate) const CONTENT_TOP_GAP: f32 = 16.0;
    pub(crate) const ACTION_GAP: f32 = 8.0;
    pub(crate) const SNACKBAR_HEIGHT: f32 = 48.0;
    pub(crate) const ICON_SIZE: f32 = 24.0;
    pub(crate) const TOUCH_TARGET: f32 = 48.0;
    pub(crate) const SLIDER_HEIGHT: f32 = 48.0;
    pub(crate) const CHIP_HEIGHT: f32 = 32.0;
    pub(crate) const SWITCH_HEIGHT: f32 = 32.0;
    pub(crate) const CHECKBOX_SIZE: f32 = 18.0;
    pub(crate) const SELECT_OPTION_HEIGHT: f32 = 48.0;
    pub(crate) const SELECT_MENU_MAX_HEIGHT: f32 = 280.0;
    pub(crate) const SETTINGS_CARD_RADIUS: f32 = 24.0;
    pub(crate) const SETTINGS_SIDE_MARGIN: f32 = 16.0;
    pub(crate) const SETTINGS_SCROLLBAR_GUTTER: f32 = 8.0;
    pub(crate) const SETTINGS_HEADER_TOP: f32 = 16.0;
    pub(crate) const SETTINGS_HEADER_BOTTOM: f32 = 8.0;
    pub(crate) const SETTINGS_ROW_PADDING: f32 = 12.0;
    pub(crate) const SETTINGS_LABEL_GAP: f32 = 8.0;
    pub(crate) const GROUP_CARD_HEIGHT: f32 = 72.0;
    pub(crate) const GROUP_LIST_PADDING_Y: f32 = 8.0;
}

/// The Material 3 text styles the UI draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextRole {
    HeadlineSmall,
    TitleLarge,
    TitleSmall,
    BodyLarge,
    BodyMedium,
    BodySmall,
    LabelLarge,
    LabelMedium,
}

/// One resolved text style: size, line height and the font to shape it with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Typography {
    pub size: f32,
    pub line_height: f32,
    pub font: Font,
}

impl Typography {
    pub(crate) fn line_height(&self) -> iced::widget::text::LineHeight {
        iced::widget::text::LineHeight::Absolute(iced::Pixels(self.line_height))
    }
}

/// The Material type scale the chrome draws. It is fixed: the chrome follows
/// the platform's own GUI font (and the system font scale the app is scaled
/// by), never the launcher's Text settings. Those settings drive only the
/// launcher's labels (`crate::fonts`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Text {
    pub font: Font,
}

impl Text {
    pub(crate) fn new() -> Self {
        Text {
            font: platform::DEFAULT_FONT,
        }
    }

    pub(crate) fn role(&self, role: TextRole) -> Typography {
        let (size, line_height, weight) = match role {
            TextRole::HeadlineSmall => (24.0, 32.0, Weight::Normal),
            TextRole::TitleLarge => (22.0, 28.0, Weight::Normal),
            TextRole::TitleSmall => (14.0, 20.0, Weight::Medium),
            TextRole::BodyLarge => (16.0, 24.0, Weight::Normal),
            TextRole::BodyMedium => (14.0, 20.0, Weight::Normal),
            TextRole::BodySmall => (12.0, 16.0, Weight::Normal),
            TextRole::LabelLarge => (14.0, 20.0, Weight::Medium),
            TextRole::LabelMedium => (12.0, 16.0, Weight::Medium),
        };
        Typography {
            size,
            line_height,
            font: Font {
                weight,
                ..self.font
            },
        }
    }
}

use iced::widget::{button, container, text_input};
use iced::{Background, Shadow, Vector};

/// The scrim and card shared by dialogs and the folder popup.
pub(crate) fn card(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(color::SURFACE_CONTAINER_HIGH.into()),
        text_color: Some(color::ON_SURFACE),
        border: iced::border::rounded(metrics::DIALOG_RADIUS),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.30),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

/// Flat, rounded surfaces for settings groups and individual action cards.
pub(crate) fn settings_card(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(color::SURFACE_CONTAINER.into()),
        text_color: Some(color::ON_SURFACE),
        border: iced::border::rounded(metrics::SETTINGS_CARD_RADIUS),
        ..Default::default()
    }
}

/// The card behind a dropped-down select menu.
pub(crate) fn menu(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(color::SURFACE_CONTAINER.into()),
        text_color: Some(color::ON_SURFACE),
        border: iced::border::rounded(4),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.30),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

pub(crate) fn scrim(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(color::SCRIM.into()),
        ..Default::default()
    }
}

/// A text button's state layer, shared by the other button styles.
fn state_background(status: button::Status) -> Background {
    let alpha = match status {
        button::Status::Hovered => 0.08,
        button::Status::Pressed => 0.12,
        _ => 0.0,
    };
    Color {
        a: alpha,
        ..color::PRIMARY
    }
    .into()
}

pub(crate) fn text_button(_: &iced::Theme, status: button::Status) -> button::Style {
    let disabled = matches!(status, button::Status::Disabled);
    button::Style {
        background: Some(state_background(status)),
        text_color: if disabled {
            Color {
                a: 0.38,
                ..color::ON_SURFACE
            }
        } else {
            color::PRIMARY
        },
        border: iced::border::rounded(20),
        ..Default::default()
    }
}

pub(crate) fn filled_button(_: &iced::Theme, status: button::Status) -> button::Style {
    let disabled = matches!(status, button::Status::Disabled);
    button::Style {
        background: Some(
            if disabled {
                Color {
                    a: 0.38,
                    ..color::ON_SURFACE
                }
            } else {
                color::PRIMARY
            }
            .into(),
        ),
        text_color: if disabled {
            color::SURFACE
        } else {
            color::ON_PRIMARY
        },
        border: iced::border::rounded(20),
        ..Default::default()
    }
}

/// A filled confirm button for a destructive action (Delete/Reset).
pub(crate) fn filled_destructive_button(_: &iced::Theme, status: button::Status) -> button::Style {
    let disabled = matches!(status, button::Status::Disabled);
    button::Style {
        background: Some(
            if disabled {
                Color {
                    a: 0.38,
                    ..color::ON_SURFACE
                }
            } else {
                color::ERROR
            }
            .into(),
        ),
        text_color: if disabled {
            color::SURFACE
        } else {
            color::ON_ERROR
        },
        border: iced::border::rounded(20),
        ..Default::default()
    }
}

pub(crate) fn destructive_button(_: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = text_button(&iced::Theme::Dark, status);
    style.text_color = if matches!(status, button::Status::Disabled) {
        Color {
            a: 0.38,
            ..color::ON_SURFACE
        }
    } else {
        color::ERROR
    };
    style
}

pub(crate) fn icon_button(_: &iced::Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered => 0.08,
        button::Status::Pressed => 0.12,
        _ => 0.0,
    };
    button::Style {
        background: Some(
            Color {
                a: alpha,
                ..color::ON_SURFACE
            }
            .into(),
        ),
        text_color: if matches!(status, button::Status::Disabled) {
            Color {
                a: 0.38,
                ..color::ON_SURFACE_VARIANT
            }
        } else {
            color::ON_SURFACE_VARIANT
        },
        border: iced::border::rounded(metrics::TOUCH_TARGET / 2.0),
        ..Default::default()
    }
}

pub(crate) fn input(_: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let _ = status;
    text_input::Style {
        background: Color::TRANSPARENT.into(),
        value: color::ON_SURFACE,
        placeholder: color::ON_SURFACE_VARIANT,
        selection: Color {
            a: 0.35,
            ..color::PRIMARY
        },
        border: iced::Border::default(),
        icon: color::ON_SURFACE_VARIANT,
    }
}

pub(crate) fn input_underline(focused: bool) -> (Color, f32) {
    if focused {
        (color::PRIMARY, 2.0)
    } else {
        (color::OUTLINE, 1.0)
    }
}

pub(crate) const SCROLLBAR_WIDTH: f32 = 6.0;

/// Scrollbars are hidden until the user scrolls, so idle content stays clean.
pub(crate) fn vertical_scrollbar(visible: bool) -> iced::widget::scrollable::Direction {
    use iced::widget::scrollable::{Direction, Scrollbar};
    Direction::Vertical(if visible {
        Scrollbar::new()
            .width(SCROLLBAR_WIDTH)
            .scroller_width(SCROLLBAR_WIDTH)
            .margin(2.0)
    } else {
        Scrollbar::hidden()
    })
}

pub(crate) fn scrollable(
    _theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    use iced::widget::scrollable::{Rail, Scroller, default};
    let mut style = default(&iced::Theme::Dark, status);
    let rail = Rail {
        background: None,
        border: iced::border::rounded(3),
        scroller: Scroller {
            background: Color {
                a: 0.55,
                ..color::ON_SURFACE
            }
            .into(),
            border: iced::border::rounded(3),
        },
    };
    style.vertical_rail = rail;
    style.horizontal_rail = rail;
    style
}

pub(crate) fn toggler(
    _: &iced::Theme,
    status: iced::widget::toggler::Status,
) -> iced::widget::toggler::Style {
    use iced::widget::toggler::{Status, Style};
    let (toggled, disabled) = match status {
        Status::Active { is_toggled } | Status::Hovered { is_toggled } => (is_toggled, false),
        Status::Disabled { is_toggled } => (is_toggled, true),
    };
    let track = if toggled {
        color::PRIMARY
    } else {
        color::SURFACE_CONTAINER_HIGHEST
    };
    let thumb = if toggled {
        if disabled {
            Color {
                a: 0.38,
                ..color::ON_PRIMARY
            }
        } else {
            color::ON_PRIMARY
        }
    } else if disabled {
        Color {
            a: 0.38,
            ..color::OUTLINE
        }
    } else {
        color::OUTLINE
    };
    Style {
        background: track.into(),
        background_border_width: 0.0,
        background_border_color: Color::TRANSPARENT,
        foreground: thumb.into(),
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: Some(color::ON_SURFACE),
        border_radius: Some((metrics::SWITCH_HEIGHT / 2.0).into()),
        padding_ratio: 0.125,
    }
}

pub(crate) fn slider(
    _: &iced::Theme,
    _status: iced::widget::slider::Status,
) -> iced::widget::slider::Style {
    use iced::widget::slider::{Handle, HandleShape, Rail, Style};
    Style {
        rail: Rail {
            backgrounds: (
                color::PRIMARY.into(),
                color::SURFACE_CONTAINER_HIGHEST.into(),
            ),
            width: 4.0,
            border: iced::border::rounded(2),
        },
        handle: Handle {
            shape: HandleShape::Circle { radius: 10.0 },
            background: color::PRIMARY.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}

pub(crate) fn checkbox(
    _: &iced::Theme,
    status: iced::widget::checkbox::Status,
) -> iced::widget::checkbox::Style {
    use iced::widget::checkbox::Status;
    let (checked, disabled) = match status {
        Status::Active { is_checked } | Status::Hovered { is_checked } => (is_checked, false),
        Status::Disabled { is_checked } => (is_checked, true),
    };
    let accent = if disabled {
        Color {
            a: 0.38,
            ..color::PRIMARY
        }
    } else {
        color::PRIMARY
    };
    iced::widget::checkbox::Style {
        background: if checked {
            accent.into()
        } else {
            Color::TRANSPARENT.into()
        },
        icon_color: color::ON_PRIMARY,
        border: iced::Border {
            color: if disabled {
                Color {
                    a: 0.38,
                    ..color::OUTLINE
                }
            } else {
                color::OUTLINE
            },
            width: 2.0,
            radius: 2.0.into(),
        },
        text_color: Some(color::ON_SURFACE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_match_flutter_dark_scheme() {
        assert_eq!(color::SURFACE, Color::from_rgb8(0x14, 0x12, 0x18));
        assert_eq!(
            color::SURFACE_CONTAINER_HIGH,
            Color::from_rgb8(0x2b, 0x29, 0x30)
        );
        assert_eq!(color::PRIMARY, Color::from_rgb8(0xd0, 0xbc, 0xff));
        assert_eq!(
            color::SECONDARY_CONTAINER,
            Color::from_rgb8(0x4a, 0x44, 0x58)
        );
        assert_eq!(color::ERROR, Color::from_rgb8(0xf2, 0xb8, 0xb5));
    }

    #[test]
    fn chrome_roles_have_fixed_material_sizes() {
        // The chrome type scale does not read the launcher's Text settings;
        // sizes are the Material 3 values and the font is the platform font.
        let text = Text::new();
        let title = text.role(TextRole::TitleSmall);
        assert_eq!(title.size, 14.0);
        assert_eq!(title.line_height, 20.0);
        assert_eq!(title.font.weight, Weight::Medium);
        assert_eq!(title.font.family, platform::DEFAULT_FONT.family);
        let headline = text.role(TextRole::HeadlineSmall);
        assert_eq!(headline.size, 24.0);
        assert_eq!(headline.font.weight, Weight::Normal);
    }

    #[test]
    fn text_button_has_primary_text_and_press_layer() {
        let active = text_button(&iced::Theme::Dark, iced::widget::button::Status::Active);
        let pressed = text_button(&iced::Theme::Dark, iced::widget::button::Status::Pressed);
        assert_eq!(active.text_color, color::PRIMARY);
        assert_ne!(active.background, pressed.background);
        assert_eq!(active.border.radius.top_left, 20.0);
    }

    #[test]
    fn filled_and_destructive_colors() {
        let filled = filled_button(&iced::Theme::Dark, iced::widget::button::Status::Active);
        assert_eq!(filled.background, Some(color::PRIMARY.into()));
        assert_eq!(filled.text_color, color::ON_PRIMARY);
        let destructive =
            destructive_button(&iced::Theme::Dark, iced::widget::button::Status::Active);
        assert_eq!(destructive.text_color, color::ERROR);
    }

    #[test]
    fn filled_destructive_uses_error_colors() {
        let style = filled_destructive_button(&iced::Theme::Dark, button::Status::Active);
        assert_eq!(style.background, Some(color::ERROR.into()));
        assert_eq!(style.text_color, color::ON_ERROR);
    }

    #[test]
    fn toggler_on_and_off_tracks() {
        use iced::widget::toggler::Status;
        let on = toggler(&iced::Theme::Dark, Status::Active { is_toggled: true });
        let off = toggler(&iced::Theme::Dark, Status::Active { is_toggled: false });
        assert_eq!(on.background, color::PRIMARY.into());
        assert_eq!(off.background, color::SURFACE_CONTAINER_HIGHEST.into());
    }

    #[test]
    fn slider_uses_primary_active_rail() {
        // backgrounds.0 is the rail left of the handle (the active part),
        // backgrounds.1 the rest.
        let style = slider(&iced::Theme::Dark, iced::widget::slider::Status::Active);
        assert_eq!(style.rail.backgrounds.0, color::PRIMARY.into());
        assert_eq!(
            style.rail.backgrounds.1,
            color::SURFACE_CONTAINER_HIGHEST.into()
        );
        assert_eq!(style.rail.width, 4.0);
    }

    #[test]
    fn scrollbar_is_hidden_until_scrolling() {
        use iced::widget::scrollable::{Direction, Scrollbar};
        assert_eq!(
            vertical_scrollbar(false),
            Direction::Vertical(Scrollbar::hidden())
        );
        assert_eq!(
            vertical_scrollbar(true),
            Direction::Vertical(
                Scrollbar::new()
                    .width(SCROLLBAR_WIDTH)
                    .scroller_width(SCROLLBAR_WIDTH)
                    .margin(2.0)
            )
        );
    }

    #[test]
    fn input_has_no_enclosing_border() {
        use iced::widget::text_input::Status;
        let idle = input(&iced::Theme::Dark, Status::Active);
        let focused = input(&iced::Theme::Dark, Status::Focused { is_hovered: false });
        assert_eq!(idle.border.width, 0.0);
        assert_eq!(focused.border.width, 0.0);
    }

    #[test]
    fn input_underline_tracks_focus_without_drawing_a_box() {
        assert_eq!(input_underline(false), (color::OUTLINE, 1.0));
        assert_eq!(input_underline(true), (color::PRIMARY, 2.0));
    }
}
