use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use flutter_rust_bridge::for_generated::futures::executor::block_on;
use qlaunch_core::api::api::{self, ModelSnapshot};
use qlaunch_core::api::events::{CoreEvent, EventBus};
use qlaunch_core::model::{AppCategory, AppEntry, AppRef, GroupRole, Item, Model};
use qlaunch_core::platform::{AppSource, FakeSource, LaunchError, PlatformError, WatchHandle};
use qlaunch_core::settings::SettingsPatch;
use tempfile::tempdir;

// The API keeps its state in a process-global, so tests that use it would race
// each other. Every test holds this lock for its whole run. A failing test
// poisons it, which says nothing about the next test, so poisoning is ignored.
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    api::reset_for_test();
    guard
}

fn entry(package: &str, label: &str, category: AppCategory) -> AppEntry {
    AppEntry {
        package: package.into(),
        activity: format!("{package}.desktop"),
        label: label.into(),
        category,
    }
}

fn fixtures() -> Vec<AppEntry> {
    vec![
        entry("org.alpha", "Alpha", AppCategory::App),
        entry("org.beta", "Beta", AppCategory::App),
        entry("org.racer", "Racer", AppCategory::Game),
    ]
}

fn app_ref(package: &str) -> AppRef {
    AppRef {
        package: package.into(),
        activity: format!("{package}.desktop"),
    }
}

fn init_fake(root: &Path) -> ModelSnapshot {
    api::init_with_source(
        Some(root.to_path_buf()),
        Box::new(FakeSource::new(fixtures())),
        Vec::new(),
    )
    .unwrap()
}

fn record_events() -> Arc<Mutex<Vec<CoreEvent>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    api::subscribe_for_test(Box::new(move |event| {
        sink.lock().unwrap().push(event);
        true
    }))
    .unwrap();
    seen
}

fn group_id(snap: &ModelSnapshot, role: GroupRole) -> String {
    snap.groups
        .iter()
        .find(|g| g.role == Some(role))
        .unwrap()
        .id
        .clone()
}

fn has_warning(snap: &ModelSnapshot, needle: &str) -> bool {
    snap.warnings.iter().any(|w| w.contains(needle))
}

#[test]
fn init_creates_the_config_root_and_the_first_run_model() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let snap = block_on(api::init(Some(dir.path().to_string_lossy().into()))).unwrap();
    assert!(dir.path().join("qlaunch.json").exists());
    assert_eq!(snap.page_ids.len(), 3); // Apps, Home, Games
    assert_eq!(snap.settings.grid_columns_portrait, 4);
}

#[test]
fn mutations_return_a_fresh_snapshot_and_persist() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    block_on(api::init(Some(dir.path().to_string_lossy().into()))).unwrap();
    let snap = block_on(api::add_group("Chat".into())).unwrap();
    assert_eq!(snap.page_ids.len(), 4);
    assert!(
        fs::read_to_string(dir.path().join("qlaunch.json"))
            .unwrap()
            .contains("Chat")
    );
    assert_eq!(block_on(api::snapshot()).unwrap(), snap);
}

#[test]
fn a_mutation_emits_model_changed() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    let seen = record_events();

    block_on(api::add_group("Chat".into())).unwrap();

    assert_eq!(*seen.lock().unwrap(), vec![CoreEvent::ModelChanged]);
}

#[test]
fn a_subscriber_that_is_gone_is_dropped_and_the_others_keep_receiving() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    let calls = Arc::new(Mutex::new(0));
    let counter = calls.clone();
    api::subscribe_for_test(Box::new(move |_| {
        *counter.lock().unwrap() += 1;
        false
    }))
    .unwrap();
    let seen = record_events();

    block_on(api::add_group("One".into())).unwrap();
    block_on(api::add_group("Two".into())).unwrap();

    assert_eq!(
        *calls.lock().unwrap(),
        1,
        "called again after it said it was gone"
    );
    assert_eq!(seen.lock().unwrap().len(), 2);
}

