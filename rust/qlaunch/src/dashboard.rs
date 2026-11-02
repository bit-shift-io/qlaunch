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
            },
            CalendarEventItem {
                event_id: 2,
                title: "Sprint Review".to_string(),
                begin: 0,
                end: 0,
                all_day: false,
                time_str: "14:00".to_string(),
                color: 0xFF0B8043u32 as i32,
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

    let (clock_main, clock_suffix) = split_time(time_str);
    // Tight line height lifts the large digits so their top lines up with the
    // first calendar row instead of sitting below its baseline box.
    let clock_main_text = text(clock_main)
        .size(42.0)
        .color(text_color)
        .line_height(1.0);
    let mut time_row = row![Element::from(clock_main_text)]
        .spacing(4)
        .align_y(Vertical::Bottom);
    if let Some(suffix) = clock_suffix {
        time_row = time_row.push(shadowed_text(suffix, 20.0, muted_color, false));
    }

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
    .spacing(2);

    // 2. Weather
    let weather_element: Element<'_, Message> = if let Some(weather) = &app.dashboard_weather {
        let symbol = condition_symbol(weather.condition_code);
        let temp_line = format!("{} {:.0}°C", symbol, weather.current_temp);
        let detail_line = format!("{:.0}° / {:.0}°", weather.temp_min, weather.temp_max);

        mouse_area(
            column![
                shadowed_text(temp_line, 24.0, text_color, true),
                shadowed_text(detail_line, 13.0, muted_color, false),
            ]
            .spacing(3)
            .align_x(iced::alignment::Horizontal::Left),
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
            .align_x(iced::alignment::Horizontal::Left),
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
            .align_x(iced::alignment::Horizontal::Left),
        )
        .on_release(message)
        .into()
    };

    // Left column: clock/date/alarm stacked above the weather.
    let left_column = column![
        mouse_area(clock_column).on_release(Message::LaunchDashboardCategory("clock".to_string())),
        weather_element,
    ]
    .spacing(14)
    .width(Fill);

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

            for event in &cal.events {
                let color_argb = event.color as i32;
                let a = ((color_argb >> 24) & 0xFF) as f32 / 255.0;
                let r = ((color_argb >> 16) & 0xFF) as f32 / 255.0;
                let g = ((color_argb >> 8) & 0xFF) as f32 / 255.0;
                let b = (color_argb & 0xFF) as f32 / 255.0;
                let event_color = Color::from_rgba(r, g, b, a);

                let color_pill = container(
                    space()
                        .width(Length::Fixed(4.0))
                        .height(Length::Fixed(16.0)),
                )
                .style(move |_| container::Style {
                    background: Some(event_color.into()),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });

                let time_text = container(shadowed_text(&event.time_str, 13.0, muted_color, false))
                    .width(Length::Fixed(70.0))
                    .align_x(iced::alignment::Horizontal::Right);
                // One line per event, clipped at the column edge, so four rows
                // fit before the list scrolls.
                let title_text = container(
                    text(event.title.clone())
                        .size(15.0)
                        .color(text_color)
                        .wrapping(iced::widget::text::Wrapping::None),
                )
                .width(Fill)
                .clip(true);

                let event_id = event.event_id;
                let row_item = mouse_area(
                    row![
                        color_pill,
                        title_text,
                        time_text,
                        // Clearance for the scrollbar that appears while scrolling.
                        space().width(Length::Fixed(12.0)),
                    ]
                    .spacing(8)
                    .width(Fill)
                    .align_y(Vertical::Center),
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

    // Clock/alarm/weather left, calendar list right. The calendar stretches to
    // the full widget height (the taller left column), then scrolls internally.
    let content = row![
        container(left_column).width(Length::FillPortion(2)),
        container(calendar_panel)
            .width(Length::FillPortion(3))
            .height(Fill),
    ]
    .spacing(20)
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
    use super::{condition_symbol, filter_past_events, split_time};
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
