use qlaunch_core::settings::{
    LabelColour, LayoutStyle, Orientation, Settings, SettingsPatch, TabVisibility, Typeface,
};

#[test]
fn defaults_match_the_spec() {
    let s = Settings::default();
    assert_eq!(s.grid_columns_portrait, 4);
    assert_eq!(s.grid_columns_landscape, 6);
    assert_eq!(s.layout, LayoutStyle::Tiles);
    assert_eq!(s.tab_visibility, TabVisibility::Always);
    assert_eq!(s.wallpaper_alpha, 0);
    assert!(s.status_bar_visible);
    assert!(s.icons_visible);
    assert_eq!(s.icon_size, 40);
    assert_eq!(s.icon_alpha, 100);
    assert!(s.labels_visible);
    assert_eq!(s.label_size, 14);
    assert_eq!(s.label_typeface, Typeface::Normal);
    assert_eq!(s.label_colour, LabelColour::White);
    assert!(s.label_shadow);
    assert_eq!(s.orientation, Orientation::Auto);
}

#[test]
fn clamping_writes_the_clamped_value_not_the_default() {
    let mut s = Settings::default();
    s.apply(&SettingsPatch {
        icon_size: Some(999),
        ..Default::default()
    });
    s.clamp();
    assert_eq!(s.icon_size, 100);
}

#[test]
fn grid_columns_clamp_to_one_through_twelve() {
    let mut s = Settings::default();
    s.apply(&SettingsPatch {
        grid_columns_portrait: Some(0),
        grid_columns_landscape: Some(50),
        ..Default::default()
    });
    s.clamp();
    assert_eq!(s.grid_columns_portrait, 1);
    assert_eq!(s.grid_columns_landscape, 12);
}

#[test]
fn every_ranged_field_clamps_to_its_maximum() {
    let mut s = Settings {
        wallpaper_alpha: 255,
        icon_size: u32::MAX,
        icon_alpha: 255,
        label_size: u32::MAX,
        ..Settings::default()
    };
    s.clamp();
    assert_eq!(s.wallpaper_alpha, 100);
    assert_eq!(s.icon_size, 100);
    assert_eq!(s.icon_alpha, 100);
    assert_eq!(s.label_size, 100);
}

#[test]
fn clamping_leaves_in_range_values_alone_and_is_idempotent() {
    let mut s = Settings {
        grid_columns_portrait: 12,
        grid_columns_landscape: 1,
        wallpaper_alpha: 0,
        icon_size: 0,
        icon_alpha: 100,
        label_size: 100,
        ..Settings::default()
    };
    let before = s.clone();
    s.clamp();
    assert_eq!(s, before);

    s.apply(&SettingsPatch {
        icon_size: Some(999),
        grid_columns_portrait: Some(0),
        ..Default::default()
    });
    s.clamp();
    let once = s.clone();
    s.clamp();
    assert_eq!(s, once);
}

#[test]
fn an_empty_patch_changes_nothing() {
    let mut s = Settings::default();
    s.apply(&SettingsPatch::default());
    assert_eq!(s, Settings::default());
}

#[test]
fn apply_overwrites_only_the_some_fields_and_does_not_clamp() {
    let mut s = Settings::default();
    s.apply(&SettingsPatch {
        layout: Some(LayoutStyle::List),
        label_shadow: Some(false),
        icon_size: Some(999),
        ..Default::default()
    });
    assert_eq!(s.layout, LayoutStyle::List);
    assert!(!s.label_shadow);
    assert_eq!(s.icon_size, 999);
    assert_eq!(
        Settings {
            layout: LayoutStyle::Tiles,
            label_shadow: true,
            icon_size: 40,
            ..s.clone()
        },
        Settings::default()
    );
}

#[test]
fn serde_uses_the_documented_lowercase_strings() {
    let s = Settings {
        layout: LayoutStyle::Centred,
        tab_visibility: TabVisibility::LandscapeOnly,
        label_typeface: Typeface::Monospace,
        label_colour: LabelColour::Grey,
        orientation: Orientation::Auto,
        ..Settings::default()
    };
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["layout"], "centred");
    assert_eq!(v["tab_visibility"], "landscape_only");
    assert_eq!(v["label_typeface"], "monospace");
    assert_eq!(v["label_colour"], "grey");
    assert_eq!(v["orientation"], "auto");
    assert_eq!(serde_json::from_value::<Settings>(v).unwrap(), s);

    let json = serde_json::to_string(&Settings::default()).unwrap();
    assert!(json.contains("\"layout\":\"tiles\""), "{json}");

    for (value, text) in [
        (serde_json::to_value(LayoutStyle::List).unwrap(), "list"),
        (serde_json::to_value(TabVisibility::Always).unwrap(), "always"),
        (serde_json::to_value(TabVisibility::PortraitOnly).unwrap(), "portrait_only"),
        (serde_json::to_value(TabVisibility::Never).unwrap(), "never"),
        (serde_json::to_value(Typeface::Normal).unwrap(), "normal"),
        (serde_json::to_value(Typeface::Sans).unwrap(), "sans"),
        (serde_json::to_value(Typeface::Serif).unwrap(), "serif"),
        (serde_json::to_value(LabelColour::White).unwrap(), "white"),
        (serde_json::to_value(LabelColour::Black).unwrap(), "black"),
        (serde_json::to_value(Orientation::Landscape).unwrap(), "landscape"),
        (serde_json::to_value(Orientation::Portrait).unwrap(), "portrait"),
    ] {
        assert_eq!(value, text);
    }
}

#[test]
fn field_names_serialise_verbatim() {
    let v = serde_json::to_value(Settings::default()).unwrap();
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "grid_columns_landscape",
            "grid_columns_portrait",
            "icon_alpha",
            "icon_size",
            "icons_visible",
            "label_colour",
            "label_shadow",
            "label_size",
            "label_typeface",
            "labels_visible",
            "layout",
            "orientation",
            "status_bar_visible",
            "tab_visibility",
            "wallpaper_alpha",
        ]
    );
}
