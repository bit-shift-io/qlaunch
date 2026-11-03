//! Settings presentation and patches; the core owns validation and persistence.

use crate::{App, Message, Screen, components, icons, theme};
use iced::widget::{column, container};
use iced::{Element, Fill};
use qlaunch_core::settings::{
    GestureAction, LabelColour, LayoutStyle, Orientation, ScrollFadeStyle, Settings, SettingsPatch,
    TabVisibility, Typeface,
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
    HomeAction,
    SwipeUpAction,
    PullDownAction,
    DoubleTapAction,
    DashboardWidget,
    DashboardOpacity,
    DashboardCalendarRefresh,
    DashboardWeatherRefresh,
    WeatherLocation,
}

impl SettingField {
    #[allow(dead_code)] // used by the coverage test
    pub(crate) fn all() -> [Self; 28] {
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
            HomeAction,
            SwipeUpAction,
            PullDownAction,
            DoubleTapAction,
            DashboardWidget,
            DashboardOpacity,
            DashboardCalendarRefresh,
            DashboardWeatherRefresh,
            WeatherLocation,
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
    TextField {
        field: SettingField,
        label: &'static str,
        value: String,
        placeholder: &'static str,
    },
    Action {
        label: &'static str,
        subtitle: String,
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
        Row::Header("Actions"),
        choices(
            HomeAction,
            "Home button",
            &["None", "Lock", "Search", "Notifications"],
            settings.home_action as usize,
        ),
        choices(
            SwipeUpAction,
            "Swipe up",
            &["None", "Lock", "Search", "Notifications"],
            settings.swipe_up_action as usize,
        ),
        choices(
            PullDownAction,
            "Swipe down",
            &["None", "Lock", "Search", "Notifications"],
            settings.pull_down_action as usize,
        ),
        choices(
            DoubleTapAction,
            "Double tap",
            &["None", "Lock", "Search", "Notifications"],
            settings.double_tap_action as usize,
        ),
        Row::Header("Dashboard"),
        switch(
            DashboardWidget,
            "Show dashboard widget",
            settings.dashboard_widget_enabled,
        ),
        number(
            DashboardOpacity,
            "Card opacity",
            settings.dashboard_opacity,
            0,
            100,
            "%",
        ),
        number(
            DashboardCalendarRefresh,
            "Calendar refresh",
            settings.dashboard_calendar_refresh_minutes,
            1,
            240,
            " min",
        ),
        number(
            DashboardWeatherRefresh,
            "Weather refresh",
            settings.dashboard_weather_refresh_minutes,
            5,
            240,
            " min",
        ),
        Row::TextField {
            field: WeatherLocation,
            label: "Weather location",
            value: settings.weather_location.clone(),
            placeholder: "City or suburb, e.g. Adelaide",
        },
        Row::Action {
            label: "Choose wallpaper",
            subtitle: "Pick one with the system wallpaper chooser.".into(),
            target: ActionTarget::Wallpaper,
        },
        Row::Action {
            label: "Groups",
            subtitle: "Add, rename, reorder and fill the tabs.".into(),
            target: ActionTarget::Groups,
        },
        Row::Action {
            label: "Advanced",
            subtitle: "Restart, backup, restore and factory reset.".into(),
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
        SettingField::DashboardWidget => patch.dashboard_widget_enabled = Some(value != 0),
        SettingField::DashboardOpacity => patch.dashboard_opacity = Some(value),
        SettingField::DashboardCalendarRefresh => {
            patch.dashboard_calendar_refresh_minutes = Some(value)
        }
        SettingField::DashboardWeatherRefresh => {
            patch.dashboard_weather_refresh_minutes = Some(value)
        }
        _ => return choice_patch(field, value as usize),
    }
    patch
}

/// A settings patch for the weather location, applied once typing settles.
pub(crate) fn location_patch(value: String) -> SettingsPatch {
    SettingsPatch {
        weather_location: Some(value),
        ..Default::default()
    }
}

/// The text input callback exposed to accessibility, which needs a plain
/// function pointer rather than a closure.
pub(crate) fn weather_location_message(value: String) -> crate::Message {
    crate::Message::DashboardLocationDraft(value)
}

/// The action choices every gesture row offers, in display order.
fn gesture_choices(index: usize) -> Option<GestureAction> {
    [
        GestureAction::None,
        GestureAction::Lock,
        GestureAction::Search,
        GestureAction::Notifications,
    ]
    .get(index)
    .copied()
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
        SettingField::HomeAction => patch.home_action = gesture_choices(index),
        SettingField::SwipeUpAction => patch.swipe_up_action = gesture_choices(index),
        SettingField::PullDownAction => patch.pull_down_action = gesture_choices(index),
        SettingField::DoubleTapAction => patch.double_tap_action = gesture_choices(index),
        _ => {}
    }
    patch
}

pub(crate) fn row_height(row: &Row, text: &theme::Text, available: f32) -> f32 {
    let body = text.role(theme::TextRole::BodyLarge).line_height;
    match row {
        Row::Header(_) => {
            text.role(theme::TextRole::TitleSmall).line_height
                + theme::metrics::SETTINGS_HEADER_TOP
                + theme::metrics::SETTINGS_HEADER_BOTTOM
        }
        Row::Switch { .. } => (body + 16.0).max(theme::metrics::LIST_ONE_LINE),
        Row::Slider { .. } => {
            body + theme::metrics::SETTINGS_ROW_PADDING + theme::metrics::SLIDER_HEIGHT
        }
        Row::Choice { options, .. } => {
            let text_size = text.role(theme::TextRole::LabelLarge).size;
            let lines = components::chip_rows(options, available, text_size)
                .len()
                .max(1) as f32;
            body + theme::metrics::SETTINGS_ROW_PADDING * 2.0
                + theme::metrics::SETTINGS_LABEL_GAP
                + lines
                    * (text
                        .role(theme::TextRole::LabelLarge)
                        .line_height
                        .max(theme::metrics::CHIP_HEIGHT)
                        + components::CHIP_SPACING)
                - components::CHIP_SPACING
        }
        Row::TextField { .. } => {
            body + theme::metrics::FIELD_HEIGHT + theme::metrics::SETTINGS_ROW_PADDING
        }
        Row::Action {
            label, subtitle, ..
        } => {
            // `available` already excludes the tile's horizontal padding.
            // Reserve the same icon and gap as components::list_tile.
            let width = (available - theme::metrics::ICON_SIZE - 16.0).max(1.0);
            let title = text.role(theme::TextRole::BodyLarge);
            let subtitle_style = text.role(theme::TextRole::BodyMedium);
            (components::wrapped_lines(label, title.size, width) * title.line_height
                + components::wrapped_lines(subtitle, subtitle_style.size, width)
                    * subtitle_style.line_height
                + 16.0)
                .max(theme::metrics::LIST_TWO_LINE)
                + theme::metrics::ACTION_GAP
        }
    }
}

/// Conservative word wrapping, matching the fixed chrome font's approximate
/// advance. Unlike a character count, this accounts for words moving together.
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let Some(snapshot) = &app.snapshot else {
        return container(column![]).into();
    };
    let typography = theme::Text::new();
    let side = theme::metrics::SETTINGS_SIDE_MARGIN;
    let right = side + theme::metrics::SETTINGS_SCROLLBAR_GUTTER;
    let available =
        components::chip_available(crate::grid::content_width(app.size, app.insets) - side - right);
    let motion = app.motion(crate::Region::Screen);
    let mut content = column![];
    let mut group = Vec::new();
    for row in rows(&snapshot.settings, app.settings_drag) {
        // Headers stay on the page surface; only their controls share a card.
        if matches!(row, Row::Header(_) | Row::Action { .. }) && !group.is_empty() {
            content = content.push(crate::elastic_scroll::row(
                container(column(std::mem::take(&mut group)))
                    .width(Fill)
                    .style(theme::settings_card)
                    .clip(true)
                    .into(),
                &motion,
            ));
        }
        let height = row_height(&row, &typography, available);
        let is_header = matches!(row, Row::Header(_));
        let is_action = matches!(row, Row::Action { .. });
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
            } => components::choice_row(&typography, label, &options, available, move |index| {
                Message::SettingPatch(choice_patch(field, index))
            }),
            Row::TextField {
                field: _,
                label,
                value,
                placeholder,
            } => {
                let value = app.dashboard_location_draft.clone().unwrap_or(value);
                container(components::text_field(
                    &typography,
                    label,
                    &value,
                    placeholder,
                    "weather-location",
                    Message::DashboardLocationDraft,
                    Message::CommitDashboardLocation,
                ))
                .padding([0, 16])
                .width(Fill)
                .into()
            }
            Row::Action {
                label,
                subtitle,
                target,
            } => components::action_card(
                &typography,
                label,
                &subtitle,
                match target {
                    ActionTarget::Groups => icons::DASHBOARD_CUSTOMIZE,
                    ActionTarget::Advanced => icons::SETTINGS,
                    ActionTarget::Wallpaper => icons::WALLPAPER,
                },
                match target {
                    ActionTarget::Groups => Message::Push(Screen::Groups),
                    ActionTarget::Advanced => Message::Push(Screen::Advanced),
                    ActionTarget::Wallpaper => {
                        Message::Dialog(crate::dialog::Message::LauncherWallpaper)
                    }
                },
                available,
            ),
        };
        if is_action {
            content = content.push(crate::elastic_scroll::row(element, &motion));
        } else if is_header {
            content = content.push(crate::elastic_scroll::row(
                container(element).width(Fill).height(height).into(),
                &motion,
            ));
        } else {
            group.push(container(element).width(Fill).height(height).into());
        }
    }
    content = content.push(iced::widget::space().height(32));
    super::scaffold(
        app,
        "Launcher Settings",
        vec![],
        container(content)
            .padding(iced::Padding {
                left: side,
                right,
                ..Default::default()
            })
            .into(),
    )
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
                | Row::Choice { field, .. }
                | Row::TextField { field, .. } => Some(*field),
                _ => None,
            })
            .collect();
        assert_eq!(controls, SettingField::all());
    }

    #[test]
    fn headings_label_the_settings_groups_not_individual_actions() {
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
            ["Display", "Window", "Icon", "Text", "Actions", "Dashboard"]
        );
    }

    #[test]
    fn action_rows_offer_none_lock_search_and_notifications() {
        use qlaunch_core::settings::GestureAction;
        let rows = rows(&Settings::default(), None);
        let actions = [
            SettingField::HomeAction,
            SettingField::SwipeUpAction,
            SettingField::PullDownAction,
            SettingField::DoubleTapAction,
        ];
        for field in actions {
            let options = rows.iter().find_map(|row| match row {
                Row::Choice {
                    field: f, options, ..
                } if *f == field => Some(options.clone()),
                _ => None,
            });
            let Some(options) = options else {
                panic!("{field:?} has no choice row");
            };
            let labels: Vec<&str> = options.iter().map(|(label, _)| label.as_str()).collect();
            assert_eq!(labels, ["None", "Lock", "Search", "Notifications"]);
        }
        assert_eq!(
            choice_patch(SettingField::HomeAction, 3),
            SettingsPatch {
                home_action: Some(GestureAction::Notifications),
                ..Default::default()
            }
        );
        assert_eq!(
            choice_patch(SettingField::SwipeUpAction, 0),
            SettingsPatch {
                swipe_up_action: Some(GestureAction::None),
                ..Default::default()
            }
        );
        assert_eq!(
            choice_patch(SettingField::PullDownAction, 3),
            SettingsPatch {
                pull_down_action: Some(GestureAction::Notifications),
                ..Default::default()
            }
        );
        assert_eq!(
            choice_patch(SettingField::DoubleTapAction, 1),
            SettingsPatch {
                double_tap_action: Some(GestureAction::Lock),
                ..Default::default()
            }
        );
    }

    #[test]
    fn action_rows_mark_the_saved_action_as_selected() {
        let settings = Settings {
            home_action: qlaunch_core::settings::GestureAction::Lock,
            swipe_up_action: qlaunch_core::settings::GestureAction::Search,
            pull_down_action: qlaunch_core::settings::GestureAction::Notifications,
            double_tap_action: qlaunch_core::settings::GestureAction::None,
            ..Settings::default()
        };
        let selected = |field| {
            rows(&settings, None)
                .into_iter()
                .find_map(|row| match row {
                    Row::Choice {
                        field: f, options, ..
                    } if f == field => options.iter().position(|(_, selected)| *selected),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(selected(SettingField::HomeAction), 1);
        assert_eq!(selected(SettingField::SwipeUpAction), 2);
        assert_eq!(selected(SettingField::PullDownAction), 3);
        assert_eq!(selected(SettingField::DoubleTapAction), 0);
    }

    #[test]
    fn dashboard_actions_then_bottom_cards_follow_all_settings() {
        let rows = rows(&Settings::default(), None);
        let first_action = rows
            .iter()
            .position(|row| matches!(row, Row::Action { .. }))
            .unwrap();
        let actions: Vec<_> = rows[first_action..]
            .iter()
            .map(|row| match row {
                Row::Action { label, .. } => *label,
                _ => panic!("settings must precede the bottom action cards"),
            })
            .collect();
        assert_eq!(actions, ["Choose wallpaper", "Groups", "Advanced"]);
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
    fn dashboard_switch_and_location_patch_the_dashboard_settings() {
        assert_eq!(
            message(SettingField::DashboardWidget, 1),
            SettingsPatch {
                dashboard_widget_enabled: Some(true),
                ..Default::default()
            }
        );
        assert_eq!(
            location_patch("5000".to_string()),
            SettingsPatch {
                weather_location: Some("5000".to_string()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn dashboard_rows_show_the_saved_location() {
        let settings = Settings {
            dashboard_widget_enabled: true,
            weather_location: "Adelaide".into(),
            preferred_clock_app: Some("com.example.clock".into()),
            ..Settings::default()
        };
        let rows = rows(&settings, None);
        assert!(rows.iter().any(|row| matches!(
            row,
            Row::TextField { value, .. } if value == "Adelaide"
        )));
    }

    #[test]
    fn dashboard_refresh_rows_patch_their_own_minutes() {
        assert_eq!(
            message(SettingField::DashboardCalendarRefresh, 10),
            SettingsPatch {
                dashboard_calendar_refresh_minutes: Some(10),
                ..Default::default()
            }
        );
        assert_eq!(
            message(SettingField::DashboardWeatherRefresh, 90),
            SettingsPatch {
                dashboard_weather_refresh_minutes: Some(90),
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
    fn action_cards_grow_to_fit_subtitles_on_narrow_screens() {
        let typography = theme::Text::new();
        let mut grown = 0;
        for action in rows(&Settings::default(), None)
            .iter()
            .filter(|r| matches!(r, Row::Action { .. }))
        {
            let narrow = row_height(action, &typography, 168.0);
            let wide = row_height(action, &typography, 388.0);
            assert!(narrow >= wide);
            if narrow > wide {
                grown += 1;
            }
        }
        // The longer subtitles wrap onto extra lines; short dashboard rows fit
        // on one line at both widths.
        assert!(grown >= 3, "expected some actions to wrap, got {grown}");
    }

    #[test]
    fn row_heights_are_positive() {
        // Chrome text is fixed now, so the heights no longer track label_size.
        let settings = Settings::default();
        let typography = theme::Text::new();
        for row in rows(&settings, None) {
            assert!(row_height(&row, &typography, 388.0) > 0.0);
            if let Row::Switch { .. } = row {
                assert!(
                    row_height(&row, &typography, 388.0)
                        >= typography.role(theme::TextRole::BodyLarge).line_height + 16.0
                );
            }
        }
    }

    #[test]
    fn a_choice_row_grows_to_a_second_line_when_the_width_is_tight() {
        let settings = Settings::default();
        let typography = theme::Text::new();
        let typeface = rows(&settings, None)
            .into_iter()
            .find(|row| {
                matches!(
                    row,
                    Row::Choice {
                        field: SettingField::LabelTypeface,
                        ..
                    }
                )
            })
            .unwrap();
        let wide = row_height(&typeface, &typography, 388.0);
        let narrow = row_height(&typeface, &typography, 120.0);
        assert!(narrow > wide, "a tighter row wraps onto more lines");
    }
}