#[test]
fn the_event_bus_delivers_in_subscription_order_and_unsubscribes() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let mut bus = EventBus::new();
    let mut tokens = Vec::new();
    for name in ["first", "second", "third"] {
        let log = order.clone();
        tokens.push(bus.subscribe(Box::new(move |_| {
            log.lock().unwrap().push(name);
            true
        })));
    }

    bus.emit(&CoreEvent::AppsChanged);
    assert_eq!(*order.lock().unwrap(), ["first", "second", "third"]);

    assert!(bus.unsubscribe(tokens[1]));
    assert!(!bus.unsubscribe(tokens[1]));
    order.lock().unwrap().clear();
    bus.emit(&CoreEvent::AppsChanged);
    assert_eq!(*order.lock().unwrap(), ["first", "third"]);
    assert_eq!(bus.subscriber_count(), 2);

    bus.subscribe(Box::new(|_| false));
    bus.emit(&CoreEvent::HomePressed);
    assert_eq!(bus.subscriber_count(), 2);
}

#[test]
fn the_home_button_reaches_subscribers_as_home_pressed() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    let seen = record_events();

    api::emit_for_test(CoreEvent::HomePressed);

    assert_eq!(*seen.lock().unwrap(), vec![CoreEvent::HomePressed]);
}

#[test]
fn the_back_button_reaches_subscribers_as_back_pressed() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    let seen = record_events();

    api::emit_for_test(CoreEvent::BackPressed);

    assert_eq!(*seen.lock().unwrap(), vec![CoreEvent::BackPressed]);
}

#[test]
fn deleting_a_role_holding_group_returns_an_error_string_and_changes_nothing() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let before = init_fake(dir.path());
    let seen = record_events();
    let on_disk = fs::read(dir.path().join("qlaunch.json")).unwrap();

    let apps = group_id(&before, GroupRole::Apps);
    let err = block_on(api::delete_group(apps)).unwrap_err();

    assert!(err.starts_with("GroupHoldsRole: "), "{err}");
    assert_eq!(block_on(api::snapshot()).unwrap(), before);
    assert!(seen.lock().unwrap().is_empty());
    assert_eq!(fs::read(dir.path().join("qlaunch.json")).unwrap(), on_disk);
}

#[test]
fn an_unknown_id_is_an_error_naming_the_variant() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    let err = block_on(api::rename_group("nope".into(), "x".into())).unwrap_err();
    assert!(err.starts_with("UnknownGroup: "), "{err}");
}

#[test]
fn discovered_apps_become_items_in_the_right_groups() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let snap = init_fake(dir.path());

    assert_eq!(snap.items.len(), 3);
    let apps = group_id(&snap, GroupRole::Apps);
    let games = group_id(&snap, GroupRole::Games);
    let group_of = |package: &str| {
        snap.items
            .iter()
            .find(|i| i.app.as_ref() == Some(&app_ref(package)))
            .unwrap()
            .group_id
            .clone()
    };
    assert_eq!(group_of("org.alpha"), apps);
    assert_eq!(group_of("org.beta"), apps);
    assert_eq!(group_of("org.racer"), games);
    assert!(snap.items.iter().all(|i| !i.uninstalled));
    assert!(snap.warnings.is_empty(), "{:?}", snap.warnings);
}

/// A source whose listing can be changed, or made to fail, between calls.
struct Switchable(Arc<Mutex<Result<Vec<AppEntry>, PlatformError>>>);

impl AppSource for Switchable {
    fn list_apps(&self) -> Result<Vec<AppEntry>, PlatformError> {
        self.0.lock().unwrap().clone()
    }
    fn icon_png(&self, _: &AppRef, _: u32) -> Result<Option<Vec<u8>>, PlatformError> {
        Ok(None)
    }
    fn launch(&self, _: &AppRef) -> Result<(), LaunchError> {
        Ok(())
    }
}

#[test]
fn a_failed_listing_does_not_mark_anything_uninstalled() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let listing = Arc::new(Mutex::new(Ok(fixtures())));
    let first = api::init_with_source(
        Some(dir.path().to_path_buf()),
        Box::new(Switchable(listing.clone())),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(first.items.len(), 3);
    let seen = record_events();

    *listing.lock().unwrap() = Err(PlatformError::Io("the directory vanished".into()));
    let after = block_on(api::refresh_apps()).unwrap();

    assert_eq!(after.items, first.items);
    assert!(after.items.iter().all(|i| !i.uninstalled));
    assert!(
        has_warning(&after, "could not be listed"),
        "{:?}",
        after.warnings
    );
    assert!(
        !seen.lock().unwrap().contains(&CoreEvent::AppsChanged),
        "a failed refresh claimed the apps changed"
    );

    // The warning describes the platform now, so it goes when the platform
    // recovers, and recovery with nothing to change is silent.
    *listing.lock().unwrap() = Ok(fixtures());
    let recovered = block_on(api::refresh_apps()).unwrap();
    assert!(recovered.warnings.is_empty(), "{:?}", recovered.warnings);
    assert!(!seen.lock().unwrap().contains(&CoreEvent::AppsChanged));
}

