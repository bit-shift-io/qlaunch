use qlaunch_core::model::{
    AppCategory, AppRef, Extra, ExtraKind, GroupRole, Id, Item, Model, ShortcutPayload,
};
use qlaunch_core::settings::Settings;
use qlaunch_core::store::{Document, SCHEMA_VERSION, Store, StoreError};
use serde_json::{Value, json};
use tempfile::tempdir;

fn store_in(dir: &tempfile::TempDir) -> Store {
    Store::new(dir.path().to_path_buf())
}

fn group_of(m: &Model, role: GroupRole) -> Id {
    m.role_holder(role).unwrap().id.clone()
}

fn app_item(id: &str, group: &Id, label: &str, package: &str, order: i32) -> Item {
    Item {
        id: id.into(),
        group_id: group.clone(),
        label: label.into(),
        label_overridden: false,
        app: Some(AppRef {
            package: package.into(),
            activity: "Main".into(),
        }),
        shortcut: None,
        uninstalled: false,
        order,
        icon_override: None,
    }
}

fn write_document(s: &Store, m: &Model, settings: &Settings) {
    let doc = Document {
        schema: SCHEMA_VERSION,
        settings: settings.clone(),
        root_group_id: m.root_group_id.clone(),
        groups: m.groups.clone(),
        items: m.items.clone(),
    };
    std::fs::create_dir_all(s.document_path().parent().unwrap()).unwrap();
    std::fs::write(s.document_path(), serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
}

fn read_json(s: &Store) -> Value {
    serde_json::from_slice(&std::fs::read(s.document_path()).unwrap()).unwrap()
}

fn leftover_temp_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "tmp"))
        .collect()
}

#[test]
fn round_trips_unicode_and_hostile_shortcut_extras() {
    let dir = tempdir().unwrap();
    let s = Store::new(dir.path().to_path_buf());
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    m.add_shortcut(
        &g,
        "Café ☕ 日本",
        ShortcutPayload::Android {
            action: Some("a=b|c".into()),
            data: Some("line1\nline2".into()),
            mime_type: None,
            categories: vec!["x|y".into()],
            package: None,
            component: None,
            flags: 0,
            extras: vec![Extra {
                key: "k=1|2".into(),
                kind: ExtraKind::String,
                value: "\"quoted\" | =".into(),
            }],
        },
    );
    s.save(&m, &Settings::default()).unwrap();
    let loaded = s.load().unwrap();
    assert_eq!(loaded.model, m);
    assert_eq!(loaded.model.items_in(&g)[0].label, "Café ☕ 日本");
    assert!(loaded.warnings.is_empty());
}

#[test]
fn a_command_shortcut_and_an_app_item_round_trip_with_settings() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    let apps = group_of(&m, GroupRole::Apps);
    let home = group_of(&m, GroupRole::Home);
    let app = m.add_app(
        AppRef {
            package: "org.mozilla.firefox".into(),
            activity: "Main".into(),
        },
        "Firefox",
    );
    m.rename_item(&app, "Fox").unwrap();
    m.items
        .iter_mut()
        .find(|i| i.id == app)
        .unwrap()
        .icon_override = Some("x.png".into());
    m.add_shortcut(
        &home,
        "List",
        ShortcutPayload::Command {
            command: "ls -la \"/tmp/a b\"".into(),
        },
    );
    m.add_group("Work");
    m.move_item(&app, &apps, 0).unwrap();
    let settings = Settings {
        icon_size: 77,
        ..Settings::default()
    };

    s.save(&m, &settings).unwrap();
    let loaded = s.load().unwrap();
    assert_eq!(loaded.model, m);
    assert_eq!(loaded.settings, settings);
}

#[test]
fn a_truncated_document_is_quarantined_and_defaults_rebuilt() {
    let dir = tempdir().unwrap();
    let s = Store::new(dir.path().to_path_buf());
    s.save(&Model::new_default(), &Settings::default()).unwrap();
    let mut bytes = std::fs::read(s.document_path()).unwrap();
    bytes.truncate(bytes.len() / 2);
    std::fs::write(s.document_path(), &bytes).unwrap();

    let loaded = s.load().unwrap();
    assert!(!loaded.warnings.is_empty());
    assert!(loaded.model.validate().is_ok());
    assert!(dir.path().join("qlaunch.json.corrupt").exists());
    assert!(!std::fs::read(s.document_path()).unwrap().is_empty());

    // The bad bytes are kept, not discarded.
    assert_eq!(
        std::fs::read(dir.path().join("qlaunch.json.corrupt")).unwrap(),
        bytes
    );
    // The rebuilt document is valid on disk and loads cleanly, as the same model.
    let again = s.load().unwrap();
    assert!(again.warnings.is_empty());
    assert_eq!(again.model, loaded.model);
}

