//! Local dark Material 3 styling, matching Flutter's default dark scheme.
use iced::widget::{button, container, text_input};
use iced::{Color, Shadow, Vector};

// Flutter material/theme_data.dart: _colorSchemeDarkM3.
pub(crate) const SURFACE: Color = Color::from_rgb8(0x2b, 0x29, 0x30);
pub(crate) const ON_SURFACE: Color = Color::from_rgb8(0xe6, 0xe0, 0xe9);
pub(crate) const PRIMARY: Color = Color::from_rgb8(0xd0, 0xbc, 0xff);
pub(crate) const ON_PRIMARY: Color = Color::from_rgb8(0x38, 0x1e, 0x72);
pub(crate) const ERROR: Color = Color::from_rgb8(0xf2, 0xb8, 0xb5);
pub(crate) const SCRIM: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.54);

pub(crate) fn card(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(SURFACE.into()),
        text_color: Some(ON_SURFACE),
        border: iced::border::rounded(28),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.30),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

pub(crate) fn action(_: &iced::Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered => 0.08,
        button::Status::Pressed => 0.12,
        _ => 0.0,
    };
    button::Style {
        background: Some(
            Color {
                a: alpha,
                ..PRIMARY
            }
            .into(),
        ),
        text_color: PRIMARY,
        border: iced::border::rounded(20),
        ..Default::default()
    }
}

pub(crate) fn filled(theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = action(theme, status);
    style.background = Some(PRIMARY.into());
    style.text_color = ON_PRIMARY;
    style
}

pub(crate) fn destructive(theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut style = action(theme, status);
    style.text_color = ERROR;
    style
}

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

pub(crate) const SCROLLBAR_WIDTH: f32 = 6.0;

pub(crate) fn scrollable(
    theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    use iced::widget::scrollable::{Rail, Scroller, default};
    let mut style = default(theme, status);
    let rail = Rail {
        background: None,
        border: iced::border::rounded(3),
        scroller: Scroller {
            background: Color {
                a: 0.55,
                ..ON_SURFACE
            }
            .into(),
            border: iced::border::rounded(3),
        },
    };
    style.vertical_rail = rail;
    style.horizontal_rail = rail;
    style
}

pub(crate) fn input(theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let mut style = text_input::default(theme, status);
    style.background = Color::TRANSPARENT.into();
    style.value = ON_SURFACE;
    style.placeholder = Color::from_rgb8(0xca, 0xc4, 0xd0);
    style.selection = Color { a: 0.35, ..PRIMARY };
    style.border = iced::Border {
        color: if matches!(status, text_input::Status::Focused { .. }) {
            PRIMARY
        } else {
            Color::from_rgb8(0x93, 0x8f, 0x99)
        },
        width: if matches!(status, text_input::Status::Focused { .. }) {
            2.0
        } else {
            1.0
        },
        radius: 4.0.into(),
    };
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::scrollable::{Direction, Scrollbar};

    #[test]
    fn scrollbar_is_hidden_until_scrolling() {
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
        assert_ne!(vertical_scrollbar(true), vertical_scrollbar(false));
    }
}
