# Feature Implementation Tasks: Dashboard Widget

This document contains the step-by-step implementation tasks for the **Dashboard Widget** in `qlaunch`.

---

## Overview & Design Baseline

- **Aesthetic**: Modernized HTC clock widget vibe with minimal chrome. Borderless, floating directly over wallpaper with subtle typography drop-shadows for high legibility.
- **Components**:
  - Clock (bold digital, system 12h/24h format)
  - Next alarm indicator (Android `AlarmManager`; cleanly hidden when no alarm is set)
  - Weather for the day (Australian Bureau of Meteorology ACCESS model via Open-Meteo, configured by postcode or suburb; zero location permissions required)
  - Upcoming calendar events (Today & Tomorrow from Android `CalendarContract`, Google Calendar sync color indicator strip, dynamic height up to 4 items, internal scroll if > 4 items)
- **Placement & Scrolling**: Home (first) page only. Pinned as a header at the top of the app grid that scrolls off-screen with apps. In landscape mode, centered at the top with left/right margins, with app icons strictly below it.
- **Interactivity**: Clicking Clock, Weather, or Calendar opens the user's preferred app. The first tap displays an in-app chooser dialog; choices are saved in `Settings` and can be changed anytime in Launcher Settings.

---

## Tasks

### Task 1: Core Settings Model & Serialization (`qlaunch_core`)
- [x] Add dashboard configuration fields to [`Settings`](file:///home/bronson/Projects/qlaunch/rust/qlaunch_core/src/settings/mod.rs) and `SettingsPatch`:
  - `dashboard_widget_enabled: bool` (default: `false` or `true` as desired)
  - `weather_location: String` (postcode or suburb name, e.g. `"5000"` or `"Adelaide"`)
  - `preferred_clock_app: Option<String>` (package name)
  - `preferred_weather_app: Option<String>` (package name)
  - `preferred_calendar_app: Option<String>` (package name)
- [x] Implement `Settings::default()` and `SettingsPatch::apply()` for the new fields.
- [x] Ensure JSON serialization and deserialization backwards compatibility with existing `qlaunch.json` configs.
- [x] Add unit tests in [`rust/qlaunch_core/tests/settings.rs`](file:///home/bronson/Projects/qlaunch/rust/qlaunch_core/tests/settings.rs) validating default values, patching, and serialization.

### Task 2: Android Platform Adapter & Manifest Permissions (`android`)
- [x] In [`android/app/src/main/AndroidManifest.xml`](file:///home/bronson/Projects/qlaunch/android/app/src/main/AndroidManifest.xml):
  - Add `<uses-permission android:name="android.permission.INTERNET"/>`
  - Add `<uses-permission android:name="android.permission.READ_CALENDAR"/>`
- [x] Implement `AlarmAdapter` in Kotlin:
  - Query next alarm using `AlarmManager.getNextAlarmClock()`.
  - Format alarm trigger time according to system time format.
  - Return `null` if no active alarm is set.
- [x] Implement `CalendarAdapter` in Kotlin:
  - Check for `READ_CALENDAR` permission.
  - Query `CalendarContract.Instances` from current timestamp to end of tomorrow.
  - Extract `title`, `begin`, `end`, `allDay`, and integer `color` (checking `EVENT_COLOR`, falling back to `CALENDAR_COLOR`).
  - Serialize event list to JSON string.
- [x] Implement `WeatherClient` in Kotlin:
  - Resolve Australian postcode/suburb to coordinates using Open-Meteo Geocoding API (`https://geocoding-api.open-meteo.com/v1/search?name=...&country=AU`).
  - Query Open-Meteo forecast endpoint using the BOM model (`&models=bom_access_global&current_weather=true&daily=temperature_2m_max,temperature_2m_min,weathercode`).
  - Use standard `HttpURLConnection` in background thread (keeps Rust binary lean with zero native TLS/NDK issues).
  - Return structured JSON (current temp, min/max, weather code, location name).
- [x] Implement `AppDiscovery` & Intent Launcher in Kotlin:
  - Query installed apps matching `Intent(AlarmClock.ACTION_SHOW_ALARMS)`, `Intent(Intent.ACTION_VIEW).setData(CalendarContract.CONTENT_URI)`, and weather packages.
  - Add methods to launch preferred apps explicitly by package name or fallback intent.
- [x] Expose static JNI bridge methods in [`QLaunchBridge.kt`](file:///home/bronson/Projects/qlaunch/android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt) and [`LauncherActions.kt`](file:///home/bronson/Projects/qlaunch/android/app/src/main/kotlin/bitshift/qlaunch/LauncherActions.kt).

### Task 3: Rust Platform JNI Seam & State Subscriptions (`qlaunch`)
- [ ] Add JNI bindings in `rust/qlaunch_core/src/platform/android/` for:
  - `get_next_alarm() -> Option<AlarmInfo>`
  - `get_calendar_events() -> Result<Vec<CalendarEventItem>, String>`
  - `request_calendar_permission()`
  - `fetch_weather(location: &str) -> Result<WeatherData, String>`
  - `get_apps_for_category(category: &str) -> Vec<AppChoice>`
  - `launch_app(category: &str, package: Option<&str>)`
- [ ] Define Rust data models:
  - `AlarmInfo { time_str: String }`
  - `CalendarEventItem { title: String, start_time_str: String, is_all_day: bool, color_argb: u32, event_id: i64 }`
  - `WeatherData { current_temp: f32, temp_min: f32, temp_max: f32, condition_code: u16, location_label: String }`
- [ ] Add state tracking to `App` struct in [`rust/qlaunch/src/lib.rs`](file:///home/bronson/Projects/qlaunch/rust/qlaunch/src/lib.rs):
  - `dashboard_time: String`
  - `dashboard_date: String`
  - `alarm_info: Option<AlarmInfo>`
  - `calendar_events: Vec<CalendarEventItem>`
  - `calendar_permission_granted: bool`
  - `weather_data: Option<WeatherData>`
  - `weather_loading: bool`
- [ ] Implement subscription in `App::subscription()`:
  - Periodic time tick (e.g. 30 seconds) active when foregrounded to advance clock.
  - Refresh alarm and calendar events on `WindowFocused` and `Foreground(true)`.
  - Periodic weather refresh (e.g. hourly or upon returning home after 30+ minutes).

### Task 4: Dashboard UI Widget & HTC Minimal Styling (`rust/qlaunch/src/dashboard.rs`)
- [ ] Create `rust/qlaunch/src/dashboard.rs` implementing `view(app: &App) -> Element<'_, Message>`.
- [ ] Render Top Section (Clock & Weather):
  - Large digital clock text (size ~44–48pt, bold, matching system 12h/24h).
  - Date line below clock (`"Mon, 2 Nov"`).
  - Alarm pill (`"⏰ 06:30"`) positioned beside date; hidden completely when no alarm is set.
  - Weather pill on the right: weather icon, temperature (`"24°C"`), suburb label, min/max summary.
- [ ] Render Calendar Section:
  - Dynamic container: expands cleanly up to 4 items without internal scroll; caps height and enables vertical scrollbar if > 4 items.
  - Cleanly collapses/hides if there are 0 upcoming events.
  - Calendar event row layout:
    - Left colored vertical strip (Google Calendar event sync color).
    - Time text (`"09:30"` or `"All day"`).
    - Event title (single line with ellipsis truncation).
- [ ] Wrap clickable areas with `mouse_area` emitting corresponding Messages:
  - Clock area -> `Message::LaunchClock`
  - Weather area -> `Message::LaunchWeather`
  - Calendar area -> `Message::LaunchCalendar(Option<i64>)`
- [ ] Apply subtle text drop-shadow styling for contrast over any wallpaper without needing opaque card backgrounds.

### Task 5: Home Page Grid Integration & Landscape Layout (`view.rs`, `grid.rs`)
- [ ] In [`rust/qlaunch/src/view.rs`](file:///home/bronson/Projects/qlaunch/rust/qlaunch/src/view.rs):
  - Conditionally insert `dashboard::view(app)` at the top of the Home (first) page's scrollable container when `dashboard_widget_enabled` is true.
  - Ensure the widget only appears on the primary/home page and does not repeat across other group pages.
  - Ensure it scrolls away with the app grid smoothly as a header.
- [ ] Landscape orientation adaptation:
  - In landscape mode, apply horizontal margins/centering to the dashboard widget.
  - Ensure app icons are strictly positioned below the widget for a clean, balanced layout.

### Task 6: App Chooser Dialog & Interactive Tap Handlers
- [ ] Implement app chooser dialog in [`rust/qlaunch/src/dialog.rs`](file:///home/bronson/Projects/qlaunch/rust/qlaunch/src/dialog.rs):
  - When user taps Clock, Weather, or Calendar and no preferred app is configured in settings:
    - Fetch candidate installed apps via Android JNI.
    - Show an in-app chooser listing candidate apps.
    - On selection: save chosen package name into `SettingsPatch`, commit settings, and launch the selected app.
- [ ] When a preferred app is already configured:
  - Tapping Clock directly launches the preferred clock app.
  - Tapping Weather directly launches the preferred weather app.
  - Tapping Calendar header launches the calendar app.
  - Tapping a specific event launches the calendar directly to that event view.

### Task 7: Settings Screen UI Card (`rust/qlaunch/src/screens/settings.rs`)
- [ ] Add "Dashboard Widget" section to [`rust/qlaunch/src/screens/settings.rs`](file:///home/bronson/Projects/qlaunch/rust/qlaunch/src/screens/settings.rs):
  - Toggle switch: **Show Dashboard Widget**.
  - Text input: **Weather Location** (Australian postcode or suburb) with "Test & Refresh" action.
  - Preferred apps row: shows currently assigned apps for Clock, Weather, and Calendar with "Change" buttons.
  - Calendar permission row: shows permission status and "Grant Permission" button if not yet granted.

### Task 8: Verification & Device Testing
- [ ] Run Rust test suite:
  ```sh
  cd rust && cargo test --workspace
  ```
- [ ] Verify Android build:
  ```sh
  ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build.sh release
  ```
- [ ] Install and test on connected device:
  ```sh
  adb install -r build/app/outputs/apk/release/app-release.apk
  adb shell am start -W -n bitshift.qlaunch/.MainActivity
  ```
- [ ] Test edge cases:
  - Empty calendar (widget collapses calendar row cleanly).
  - No alarms set (alarm pill hidden cleanly).
  - Offline mode / invalid postcode (graceful weather fallback text, no launcher crash).
  - Orientation change between portrait and landscape.
  - Re-entering launcher from other apps (time, alarm, and calendar refresh promptly without frozen surface).