#[test]
fn a_successful_refresh_reconciles_and_announces_it() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let listing = Arc::new(Mutex::new(Ok(fixtures())));
    api::init_with_source(
        Some(dir.path().to_path_buf()),
        Box::new(Switchable(listing.clone())),
        Vec::new(),
    )
    .unwrap();
    let seen = record_events();

    *listing.lock().unwrap() = Ok(vec![
        entry("org.alpha", "Alpha", AppCategory::App),
        entry("org.new", "New", AppCategory::App),
    ]);
    let after = block_on(api::refresh_apps()).unwrap();

    let uninstalled = |package: &str| {
        after
            .items
            .iter()
            .find(|i| i.app.as_ref() == Some(&app_ref(package)))
            .map(|i| i.uninstalled)
    };
    assert_eq!(uninstalled("org.alpha"), Some(false));
    assert_eq!(uninstalled("org.beta"), Some(true));
    assert_eq!(uninstalled("org.new"), Some(false));
    assert_eq!(*seen.lock().unwrap(), vec![CoreEvent::AppsChanged]);
    assert!(
        fs::read_to_string(dir.path().join("qlaunch.json"))
            .unwrap()
            .contains("org.new")
    );
}

#[test]
fn the_failing_listing_at_init_does_not_mark_anything_either() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let listing = Arc::new(Mutex::new(Ok(fixtures())));
    api::init_with_source(
        Some(dir.path().to_path_buf()),
        Box::new(Switchable(listing.clone())),
        Vec::new(),
    )
    .unwrap();

    *listing.lock().unwrap() = Err(PlatformError::NotFound);
    let again = api::init_with_source(
        Some(dir.path().to_path_buf()),
        Box::new(Switchable(listing)),
        Vec::new(),
    )
    .unwrap();

    assert_eq!(again.items.len(), 3);
    assert!(again.items.iter().all(|i| !i.uninstalled));
    assert!(has_warning(&again, "could not be listed"));
}

#[test]
fn an_unreadable_document_is_left_alone_and_the_change_still_counts() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    // Reading a directory fails with EISDIR, which the store does not treat as
    // an absent document, so the document counts as unreadable.
    let document = dir.path().join("qlaunch.json");
    fs::create_dir(&document).unwrap();

    let first = init_fake(dir.path());
    assert!(
        has_warning(&first, "not being saved"),
        "{:?}",
        first.warnings
    );

    let snap = block_on(api::add_group("Chat".into())).unwrap();

    assert!(snap.groups.iter().any(|g| g.label == "Chat"));
    assert_eq!(snap.page_ids.len(), 4);
    assert!(has_warning(&snap, "not being saved"), "{:?}", snap.warnings);
    assert!(document.is_dir(), "the unreadable document was replaced");
    assert_eq!(block_on(api::snapshot()).unwrap(), snap);
}

#[test]
fn a_failed_write_is_a_warning_and_the_change_still_counts() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    // `save` creates `icons/`, which cannot be done over a file.
    fs::write(dir.path().join("icons"), b"in the way").unwrap();

    let first = init_fake(dir.path());
    assert!(
        has_warning(&first, "not being saved"),
        "{:?}",
        first.warnings
    );

    let snap = block_on(api::add_group("Chat".into())).unwrap();
    assert!(snap.groups.iter().any(|g| g.label == "Chat"));
    assert!(has_warning(&snap, "not being saved"));

    // Once the obstacle is gone the next save works and the warning with it.
    fs::remove_file(dir.path().join("icons")).unwrap();
    let snap = block_on(api::add_group("More".into())).unwrap();
    assert!(
        !has_warning(&snap, "not being saved"),
        "{:?}",
        snap.warnings
    );
    assert!(
        fs::read_to_string(dir.path().join("qlaunch.json"))
            .unwrap()
            .contains("Chat")
    );
}