#[test]
fn a_wrong_schema_is_quarantined_not_reinterpreted() {
    for schema in [0u64, 2, 99] {
        let dir = tempdir().unwrap();
        let s = store_in(&dir);
        let mut m = Model::new_default();
        m.add_group("From the future");
        s.save(&m, &Settings::default()).unwrap();

        let mut doc = read_json(&s);
        doc["schema"] = json!(schema);
        let original = serde_json::to_vec_pretty(&doc).unwrap();
        std::fs::write(s.document_path(), &original).unwrap();

        let loaded = s.load().unwrap();
        assert!(!loaded.warnings.is_empty(), "schema {schema}");
        assert!(
            loaded
                .warnings
                .iter()
                .any(|w| w.contains(&format!("schema version {schema}"))),
            "the warning names the schema: {:?}",
            loaded.warnings
        );
        // Not read as v1: the group it contained did not make it into the model.
        assert!(
            loaded
                .model
                .groups
                .iter()
                .all(|g| g.label != "From the future")
        );
        assert!(loaded.model.validate().is_ok());
        assert_eq!(
            std::fs::read(dir.path().join("qlaunch.json.corrupt")).unwrap(),
            original
        );
        assert_eq!(read_json(&s)["schema"], json!(SCHEMA_VERSION));
    }
}

#[test]
fn a_structurally_invalid_document_is_quarantined() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    let dup = m.role_holder(GroupRole::Apps).unwrap().clone();
    m.groups.push(qlaunch_core::model::Group {
        id: "second-apps".into(),
        ..dup
    });
    write_document(&s, &m, &Settings::default());
    let original = std::fs::read(s.document_path()).unwrap();

    let loaded = s.load().unwrap();
    assert!(!loaded.warnings.is_empty());
    assert!(loaded.model.validate().is_ok());
    assert_eq!(
        std::fs::read(dir.path().join("qlaunch.json.corrupt")).unwrap(),
        original
    );
}

#[test]
fn an_item_with_both_an_app_and_a_shortcut_is_quarantined() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    let apps = group_of(&m, GroupRole::Apps);
    let mut item = app_item("i1", &apps, "Both", "p", 0);
    item.shortcut = Some(ShortcutPayload::Command {
        command: "ls".into(),
    });
    m.items.push(item);
    write_document(&s, &m, &Settings::default());

    let loaded = s.load().unwrap();
    assert!(!loaded.warnings.is_empty());
    assert!(loaded.model.items.is_empty());
    assert!(dir.path().join("qlaunch.json.corrupt").exists());
}

#[test]
fn a_missing_required_key_is_quarantined() {
    for key in ["schema", "root_group_id", "groups", "items"] {
        let dir = tempdir().unwrap();
        let s = store_in(&dir);
        s.save(&Model::new_default(), &Settings::default()).unwrap();
        let mut doc = read_json(&s);
        doc.as_object_mut().unwrap().remove(key);
        std::fs::write(s.document_path(), serde_json::to_vec(&doc).unwrap()).unwrap();

        let loaded = s.load().unwrap();
        assert!(!loaded.warnings.is_empty(), "without {key}");
        assert!(
            dir.path().join("qlaunch.json.corrupt").exists(),
            "without {key}"
        );
    }
}

#[test]
fn missing_settings_fall_back_to_defaults_without_losing_the_model() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    m.add_group("Mine");
    s.save(&m, &Settings::default()).unwrap();

    let mut doc = read_json(&s);
    doc.as_object_mut().unwrap().remove("settings");
    std::fs::write(s.document_path(), serde_json::to_vec(&doc).unwrap()).unwrap();
    let loaded = s.load().unwrap();
    assert_eq!(loaded.model, m);
    assert_eq!(loaded.settings, Settings::default());
    assert!(loaded.warnings.is_empty());
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());

    // A settings object missing one key keeps the others.
    doc["settings"] = json!({ "icon_size": 55 });
    std::fs::write(s.document_path(), serde_json::to_vec(&doc).unwrap()).unwrap();
    let loaded = s.load().unwrap();
    assert_eq!(loaded.model, m);
    assert_eq!(loaded.settings.icon_size, 55);
    assert_eq!(
        loaded.settings.grid_columns_portrait,
        Settings::default().grid_columns_portrait
    );
}

