use qlaunch_core::model::{AppCategory, AppRef, IconImage};
use qlaunch_core::platform::android::{
    AndroidHost, AndroidList, AndroidSource, parse_list_json,
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