#[test]
fn init_loads_before_it_saves_so_a_document_that_must_be_kept_is_kept() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let document = dir.path().join("qlaunch.json");
    let kept = dir.path().join("qlaunch.json.corrupt");
    let kept_again = dir.path().join("qlaunch.json.corrupt.1");

    // If anything saved before `load` ran, it would overwrite this document
    // and `load` would then find a valid one and have nothing to set aside.
    let seed = b"{ this is somebody's layout, and it is not JSON";
    fs::write(&document, seed).unwrap();

    let snap = init_fake(dir.path());
    assert!(has_warning(&snap, "kept as"), "{:?}", snap.warnings);
    assert_eq!(fs::read(&kept).unwrap(), seed, "the original was not kept");
    let first_document = fs::read_to_string(&document).unwrap();
    assert!(first_document.contains("\"schema\""), "no fresh document");
    assert_eq!(snap.page_ids.len(), 3);

    // The same on a re-init, which starts from a running state.
    let second_seed = b"[ a second document that cannot be read";
    fs::write(&document, second_seed).unwrap();
    let again = init_fake(dir.path());
    assert!(has_warning(&again, "kept as"), "{:?}", again.warnings);
    assert_eq!(fs::read(&kept_again).unwrap(), second_seed);
    assert_eq!(
        fs::read(&kept).unwrap(),
        seed,
        "the first original was overwritten"
    );
    assert!(
        fs::read_to_string(&document)
            .unwrap()
            .contains("\"schema\"")
    );

    let snap = block_on(api::add_group("Chat".into())).unwrap();
    assert_eq!(snap.page_ids.len(), 4);
    assert!(fs::read_to_string(&document).unwrap().contains("Chat"));
    assert_eq!(fs::read(&kept).unwrap(), seed);
}

/// What a consumer does with a snapshot: keep one group's items, as they come.
fn items_of(snap: &ModelSnapshot, group: &str) -> Vec<Item> {
    snap.items
        .iter()
        .filter(|i| i.group_id == group)
        .cloned()
        .collect()
}

#[test]
fn a_snapshot_lists_each_groups_items_in_display_order() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    // Inserted as Zed, Mid, Alpha (apps) and Racer, Kart (games), which is
    // neither alphabetical nor the order the moves below will produce.
    let source = FakeSource::new(vec![
        entry("org.zed", "Zed", AppCategory::App),
        entry("org.mid", "Mid", AppCategory::App),
        entry("org.alpha", "Alpha", AppCategory::App),
        entry("org.racer", "Racer", AppCategory::Game),
        entry("org.kart", "Kart", AppCategory::Game),
    ]);
    let first = api::init_with_source(Some(dir.path().to_path_buf()), Box::new(source), Vec::new())
        .unwrap();
    let apps = group_id(&first, GroupRole::Apps);
    let games = group_id(&first, GroupRole::Games);
    let id_of = |package: &str| {
        first
            .items
            .iter()
            .find(|i| i.app.as_ref() == Some(&app_ref(package)))
            .unwrap()
            .id
            .clone()
    };

    // A model that gets the same operations, as the reference for the order.
    let mut reference = Model {
        root_group_id: first.root_group_id.clone(),
        groups: first.groups.clone(),
        items: first.items.clone(),
    };
    let check = |snap: &ModelSnapshot, reference: &Model| {
        for group in [&apps, &games] {
            let expected: Vec<Item> = reference.items_in(group).into_iter().cloned().collect();
            assert_eq!(items_of(snap, group), expected);
        }
        assert_eq!(snap.items.len(), reference.items.len());
    };
    check(&first, &reference);

    // Move a game to the front of the apps, and an app to the middle of games.
    let snap = block_on(api::move_item(id_of("org.racer"), apps.clone(), 0)).unwrap();
    reference.move_item(&id_of("org.racer"), &apps, 0).unwrap();
    check(&snap, &reference);
    let snap = block_on(api::move_item(id_of("org.mid"), games.clone(), 1)).unwrap();
    reference.move_item(&id_of("org.mid"), &games, 1).unwrap();
    check(&snap, &reference);

    // Sorting rewrites the order of the group and moves nothing in the model.
    let snap = block_on(api::sort_group(apps.clone())).unwrap();
    reference.sort_group(&apps).unwrap();
    check(&snap, &reference);
    let labels: Vec<String> = items_of(&snap, &apps)
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert_eq!(labels, ["Alpha", "Racer", "Zed"]);

    assert_eq!(block_on(api::snapshot()).unwrap(), snap);
}

