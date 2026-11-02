//! Dashboard widget: modernized minimal HTC-inspired clock, alarm, Australian BOM weather and calendar events.

use iced::alignment::Vertical;
use iced::widget::{column, container, mouse_area, row, scrollable, space, text};
use iced::{Color, Element, Fill, Length};
use qlaunch_core::platform::android::{
    AlarmInfo, AppChoice, CalendarEventItem, CalendarEventsPayload, TimeInfo, WeatherData,
};

use crate::{App, Message, icons};

/// Timed events may stay visible for this long after they start, so a meeting
/// that just began isn't yanked off the list mid-glance.
const PAST_EVENT_GRACE_MS: i64 = 10 * 60 * 1000;

/// Drops timed events whose start is more than [`PAST_EVENT_GRACE_MS`] before
/// `now_ms`. All-day events are kept: their midnight start is not "late".
pub(crate) fn filter_past_events(events: &mut Vec<CalendarEventItem>, now_ms: i64) {
    let cutoff = now_ms - PAST_EVENT_GRACE_MS;
    events.retain(|event| event.all_day || event.begin >= cutoff);
}

/// Splits a formatted clock time into its digits and an optional am/pm suffix,
/// so the suffix can be drawn smaller than the time.
pub(crate) fn split_time(time: &str) -> (String, Option<String>) {
    if let Some((main, suffix)) = time.rsplit_once(' ')
        && (suffix.eq_ignore_ascii_case("am") || suffix.eq_ignore_ascii_case("pm"))
    {
        return (main.to_string(), Some(suffix.to_string()));
    }
    (time.to_string(), None)
}

/// The colour of the calendar event currently in progress, if any. When two
/// overlap, the one that started most recently wins.
pub(crate) fn active_event_color(events: &[CalendarEventItem], now_ms: i64) -> Option<Color> {
    events
        .iter()
        .filter(|event| event.begin <= now_ms && now_ms < event.end)
        .max_by_key(|event| event.begin)
        .map(|event| {
            let argb = event.color as i32;
            Color::from_rgba(
                ((argb >> 16) & 0xFF) as f32 / 255.0,
                ((argb >> 8) & 0xFF) as f32 / 255.0,
                (argb & 0xFF) as f32 / 255.0,
                1.0,
            )
        })
}

/// Wall-clock epoch milliseconds.
pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn condition_symbol(code: i32) -> &'static str {
    match code {
        0 => "☀️",
        1..=3 => "⛅",
        45 | 48 => "🌫️",
        51..=57 => "🌦️",
        61..=67 => "🌧️",
        71..=77 => "❄️",
        80..=82 => "🌧️",
        85 | 86 => "🌨️",
        95..=99 => "⛈️",
        _ => "🌤️",
    }
}

// --- Platform calls ---

#[cfg(target_os = "android")]
pub(crate) fn current_time() -> Option<TimeInfo> {
    qlaunch_core::platform::android::jni_host::JniHost::current_time()
}

#[cfg(not(target_os = "android"))]
pub(crate) fn current_time() -> Option<TimeInfo> {
    Some(TimeInfo {
        time: "10:14".to_string(),
        date: "Mon, 2 Nov".to_string(),
    })
}

#[cfg(target_os = "android")]
pub(crate) fn next_alarm() -> Option<AlarmInfo> {
    qlaunch_core::platform::android::jni_host::JniHost::next_alarm()
}

#[cfg(not(target_os = "android"))]
pub(crate) fn next_alarm() -> Option<AlarmInfo> {
    Some(AlarmInfo {
        trigger_time: 0,
        time_str: "06:30".to_string(),
    })
}

