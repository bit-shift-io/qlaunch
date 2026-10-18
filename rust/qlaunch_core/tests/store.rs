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
    let on_disk = std::fs::read(s.document_path()).unwrap();

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

    assert_eq!(loaded.warnings.len(), 2, "{:?}", loaded.warnings);
    for (w, label) in loaded
        .warnings
        .iter()
        .zip(["Firefox copy", "Firefox again"])
    {
        assert!(w.contains("org.mozilla.firefox"), "{w}");
        assert!(w.contains(label), "{w}");
    }
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
    assert_eq!(std::fs::read(s.document_path()).unwrap(), on_disk);
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
    // A stale temp file from an earlier crash must not get in the way.
    std::fs::write(dir.path().join(".qlaunch.json.tmp"), b"half a wri").unwrap();
    std::fs::write(s.document_path(), b"{ not json").unwrap();

    let m = Model::new_default();
    s.save(&m, &Settings::default()).unwrap();

    assert!(leftover_temp_files(dir.path()).is_empty());
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
fn an_unreadable_document_is_an_error_not_a_reason_to_overwrite() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    std::fs::create_dir(s.document_path()).unwrap();
    assert!(matches!(s.load(), Err(StoreError::Io(_))));
    assert!(s.document_path().is_dir());
    assert!(!dir.path().join("qlaunch.json.corrupt").exists());
}

#[test]
fn quarantine_moves_the_file_aside_and_replaces_an_earlier_quarantine() {
    let dir = tempdir().unwrap();
    let s = store_in(&dir);
    let corrupt = dir.path().join("qlaunch.json.corrupt");

    std::fs::write(s.document_path(), b"one").unwrap();
    assert_eq!(s.quarantine().unwrap(), corrupt);
    assert!(!s.document_path().exists());
    assert_eq!(std::fs::read(&corrupt).unwrap(), b"one");

    std::fs::write(s.document_path(), b"two").unwrap();
    s.quarantine().unwrap();
    assert_eq!(std::fs::read(&corrupt).unwrap(), b"two");

    assert!(matches!(s.quarantine(), Err(StoreError::Io(_))));
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
