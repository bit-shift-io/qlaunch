use qlaunch_core::model::{AppCategory, AppRef, IconImage};
use qlaunch_core::platform::android::{
    AndroidHost, AndroidList, AndroidSource, icon_ref, parse_alarm_json, parse_backup_names_json,
    parse_calendar_events_json, parse_category_apps_json, parse_icon_ref, parse_list_json,
    parse_weather_json,
};
use qlaunch_core::platform::{AppSource, LaunchError, PlatformError};

struct FakeHost {
    list: AndroidList,
    icon: Option<Vec<u8>>,
}

impl AndroidHost for FakeHost {
    fn list(&self) -> Result<AndroidList, PlatformError> {
        Ok(self.list.clone())
    }
    fn icon_image(&self, _app: &AppRef, _size_px: u32) -> Result<Option<IconImage>, PlatformError> {
        Ok(self.icon.clone().map(IconImage::png))
    }
    fn launch(&self, _app: &AppRef) -> Result<(), LaunchError> {
        Ok(())
    }
}

fn host() -> FakeHost {
    FakeHost {
        list: parse_list_json(
            r#"{"selfPackage":"bitshift.qlaunch","activities":[
                {"package":"org.mozilla.firefox","activity":"org.mozilla.firefox.App","label":"Firefox","isGame":false},
                {"package":"com.example.zombie","activity":"com.example.zombie.Main","label":"Zombie","isGame":true},
                {"package":"bitshift.qlaunch","activity":"bitshift.qlaunch.MainActivity","label":"QLaunch","isGame":false},
                {"package":"jp.example","activity":"jp.example.Main","label":"日本 ゲーム 🙂","isGame":false}
            ]}"#,
        )
        .unwrap(),
        icon: Some(vec![1, 2, 3]),
    }
}

#[test]
fn the_launcher_lists_games_last_and_drops_its_own_package() {
    let source = AndroidSource::new(host());
    let apps = source.list_apps().unwrap();
    let packages: Vec<_> = apps.iter().map(|a| a.package.as_str()).collect();
    assert_eq!(
        packages,
        ["org.mozilla.firefox", "com.example.zombie", "jp.example"]
    );
    assert!(!packages.contains(&"bitshift.qlaunch"));
}

#[test]
fn the_game_flag_routes_to_the_games_category_and_everything_else_to_app() {
    let source = AndroidSource::new(host());
    let apps = source.list_apps().unwrap();
    let by = |p: &str| apps.iter().find(|a| a.package == p).unwrap().category;
    assert_eq!(by("com.example.zombie"), AppCategory::Game);
    assert_eq!(by("org.mozilla.firefox"), AppCategory::App);
}

#[test]
fn a_unicode_label_survives_the_parse_and_the_mapping() {
    let source = AndroidSource::new(host());
    let apps = source.list_apps().unwrap();
    let jp = apps.iter().find(|a| a.package == "jp.example").unwrap();
    assert_eq!(jp.label, "日本 ゲーム 🙂");
    assert_eq!(jp.activity, "jp.example.Main");
}

#[test]
fn icon_and_launch_delegate_to_the_host() {
    let source = AndroidSource::new(host());
    let app = AppRef {
        package: "org.mozilla.firefox".into(),
        activity: "org.mozilla.firefox.App".into(),
    };
    assert_eq!(
        source.icon_image(&app, 64).unwrap(),
        Some(IconImage::png(vec![1, 2, 3]))
    );
    assert!(source.launch(&app).is_ok());
}

#[test]
fn a_backup_listing_is_filtered_and_newest_first() {
    let json = r#"[
        "qlaunch-backup-2026-10-20-090000.json",
        "holiday.jpg",
        "qlaunch-backup-2026-10-21-140530.json"
    ]"#;
    assert_eq!(
        parse_backup_names_json(json).unwrap(),
        [
            "qlaunch-backup-2026-10-21-140530.json",
            "qlaunch-backup-2026-10-20-090000.json",
        ]
    );
}

