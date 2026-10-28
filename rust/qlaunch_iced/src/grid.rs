//! Geometry shared by the launcher grid and the title strip.

use iced::Size;
use iced_android::Insets;
use qlaunch_core::settings::{LayoutStyle, Settings};

pub fn title_height(settings: &Settings) -> f32 {
    (settings.label_size as f32 * 1.4 + 20.0).max(48.0)
}

/// The exact height `view::cell` gives a tile or list row.
pub fn row_height(settings: &Settings) -> f32 {
    if settings.layout == LayoutStyle::List {
        settings.icon_size.max(settings.label_size * 2) as f32 + 10.0
    } else {
        let label_height = settings.label_size as f32 * 2.34;
        (if settings.icons_visible {
            settings.icon_size as f32
        } else {
            0.0
        }) + (if settings.labels_visible {
            label_height + 2.0
        } else {
            0.0
        }) + (label_height * 0.3).max(8.0)
    }
}

/// Width available to the grid after the system insets, used as one pager
/// page width.
pub fn content_width(size: Size, insets: Insets) -> f32 {
    (size.width - insets.left - insets.right).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qlaunch_core::settings::Settings;

    #[test]
    fn title_height_is_at_least_48() {
        let mut settings = Settings {
            label_size: 10,
            ..Settings::default()
        };
        assert_eq!(title_height(&settings), 48.0);
        settings.label_size = 40;
        assert!(title_height(&settings) > 48.0);
    }

    #[test]
    fn row_height_matches_the_cell_body() {
        let settings = Settings::default();
        // 54 icon + (42.12 label + 2) + 12.636 label breathing room.
        assert!((row_height(&settings) - 110.756).abs() < 0.01);
    }

    #[test]
    fn content_width_subtracts_insets() {
        let insets = Insets {
            left: 10.0,
            right: 30.0,
            ..Default::default()
        };
        assert_eq!(
            content_width(iced::Size::new(1080.0, 2400.0), insets),
            1040.0
        );
    }
}