#[test]
fn duplicate_app_refs_are_deduplicated_with_a_warning() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    let apps = group_of(&m, GroupRole::Apps);
    let games = group_of(&m, GroupRole::Games);
    m.items.push(app_item(
        "first",
        &apps,
        "Firefox",
        "org.mozilla.firefox",
        0,
    ));
    m.items
        .push(app_item("between", &apps, "Between", "other", 1));
    m.items.push(app_item(
        "second",
        &apps,
        "Firefox copy",
        "org.mozilla.firefox",
        2,
    ));
    m.items.push(app_item("last", &apps, "Last", "last.pkg", 3));
    m.items.push(app_item(
        "elsewhere",
        &games,
        "Firefox again",
        "org.mozilla.firefox",
        0,
    ));
    write_document(&s, &m, &Settings::default());

    let loaded = s.load().unwrap();

    let ids: Vec<&str> = loaded.model.items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids, ["first", "between", "last"]);
    assert_eq!(loaded.model.validate(), Ok(()));
    // The survivor keeps its own label and the group is re-indexed densely.
    assert_eq!(
        loaded.model.item(&"first".to_string()).unwrap().label,
        "Firefox"
    );
    let orders: Vec<i32> = loaded
        .model
        .items_in(&apps)
        .iter()
        .map(|i| i.order)
        .collect();
    assert_eq!(orders, [0, 1, 2]);
    assert_eq!(
        loaded
            .model
            .items_in(&apps)
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["first", "between", "last"]
    );

    assert_eq!(loaded.warnings.len(), 3, "{:?}", loaded.warnings);
    for (w, label) in loaded
        .warnings
        .iter()
        .zip(["Firefox copy", "Firefox again"])
    {
        assert!(w.contains("org.mozilla.firefox"), "{w}");
        assert!(w.contains(label), "{w}");
    }
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
    // The repair is persisted, so it is reported once.
    let on_disk = read_json(&s);
    let stored: Vec<&str> = on_disk["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(stored, ["first", "between", "last"]);
}

#[test]
fn absent_document_creates_defaults_without_warnings() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("nested").join("qlaunch");
    let s = Store::new(root.clone());
    let loaded = s.load().unwrap();
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.settings, Settings::default());
    assert_eq!(loaded.model.validate(), Ok(()));
    assert!(root.join("qlaunch.json").exists());
    assert!(root.join("icons").is_dir());
    assert!(!root.join("qlaunch.json.corrupt").exists());
    // The default ids were persisted, not regenerated.
    assert_eq!(s.load().unwrap().model, loaded.model);
}

#[test]
fn on_disk_strings_are_the_documented_ones() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    let apps = group_of(&m, GroupRole::Apps);
    m.add_shortcut(
        &apps,
        "android",
        ShortcutPayload::Android {
            action: Some("act".into()),
            data: None,
            mime_type: Some("text/plain".into()),
            categories: vec!["c".into()],
            package: Some("p".into()),
            component: Some("p/.C".into()),
            flags: 7,
            extras: vec![
                Extra {
                    key: "s".into(),
                    kind: ExtraKind::String,
                    value: "v".into(),
                },
                Extra {
                    key: "b".into(),
                    kind: ExtraKind::Bool,
                    value: "true".into(),
                },
                Extra {
                    key: "i".into(),
                    kind: ExtraKind::Int,
                    value: "3".into(),
                },
            ],
        },
    );
    m.add_shortcut(
        &apps,
        "command",
        ShortcutPayload::Command {
            command: "ls".into(),
        },
    );
    m.add_app(
        AppRef {
            package: "pkg".into(),
            activity: "act".into(),
        },
        "App",
    );
    s.save(&m, &Settings::default()).unwrap();
    let doc = read_json(&s);

    let mut top: Vec<&str> = doc
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    top.sort();
    assert_eq!(
        top,
        ["groups", "items", "root_group_id", "schema", "settings"]
    );
    assert_eq!(doc["schema"], json!(1));
    assert_eq!(doc["root_group_id"], json!(m.root_group_id));
    assert_eq!(doc["settings"]["layout"], json!("tiles"));
    assert_eq!(doc["settings"]["tab_visibility"], json!("always"));

    let mut roles: Vec<&str> = doc["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["role"].as_str().unwrap())
        .collect();
    roles.sort();
    assert_eq!(roles, ["apps", "games", "hidden", "home", "root"]);
    let root = &doc["groups"][0];
    let mut group_keys: Vec<&str> = root
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    group_keys.sort();
    assert_eq!(group_keys, ["id", "label", "order", "parent_id", "role"]);
    assert_eq!(root["parent_id"], Value::Null);

    let items = doc["items"].as_array().unwrap();
    let by_label = |l: &str| items.iter().find(|i| i["label"] == l).unwrap();
    let mut item_keys: Vec<&str> = by_label("App")
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    item_keys.sort();
    assert_eq!(
        item_keys,
        [
            "app",
            "group_id",
            "icon_override",
            "id",
            "label",
            "label_overridden",
            "order",
            "shortcut",
            "uninstalled"
        ]
    );
    assert_eq!(
        by_label("App")["app"],
        json!({ "package": "pkg", "activity": "act" })
    );
    assert_eq!(by_label("App")["shortcut"], Value::Null);
    assert_eq!(by_label("App")["icon_override"], Value::Null);

    let android = &by_label("android")["shortcut"];
    assert_eq!(android["type"], json!("android"));
    assert_eq!(android["mime_type"], json!("text/plain"));
    assert_eq!(android["flags"], json!(7));
    let kinds: Vec<&str> = android["extras"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["string", "bool", "int"]);
    assert_eq!(
        by_label("command")["shortcut"],
        json!({ "type": "command", "command": "ls" })
    );
}

