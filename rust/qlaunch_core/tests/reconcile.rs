use qlaunch_core::model::{AppCategory, AppEntry, AppRef, GroupRole, Id, Model, ShortcutPayload};
use qlaunch_core::reconcile::{ReconcileOutcome, reconcile};

fn entry(package: &str, label: &str, category: AppCategory) -> AppEntry {
    AppEntry {
        package: package.into(),
        activity: "Main".into(),
        label: label.into(),
        category,
        last_update_time: 0,
    }
}

fn appref(package: &str, activity: &str) -> AppRef {
    AppRef {
        package: package.into(),
        activity: activity.into(),
    }
}

fn role_id(m: &Model, role: GroupRole) -> Id {
    m.role_holder(role).unwrap().id.clone()
}

#[test]
fn routes_new_apps_by_category_to_the_role_holder() {
    let mut m = Model::new_default();
    let games = role_id(&m, GroupRole::Games);
    let apps = role_id(&m, GroupRole::Apps);
    let out = reconcile(
        &mut m,
        &[
            entry("com.g", "G", AppCategory::Game),
            entry("com.t", "T", AppCategory::App),
            entry("com.u", "U", AppCategory::Unknown),
        ],
    );
    assert_eq!(out.added.len(), 3);
    assert_eq!(m.item(&out.added[0]).unwrap().group_id, games);
    assert_eq!(m.item(&out.added[1]).unwrap().group_id, apps);
    assert_eq!(m.item(&out.added[2]).unwrap().group_id, apps);
    assert!(out.removed.is_empty());
    assert!(m.validate().is_ok());
}

#[test]
fn added_items_take_the_entry_label_and_app_and_follow_entry_order() {
    let mut m = Model::new_default();
    let out = reconcile(
        &mut m,
        &[
            entry("com.b", "Bravo", AppCategory::App),
            entry("com.a", "Alpha", AppCategory::App),
        ],
    );
    let first = m.item(&out.added[0]).unwrap();
    let second = m.item(&out.added[1]).unwrap();
    assert_eq!(first.label, "Bravo");
    assert_eq!(first.app, Some(appref("com.b", "Main")));
    assert!(!first.label_overridden && !first.uninstalled);
    assert_eq!(second.label, "Alpha");
    assert_eq!(second.app, Some(appref("com.a", "Main")));
    assert!(m.validate().is_ok());
}

#[test]
fn a_reinstalled_app_is_added_fresh_and_not_restored_in_place() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let chat = m.add_group("Chat");
    let it = m.add_app(appref("org.signal", "Main"), "Signal");
    m.move_item(&it, &chat, 0).unwrap();
    let out = reconcile(&mut m, &[]);
    assert_eq!(out.removed, vec![it.clone()]);
    assert!(m.item(&it).is_none());
    assert!(m.validate().is_ok());

    let out = reconcile(&mut m, &[entry("org.signal", "Main", AppCategory::App)]);
    assert_eq!(out.added.len(), 1);
    assert_ne!(out.added[0], it, "an uninstalled item's id is not reused");
    assert_eq!(m.item(&out.added[0]).unwrap().group_id, apps);
    assert!(m.validate().is_ok());
}

#[test]
fn a_removed_app_does_not_keep_its_user_label() {
    let mut m = Model::new_default();
    let it = m.add_app(appref("org.signal", "Main"), "Signal");
    m.rename_item(&it, "Messages").unwrap();
    assert!(m.item(&it).unwrap().label_overridden);

    let out = reconcile(&mut m, &[]);
    assert_eq!(out.removed, vec![it.clone()]);
    assert!(m.item(&it).is_none());

    let out = reconcile(&mut m, &[entry("org.signal", "Signal", AppCategory::App)]);
    assert_eq!(out.added.len(), 1);
    assert_eq!(m.items.len(), 1);
    let item = m.item(&out.added[0]).unwrap();
    assert_eq!(item.label, "Signal");
    assert!(!item.label_overridden);
    assert!(!item.uninstalled);
    assert!(m.validate().is_ok());
}

