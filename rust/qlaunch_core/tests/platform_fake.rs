use qlaunch_core::model::{AppCategory, AppEntry, AppRef, IconImage};
use qlaunch_core::platform::{AppSource, FakeSource, LaunchError};

fn app(package: &str, activity: &str, label: &str) -> AppEntry {
    AppEntry {
        package: package.into(),
        activity: activity.into(),
        label: label.into(),
        category: AppCategory::App,
        last_update_time: 0,
    }
}

fn app_ref(package: &str, activity: &str) -> AppRef {
    AppRef {
        package: package.into(),
        activity: activity.into(),
    }
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn list_apps_returns_given_apps_in_order() {
    let apps = vec![
        app("com.a", ".Main", "Alpha"),
        app("com.b", ".Main", "Bravo"),
    ];
    let source = FakeSource::new(apps.clone());
    assert_eq!(source.list_apps().unwrap(), apps);
}

#[test]
fn icon_image_returns_registered_icon() {
    let target = app_ref("com.a", ".Main");
    let source = FakeSource::new(vec![app("com.a", ".Main", "Alpha")])
        .with_icon(target.clone(), vec![1, 2, 3]);
    assert_eq!(
        source.icon_image(&target, 48).unwrap(),
        Some(IconImage::png(vec![1, 2, 3]))
    );
}

#[test]
fn icon_image_returns_none_without_registered_icon() {
    let source = FakeSource::new(vec![app("com.a", ".Main", "Alpha")]);
    let target = app_ref("com.a", ".Main");
    assert_eq!(source.icon_image(&target, 48).unwrap(), None);
}

#[test]
fn launch_records_apps_in_call_order() {
    let first = app_ref("com.a", ".Main");
    let second = app_ref("com.b", ".Main");
    let source = FakeSource::new(vec![
        app("com.a", ".Main", "Alpha"),
        app("com.b", ".Main", "Bravo"),
    ]);
    assert_eq!(source.launched(), Vec::<AppRef>::new());
    source.launch(&first).unwrap();
    source.launch(&second).unwrap();
    source.launch(&first).unwrap();
    assert_eq!(source.launched(), vec![first.clone(), second, first]);
}

#[test]
fn launch_fails_when_configured_to() {
    let source = FakeSource::new(vec![app("com.a", ".Main", "Alpha")]).with_launch_failure();
    let result = source.launch(&app_ref("com.a", ".Main"));
    assert!(matches!(result, Err(LaunchError::Failed(_))));
}

#[test]
fn launch_of_unlisted_app_is_not_found() {
    let source = FakeSource::new(vec![app("com.a", ".Main", "Alpha")]);
    let unknown = app_ref("com.missing", ".Main");
    assert_eq!(
        source.launch(&unknown),
        Err(LaunchError::NotFound(unknown.clone()))
    );
    assert_eq!(source.launched(), Vec::<AppRef>::new());
}

#[test]
fn scripted_failure_wins_over_not_found_for_listed_app() {
    let listed = app_ref("com.a", ".Main");
    let source = FakeSource::new(vec![app("com.a", ".Main", "Alpha")]).with_launch_failure();
    assert!(matches!(
        source.launch(&listed),
        Err(LaunchError::Failed(_))
    ));
}

#[test]
fn fake_source_is_send_sync_and_object_safe() {
    assert_send_sync::<FakeSource>();
    assert_send_sync::<Box<dyn AppSource>>();
    let boxed: Box<dyn AppSource> = Box::new(FakeSource::new(Vec::new()));
    assert_eq!(boxed.list_apps().unwrap(), Vec::<AppEntry>::new());
}