#[test]
fn app_category_strings_are_stable() {
    assert_eq!(
        serde_json::to_value(AppCategory::Game).unwrap(),
        json!("game")
    );
    assert_eq!(
        serde_json::to_value(AppCategory::App).unwrap(),
        json!("app")
    );
    assert_eq!(
        serde_json::to_value(AppCategory::Unknown).unwrap(),
        json!("unknown")
    );
}

#[test]
fn save_leaves_no_temp_file_and_overwrites_a_corrupt_document() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    // A stale temp file from an earlier crash must not get in the way, and is
    // not ours to delete.
    let stale = dir.path().join(".qlaunch.json.tmp");
    std::fs::write(&stale, b"half a wri").unwrap();
    std::fs::write(s.document_path(), b"{ not json").unwrap();

    let m = Model::new_default();
    s.save(&m, &Settings::default()).unwrap();

    assert_eq!(
        leftover_temp_files(dir.path()),
        std::slice::from_ref(&stale)
    );
    assert_eq!(std::fs::read(&stale).unwrap(), b"half a wri");
    assert_eq!(s.load().unwrap().model, m);
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
}

#[test]
fn save_refuses_a_model_that_load_would_reject() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let good = Model::new_default();
    s.save(&good, &Settings::default()).unwrap();
    let before = std::fs::read(s.document_path()).unwrap();

    let mut bad = good.clone();
    bad.groups.retain(|g| g.role != Some(GroupRole::Apps));
    assert!(matches!(
        s.save(&bad, &Settings::default()),
        Err(StoreError::Invalid(_))
    ));
    assert_eq!(std::fs::read(s.document_path()).unwrap(), before);
}

#[test]
fn save_writes_nothing_at_all_for_an_invalid_model() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut bad = Model::new_default();
    let apps = group_of(&bad, GroupRole::Apps);
    bad.items.push(app_item("a", &apps, "A", "dup", 0));
    bad.items.push(app_item("b", &apps, "B", "dup", 1));

    let err = s.save(&bad, &Settings::default()).unwrap_err();
    assert!(matches!(err, StoreError::Invalid(_)), "{err:?}");
    assert!(!s.document_path().exists());
    assert!(!dir.path().join("icons").exists());
    assert!(leftover_temp_files(dir.path()).is_empty());
}

#[test]
fn an_unreadable_document_warns_and_then_refuses_every_save_and_leaves_it_alone() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    std::fs::create_dir(s.document_path()).unwrap();

    let loaded = s.load().unwrap();
    loaded.model.validate().unwrap();
    assert!(!loaded.warnings.is_empty());

    let err = s.save(&loaded.model, &loaded.settings).unwrap_err();
    assert!(matches!(err, StoreError::WritesBlocked), "{err:?}");
    assert!(matches!(s.quarantine(), Err(StoreError::WritesBlocked)));

    assert!(s.document_path().is_dir());
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
    assert!(leftover_temp_files(dir.path()).is_empty());
}

#[test]
fn quarantine_moves_the_file_aside_and_never_overwrites_an_earlier_one() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let corrupt = dir.path().join("qlaunch.json.corrupt");

    std::fs::write(s.document_path(), b"one").unwrap();
    assert_eq!(s.quarantine().unwrap(), corrupt);
    assert!(!s.document_path().exists());
    assert_eq!(std::fs::read(&corrupt).unwrap(), b"one");

    std::fs::write(s.document_path(), b"two").unwrap();
    let second = s.quarantine().unwrap();
    assert_eq!(second, dir.path().join("qlaunch.json.corrupt.1"));
    std::fs::write(s.document_path(), b"three").unwrap();
    let third = s.quarantine().unwrap();
    assert_eq!(third, dir.path().join("qlaunch.json.corrupt.2"));

    // The first corruption, the precious one, is intact.
    assert_eq!(std::fs::read(&corrupt).unwrap(), b"one");
    assert_eq!(std::fs::read(&second).unwrap(), b"two");
    assert_eq!(std::fs::read(&third).unwrap(), b"three");

    assert!(matches!(s.quarantine(), Err(StoreError::Io(_))));
}