#[cfg(target_os = "android")]
pub(crate) fn calendar_events() -> Result<CalendarEventsPayload, String> {
    qlaunch_core::platform::android::jni_host::JniHost::calendar_events().map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
pub(crate) fn calendar_events() -> Result<CalendarEventsPayload, String> {
    Ok(CalendarEventsPayload {
        permission_granted: true,
        events: vec![
            CalendarEventItem {
                event_id: 1,
                title: "Design Team Standup".to_string(),
                begin: 0,
                end: 0,
                all_day: false,
                time_str: "11:30".to_string(),
                color: 0xFF4285F4u32 as i32,
                tomorrow: false,
            },
            CalendarEventItem {
                event_id: 2,
                title: "Sprint Review".to_string(),
                begin: 0,
                end: 0,
                all_day: false,
                time_str: "14:00".to_string(),
                color: 0xFF0B8043u32 as i32,
                tomorrow: false,
            },
        ],
    })
}

#[cfg(target_os = "android")]
pub(crate) fn fetch_weather(location: &str) -> Result<WeatherData, String> {
    qlaunch_core::platform::android::jni_host::JniHost::fetch_weather(location)
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
pub(crate) fn fetch_weather(location: &str) -> Result<WeatherData, String> {
    Ok(WeatherData {
        location_label: if location.is_empty() {
            "Adelaide".to_string()
        } else {
            location.to_string()
        },
        current_temp: 24.0,
        temp_min: 15.0,
        temp_max: 27.0,
        condition_code: 1,
    })
}

#[cfg(target_os = "android")]
pub(crate) fn category_apps(category: &str) -> Result<Vec<AppChoice>, String> {
    qlaunch_core::platform::android::jni_host::JniHost::category_apps(category)
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
pub(crate) fn category_apps(_category: &str) -> Result<Vec<AppChoice>, String> {
    Ok(vec![AppChoice {
        package: "com.google.android.deskclock".to_string(),
        label: "Clock".to_string(),
        is_recommended: true,
    }])
}

#[cfg(target_os = "android")]
pub(crate) fn launch_category_app(
    category: &str,
    package: Option<&str>,
    event_id: i64,
) -> Result<(), String> {
    qlaunch_core::platform::android::jni_host::JniHost::launch_category_app(
        category, package, event_id,
    )
}

#[cfg(not(target_os = "android"))]
pub(crate) fn launch_category_app(
    _category: &str,
    _package: Option<&str>,
    _event_id: i64,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
pub(crate) fn request_calendar_permission() -> Result<(), String> {
    qlaunch_core::platform::android::jni_host::JniHost::request_calendar_permission()
}

#[cfg(not(target_os = "android"))]
pub(crate) fn request_calendar_permission() -> Result<(), String> {
    Ok(())
}

// --- Text with Drop Shadow for legibility over any wallpaper ---

fn shadowed_text<'a>(
    content: impl Into<String>,
    size: f32,
    color: Color,
    bold: bool,
) -> Element<'a, Message> {
    let owned = content.into();
    let _ = bold;
    // Do NOT set an explicit `.font(...)`. On this Android iced build,
    // `iced::Font::DEFAULT` resolves to a face that draws no glyphs, so the
    // clock/date/calendar text disappeared. The app's default font renders fine.
    let fg = text(owned).size(size).color(color);

    fg.into()
}

// --- Main Widget View ---

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let text_color = Color::WHITE;
    let muted_color = Color::from_rgba(1.0, 1.0, 1.0, 0.75);

    // 1. Clock & Date
    let time_str = app.dashboard_time.as_deref().unwrap_or("10:14");
    let date_str = app.dashboard_date.as_deref().unwrap_or("Mon, 2 Nov");

    let alarm_label = app
        .dashboard_alarm
        .as_ref()
        .map(|alarm| alarm.time_str.clone())
        .unwrap_or_else(|| "None".to_string());

    let (clock_main, _) = split_time(time_str);
    // Tight line height lifts the large digits so their top lines up with the
    // first calendar row. The am/pm suffix is intentionally not drawn.
    let time_row = row![Element::from(
        text(clock_main).size(28.0).color(text_color).line_height(1.0)
    )];

    let alarm_line = row![
        text(icons::ALARM.to_string())
            .font(icons::FONT)
            .size(15.0)
            .color(muted_color),
        shadowed_text(alarm_label, 15.0, muted_color, false),
    ]
    .spacing(6)
    .align_y(Vertical::Center);

    let clock_column = column![
        time_row,
        shadowed_text(date_str, 15.0, muted_color, false),
        alarm_line,
    ]
    .spacing(2)
    .align_x(iced::alignment::Horizontal::Center);

    // 2. Weather
    let weather_element: Element<'_, Message> = if let Some(weather) = &app.dashboard_weather {
        let symbol = condition_symbol(weather.condition_code);
        let temp_line = format!("{} {:.0}°", symbol, weather.current_temp);
        let detail_line = format!("{:.0}° / {:.0}°", weather.temp_min, weather.temp_max);

        mouse_area(
            column![
                shadowed_text(temp_line, 24.0, text_color, false),
                shadowed_text(detail_line, 15.0, muted_color, false),
            ]
            .spacing(3)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .on_release(Message::LaunchDashboardCategory("weather".to_string()))
        .into()
    } else if let Some(err) = &app.dashboard_weather_error {
        mouse_area(
            column![
                shadowed_text("Weather", 16.0, text_color, true),
                shadowed_text(err, 12.0, Color::from_rgb8(255, 180, 180), false),
            ]
            .spacing(2)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .on_release(Message::LaunchDashboardCategory("weather".to_string()))
        .into()
    } else {
        let configured = app
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| !snapshot.settings.weather_location.trim().is_empty());
        let (subtitle, message) = if configured {
            (
                "Updating…",
                Message::LaunchDashboardCategory("weather".to_string()),
            )
        } else {
            (
                "Tap to set location",
                Message::Push(crate::Screen::Settings),
            )
        };
        mouse_area(
            column![
                shadowed_text("Weather", 16.0, text_color, true),
                shadowed_text(subtitle, 12.0, muted_color, false),
            ]
            .spacing(2)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .on_release(message)
        .into()
    };

    // Left column: clock/date/alarm stacked above the weather.
    let left_column = column![
        mouse_area(clock_column).on_release(Message::LaunchDashboardCategory("clock".to_string())),
        weather_element,
    ]
    .spacing(2)
    .width(Fill)
    .align_x(iced::alignment::Horizontal::Center);

    // Right column: today/tomorrow calendar list.
    let has_permission = app
        .dashboard_calendar
        .as_ref()
        .map(|c| c.permission_granted)
        .unwrap_or(true);

    let calendar_panel: Element<'_, Message> = if !has_permission {
        mouse_area(
            container(
                row![shadowed_text(
                    "📅 Tap to show calendar events",
                    13.0,
                    muted_color,
                    false
                ),]
                .padding([6.0, 10.0]),
            )
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.25).into()),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        )
        .on_release(Message::RequestCalendarPermission)
        .into()
    } else if let Some(cal) = &app.dashboard_calendar {
        if cal.events.is_empty() {
            space().into()
        } else {
            let mut event_rows = column![].spacing(6);
            let mut saw_tomorrow = false;

            for event in &cal.events {
                if event.tomorrow && !saw_tomorrow {
                    saw_tomorrow = true;
                    event_rows = event_rows.push(
                        container(space())
                            .width(Fill)
                            .height(Length::Fixed(1.0))
                            .style(|_| container::Style {
                                background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.25).into()),
                                ..Default::default()
                            }),
                    );
                }
                let color_argb = event.color as i32;
                let a = ((color_argb >> 24) & 0xFF) as f32 / 255.0;
                let r = ((color_argb >> 16) & 0xFF) as f32 / 255.0;
                let g = ((color_argb >> 8) & 0xFF) as f32 / 255.0;
                let b = (color_argb & 0xFF) as f32 / 255.0;
                let event_color = Color::from_rgba(r, g, b, a);

                // The line is an overlay in a stack whose base is the title, so
                // it stretches to the full text height, including wrapped lines.
                let color_line = container(
                    container(space())
                        .width(Length::Fixed(3.0))
                        .height(Fill)
                        .style(move |_| container::Style {
                            background: Some(event_color.into()),
                            border: iced::Border {
                                radius: 1.5.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                )
                .height(Fill)
                .padding(iced::Padding {
                    top: 2.0,
                    bottom: 2.0,
                    left: 0.0,
                    right: 0.0,
                });

                let time_text = container(shadowed_text(&event.time_str, 13.0, muted_color, false))
                    .width(Length::Fixed(38.0))
                    .align_x(iced::alignment::Horizontal::Right);
                // Long titles wrap onto extra lines; the row and the scrollable
                // grow with them.
                let title_text = container(
                    text(event.title.clone())
                        .size(15.0)
                        .color(text_color),
                )
                .width(Fill);

                let title_with_line = iced::widget::stack![
                    // Base layer reserves the gutter and determines the height.
                    row![space().width(Length::Fixed(11.0)), title_text].width(Fill),
                    iced::widget::pin(color_line).x(0.0).y(0.0),
                ]
                .width(Fill);

                let event_id = event.event_id;
                let row_item = mouse_area(
                    row![
                        title_with_line,
                        time_text,
                        // Clearance for the scrollbar that appears while scrolling.
                        space().width(Length::Fixed(12.0)),
                    ]
                    .spacing(8)
                    .width(Fill)
                    .align_y(Vertical::Top),
                )
                .on_release(Message::LaunchCalendarEvent(event_id));

                event_rows = event_rows.push(row_item);
            }

            // About four rows fit; more than that scrolls inside the panel.
            if cal.events.len() <= 4 {
                event_rows.into()
            } else {
                scrollable(event_rows)
                    .height(Fill)
                    // The shared scrollbar state hides it until the user
                    // actually scrolls this panel.
                    .direction(crate::theme::vertical_scrollbar(
                        app.scrollbars_visible(),
                    ))
                    .on_scroll(|_| Message::DashboardScrolled)
                    .into()
            }
        }
    } else {
        space().into()
    };

    // Two translucent cards: clock/alarm/weather on the left and the calendar
    // list on the right. The opacity comes from the dashboard setting, and an
    // active event tints both cards.
    let card_alpha = app
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.settings.dashboard_opacity)
        .unwrap_or(40) as f32
        / 100.0;
    let tint = app
        .dashboard_calendar
        .as_ref()
        .and_then(|calendar| active_event_color(&calendar.events, now_ms()));

    let left_card = container(left_column)
        .width(Length::FillPortion(1))
        .padding(12)
        .style(crate::theme::dashboard_card(card_alpha, tint));
    let calendar_card = container(calendar_panel)
        .width(Length::FillPortion(2))
        .height(Fill)
        .padding(12)
        .style(crate::theme::dashboard_card(card_alpha, tint));

    let content = row![left_card, calendar_card]
    .spacing(12)
    .width(Fill)
    .align_y(Vertical::Top);

    container(content)
        .padding(iced::Padding {
            top: 8.0,
            right: if app.size.width > app.size.height {
                ((app.size.width - 640.0) / 2.0).max(12.0)
            } else {
                12.0
            },
            bottom: 14.0,
            left: if app.size.width > app.size.height {
                ((app.size.width - 640.0) / 2.0).max(12.0)
            } else {
                12.0
            },
        })
        .width(Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{
        active_event_color, condition_symbol, filter_past_events, split_time,
    };
    use qlaunch_core::platform::android::CalendarEventItem;

    fn event(id: i64, begin: i64, all_day: bool) -> CalendarEventItem {
        CalendarEventItem {
            event_id: id,
            title: format!("event {id}"),
            begin,
            end: begin + 3_600_000,
            all_day,
            time_str: "10:00".to_string(),
            color: 0,
            tomorrow: false,
        }
    }

    #[test]
    fn past_events_more_than_ten_minutes_old_are_dropped() {
        let now = 100_000_000i64;
        let mut events = vec![
            event(1, now - 11 * 60 * 1000, false), // 11 min ago: hidden
            event(2, now - 5 * 60 * 1000, false),  // 5 min ago: still shown
            event(3, now + 60_000, false),         // future: shown
            event(4, 0, true),                     // all-day: always shown
        ];
        filter_past_events(&mut events, now);
        assert_eq!(
            events.iter().map(|e| e.event_id).collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
    }

    #[test]
    fn active_event_color_tracks_running_events_only() {
        let now = 100_000_000i64;
        let running = event(1, now - 1000, false);
        let future = event(2, now + 5000, false);
        let ended = CalendarEventItem {
            event_id: 3,
            begin: now - 9_000,
            end: now - 5_000,
            ..running.clone()
        };
        assert!(active_event_color(&[running], now).is_some());
        assert!(active_event_color(&[future, ended], now).is_none());
    }

    #[test]
    fn clock_time_splits_into_digits_and_am_pm() {
        assert_eq!(split_time("2:17 pm"), ("2:17".into(), Some("pm".into())));
        assert_eq!(split_time("11:05 AM"), ("11:05".into(), Some("AM".into())));
        // 24-hour times have no suffix to draw smaller.
        assert_eq!(split_time("14:17"), ("14:17".into(), None));
    }

    #[test]
    fn weather_codes_map_to_recognisable_symbols() {
        assert_eq!(condition_symbol(0), "☀️");
        assert_eq!(condition_symbol(2), "⛅");
        assert_eq!(condition_symbol(45), "🌫️");
        assert_eq!(condition_symbol(63), "🌧️");
        assert_eq!(condition_symbol(95), "⛈️");
        // Unknown codes still draw a neutral symbol rather than nothing.
        assert_eq!(condition_symbol(-1), "🌤️");
    }
}