#[test]
fn a_category_change_never_moves_a_listed_item() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let it = m.add_app(appref("com.x", "Main"), "X");

    let out = reconcile(&mut m, &[entry("com.x", "X", AppCategory::Game)]);
    assert_eq!(out, ReconcileOutcome::default());
    assert_eq!(m.item(&it).unwrap().group_id, apps);

    let out = reconcile(&mut m, &[]);
    assert_eq!(out.removed, vec![it.clone()]);
    assert!(m.item(&it).is_none());
    assert!(m.validate().is_ok());
}

#[test]
fn shortcut_items_without_an_app_target_are_never_removed() {
    let mut m = Model::new_default();
    let home = role_id(&m, GroupRole::Home);
    let sc = m.add_shortcut(
        &home,
        "Run it",
        ShortcutPayload::Command {
            command: "echo hi".into(),
        },
    );
    let plain = m.add_item_to(&home, "Plain");

    let out = reconcile(&mut m, &[]);
    assert_eq!(out, ReconcileOutcome::default());
    assert!(m.item(&sc).is_some());
    assert!(m.item(&plain).is_some());

    let out = reconcile(&mut m, &[entry("com.t", "T", AppCategory::App)]);
    assert_eq!(out.added.len(), 1);
    assert!(out.removed.is_empty());
    assert!(m.item(&sc).is_some());
    assert!(m.item(&sc).unwrap().app.is_none());
    assert!(m.validate().is_ok());
}

#[test]
fn reconciling_twice_with_the_same_entries_changes_nothing_the_second_time() {
    let mut m = Model::new_default();
    let gone = m.add_app(appref("com.gone", "Main"), "Gone");
    let entries = [
        entry("com.g", "G", AppCategory::Game),
        entry("com.t", "T", AppCategory::App),
    ];

    let first = reconcile(&mut m, &entries);
    assert_eq!(first.added.len(), 2);
    assert_eq!(first.removed, vec![gone]);
    let after_first = m.clone();

    let second = reconcile(&mut m, &entries);
    assert_eq!(second, ReconcileOutcome::default());
    assert_eq!(m, after_first);
    assert!(m.validate().is_ok());
}

#[test]
fn an_uninstalled_item_is_reported_once() {
    let mut m = Model::new_default();
    let it = m.add_app(appref("com.gone", "Main"), "Gone");
    let first = reconcile(&mut m, &[]);
    assert_eq!(first.removed, vec![it]);
    let second = reconcile(&mut m, &[]);
    assert!(second.removed.is_empty());
    assert!(m.validate().is_ok());
}

#[test]
fn the_same_package_with_another_activity_is_a_different_app() {
    let mut m = Model::new_default();
    let it = m.add_app(appref("com.x", "Main"), "X");
    let out = reconcile(
        &mut m,
        &[
            entry("com.x", "X", AppCategory::App),
            AppEntry {
                package: "com.x".into(),
                activity: "Settings".into(),
                label: "X Settings".into(),
                category: AppCategory::App,
                last_update_time: 0,
            },
        ],
    );
    assert_eq!(out.added.len(), 1);
    assert_ne!(out.added[0], it);
    assert_eq!(m.items.len(), 2);
    assert!(m.validate().is_ok());
}

#[test]
fn a_duplicate_entry_is_added_once() {
    let mut m = Model::new_default();
    let out = reconcile(
        &mut m,
        &[
            entry("com.x", "X", AppCategory::App),
            entry("com.x", "X", AppCategory::App),
        ],
    );
    assert_eq!(out.added.len(), 1);
    assert_eq!(m.items.len(), 1);
    assert!(m.validate().is_ok());
}

