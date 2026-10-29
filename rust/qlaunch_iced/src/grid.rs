//! Geometry shared by the launcher grid and the title strip.

use iced::{Rectangle, Size};
use iced_android::Insets;
use qlaunch_core::settings::{LayoutStyle, Settings, TabVisibility};

pub const SEARCH_ROW_HEIGHT: f32 = 44.0;
pub const SEARCH_CONTENT_HEIGHT: f32 = 40.0;
pub const SEARCH_TABS_HEIGHT: f32 = 48.0;
pub const SEARCH_HORIZONTAL_PADDING: f32 = 8.0;
pub const SEARCH_GAP: f32 = 6.0;
pub const SEARCH_CLOSE_WIDTH: f32 = 48.0;
pub const FOLDER_BACK_WIDTH: f32 = 56.0;
pub const FOLDER_PADDING: f32 = 8.0;
pub const ERROR_LINE_HEIGHT: f32 = 18.0;
pub const WARNING_LINE_HEIGHT: f32 = 16.0;

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
    Rectangle {
        x,
        y,
        width,
        height,
    }
}

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

/// Whether the launcher draws the page-title strip for this size.
pub fn tabs_visible(settings: &Settings, size: Size, page_count: usize) -> bool {
    if page_count == 0 {
        return false;
    }
    let landscape = size.width > size.height;
    match settings.tab_visibility {
        TabVisibility::Always => true,
        TabVisibility::Never => false,
        TabVisibility::LandscapeOnly => landscape,
        TabVisibility::PortraitOnly => !landscape,
    }
}

/// Bounds of the search input inside its padded row.
pub fn search_field(size: Size, insets: Insets) -> Rectangle {
    let width = content_width(size, insets);
    rect(
        insets.left + SEARCH_HORIZONTAL_PADDING,
        insets.top + 2.0,
        (width - 2.0 * SEARCH_HORIZONTAL_PADDING - SEARCH_GAP - SEARCH_CLOSE_WIDTH).max(0.0),
        SEARCH_CONTENT_HEIGHT,
    )
}

/// Bounds of the close button inside the search row.
pub fn search_close(size: Size, insets: Insets) -> Rectangle {
    let width = content_width(size, insets);
    rect(
        insets.left + width - SEARCH_HORIZONTAL_PADDING - SEARCH_CLOSE_WIDTH,
        insets.top + 2.0,
        SEARCH_CLOSE_WIDTH,
        SEARCH_CONTENT_HEIGHT,
    )
}

/// Bounds of the optional Frequent/Recent row under the search field.
pub fn search_tabs(size: Size, insets: Insets) -> Rectangle {
    rect(
        insets.left,
        insets.top + SEARCH_ROW_HEIGHT,
        content_width(size, insets),
        SEARCH_TABS_HEIGHT,
    )
}

/// One half of the Frequent/Recent row.
pub fn search_tab(size: Size, insets: Insets, recent: bool) -> Rectangle {
    let tabs = search_tabs(size, insets);
    rect(
        tabs.x + if recent { tabs.width / 2.0 } else { 0.0 },
        tabs.y,
        tabs.width / 2.0,
        tabs.height,
    )
}

pub fn title_strip(size: Size, insets: Insets, settings: &Settings) -> Rectangle {
    rect(
        insets.left,
        insets.top,
        content_width(size, insets),
        title_height(settings),
    )
}

pub fn folder_header(size: Size, insets: Insets, settings: &Settings) -> Rectangle {
    rect(
        insets.left,
        insets.top,
        content_width(size, insets),
        title_height(settings),
    )
}

pub fn folder_back(_size: Size, insets: Insets, settings: &Settings) -> Rectangle {
    rect(
        insets.left + FOLDER_PADDING,
        insets.top,
        FOLDER_BACK_WIDTH,
        title_height(settings),
    )
}

pub fn folder_title(size: Size, insets: Insets, settings: &Settings) -> Rectangle {
    let width = content_width(size, insets);
    rect(
        insets.left + FOLDER_PADDING + FOLDER_BACK_WIDTH,
        insets.top,
        (width - 2.0 * FOLDER_PADDING - FOLDER_BACK_WIDTH).max(0.0),
        title_height(settings),
    )
}

/// Top of the scrollable grid for the screen being drawn. Unlike
/// [`crate::App::body_top`], search has a real top: its chrome occupies rows.
pub fn body_top(
    size: Size,
    insets: Insets,
    settings: &Settings,
    searching: bool,
    search_history: bool,
    folder: bool,
    page_count: usize,
) -> f32 {
    let top = insets.top;
    if searching {
        return top
            + SEARCH_ROW_HEIGHT
            + if search_history {
                SEARCH_TABS_HEIGHT
            } else {
                0.0
            };
    }
    if folder {
        return top + title_height(settings);
    }
    if tabs_visible(settings, size, page_count) {
        top + title_height(settings)
    } else {
        top
    }
}