#[test]
fn the_snapshot_names_the_home_group_the_model_names() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let first = init_fake(dir.path());
    let model_says = |snap: &ModelSnapshot| {
        Model {
            root_group_id: snap.root_group_id.clone(),
            groups: snap.groups.clone(),
            items: snap.items.clone(),
        }
        .home_group_id()
    };

    assert!(first.home_group_id.is_some());
    assert_eq!(first.home_group_id, model_says(&first));
    assert_eq!(first.home_group_id, Some(group_id(&first, GroupRole::Home)));
    assert_ne!(first.home_group_id.as_ref(), Some(&first.root_group_id));

    // Another group takes the role over: the snapshot follows, and the old
    // holder is no longer named.
    let snap = block_on(api::add_group("Chat".into())).unwrap();
    let chat = snap
        .groups
        .iter()
        .find(|g| g.label == "Chat")
        .unwrap()
        .id
        .clone();
    let snap = block_on(api::set_group_role(chat.clone(), Some(GroupRole::Home))).unwrap();
    assert_eq!(snap.home_group_id, Some(chat.clone()));
    assert_eq!(snap.home_group_id, model_says(&snap));
    assert_ne!(snap.home_group_id, first.home_group_id);
    assert_eq!(
        block_on(api::snapshot()).unwrap().home_group_id,
        Some(chat.clone())
    );

    // With the role taken away nobody is named.
    let snap = block_on(api::set_group_role(chat, None)).unwrap();
    assert_eq!(snap.home_group_id, None);
    assert_eq!(snap.home_group_id, model_says(&snap));
    assert_eq!(block_on(api::home_group_id()), None);
}

#[test]
fn warnings_do_not_accumulate_across_mutations() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("qlaunch.json")).unwrap();
    let first = init_fake(dir.path());

    let mut last = first.warnings.len();
    for i in 0..5 {
        let snap = block_on(api::add_group(format!("Group {i}"))).unwrap();
        assert_eq!(snap.warnings.len(), last, "{:?}", snap.warnings);
        last = snap.warnings.len();
    }
    assert_eq!(block_on(api::refresh_apps()).unwrap().warnings.len(), last);
    assert_eq!(block_on(api::snapshot()).unwrap().warnings.len(), last);
}

#[test]
fn starting_without_a_config_directory_works_and_says_so() {
    let _guard = serial();
    let snap = api::init_with_source(
        None,
        Box::new(FakeSource::new(fixtures())),
        vec!["The configuration directory could not be found.".into()],
    )
    .unwrap();

    assert!(!snap.warnings.is_empty());
    assert!(has_warning(&snap, "could not be found"));
    assert!(has_warning(&snap, "nothing will be saved"));
    assert_eq!(snap.page_ids.len(), 3);
    assert_eq!(snap.items.len(), 3);

    let snap = block_on(api::add_group("Chat".into())).unwrap();
    assert_eq!(snap.page_ids.len(), 4);
    assert!(snap.groups.iter().any(|g| g.label == "Chat"));
    let snap = block_on(api::update_settings(SettingsPatch {
        icon_size: Some(60),
        ..Default::default()
    }))
    .unwrap();
    assert_eq!(snap.settings.icon_size, 60);
}

#[test]
fn a_custom_icon_needs_somewhere_to_live() {
    let _guard = serial();
    let snap =
        api::init_with_source(None, Box::new(FakeSource::new(fixtures())), Vec::new()).unwrap();
    let id = snap.items[0].id.clone();

    let err = block_on(api::set_item_icon(id, vec![1, 2, 3])).unwrap_err();

    assert!(err.starts_with("NoStore: "), "{err}");
}

