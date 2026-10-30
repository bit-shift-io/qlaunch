//! Settings presentation and patches; the core owns validation and persistence.

use crate::{App, Message, Screen, components, theme};
use iced::widget::{column, container};
use iced::{Element, Fill};
use qlaunch_core::settings::{
    LabelColour, LayoutStyle, Orientation, ScrollFadeStyle, Settings, SettingsPatch, TabVisibility,
    Typeface,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingField {
    ContinuousScroll,
    PageTransitionMs,
    GridColumnsPortrait,
    GridColumnsLandscape,
    Layout,
    TabVisibility,
    ScrollFade,
    WallpaperAlpha,
    StatusBarVisible,
    Orientation,
    IconsVisible,
    IconSize,
    IconAlpha,
    LabelsVisible,
    LabelSize,
    LabelTypeface,
    LabelColour,
    LabelShadow,
    LabelShadowBlur,
}

impl SettingField {
    #[allow(dead_code)] // used by the coverage test
    pub(crate) fn all() -> [Self; 19] {
        use SettingField::*;
        [
            ContinuousScroll,
            PageTransitionMs,
            GridColumnsPortrait,
            GridColumnsLandscape,
            Layout,
            TabVisibility,
            ScrollFade,
            WallpaperAlpha,
            StatusBarVisible,
            Orientation,
            IconsVisible,
            IconSize,
            IconAlpha,
            LabelsVisible,
            LabelSize,
            LabelTypeface,
            LabelColour,
            LabelShadow,
            LabelShadowBlur,
        ]
    }
}

pub(crate) enum Row {
    Header(&'static str),
    Switch {
        field: SettingField,
        label: &'static str,
        value: bool,
    },
    Slider {
        field: SettingField,
        label: &'static str,
        value: i64,
        unit: &'static str,
        min: i64,
        max: i64,
    },
    Choice {
        field: SettingField,
        label: &'static str,
        options: Vec<(String, bool)>,
    },
    Action {
        label: &'static str,
        subtitle: &'static str,
        target: ActionTarget,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ActionTarget {
    Groups,
    Advanced,
    Wallpaper,
}

fn choices(field: SettingField, label: &'static str, labels: &[&str], selected: usize) -> Row {
    Row::Choice {
        field,
        label,
        options: labels
            .iter()
            .enumerate()
            .map(|(i, label)| (label.to_string(), i == selected))
            .collect(),
    }
}

pub(crate) fn rows(settings: &Settings, drag: Option<(SettingField, f32)>) -> Vec<Row> {
    use SettingField::*;
    let switch = |field, label, value| Row::Switch {
        field,
        label,
        value,
    };
    let number = |field, label, value, min, max, unit| {
        let shown = match drag {
            Some((active, value)) if active == field => value.round() as i64,
            _ => value,
        };
        Row::Slider {
            field,
            label,
            value: shown.clamp(min, max),
            min,
            max,
            unit,
        }
    };
    vec![
        Row::Header("Display"),
        switch(
            ContinuousScroll,
            "Continuous scrolling",
            settings.continuous_scroll,
        ),
        number(
            PageTransitionMs,
            "Page transition duration",
            settings.page_transition_ms,
            100,
            800,
            " ms",
        ),
        number(
            GridColumnsPortrait,
            "Portrait columns",
            settings.grid_columns_portrait,
            1,
            12,
            "",
        ),
        number(
            GridColumnsLandscape,
            "Landscape columns",
            settings.grid_columns_landscape,
            1,
            12,
            "",
        ),
        choices(
            Layout,
            "Cell layout",
            &["Tiles", "List", "Centred"],
            settings.layout as usize,
        ),
        choices(
            TabVisibility,
            "Tab strip",
            &["Always", "Landscape", "Portrait", "Never"],
            settings.tab_visibility as usize,
        ),
        choices(
            ScrollFade,
            "Scroll fade",
            &["None", "Gradient", "Tile"],
            settings.scroll_fade as usize,
        ),
        Row::Header("Window"),
        Row::Action {
            label: "Choose wallpaper",
            subtitle: "Pick one with the system wallpaper chooser.",
            target: ActionTarget::Wallpaper,
        },
        number(
            WallpaperAlpha,
            "Wallpaper dimming",
            settings.wallpaper_alpha,
            0,
            100,
            "",
        ),
        switch(
            StatusBarVisible,
            "Show the status bar",
            settings.status_bar_visible,
        ),
        choices(
            Orientation,
            "Requested orientation",
            &["Auto", "Landscape", "Portrait"],
            settings.orientation as usize,
        ),
        Row::Header("Icon"),
        switch(IconsVisible, "Show icons", settings.icons_visible),
        number(IconSize, "Icon size", settings.icon_size, 0, 100, ""),
        number(IconAlpha, "Icon opacity", settings.icon_alpha, 0, 100, ""),
        Row::Header("Text"),
        switch(LabelsVisible, "Show labels", settings.labels_visible),
        number(LabelSize, "Font size", settings.label_size, 0, 100, ""),
        choices(
            LabelTypeface,
            "Typeface",
            &["Normal", "Sans", "Serif", "Mono"],
            settings.label_typeface as usize,
        ),
        choices(
            LabelColour,
            "Colour",
            &["White", "Black", "Grey"],
            settings.label_colour as usize,
        ),
        switch(LabelShadow, "Label shadow", settings.label_shadow),
        number(
            LabelShadowBlur,
            "Shadow blur",
            settings.label_shadow_blur,
            0,
            10,
            "",
        ),
        Row::Header("Groups"),
        Row::Action {
            label: "Groups",
            subtitle: "Add, rename, reorder and fill the tabs.",
            target: ActionTarget::Groups,
        },
        Row::Header("Advanced"),
        Row::Action {
            label: "Advanced",
            subtitle: "Restart, backup, restore and factory reset.",
            target: ActionTarget::Advanced,
        },
    ]
}

pub(crate) fn message(field: SettingField, value: i64) -> SettingsPatch {
    let mut patch = SettingsPatch::default();
    match field {
        SettingField::ContinuousScroll => patch.continuous_scroll = Some(value != 0),
        SettingField::PageTransitionMs => patch.page_transition_ms = Some(value),
        SettingField::GridColumnsPortrait => patch.grid_columns_portrait = Some(value),
        SettingField::GridColumnsLandscape => patch.grid_columns_landscape = Some(value),
        SettingField::WallpaperAlpha => patch.wallpaper_alpha = Some(value),
        SettingField::StatusBarVisible => patch.status_bar_visible = Some(value != 0),
        SettingField::IconsVisible => patch.icons_visible = Some(value != 0),
        SettingField::IconSize => patch.icon_size = Some(value),
        SettingField::IconAlpha => patch.icon_alpha = Some(value),
        SettingField::LabelsVisible => patch.labels_visible = Some(value != 0),
        SettingField::LabelSize => patch.label_size = Some(value),
        SettingField::LabelShadow => patch.label_shadow = Some(value != 0),
        SettingField::LabelShadowBlur => patch.label_shadow_blur = Some(value),
        _ => return choice_patch(field, value as usize),
    }
    patch
}

pub(crate) fn choice_patch(field: SettingField, index: usize) -> SettingsPatch {
    let mut patch = SettingsPatch::default();
    match field {
        SettingField::Layout => {
            patch.layout = [LayoutStyle::Tiles, LayoutStyle::List, LayoutStyle::Centred]
                .get(index)
                .copied()
        }
        SettingField::TabVisibility => {
            patch.tab_visibility = [
                TabVisibility::Always,
                TabVisibility::LandscapeOnly,
                TabVisibility::PortraitOnly,
                TabVisibility::Never,
            ]
            .get(index)
            .copied()
        }
        SettingField::ScrollFade => {
            patch.scroll_fade = [
                ScrollFadeStyle::None,
                ScrollFadeStyle::Gradient,
                ScrollFadeStyle::Tile,
            ]
            .get(index)
            .copied()
        }
        SettingField::Orientation => {
            patch.orientation = [
                Orientation::Auto,
                Orientation::Landscape,
                Orientation::Portrait,
            ]
            .get(index)
            .copied()
        }
        SettingField::LabelTypeface => {
            patch.label_typeface = [
                Typeface::Normal,
                Typeface::Sans,
                Typeface::Serif,
                Typeface::Monospace,
            ]
            .get(index)
            .copied()
        }
        SettingField::LabelColour => {
            patch.label_colour = [LabelColour::White, LabelColour::Black, LabelColour::Grey]
                .get(index)
                .copied()
        }
        _ => {}
    }
    patch
}

pub(crate) fn row_height(row: &Row, text: &theme::Text) -> f32 {
    let body = text.role(theme::TextRole::BodyLarge).line_height;
    match row {
        Row::Header(_) => text.role(theme::TextRole::TitleSmall).line_height + 48.0,
        Row::Switch { .. } => (body + 16.0).max(theme::metrics::LIST_ONE_LINE),
        Row::Slider { .. } => body + 24.0 + theme::metrics::SLIDER_HEIGHT,
        Row::Choice { options, .. } => {
            body + 24.0
                + options.len().div_ceil(2) as f32
                    * (text
                        .role(theme::TextRole::LabelLarge)
                        .line_height
                        .max(theme::metrics::CHIP_HEIGHT)
                        + 8.0)
                - 8.0
        }
        Row::Action { .. } => (body + text.role(theme::TextRole::BodyMedium).line_height + 16.0)
            .max(theme::metrics::LIST_TWO_LINE),
    }
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let Some(snapshot) = &app.snapshot else {
        return container(column![]).into();
    };
    let typography = theme::Text::new();
    let mut content = column![];
    for row in rows(&snapshot.settings, app.settings_drag) {
        let height = row_height(&row, &typography);
        let element = match row {
            Row::Header(label) => components::section_header(&typography, label),
            Row::Switch {
                field,
                label,
                value,
            } => components::switch_row(
                &typography,
                label,
                value,
                Message::SettingPatch(message(field, i64::from(!value))),
                true,
            ),
            Row::Slider {
                field,
                label,
                value,
                unit,
                min,
                max,
            } => components::slider_row(
                &typography,
                label,
                value as f32,
                unit,
                min as f32..=max as f32,
                move |value| Message::SettingDrag(field, value),
                Message::SettingRelease(field),
            ),
            Row::Choice {
                field,
                label,
                options,
            } => components::choice_row(&typography, label, &options, move |index| {
                Message::SettingPatch(choice_patch(field, index))
            }),
            Row::Action {
                label,
                subtitle,
                target,
            } => components::action_tile(
                &typography,
                label,
                subtitle,
                match target {
                    ActionTarget::Groups => Message::Push(Screen::Groups),
                    ActionTarget::Advanced => Message::Push(Screen::Advanced),
                    ActionTarget::Wallpaper => {
                        Message::Dialog(crate::dialog::Message::LauncherWallpaper)
                    }
                },
            ),
        };
        content = content.push(container(element).width(Fill).height(height));
    }
    content = content.push(iced::widget::space().height(32));
    super::scaffold(app, "Launcher Settings", vec![], content.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_cover_every_setting_once_in_order() {
        let rows = rows(&Settings::default(), None);
        let controls: Vec<_> = rows
            .iter()
            .filter_map(|row| match row {
                Row::Switch { field, .. }
                | Row::Slider { field, .. }
                | Row::Choice { field, .. } => Some(*field),
                _ => None,
            })
            .collect();
        assert_eq!(controls, SettingField::all());
    }

    #[test]
    fn rows_are_grouped_display_window_icon_text_groups_advanced() {
        let rows = rows(&Settings::default(), None);
        let headers: Vec<_> = rows
            .iter()
            .filter_map(|row| match row {
                Row::Header(h) => Some(*h),
                _ => None,
            })
            .collect();
        assert_eq!(
            headers,
            ["Display", "Window", "Icon", "Text", "Groups", "Advanced"]
        );
    }

    #[test]
    fn slider_range_matches_core_clamp() {
        let rows = rows(&Settings::default(), None);
        let range = rows.iter().find_map(|row| match row {
            Row::Slider {
                field: SettingField::PageTransitionMs,
                min,
                max,
                ..
            } => Some((*min, *max)),
            _ => None,
        });
        assert_eq!(range, Some((100, 800)));
    }

    #[test]
    fn drag_value_overrides_the_saved_value() {
        let rows = rows(
            &Settings::default(),
            Some((SettingField::PageTransitionMs, 480.0)),
        );
        let shown = rows.iter().find_map(|row| match row {
            Row::Slider {
                field: SettingField::PageTransitionMs,
                value,
                ..
            } => Some(*value),
            _ => None,
        });
        assert_eq!(shown, Some(480));
    }

    #[test]
    fn switch_and_choice_patches_target_the_field() {
        assert_eq!(
            message(SettingField::IconsVisible, 0),
            SettingsPatch {
                icons_visible: Some(false),
                ..Default::default()
            }
        );
        assert_eq!(
            message(SettingField::LabelShadowBlur, 3),
            SettingsPatch {
                label_shadow_blur: Some(3),
                ..Default::default()
            }
        );
        assert_eq!(
            choice_patch(SettingField::Layout, 2),
            SettingsPatch {
                layout: Some(LayoutStyle::Centred),
                ..Default::default()
            }
        );
    }

    #[test]
    fn slider_drag_is_transient_and_completion_clears_it() {
        let mut app = crate::tests::app();
        let saved = app.snapshot.as_ref().unwrap().settings.page_transition_ms;
        let _ = app.update(Message::SettingDrag(SettingField::PageTransitionMs, 620.0));
        assert_eq!(
            app.snapshot.as_ref().unwrap().settings.page_transition_ms,
            saved
        );
        assert_eq!(
            app.settings_drag,
            Some((SettingField::PageTransitionMs, 620.0))
        );
        let _ = app.update(Message::SettingsDone(Err("cannot save".into())));
        assert_eq!(app.settings_drag, None);
        assert_eq!(app.snackbar.as_ref().unwrap().1, "cannot save");
    }

    #[test]
    fn row_heights_are_positive() {
        // Chrome text is fixed now, so the heights no longer track label_size.
        let settings = Settings::default();
        let typography = theme::Text::new();
        for row in rows(&settings, None) {
            assert!(row_height(&row, &typography) > 0.0);
            if let Row::Switch { .. } = row {
                assert!(
                    row_height(&row, &typography)
                        >= typography.role(theme::TextRole::BodyLarge).line_height + 16.0
                );
            }
        }
    }
}
