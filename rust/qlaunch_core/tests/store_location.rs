use std::fs;
use std::path::{Path, PathBuf};

use qlaunch_core::settings::Settings;
use qlaunch_core::store::{Store, StoreError, config_root_from};
use qlaunch_core::usage::Usage;
use tempfile::tempdir;

fn root(xdg: Option<&str>, home: Option<&str>) -> Result<PathBuf, StoreError> {
    config_root_from(xdg.map(Path::new), home.map(Path::new))
}

#[test]
fn an_absolute_xdg_config_home_is_used() {
    assert_eq!(
        root(Some("/tmp/xdg-test"), Some("/home/me")).unwrap(),
        PathBuf::from("/tmp/xdg-test/bitshift/qlaunch")
    );
}

#[test]
fn an_absolute_xdg_config_home_is_used_without_a_home_directory() {
    assert_eq!(
        root(Some("/tmp/xdg-test"), None).unwrap(),
        PathBuf::from("/tmp/xdg-test/bitshift/qlaunch")
    );
}

#[test]
fn an_unset_xdg_config_home_falls_back_to_home_dot_config() {
    assert_eq!(
        root(None, Some("/tmp/home-test")).unwrap(),
        PathBuf::from("/tmp/home-test/.config/bitshift/qlaunch")
    );
}

#[test]
fn an_empty_xdg_config_home_falls_back() {
    assert_eq!(
        root(Some(""), Some("/tmp/home-test")).unwrap(),
        PathBuf::from("/tmp/home-test/.config/bitshift/qlaunch")
    );
}

#[test]
fn a_relative_xdg_config_home_falls_back() {
    for relative in ["config", "./config", "../config", "."] {
        assert_eq!(
            root(Some(relative), Some("/tmp/home-test")).unwrap(),
            PathBuf::from("/tmp/home-test/.config/bitshift/qlaunch"),
            "XDG_CONFIG_HOME={relative:?}"
        );
    }
}

#[test]
fn a_relative_home_is_unavailable() {
    for xdg in [None, Some(""), Some("relative/path")] {
        assert!(
            matches!(root(xdg, Some("home")), Err(StoreError::Io(_))),
            "XDG_CONFIG_HOME={xdg:?}"
        );
    }
    assert_eq!(
        root(Some("/tmp/xdg-test"), Some("home")).unwrap(),
        PathBuf::from("/tmp/xdg-test/bitshift/qlaunch")
    );
}

#[test]
fn no_usable_xdg_path_and_no_home_is_an_error() {
    for xdg in [None, Some(""), Some("relative/path")] {
        assert!(
            matches!(root(xdg, None), Err(StoreError::Io(_))),
            "XDG_CONFIG_HOME={xdg:?}"
        );
    }
}

#[test]
fn an_unwritable_config_root_still_yields_a_default_model_and_a_warning() {
    let dir = tempdir().unwrap();
    // A plain file where a directory must be created fails for every user,
    // root included, which a read-only directory would not.
    fs::write(dir.path().join("bitshift"), b"not a directory").unwrap();
    let root = config_root_from(Some(dir.path()), None).unwrap();

    let loaded = Store::new(root.clone()).load().unwrap();

    loaded.model.validate().unwrap();
    assert!(loaded.model.items.is_empty());
    assert!(!loaded.model.groups.is_empty());
    assert_eq!(loaded.settings, qlaunch_core::settings::Settings::default());
    assert!(!loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert!(!root.exists());
}

#[test]
fn a_corrupt_document_is_still_quarantined_and_reported_when_the_defaults_cannot_be_written() {
    let dir = tempdir().unwrap();
    let store = Store::new(dir.path().to_path_buf());
    fs::write(store.document_path(), b"{ not json").unwrap();
    // Block the icons directory so that writing the defaults fails.
    fs::write(dir.path().join("icons"), b"not a directory").unwrap();

    let loaded = store.load().unwrap();

    loaded.model.validate().unwrap();
    assert!(!store.document_path().exists());
    assert_eq!(
        fs::read(dir.path().join("qlaunch.json.corrupt")).unwrap(),
        b"{ not json"
    );
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains("qlaunch.json.corrupt")),
        "{:?}",
        loaded.warnings
    );
    assert!(loaded.warnings.len() >= 2, "{:?}", loaded.warnings);
}

#[test]
fn a_document_that_exists_but_cannot_be_read_is_left_untouched() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let store = Store::new(dir.path().to_path_buf());
    let original = br#"{"schema": 1, "this": "is somebody's real document"}"#;
    fs::write(store.document_path(), original).unwrap();
    fs::set_permissions(store.document_path(), fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(store.document_path()).is_ok() {
        // Running as root: mode 000 does not stop reads, so nothing to test.
        return;
    }

    let loaded = store.load().unwrap();

    fs::set_permissions(store.document_path(), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(fs::read(store.document_path()).unwrap(), original);
    loaded.model.validate().unwrap();
    assert!(!loaded.warnings.is_empty());
    let mut names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["qlaunch.json"], "nothing else is created or moved");
}