#[test]
fn a_bad_backup_name_is_refused_before_the_host_sees_it() {
    let source = AndroidSource::new(host());
    assert!(source.read_backup("../../etc/passwd").is_err());
    assert!(source.write_backup("not-a-backup.json", b"x").is_err());
}

#[test]
fn an_icon_reference_round_trips_package_and_resource() {
    let reference = icon_ref("com.example", "com.example:drawable/icon").unwrap();
    assert_eq!(
        reference,
        "android-res://com.example/com.example:drawable/icon"
    );
    assert_eq!(
        parse_icon_ref(&reference),
        Some(("com.example", "com.example:drawable/icon"))
    );
}

#[test]
fn a_malformed_icon_reference_is_not_mistaken_for_a_real_one() {
    assert_eq!(parse_icon_ref("com.example/drawable/icon"), None);
    assert_eq!(parse_icon_ref("android-res://"), None);
    assert_eq!(parse_icon_ref("android-res://com.example/"), None);
    assert_eq!(icon_ref("", "drawable/icon"), None);
    assert_eq!(icon_ref("com.example", ""), None);
}

#[test]
fn parse_alarm_json_handles_active_and_empty() {
    let active = r#"{"has_alarm":true,"trigger_time":1700000000000,"time_str":"07:00"}"#;
    let alarm = parse_alarm_json(active).expect("should parse alarm");
    assert_eq!(alarm.trigger_time, 1700000000000);
    assert_eq!(alarm.time_str, "07:00");

    let no_alarm = r#"{"has_alarm":false}"#;
    assert_eq!(parse_alarm_json(no_alarm), None);

    let empty = "{}";
    assert_eq!(parse_alarm_json(empty), None);
}

#[test]
fn parse_calendar_events_json_handles_list() {
    let json = r#"{
        "permission_granted": true,
        "events": [
            {
                "event_id": 42,
                "title": "Sprint Planning",
                "begin": 1700000000000,
                "end": 1700003600000,
                "all_day": false,
                "time_str": "10:00",
                "end_time_str": "11:00",
                "color": -12417548
            }
        ]
    }"#;
    let payload = parse_calendar_events_json(json).expect("should parse calendar payload");
    assert!(payload.permission_granted);
    assert_eq!(payload.events.len(), 1);
    assert_eq!(payload.events[0].title, "Sprint Planning");
    assert_eq!(payload.events[0].event_id, 42);
    assert_eq!(payload.events[0].time_str, "10:00");
    assert_eq!(payload.events[0].end_time_str, "11:00");
    assert!(!payload.events[0].all_day);
}

#[test]
fn parse_weather_json_handles_success_and_error() {
    let json = r#"{
        "location_label": "Adelaide",
        "current_temp": 23.5,
        "temp_min": 14.0,
        "temp_max": 25.0,
        "condition_code": 1
    }"#;
    let weather = parse_weather_json(json).expect("should parse weather");
    assert_eq!(weather.location_label, "Adelaide");
    assert_eq!(weather.current_temp, 23.5);
    assert_eq!(weather.temp_min, 14.0);
    assert_eq!(weather.temp_max, 25.0);
    assert_eq!(weather.condition_code, 1);

    let error_json = r#"{"error": "Location not found"}"#;
    assert!(parse_weather_json(error_json).is_err());
}

#[test]
fn parse_category_apps_json_handles_candidates() {
    let json = r#"[
        {"package": "com.google.android.deskclock", "label": "Clock", "is_recommended": true},
        {"package": "com.example.calculator", "label": "Calculator", "is_recommended": false}
    ]"#;
    let apps = parse_category_apps_json(json).expect("should parse apps");
    assert_eq!(apps.len(), 2);
    assert_eq!(apps[0].package, "com.google.android.deskclock");
    assert!(apps[0].is_recommended);
    assert!(!apps[1].is_recommended);
}