#[test]
fn reconciliation_adds_and_removes_without_moving_kept_items() {
    let mut m = Model::new_default();
    let chat = m.add_group("Chat");
    let work = m.add_group("Work");
    let hidden = role_id(&m, GroupRole::Hidden);
    let games = role_id(&m, GroupRole::Games);

    let signal = m.add_app(appref("org.signal", "Main"), "Signal");
    let slack = m.add_app(appref("com.slack", "Main"), "Slack");
    let doom = m.add_app(appref("com.doom", "Main"), "Doom");
    let secret = m.add_app(appref("com.secret", "Main"), "Secret");
    let kept = m.add_app(appref("com.kept", "Main"), "Kept");
    m.move_item(&signal, &chat, 0).unwrap();
    m.move_item(&slack, &chat, 1).unwrap();
    m.move_item(&doom, &work, 0).unwrap();
    m.move_item(&secret, &hidden, 0).unwrap();
    m.rename_item(&slack, "Team chat").unwrap();
    m.move_item(&kept, &games, 0).unwrap();
    m.set_group_role(&work, Some(GroupRole::Home)).unwrap();
    m.rename_group(&chat, "Talk").unwrap();
    m.sort_group(&chat).unwrap();

    let groups_before = m.groups.clone();
    let slack_before = m.item(&slack).unwrap().clone();
    let secret_before = m.item(&secret).unwrap().clone();
    let kept_before = m.item(&kept).unwrap().clone();
    let count_before = m.items.len();

    // signal and doom vanish, secret and kept stay, a brand new game appears.
    let out = reconcile(
        &mut m,
        &[
            entry("com.slack", "Slack", AppCategory::Game),
            entry("com.secret", "Secret", AppCategory::Game),
            entry("com.kept", "Kept", AppCategory::App),
            entry("com.new", "New", AppCategory::Game),
        ],
    );
    assert_eq!(out.removed, vec![signal.clone(), doom.clone()]);
    assert_eq!(out.added.len(), 1);

    assert_eq!(m.groups, groups_before);
    assert_eq!(m.items.len(), count_before - 2 + 1);
    assert!(m.item(&signal).is_none());
    assert!(m.item(&doom).is_none());
    assert_eq!(m.item(&slack).unwrap().label, "Team chat");
    assert_eq!(m.item(&slack).unwrap().group_id, slack_before.group_id);
    assert_eq!(
        m.item(&slack).unwrap().label_overridden,
        slack_before.label_overridden
    );
    assert_eq!(m.item(&secret).unwrap().group_id, secret_before.group_id);
    assert_eq!(m.item(&kept).unwrap().group_id, kept_before.group_id);
    assert_eq!(m.item(&out.added[0]).unwrap().group_id, games);
    assert!(m.validate().is_ok());

    // Reinstalling them adds fresh items; the old ids and placements are gone.
    let out = reconcile(
        &mut m,
        &[
            entry("org.signal", "Signal", AppCategory::App),
            entry("com.slack", "Slack", AppCategory::App),
            entry("com.doom", "Doom", AppCategory::Game),
            entry("com.secret", "Secret", AppCategory::App),
            entry("com.kept", "Kept", AppCategory::App),
            entry("com.new", "New", AppCategory::Game),
        ],
    );
    assert_eq!(out.added.len(), 2);
    assert!(out.removed.is_empty());
    assert!(out.added.iter().all(|id| id != &signal && id != &doom));
    assert_eq!(m.item(&slack).unwrap().group_id, slack_before.group_id);
    assert_eq!(m.item(&secret).unwrap().group_id, secret_before.group_id);
    assert_eq!(m.item(&kept).unwrap().group_id, kept_before.group_id);
    assert!(m.items.iter().all(|i| !i.uninstalled));
    assert!(m.validate().is_ok());
}

#[test]
fn an_uninstalled_app_is_removed_from_its_group() {
    let mut m = Model::new_default();
    let chat = m.add_group("Chat");
    let kept = m.add_app(appref("com.kept", "Main"), "Kept");
    let gone = m.add_app(appref("org.gone", "Main"), "Gone");
    m.move_item(&kept, &chat, 0).unwrap();
    m.move_item(&gone, &chat, 1).unwrap();

    reconcile(&mut m, &[entry("com.kept", "Kept", AppCategory::App)]);

    assert!(m.item(&gone).is_none(), "an uninstalled app is gone");
    assert!(
        m.items
            .iter()
            .all(|i| i.app != Some(appref("org.gone", "Main"))),
        "no entry for the uninstalled app remains"
    );
    assert_eq!(m.item(&kept).unwrap().group_id, chat);
    assert_eq!(m.item(&kept).unwrap().order, 0);
    assert!(m.validate().is_ok());
}