#[test]
fn a_corrupt_but_readable_document_is_still_quarantined() {
    let dir = tempdir().unwrap();
    let store = Store::new(dir.path().to_path_buf());
    fs::write(store.document_path(), b"{ not json").unwrap();

    let loaded = store.load().unwrap();

    assert_eq!(
        fs::read(dir.path().join("qlaunch.json.corrupt")).unwrap(),
        b"{ not json"
    );
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| w.contains("qlaunch.json.corrupt"))
    );
    assert_eq!(store.load().unwrap().warnings, Vec::<String>::new());
}

fn unreadable_document(dir: &std::path::Path) -> (Store, &'static [u8]) {
    use std::os::unix::fs::PermissionsExt;

    let store = Store::new(dir.to_path_buf());
    let original: &[u8] = br#"{"schema": 1, "this": "is somebody's real document"}"#;
    fs::write(store.document_path(), original).unwrap();
    fs::set_permissions(store.document_path(), fs::Permissions::from_mode(0o000)).unwrap();
    (store, original)
}

fn running_as_root(store: &Store) -> bool {
    // Mode 000 does not stop root from reading.
    fs::read(store.document_path()).is_ok()
}

fn make_readable(store: &Store) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(store.document_path(), fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn after_an_unreadable_load_save_is_refused_and_the_document_is_untouched() {
    let dir = tempdir().unwrap();
    let (store, original) = unreadable_document(dir.path());
    if running_as_root(&store) {
        return;
    }
    let loaded = store.load().unwrap();
    assert!(!loaded.warnings.is_empty());

    let saved = store.save(&loaded.model, &loaded.settings, &Usage::default());
    let quarantined = store.quarantine();

    make_readable(&store);
    assert!(matches!(saved, Err(StoreError::WritesBlocked)), "{saved:?}");
    assert!(matches!(quarantined, Err(StoreError::WritesBlocked)));
    assert_eq!(fs::read(store.document_path()).unwrap(), original);
    let mut names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["qlaunch.json"]);
}

#[test]
fn the_block_is_shared_by_clones() {
    let dir = tempdir().unwrap();
    let (store, original) = unreadable_document(dir.path());
    if running_as_root(&store) {
        return;
    }
    let clone = store.clone();
    let loaded = store.load().unwrap();

    let saved = clone.save(&loaded.model, &loaded.settings, &Usage::default());

    make_readable(&store);
    assert!(matches!(saved, Err(StoreError::WritesBlocked)));
    assert_eq!(fs::read(store.document_path()).unwrap(), original);
}

#[test]
fn a_later_successful_load_lifts_the_block() {
    let dir = tempdir().unwrap();
    let (store, _) = unreadable_document(dir.path());
    if running_as_root(&store) {
        return;
    }
    store.load().unwrap();
    make_readable(&store);
    fs::write(store.document_path(), b"{ not json").unwrap();

    let loaded = store.load().unwrap();

    store
        .save(&loaded.model, &loaded.settings, &Usage::default())
        .unwrap();
}

#[test]
fn save_works_after_loading_a_good_document() {
    let dir = tempdir().unwrap();
    let store = Store::new(dir.path().to_path_buf());
    store.load().unwrap();

    let mut loaded = store.load().unwrap();
    loaded.settings.icon_size = 80;

    store
        .save(&loaded.model, &loaded.settings, &Usage::default())
        .unwrap();
    assert_eq!(store.load().unwrap().settings.icon_size, 80);
}

#[test]
fn save_works_after_a_load_that_quarantined_a_corrupt_document() {
    let dir = tempdir().unwrap();
    let store = Store::new(dir.path().to_path_buf());
    fs::write(store.document_path(), b"{ not json").unwrap();
    let mut loaded = store.load().unwrap();
    loaded.settings.icon_size = 80;

    store
        .save(&loaded.model, &loaded.settings, &Usage::default())
        .unwrap();

    assert_eq!(store.load().unwrap().settings.icon_size, 80);
    assert_eq!(
        fs::read(dir.path().join("qlaunch.json.corrupt")).unwrap(),
        b"{ not json"
    );
}

#[test]
fn save_works_after_a_load_found_nothing_even_if_creating_it_failed() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("icons"), b"not a directory").unwrap();
    let store = Store::new(dir.path().to_path_buf());
    let loaded = store.load().unwrap();
    assert!(!loaded.warnings.is_empty());

    fs::remove_file(dir.path().join("icons")).unwrap();

    store
        .save(&loaded.model, &loaded.settings, &Usage::default())
        .unwrap();
}

#[test]
fn a_new_store_can_save_without_loading_first() {
    let dir = tempdir().unwrap();
    let store = Store::new(dir.path().to_path_buf());
    let model = qlaunch_core::model::Model::new_default();

    store
        .save(&model, &Settings::default(), &Usage::default())
        .unwrap();

    assert!(store.document_path().exists());
}

#[test]
fn store_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Store>();
}
