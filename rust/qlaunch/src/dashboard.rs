//! Dashboard widget: modernized minimal HTC-inspired clock, alarm, Australian BOM weather and calendar events.

use iced::alignment::Vertical;
use iced::widget::{column, container, mouse_area, row, scrollable, space, text};
use iced::{Color, Element, Fill, Length};
#[cfg(not(target_os = "android"))]
use qlaunch_core::platform::android::CalendarEventItem;
use qlaunch_core::platform::android::{
    AlarmInfo, AppChoice, CalendarEventsPayload, TimeInfo, WeatherData,
};

use crate::{App, Message};

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
    // Bold weight is deliberately avoided: on Android a hand-built bold
    // `iced::Font` resolves to a face with no renderable glyphs, so the clock
    // and temperature came out invisible. Size alone gives the emphasis.
    let _ = bold;
    let font = iced::Font::DEFAULT;

    let shadow_color = Color::from_rgba(0.0, 0.0, 0.0, 0.65);

    let fg = text(owned.clone())
        .size(size)
        .color(color)
        .font(font);

    fg.into()
}

// --- Main Widget View ---

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let text_color = Color::WHITE;
    let muted_color = Color::from_rgba(1.0, 1.0, 1.0, 0.75);

    // 1. Clock & Date
    let time_str = app.dashboard_time.as_deref().unwrap_or("10:14");
    let date_str = app.dashboard_date.as_deref().unwrap_or("Mon, 2 Nov");
    log::info!("dashboard view: time={time_str:?} date={date_str:?} alarm={:?} weather={:?} cal={:?}", app.dashboard_alarm.is_some(), app.dashboard_weather.is_some(), app.dashboard_calendar.as_ref().map(|c| c.events.len()).unwrap_or(0));

    let mut date_line = format!("{date_str}");
    if let Some(alarm) = &app.dashboard_alarm {
        date_line.push_str(&format!("  •  ⏰ {}", alarm.time_str));
    }

    let clock_column = column![
        shadowed_text(time_str, 28.0, text_color, false),
        shadowed_text(date_line, 15.0, muted_color, false),
    ]
    .spacing(2);

    // 2. Weather
    let weather_element: Element<'_, Message> = if let Some(weather) = &app.dashboard_weather {
        let symbol = condition_symbol(weather.condition_code);
        let temp_line = format!("{} {:.0}°C", symbol, weather.current_temp);
        let detail_line = format!(
            "{}  {:.0}° / {:.0}°",
            weather.location_label, weather.temp_min, weather.temp_max
        );

        mouse_area(
            column![
                shadowed_text(temp_line, 24.0, text_color, true),
                shadowed_text(detail_line, 13.0, muted_color, false),
            ]
            .spacing(3)
            .align_x(iced::alignment::Horizontal::Right),
        )
        .on_press(Message::LaunchDashboardCategory("weather".to_string()))
        .into()
    } else if let Some(err) = &app.dashboard_weather_error {
        mouse_area(
            column![
                shadowed_text("Weather", 16.0, text_color, true),
                shadowed_text(err, 12.0, Color::from_rgb8(255, 180, 180), false),
            ]
            .spacing(2)
            .align_x(iced::alignment::Horizontal::Right),
        )
        .on_press(Message::LaunchDashboardCategory("weather".to_string()))
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
            .align_x(iced::alignment::Horizontal::Right),
        )
        .on_press(message)
        .into()
    };

    let top_row = row![clock_column, space().width(Fill), weather_element,]
        .align_y(Vertical::Center)
        .width(Fill);

    // 3. Calendar Events
    let mut widget_column = column![top_row].spacing(10);

    let has_permission = app
        .dashboard_calendar
        .as_ref()
        .map(|c| c.permission_granted)
        .unwrap_or(true);

    if !has_permission {
        let perm_prompt = mouse_area(
            container(
                row![shadowed_text(
                    "📅 Tap to show calendar events",
                    13.5,
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
        .on_press(Message::RequestCalendarPermission);

        widget_column = widget_column.push(perm_prompt);
    } else if let Some(cal) = &app.dashboard_calendar {
        if !cal.events.is_empty() {
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

                let time_text = shadowed_text(&event.time_str, 12.5, muted_color, false);
                let title_text = container(shadowed_text(&event.title, 13.5, text_color, false))
                    .width(Fill)
                    .clip(true);

                let event_id = event.event_id;
                let row_item = mouse_area(
                    row![
                        color_pill,
                        container(time_text).width(Length::Fixed(60.0)),
                        title_text,
                    ]
                    .spacing(8)
                    .width(Fill)
                    .align_y(Vertical::Center),
                )
                .on_press(Message::LaunchCalendarEvent(event_id));

                event_rows = event_rows.push(row_item);
            }

            if cal.events.len() <= 4 {
                widget_column = widget_column.push(event_rows);
            } else {
                let scroll_box = scrollable(event_rows)
                    .height(Length::Fixed(110.0))
                    .direction(crate::theme::vertical_scrollbar(true));
                widget_column = widget_column.push(scroll_box);
            }
        }
    }

    container(widget_column)
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
    use super::condition_symbol;

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