/// The scrollable area under the chrome and above the footer.
#[allow(clippy::too_many_arguments)]
pub fn viewport(
    size: Size,
    insets: Insets,
    settings: &Settings,
    searching: bool,
    search_history: bool,
    folder: bool,
    page_count: usize,
    footer_height: f32,
) -> Rectangle {
    let top = body_top(
        size,
        insets,
        settings,
        searching,
        search_history,
        folder,
        page_count,
    );
    rect(
        insets.left,
        top,
        content_width(size, insets),
        (size.height - top - insets.bottom - footer_height).max(0.0),
    )
}

/// Height of the transient error line plus every warning line.
pub fn footer_height(error: bool, warnings: usize) -> f32 {
    let error = if error { ERROR_LINE_HEIGHT } else { 0.0 };
    error + warnings as f32 * WARNING_LINE_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;
    use qlaunch_core::settings::{Settings, TabVisibility};

    fn insets() -> Insets {
        Insets {
            left: 10.0,
            right: 30.0,
            top: 50.0,
            bottom: 20.0,
        }
    }

    fn size() -> Size {
        Size::new(1080.0, 2400.0)
    }

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

    #[test]
    fn tabs_visibility_is_size_and_setting_driven() {
        let portrait = Size::new(1080.0, 2400.0);
        let landscape = Size::new(2400.0, 1080.0);
        let mut settings = Settings::default();
        assert!(tabs_visible(&settings, portrait, 3));
        assert!(!tabs_visible(&settings, portrait, 0));

        settings.tab_visibility = TabVisibility::Never;
        assert!(!tabs_visible(&settings, landscape, 3));

        settings.tab_visibility = TabVisibility::LandscapeOnly;
        assert!(tabs_visible(&settings, landscape, 3));
        assert!(!tabs_visible(&settings, portrait, 3));

        settings.tab_visibility = TabVisibility::PortraitOnly;
        assert!(tabs_visible(&settings, portrait, 3));
        assert!(!tabs_visible(&settings, landscape, 3));
    }

    #[test]
    fn search_geometry_uses_explicit_rows() {
        let insets = insets();
        let field = search_field(size(), insets);
        assert_eq!(field, rect(18.0, 52.0, 970.0, 40.0));
        let close = search_close(size(), insets);
        assert_eq!(close, rect(994.0, 52.0, 48.0, 40.0));
        let tabs = search_tabs(size(), insets);
        assert_eq!(tabs, rect(10.0, 94.0, 1040.0, 48.0));
        assert_eq!(
            search_tab(size(), insets, false),
            rect(10.0, 94.0, 520.0, 48.0)
        );
        assert_eq!(
            search_tab(size(), insets, true),
            rect(530.0, 94.0, 520.0, 48.0)
        );
    }

    #[test]
    fn title_and_folder_headers_use_title_height() {
        let settings = Settings::default();
        let height = title_height(&settings);
        assert_eq!(
            title_strip(size(), insets(), &settings),
            rect(10.0, 50.0, 1040.0, height)
        );
        assert_eq!(
            folder_header(size(), insets(), &settings),
            rect(10.0, 50.0, 1040.0, height)
        );
    }

    #[test]
    fn folder_back_and_title_split_the_header() {
        let settings = Settings::default();
        let height = title_height(&settings);
        assert_eq!(
            folder_back(size(), insets(), &settings),
            rect(18.0, 50.0, 56.0, height)
        );
        assert_eq!(
            folder_title(size(), insets(), &settings),
            rect(74.0, 50.0, 968.0, height)
        );
    }

    #[test]
    fn viewport_stops_above_footer_and_bottom_inset() {
        let settings = Settings {
            tab_visibility: TabVisibility::Never,
            ..Settings::default()
        };
        let area = viewport(
            Size::new(1080.0, 1000.0),
            insets(),
            &settings,
            false,
            false,
            false,
            3,
            36.0,
        );
        assert_eq!(area, rect(10.0, 50.0, 1040.0, 894.0));
    }

    #[test]
    fn footer_height_counts_error_and_warnings() {
        assert_eq!(footer_height(false, 0), 0.0);
        assert_eq!(footer_height(true, 2), 50.0);
        assert_eq!(footer_height(false, 3), 48.0);
    }
}
