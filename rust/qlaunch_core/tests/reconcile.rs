use qlaunch_core::model::{AppCategory, AppEntry, AppRef, GroupRole, Id, Model, ShortcutPayload};
use qlaunch_core::reconcile::{ReconcileOutcome, reconcile};

fn entry(package: &str, label: &str, category: AppCategory) -> AppEntry {
    AppEntry {
        package: package.into(),
        activity: "Main".into(),
        label: label.into(),
        category,
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

type Placement = (Id, Id, i32, String, bool);

fn placements(m: &Model) -> Vec<Placement> {
    m.items
        .iter()
        .map(|i| {
            (
                i.id.clone(),
                i.group_id.clone(),
                i.order,
                i.label.clone(),
                i.label_overridden,
            )
        })
        .collect()
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
    assert!(out.marked_uninstalled.is_empty());
    assert!(out.restored.is_empty());
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
fn a_reinstalled_app_returns_to_its_previous_group() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    let it = m.add_item_to(&g, "Signal");
    m.set_item_app(&it, appref("org.signal", "Main")).unwrap();
    let out = reconcile(&mut m, &[]);
    assert_eq!(out.marked_uninstalled, vec![it.clone()]);
    assert!(m.item(&it).unwrap().uninstalled);
    assert!(m.validate().is_ok());

    let out = reconcile(&mut m, &[entry("org.signal", "Main", AppCategory::App)]);
    assert_eq!(out.restored, vec![it.clone()]);
    assert!(out.added.is_empty());
    assert_eq!(m.item(&it).unwrap().group_id, g);
    assert!(!m.item(&it).unwrap().uninstalled);
    assert!(m.validate().is_ok());
}

#[test]
fn a_reinstall_does_not_duplicate_and_does_not_override_a_user_label() {
    let mut m = Model::new_default();
    let it = m.add_app(appref("org.signal", "Main"), "Signal");
    m.rename_item(&it, "Messages").unwrap();
    assert!(m.item(&it).unwrap().label_overridden);

    reconcile(&mut m, &[]);
    assert!(m.item(&it).unwrap().uninstalled);

    let out = reconcile(&mut m, &[entry("org.signal", "Signal", AppCategory::App)]);
    assert_eq!(out.restored, vec![it.clone()]);
    assert!(out.added.is_empty());
    assert_eq!(m.items.len(), 1);
    let item = m.item(&it).unwrap();
    assert_eq!(item.label, "Messages");
    assert!(item.label_overridden);
    assert!(!item.uninstalled);
    assert!(m.validate().is_ok());
}

#[test]
fn a_category_change_never_moves_an_existing_item() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let it = m.add_app(appref("com.x", "Main"), "X");

    let out = reconcile(&mut m, &[entry("com.x", "X", AppCategory::Game)]);
    assert_eq!(out, ReconcileOutcome::default());
    assert_eq!(m.item(&it).unwrap().group_id, apps);

    let out = reconcile(&mut m, &[]);
    assert_eq!(out.marked_uninstalled, vec![it.clone()]);
    let out = reconcile(&mut m, &[entry("com.x", "X", AppCategory::Game)]);
    assert_eq!(out.restored, vec![it.clone()]);
    assert_eq!(m.item(&it).unwrap().group_id, apps);
    assert!(m.validate().is_ok());
}

#[test]
fn shortcut_items_are_never_marked_uninstalled_or_reported() {
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
    assert!(!m.item(&sc).unwrap().uninstalled);
    assert!(!m.item(&plain).unwrap().uninstalled);

    let out = reconcile(&mut m, &[entry("com.t", "T", AppCategory::App)]);
    assert_eq!(out.added.len(), 1);
    assert!(out.restored.is_empty() && out.marked_uninstalled.is_empty());
    assert!(!m.item(&sc).unwrap().uninstalled);
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
    assert_eq!(first.marked_uninstalled, vec![gone]);
    let after_first = m.clone();

    let second = reconcile(&mut m, &entries);
    assert_eq!(second, ReconcileOutcome::default());
    assert_eq!(m, after_first);
    assert!(m.validate().is_ok());
}

#[test]
fn an_already_uninstalled_item_is_not_reported_again() {
    let mut m = Model::new_default();
    let it = m.add_app(appref("com.gone", "Main"), "Gone");
    let first = reconcile(&mut m, &[]);
    assert_eq!(first.marked_uninstalled, vec![it]);
    let second = reconcile(&mut m, &[]);
    assert!(second.marked_uninstalled.is_empty());
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
fn reconciliation_only_adds_marks_and_restores_never_moves_or_deletes() {
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
    let before = placements(&m);
    let count_before = m.items.len();

    // signal and doom vanish, secret and kept stay, a brand new game appears
    // and `doom` is reported as a game (it lives in `work`).
    let out = reconcile(
        &mut m,
        &[
            entry("com.slack", "Slack", AppCategory::Game),
            entry("com.secret", "Secret", AppCategory::Game),
            entry("com.kept", "Kept", AppCategory::App),
            entry("com.new", "New", AppCategory::Game),
        ],
    );
    assert_eq!(out.marked_uninstalled, vec![signal.clone(), doom.clone()]);
    assert_eq!(out.added.len(), 1);
    assert!(out.restored.is_empty());

    assert_eq!(m.groups, groups_before);
    assert_eq!(m.items.len(), count_before + 1);
    assert_eq!(placements(&m)[..count_before], before[..]);
    assert_eq!(m.item(&out.added[0]).unwrap().group_id, games);
    assert_eq!(m.item(&slack).unwrap().label, "Team chat");
    assert!(m.item(&signal).unwrap().uninstalled);
    assert!(m.item(&doom).unwrap().uninstalled);
    assert!(!m.item(&slack).unwrap().uninstalled);
    assert!(!m.item(&secret).unwrap().uninstalled);
    assert!(m.validate().is_ok());

    // everything comes back: all are restored in place, nothing is added.
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
    assert!(out.added.is_empty());
    assert_eq!(out.restored, vec![signal.clone(), doom.clone()]);
    assert_eq!(m.groups, groups_before);
    assert_eq!(placements(&m)[..count_before], before[..]);
    assert!(m.items.iter().all(|i| !i.uninstalled));
    assert!(m.validate().is_ok());
}