#[test]
fn a_second_corruption_during_load_keeps_the_first_quarantined_file() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);

    std::fs::write(s.document_path(), b"{ the user's first mess").unwrap();
    let first = s.load().unwrap();
    assert!(first.warnings[0].contains("qlaunch.json.corrupt"));

    std::fs::write(s.document_path(), b"{ a second mess").unwrap();
    let second = s.load().unwrap();
    assert!(second.warnings[0].contains("qlaunch.json.corrupt.1"));

    assert_eq!(
        std::fs::read(dir.path().join("qlaunch.json.corrupt")).unwrap(),
        b"{ the user's first mess"
    );
    assert_eq!(
        std::fs::read(dir.path().join("qlaunch.json.corrupt.1")).unwrap(),
        b"{ a second mess"
    );
}

#[test]
fn an_out_of_range_number_is_clamped_and_the_model_survives() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    let mine = m.add_group("My hand-arranged group");
    let apps = group_of(&m, GroupRole::Apps);
    m.items
        .push(app_item("i1", &apps, "My renamed app", "p", 0));
    s.save(&m, &Settings::default()).unwrap();
    let mut doc = read_json(&s);
    doc["settings"]["icon_size"] = json!(-5);
    doc["settings"]["wallpaper_alpha"] = json!(999);
    std::fs::write(s.document_path(), serde_json::to_vec_pretty(&doc).unwrap()).unwrap();

    let loaded = s.load().unwrap();
    assert_eq!(loaded.settings.icon_size, 0);
    assert_eq!(loaded.settings.wallpaper_alpha, 100);
    assert!(
        loaded.warnings.iter().any(|w| w.contains("icon_size")),
        "{:?}",
        loaded.warnings
    );
    // Not rebuilt from defaults: the user's group and item are still there.
    assert_eq!(loaded.model, m);
    assert!(loaded.model.groups.iter().any(|g| g.id == mine));
    assert_eq!(
        loaded.model.item(&"i1".to_string()).unwrap().label,
        "My renamed app"
    );
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
}

#[test]
fn a_number_field_of_the_wrong_json_type_is_quarantined() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    m.add_group("Mine");
    s.save(&m, &Settings::default()).unwrap();
    let mut doc = read_json(&s);
    doc["settings"]["icon_size"] = json!("big");
    let original = serde_json::to_vec_pretty(&doc).unwrap();
    std::fs::write(s.document_path(), &original).unwrap();

    let loaded = s.load().unwrap();
    assert!(!loaded.warnings.is_empty());
    assert_ne!(loaded.model, m);
    assert_eq!(
        std::fs::read(dir.path().join("qlaunch.json.corrupt")).unwrap(),
        original
    );
}

#[test]
fn a_hand_written_android_shortcut_may_omit_its_lists() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    let home = group_of(&m, GroupRole::Home);
    let mut item = app_item("sc", &home, "Hand written", "p", 0);
    item.app = None;
    item.shortcut = Some(ShortcutPayload::Android {
        action: Some("android.intent.action.VIEW".into()),
        data: None,
        mime_type: None,
        categories: Vec::new(),
        package: None,
        component: None,
        flags: 0,
        extras: Vec::new(),
    });
    m.items.push(item);
    s.save(&m, &Settings::default()).unwrap();
    let mut doc = read_json(&s);
    let shortcut = doc["items"][0]["shortcut"].as_object_mut().unwrap();
    shortcut.remove("categories");
    shortcut.remove("extras");
    std::fs::write(s.document_path(), serde_json::to_vec_pretty(&doc).unwrap()).unwrap();

    let loaded = s.load().unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.model, m);
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
}

fn duplicate_app_document(s: &Store) -> Vec<u8> {
    let mut m = Model::new_default();
    let apps = group_of(&m, GroupRole::Apps);
    m.items.push(app_item("a", &apps, "Kept", "dup.pkg", 0));
    let mut copy = app_item("b", &apps, "My custom copy", "dup.pkg", 1);
    copy.icon_override = Some("b.png".into());
    m.items.push(copy);
    write_document(s, &m, &Settings::default());
    std::fs::read(s.document_path()).unwrap()
}