#[test]
fn every_item_for_an_uninstalled_package_is_removed() {
    let mut m = Model::new_default();
    let first = m.add_app(appref("org.gone", "Main"), "Gone");
    let second = m.add_app(appref("org.gone", "Settings"), "Gone settings");
    let kept = m.add_app(appref("org.kept", "Main"), "Kept");

    reconcile(&mut m, &[entry("org.kept", "Kept", AppCategory::App)]);

    assert!(m.item(&first).is_none());
    assert!(m.item(&second).is_none());
    assert!(
        m.items
            .iter()
            .all(|i| { i.app.as_ref().is_none_or(|app| app.package != "org.gone") })
    );
    assert!(m.item(&kept).is_some());
    assert!(m.validate().is_ok());
}

#[test]
fn shortcuts_to_an_uninstalled_app_are_removed() {
    let mut m = Model::new_default();
    let home = role_id(&m, GroupRole::Home);
    let gone = m.add_app(appref("org.gone", "Main"), "Gone");
    let by_package = m.add_shortcut(
        &home,
        "Gone by package",
        ShortcutPayload::Android {
            action: Some("android.intent.action.VIEW".into()),
            data: None,
            mime_type: None,
            categories: Vec::new(),
            package: Some("org.gone".into()),
            component: None,
            flags: 0,
            extras: Vec::new(),
        },
    );
    let by_component = m.add_shortcut(
        &home,
        "Gone by component",
        ShortcutPayload::Android {
            action: Some("android.intent.action.VIEW".into()),
            data: None,
            mime_type: None,
            categories: Vec::new(),
            package: None,
            component: Some("org.gone/.ShortcutActivity".into()),
            flags: 0,
            extras: Vec::new(),
        },
    );
    let live = m.add_shortcut(
        &home,
        "Live",
        ShortcutPayload::Android {
            action: None,
            data: None,
            mime_type: None,
            categories: Vec::new(),
            package: Some("org.live".into()),
            component: None,
            flags: 0,
            extras: Vec::new(),
        },
    );
    let command = m.add_shortcut(
        &home,
        "Command",
        ShortcutPayload::Command {
            command: "echo hi".into(),
        },
    );
    let lock = m.add_shortcut(&home, "Lock", ShortcutPayload::LockScreen);

    reconcile(
        &mut m,
        &[
            entry("org.live", "Live", AppCategory::App),
            entry("org.kept", "Kept", AppCategory::App),
        ],
    );

    assert!(m.item(&gone).is_none());
    assert!(m.item(&by_package).is_none());
    assert!(
        m.item(&by_component).is_none(),
        "the package in the component counts too"
    );
    assert!(m.item(&live).is_some(), "a shortcut to a live app is kept");
    assert!(m.item(&command).is_some(), "command shortcuts are kept");
    assert!(m.item(&lock).is_some(), "the lock shortcut is kept");
    assert!(m.validate().is_ok());
}

#[test]
fn a_shortcut_without_a_target_is_kept() {
    let mut m = Model::new_default();
    let home = role_id(&m, GroupRole::Home);
    let implicit = m.add_shortcut(
        &home,
        "Implicit",
        ShortcutPayload::Android {
            action: Some("android.intent.action.VIEW".into()),
            data: Some("https://example.com".into()),
            mime_type: None,
            categories: Vec::new(),
            package: None,
            component: None,
            flags: 0,
            extras: Vec::new(),
        },
    );

    reconcile(&mut m, &[]);

    assert!(m.item(&implicit).is_some());
    assert!(m.validate().is_ok());
}

#[test]
fn a_legacy_uninstalled_flag_is_cleared_when_the_app_is_installed() {
    let mut m = Model::new_default();
    let it = m.add_app(appref("org.back", "Main"), "Back");
    m.items.iter_mut().find(|i| i.id == it).unwrap().uninstalled = true;

    reconcile(&mut m, &[entry("org.back", "Back", AppCategory::App)]);

    assert!(m.item(&it).is_some());
    assert!(!m.item(&it).unwrap().uninstalled);
    assert!(m.validate().is_ok());
}
