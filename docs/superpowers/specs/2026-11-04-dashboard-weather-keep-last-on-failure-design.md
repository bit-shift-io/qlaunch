# Dashboard weather: keep last reading on update failure

## Intent

When a weather refresh fails — for example a platform I/O error like
"Unable to resolve host …" — the dashboard card currently clears the last good
reading and renders the raw error string inside the widget. A transient network
or platform failure should neither blank the weather nor spill a long technical
message into the launcher. Keep showing the last successful reading, and when
there is nothing cached, show a short friendly fallback instead.

## Goals

- On a failed weather update, keep the previously shown reading.
- Never render the raw error string in the widget; log it instead, mirroring how
  the calendar refresh already handles failures (`lib.rs`).
- When an update fails and no reading has ever been cached, show
  "Weather unavailable".
- Preserve the no-location case: the card keeps showing "Tap to set location".

## Non-goals

- Persisting weather across app restarts. The cache stays in-memory; the
  first-failure fallback covers the restart-while-offline case.
- Changing refresh cadence, location editing, or the weather-app launch action.
- Showing a staleness or "last updated" indicator.

## Approaches

- **A (chosen):** replace the stored error string with a boolean failure flag.
  On failure, keep the cached reading and log the real host message. The view
  renders the raw string nowhere; with no cached reading and a failed last
  update it shows a fixed "Weather unavailable".
- **B (rejected):** keep the string but stop displaying it, and stop clearing the
  reading on failure. A smaller diff, but it leaves a field whose value is never
  displayed and only its presence matters — misleading to the next reader.

## Shape

State on `App`:

- `dashboard_weather: Option<WeatherData>` — unchanged.
- `dashboard_weather_error: Option<String>` becomes `dashboard_weather_failed: bool`.

New pure helpers in `dashboard.rs`:

```rust
/// Applies a weather refresh result, keeping the last good reading on failure.
/// An empty error means no location is configured, so the reading is cleared.
pub(crate) fn apply_weather_result(
    current: &mut Option<WeatherData>,
    failed: &mut bool,
    result: Result<WeatherData, String>,
) {
    match result {
        Ok(weather) => {
            *current = Some(weather);
            *failed = false;
        }
        Err(error) if error.is_empty() => {
            *current = None;
            *failed = false;
        }
        Err(_) => *failed = true,
    }
}

/// Fallback line for the weather card when there is no reading to show.
pub(crate) fn weather_placeholder(configured: bool, failed: bool) -> &'static str {
    if !configured {
        "Tap to set location"
    } else if failed {
        "Weather unavailable"
    } else {
        "Updating…"
    }
}
```

## Behaviour

| Cached reading | Location set | Last update failed | Card shows            |
| :------------- | :----------- | :----------------- | :-------------------- |
| yes            | any          | any                | the reading           |
| no             | yes          | yes                | "Weather unavailable"  |
| no             | yes          | no                 | "Updating…"            |
| no             | no           | any                | "Tap to set location"  |

The fallback line keeps the current tap action: weather-app launch when a
location is configured, Settings when it is not.

## Code changes

- `lib.rs`: rename the field to `dashboard_weather_failed: bool`; initialise it
  to `false`; in the `Message::DashboardWeather` arm, `log::warn!` a non-empty
  error and then call `apply_weather_result`; reset the flag to `false` in the
  location-cleared path of `SettingsDone`.
- `dashboard.rs`: drop the `dashboard_weather_error` render branch; the
  no-reading branch uses `weather_placeholder`; add the two helpers and tests.

## Verification

- Unit tests for `apply_weather_result`:
  - success stores the reading and clears the failure flag;
  - failure with a cached reading keeps the reading and sets the flag;
  - failure with no cached reading sets the flag and leaves the reading empty;
  - an empty error (no location) clears both.
- Unit tests for `weather_placeholder` across the four rows above.
- `cargo test -p qlaunch`.
- Manual: enable the dashboard, let weather load, then force a failure (airplane
  mode or a bad location) and confirm the reading stays with no host text; start
  fresh while offline and confirm "Weather unavailable".