#[test]
fn a_repair_that_removes_items_keeps_the_original_bytes_in_a_bak_file() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let original = duplicate_app_document(&s);

    let loaded = s.load().unwrap();
    assert_eq!(loaded.model.items.len(), 1);

    let bak = dir.path().join("qlaunch.json.bak");
    assert_eq!(std::fs::read(&bak).unwrap(), original);
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains(&bak.display().to_string())),
        "{:?}",
        loaded.warnings
    );
    // A backup is not a quarantine.
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
    // The document itself is the repaired one.
    assert_eq!(read_json(&s)["items"].as_array().unwrap().len(), 1);
    assert!(leftover_temp_files(dir.path()).is_empty());
}

#[test]
fn a_second_repair_takes_the_next_bak_name_and_leaves_the_first() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let first = duplicate_app_document(&s);
    s.load().unwrap();

    let second = {
        let mut doc = read_json(&s);
        doc["settings"]["icon_size"] = json!(900);
        let bytes = serde_json::to_vec_pretty(&doc).unwrap();
        std::fs::write(s.document_path(), &bytes).unwrap();
        bytes
    };
    let loaded = s.load().unwrap();
    let bak1 = dir.path().join("qlaunch.json.bak.1");
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains(&bak1.display().to_string())),
        "{:?}",
        loaded.warnings
    );

    assert_eq!(
        std::fs::read(dir.path().join("qlaunch.json.bak")).unwrap(),
        first
    );
    assert_eq!(std::fs::read(&bak1).unwrap(), second);
    assert!(!dir.path().join("qlaunch.json.bak.2").exists());
}

#[test]
fn a_clean_load_creates_no_backup() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    // Absent document, then a saved one, then a load of each.
    assert!(s.load().unwrap().warnings.is_empty());
    assert!(s.load().unwrap().warnings.is_empty());
    let mut m = Model::new_default();
    m.add_group("Mine");
    s.save(&m, &Settings::default()).unwrap();
    assert!(s.load().unwrap().warnings.is_empty());

    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.iter().all(|n| !n.starts_with("qlaunch.json.")),
        "{names:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_repair_is_not_written_back_when_its_original_cannot_be_kept() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let original = duplicate_app_document(&s);
    let writable = std::fs::metadata(dir.path()).unwrap().permissions();
    let mut read_only = writable.clone();
    read_only.set_mode(0o555);
    std::fs::set_permissions(dir.path(), read_only).unwrap();
    if std::fs::File::create(dir.path().join("probe")).is_ok() {
        // Running as root: permissions do not stop us, so nothing to test.
        std::fs::remove_file(dir.path().join("probe")).unwrap();
        std::fs::set_permissions(dir.path(), writable).unwrap();
        return;
    }

    let loaded = s.load();
    std::fs::set_permissions(dir.path(), writable).unwrap();
    let loaded = loaded.unwrap();

    assert_eq!(loaded.model.items.len(), 1);
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains("could not be kept")),
        "{:?}",
        loaded.warnings
    );
    assert_eq!(std::fs::read(s.document_path()).unwrap(), original);
    assert!(!dir.path().join("qlaunch.json.bak").exists());
}

#[test]
fn two_writers_never_leave_a_mixed_document() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let make = |label: &str, size: i64, reps: usize| {
        let mut m = Model::new_default();
        m.add_group(&label.repeat(reps));
        let settings = Settings {
            icon_size: size,
            ..Settings::default()
        };
        (m, settings)
    };
    // Different lengths, so a mixture or truncation could not pass for either.
    let a = make("a", 11, 40_000);
    let b = make("b", 22, 5_000);
    let expect = |(m, settings): &(Model, Settings)| Document {
        schema: SCHEMA_VERSION,
        settings: settings.clone(),
        root_group_id: m.root_group_id.clone(),
        groups: m.groups.clone(),
        items: m.items.clone(),
    };
    let (doc_a, doc_b) = (expect(&a), expect(&b));
    let read_document = |s: &Store| -> Document {
        serde_json::from_slice(&std::fs::read(s.document_path()).unwrap())
            .expect("the document is always complete JSON")
    };

    s.save(&a.0, &a.1).unwrap();
    let done = Arc::new(AtomicBool::new(false));
    let writers: Vec<_> = [a, b]
        .into_iter()
        .map(|(m, settings)| {
            let s = s.clone();
            std::thread::spawn(move || {
                for _ in 0..150 {
                    s.save(&m, &settings).unwrap();
                }
            })
        })
        .collect();
    let reader = {
        let (s, done, doc_a, doc_b) = (s.clone(), done.clone(), doc_a.clone(), doc_b.clone());
        std::thread::spawn(move || {
            let mut reads = 0;
            while !done.load(Ordering::Relaxed) {
                let doc = read_document(&s);
                assert!(doc == doc_a || doc == doc_b, "a reader saw a mixture");
                reads += 1;
            }
            reads
        })
    };
    for w in writers {
        w.join().unwrap();
    }
    done.store(true, Ordering::Relaxed);
    reader.join().unwrap();

    let last = read_document(&s);
    assert!(last == doc_a || last == doc_b, "the file is one of the two");
    assert!(leftover_temp_files(dir.path()).is_empty());
    assert_eq!(s.load().unwrap().warnings, Vec::<String>::new());
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
}