#[test]
fn a_custom_icon_is_written_and_recorded() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let snap = init_fake(dir.path());
    let id = snap.items[0].id.clone();

    let snap = block_on(api::set_item_icon(id.clone(), vec![9, 8, 7])).unwrap();

    let name = snap
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .icon_override
        .clone()
        .unwrap();
    assert_eq!(
        fs::read(dir.path().join("icons").join(&name)).unwrap(),
        [9, 8, 7]
    );
    assert!(
        fs::read_to_string(dir.path().join("qlaunch.json"))
            .unwrap()
            .contains(&name)
    );

    let err = block_on(api::set_item_icon("nope".into(), vec![1])).unwrap_err();
    assert!(err.starts_with("UnknownItem: "), "{err}");
}

#[test]
fn settings_are_patched_clamped_and_saved() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    let seen = record_events();

    let snap = block_on(api::update_settings(SettingsPatch {
        grid_columns_portrait: Some(99),
        icon_size: Some(55),
        ..Default::default()
    }))
    .unwrap();

    assert_eq!(snap.settings.grid_columns_portrait, 12);
    assert_eq!(snap.settings.icon_size, 55);
    assert_eq!(*seen.lock().unwrap(), vec![CoreEvent::ModelChanged]);
    let on_disk = fs::read_to_string(dir.path().join("qlaunch.json")).unwrap();
    assert!(on_disk.contains("\"icon_size\": 55"), "{on_disk}");
}

#[test]
fn groups_and_items_can_be_arranged() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let snap = init_fake(dir.path());
    let games = group_id(&snap, GroupRole::Games);
    let alpha = snap
        .items
        .iter()
        .find(|i| i.app.as_ref() == Some(&app_ref("org.alpha")))
        .unwrap()
        .id
        .clone();

    let snap = block_on(api::move_item(alpha.clone(), games.clone(), 0)).unwrap();
    assert_eq!(
        snap.items.iter().find(|i| i.id == alpha).unwrap().group_id,
        games
    );
    let snap = block_on(api::rename_item(alpha.clone(), "Renamed".into())).unwrap();
    assert!(snap.items.iter().any(|i| i.label == "Renamed"));
    let snap = block_on(api::sort_group(games.clone())).unwrap();
    assert_eq!(snap.items.iter().filter(|i| i.group_id == games).count(), 2);

    let snap = block_on(api::add_group("Chat".into())).unwrap();
    let chat = snap
        .groups
        .iter()
        .find(|g| g.label == "Chat")
        .unwrap()
        .id
        .clone();
    let snap = block_on(api::move_group(chat.clone(), 0)).unwrap();
    assert_eq!(snap.page_ids[0], chat);
    let snap = block_on(api::set_group_role(chat.clone(), None)).unwrap();
    assert_eq!(snap.page_ids.len(), 4);
    let snap = block_on(api::rename_group(chat.clone(), "Talk".into())).unwrap();
    assert!(snap.groups.iter().any(|g| g.label == "Talk"));
    let snap = block_on(api::delete_group(chat)).unwrap();
    assert_eq!(snap.page_ids.len(), 3);
    let snap = block_on(api::remove_item(alpha.clone())).unwrap();
    assert!(snap.items.iter().all(|i| i.id != alpha));
}

#[test]
fn home_group_id_search_and_launch() {
    let _guard = serial();
    assert_eq!(block_on(api::home_group_id()), None);
    assert!(block_on(api::search_all_apps("a".into())).is_empty());
    assert_eq!(block_on(api::icon_for("p".into(), "a".into(), 48)), None);

    let dir = tempdir().unwrap();
    let source = FakeSource::new(fixtures()).with_icon(app_ref("org.alpha"), vec![1, 2]);
    let snap = api::init_with_source(Some(dir.path().to_path_buf()), Box::new(source), Vec::new())
        .unwrap();

    assert_eq!(
        block_on(api::home_group_id()),
        Some(group_id(&snap, GroupRole::Home))
    );
    let hits = block_on(api::search_all_apps("al".into()));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].app, app_ref("org.alpha"));
    assert_eq!(
        block_on(api::icon_for(
            "org.alpha".into(),
            "org.alpha.desktop".into(),
            48
        )),
        Some(vec![1, 2])
    );
    assert_eq!(
        block_on(api::icon_for(
            "org.beta".into(),
            "org.beta.desktop".into(),
            48
        )),
        None
    );
    block_on(api::launch_app(
        "org.alpha".into(),
        "org.alpha.desktop".into(),
    ))
    .unwrap();
    let err = block_on(api::launch_app("org.none".into(), "x".into())).unwrap_err();
    assert!(err.starts_with("NotFound: "), "{err}");
}

#[test]
fn everything_before_init_is_an_error_that_says_so() {
    let _guard = serial();
    let errors = [
        block_on(api::snapshot()).unwrap_err(),
        block_on(api::refresh_apps()).unwrap_err(),
        block_on(api::add_group("x".into())).unwrap_err(),
        block_on(api::delete_group("x".into())).unwrap_err(),
        block_on(api::update_settings(SettingsPatch::default())).unwrap_err(),
        block_on(api::set_item_icon("x".into(), vec![])).unwrap_err(),
        block_on(api::launch_app("x".into(), "y".into())).unwrap_err(),
    ];
    for err in errors {
        assert!(err.starts_with("NotInitialised: "), "{err}");
    }
}

#[test]
fn a_reset_forgets_the_state_and_the_subscribers() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    let seen = record_events();
    api::reset_for_test();
    assert!(block_on(api::snapshot()).is_err());

    init_fake(dir.path());
    block_on(api::add_group("Chat".into())).unwrap();
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn a_second_init_loads_what_the_first_saved() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    init_fake(dir.path());
    block_on(api::add_group("Chat".into())).unwrap();
    let seen = record_events();

    let again = init_fake(dir.path());

    assert!(again.groups.iter().any(|g| g.label == "Chat"));
    assert_eq!(again.items.len(), 3);
    // Subscribers outlive a re-init.
    block_on(api::add_group("More".into())).unwrap();
    assert_eq!(*seen.lock().unwrap(), vec![CoreEvent::ModelChanged]);
}

/// A source whose watcher is a thread that calls the callback in a tight loop,
/// like a debounce thread with a steady stream of changes, and whose handle
/// joins that thread when dropped, as the real one does.
struct Hammering;

impl AppSource for Hammering {
    fn list_apps(&self) -> Result<Vec<AppEntry>, PlatformError> {
        Ok(fixtures())
    }
    fn icon_png(&self, _: &AppRef, _: u32) -> Result<Option<Vec<u8>>, PlatformError> {
        Ok(None)
    }
    fn launch(&self, _: &AppRef) -> Result<(), LaunchError> {
        Ok(())
    }
    fn watch(&self, on_change: Box<dyn Fn() + Send + Sync>) -> Result<WatchHandle, PlatformError> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = thread::spawn(move || {
            while !flag.load(Ordering::Acquire) {
                on_change();
            }
        });
        let thread = Mutex::new(Some(thread));
        Ok(WatchHandle::new(move || {
            stop.store(true, Ordering::Release);
            if let Some(thread) = thread.lock().unwrap().take() {
                let _ = thread.join();
            }
        }))
    }
}

/// Runs `work` on another thread and fails the test, rather than hanging it,
/// if that takes more than ten seconds.
fn finishes_promptly(what: &str, work: impl FnOnce() + Send + 'static) {
    let (done, finished) = mpsc::channel();
    thread::spawn(move || {
        work();
        let _ = done.send(());
    });
    finished
        .recv_timeout(Duration::from_secs(10))
        .unwrap_or_else(|_| {
            panic!("{what} did not return: the watcher was dropped under the lock")
        });
}

#[test]
fn replacing_a_state_that_owns_a_busy_watcher_does_not_deadlock() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    finishes_promptly("the second init", move || {
        for _ in 0..5 {
            api::init_with_source(Some(root.clone()), Box::new(Hammering), Vec::new()).unwrap();
        }
    });
    finishes_promptly("reset_for_test", api::reset_for_test);
}

#[cfg(not(target_os = "android"))]
#[test]
fn uninstall_app_is_unsupported_off_android() {
    let result = block_on(api::uninstall_app("com.example".into()));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Android"));
}

#[test]
fn a_mutation_works_while_a_watcher_keeps_refreshing() {
    let _guard = serial();
    let dir = tempdir().unwrap();
    api::init_with_source(
        Some(dir.path().to_path_buf()),
        Box::new(Hammering),
        Vec::new(),
    )
    .unwrap();

    finishes_promptly("the mutations", || {
        for i in 0..20 {
            block_on(api::add_group(format!("Group {i}"))).unwrap();
        }
    });
    assert_eq!(block_on(api::snapshot()).unwrap().page_ids.len(), 23);
}