#[test]
fn an_android_shortcut_needs_nothing_but_its_type() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let m = Model::new_default();
    s.save(&m, &Settings::default()).unwrap();
    let home = group_of(&m, GroupRole::Home);
    let mut doc = read_json(&s);
    doc["items"] = json!([
        {
            "id": "min", "group_id": home, "label": "Minimal",
            "label_overridden": false, "app": null, "uninstalled": false, "order": 0,
            "icon_override": null,
            "shortcut": {"type": "android", "action": "android.intent.action.VIEW"}
        },
        {
            "id": "bare", "group_id": home, "label": "Bare",
            "label_overridden": false, "app": null, "uninstalled": false, "order": 1,
            "icon_override": null,
            "shortcut": {"type": "android"}
        }
    ]);
    std::fs::write(s.document_path(), serde_json::to_vec_pretty(&doc).unwrap()).unwrap();

    let loaded = s.load().unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
    assert_eq!(
        loaded.model.item(&"min".to_string()).unwrap().shortcut,
        Some(ShortcutPayload::Android {
            action: Some("android.intent.action.VIEW".into()),
            data: None,
            mime_type: None,
            categories: Vec::new(),
            package: None,
            component: None,
            flags: 0,
            extras: Vec::new(),
        })
    );
    assert_eq!(
        loaded.model.item(&"bare".to_string()).unwrap().shortcut,
        Some(ShortcutPayload::Android {
            action: None,
            data: None,
            mime_type: None,
            categories: Vec::new(),
            package: None,
            component: None,
            flags: 0,
            extras: Vec::new(),
        })
    );
}

#[test]
fn a_command_shortcut_without_its_command_is_quarantined() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let m = Model::new_default();
    s.save(&m, &Settings::default()).unwrap();
    let home = group_of(&m, GroupRole::Home);
    let mut doc = read_json(&s);
    doc["items"] = json!([{
        "id": "c", "group_id": home, "label": "No command",
        "label_overridden": false, "app": null, "uninstalled": false, "order": 0,
        "icon_override": null, "shortcut": {"type": "command"}
    }]);
    std::fs::write(s.document_path(), serde_json::to_vec_pretty(&doc).unwrap()).unwrap();

    let loaded = s.load().unwrap();
    assert!(!loaded.warnings.is_empty());
    assert!(loaded.model.items.is_empty());
    assert!(dir.path().join("qlaunch.json.corrupt").exists());
}

#[test]
fn a_setting_that_is_not_an_i64_integer_is_quarantined() {
    // The boundary of "clamped" versus "corrupt" is deliberate: only integers
    // that fit in i64 are repaired.
    for literal in [
        "40.5",
        "1e30",
        "99999999999999999999",
        "18446744073709551615",
        "-1e30",
        "true",
        "null",
        "\"big\"",
    ] {
        let dir = tempdir().unwrap();
        let s = store_in(&dir);
        let mut m = Model::new_default();
        m.add_group("Mine");
        s.save(&m, &Settings::default()).unwrap();
        let mut doc = read_json(&s);
        doc["settings"]["icon_size"] = serde_json::from_str(literal).unwrap();
        let original = serde_json::to_vec_pretty(&doc).unwrap();
        std::fs::write(s.document_path(), &original).unwrap();

        let loaded = s.load().unwrap();
        assert!(!loaded.warnings.is_empty(), "{literal}");
        assert_ne!(loaded.model, m, "{literal}");
        assert_eq!(
            std::fs::read(dir.path().join("qlaunch.json.corrupt")).unwrap(),
            original,
            "{literal}"
        );
    }
}

#[test]
fn load_persists_its_repairs_and_reports_them_once() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    let apps = group_of(&m, GroupRole::Apps);
    m.items.push(app_item("a", &apps, "Kept", "dup.pkg", 0));
    m.items.push(app_item("b", &apps, "Copy", "dup.pkg", 1));
    let settings = Settings {
        grid_columns_portrait: 0,
        icon_size: -5,
        label_size: 500,
        ..Settings::default()
    };
    write_document(&s, &m, &settings);

    let first = s.load().unwrap();
    assert_eq!(first.warnings.len(), 5, "{:?}", first.warnings);
    for needle in [
        "grid_columns_portrait",
        "icon_size",
        "label_size",
        "Copy",
        "qlaunch.json.bak",
    ] {
        assert!(
            first.warnings.iter().any(|w| w.contains(needle)),
            "{needle}: {:?}",
            first.warnings
        );
    }
    assert_eq!(first.settings.grid_columns_portrait, 1);
    assert_eq!(first.settings.icon_size, 0);
    assert_eq!(first.settings.label_size, 100);
    assert_eq!(first.model.items.len(), 1);

    let on_disk = read_json(&s);
    assert_eq!(on_disk["settings"]["grid_columns_portrait"], json!(1));
    assert_eq!(on_disk["settings"]["icon_size"], json!(0));
    assert_eq!(on_disk["settings"]["label_size"], json!(100));
    assert_eq!(on_disk["items"].as_array().unwrap().len(), 1);

    let second = s.load().unwrap();
    assert!(second.warnings.is_empty(), "{:?}", second.warnings);
    assert_eq!(second.model, first.model);
    assert_eq!(second.settings, first.settings);
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
}

#[test]
fn a_clean_document_is_not_rewritten_by_load() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    m.add_group("Mine");
    // Hand-formatted, so a rewrite would change the bytes.
    write_document(&s, &m, &Settings::default());
    let mut bytes = std::fs::read(s.document_path()).unwrap();
    bytes.extend_from_slice(b"\n\n");
    std::fs::write(s.document_path(), &bytes).unwrap();

    let loaded = s.load().unwrap();
    assert!(loaded.warnings.is_empty());
    assert_eq!(std::fs::read(s.document_path()).unwrap(), bytes);
}

#[test]
fn a_repair_that_cannot_be_written_still_returns_the_model_with_a_warning() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let mut m = Model::new_default();
    m.add_group("Mine");
    let settings = Settings {
        icon_size: 500,
        ..Settings::default()
    };
    write_document(&s, &m, &settings);
    let before = std::fs::read(s.document_path()).unwrap();
    // A file where `icons/` should be makes `save` fail.
    std::fs::write(dir.path().join("icons"), b"in the way").unwrap();

    let loaded = s.load().unwrap();
    assert_eq!(loaded.model, m);
    assert_eq!(loaded.settings.icon_size, 100);
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains("could not be saved")),
        "{:?}",
        loaded.warnings
    );
    assert_eq!(std::fs::read(s.document_path()).unwrap(), before);
}

#[test]
fn icon_path_rejects_anything_that_is_not_a_plain_file_name() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    for name in [
        "",
        ".",
        "..",
        "../../../etc/passwd",
        "../x.png",
        "a/b.png",
        "a\\b.png",
        "/etc/passwd",
        "dir/",
        "x\0.png",
    ] {
        assert!(
            matches!(s.icon_path(name), Err(StoreError::InvalidIconName(_))),
            "{name:?} must be rejected"
        );
    }

    let ok = "0b0f3e0e-6f5e-4b0e-9a56-0d1d1a9c4f10.png";
    let path = s.icon_path(ok).unwrap();
    assert_eq!(path, dir.path().join("icons").join(ok));
    assert_eq!(path.parent().unwrap(), dir.path().join("icons"));
}

#[test]
fn write_icon_derives_the_file_name_from_the_item_id() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let id: Id = "0b0f3e0e-6f5e-4b0e-9a56-0d1d1a9c4f10".into();

    let name = s.write_icon(&id, b"\x89PNG fake").unwrap();
    assert_eq!(name, format!("{id}.png"));
    assert_eq!(
        std::fs::read(s.icon_path(&name).unwrap()).unwrap(),
        b"\x89PNG fake"
    );
    assert!(leftover_temp_files(&dir.path().join("icons")).is_empty());

    let name2 = s.write_icon(&id, b"second").unwrap();
    assert_eq!(name2, name);
    assert_eq!(
        std::fs::read(s.icon_path(&name).unwrap()).unwrap(),
        b"second"
    );

    assert!(matches!(
        s.write_icon(&"../../escape".to_string(), b"x"),
        Err(StoreError::InvalidIconName(_))
    ));
    assert!(!dir.path().join("escape.png").exists());
    assert!(!dir.path().parent().unwrap().join("escape.png").exists());
}
