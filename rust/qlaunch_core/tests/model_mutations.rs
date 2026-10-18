use qlaunch_core::model::{AppRef, GroupRole, Id, Model, ModelError, ShortcutPayload};

fn page_labels(m: &Model) -> Vec<String> {
    m.pages().iter().map(|g| g.label.clone()).collect()
}

fn page_orders(m: &Model) -> Vec<i32> {
    m.pages().iter().map(|g| g.order).collect()
}

fn item_labels(m: &Model, group: &Id) -> Vec<String> {
    m.items_in(group).iter().map(|i| i.label.clone()).collect()
}

fn item_orders(m: &Model, group: &Id) -> Vec<i32> {
    m.items_in(group).iter().map(|i| i.order).collect()
}

fn role_id(m: &Model, role: GroupRole) -> Id {
    m.role_holder(role).unwrap().id.clone()
}

#[test]
fn set_group_role_moves_the_role_and_never_duplicates() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    m.set_group_role(&g, Some(GroupRole::Apps)).unwrap();
    assert_eq!(m.role_holder(GroupRole::Apps).unwrap().id, g);
    assert!(m.validate().is_ok());
}

#[test]
fn root_hidden_apps_games_cannot_be_unset() {
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    assert!(matches!(
        m.set_group_role(&g, None),
        Err(ModelError::RoleNotAssignable)
    ));
}

#[test]
fn deleting_an_ordinary_group_moves_its_items_to_apps() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    let a = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    let it = m.add_item_to(&g, "Signal");
    m.delete_group(&g).unwrap();
    assert_eq!(m.item(&it).unwrap().group_id, a);
}

#[test]
fn deleting_a_role_holding_group_is_refused() {
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Games).unwrap().id.clone();
    assert!(matches!(
        m.delete_group(&g),
        Err(ModelError::GroupHoldsRole(_))
    ));
}

#[test]
fn sort_group_is_case_insensitive_and_unicode_safe() {
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    for l in ["zebra", "Ápple", "apple", "日本", "🙂 emoji"] {
        m.add_item_to(&g, l);
    }
    m.sort_group(&g).unwrap();
    let labels: Vec<_> = m.items_in(&g).iter().map(|i| i.label.clone()).collect();
    assert_eq!(labels, ["apple", "Ápple", "zebra", "日本", "🙂 emoji"]);
}

#[test]
fn pages_are_root_children_except_hidden_in_order() {
    let m = Model::new_default();
    assert_eq!(page_labels(&m), ["Apps", "Home", "Games"]);
}

#[test]
fn add_group_goes_after_the_last_page_and_orders_stay_dense() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    assert_eq!(page_labels(&m), ["Apps", "Home", "Games", "Chat"]);
    assert_eq!(page_orders(&m), [0, 1, 2, 3]);
    let added = m.groups.iter().find(|x| x.id == g).unwrap();
    assert_eq!(added.parent_id.as_ref(), Some(&m.root_group_id));
    assert_eq!(added.role, None);
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn add_group_after_a_hidden_group_does_not_use_the_hidden_order() {
    let mut m = Model::new_default();
    m.add_group("Chat");
    m.add_group("Music");
    assert_eq!(page_orders(&m), [0, 1, 2, 3, 4]);
    assert_eq!(m.role_holder(GroupRole::Hidden).unwrap().order, 999);
}

#[test]
fn rename_group_changes_the_label_and_rejects_unknown_ids() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    m.rename_group(&g, "Talk").unwrap();
    assert_eq!(page_labels(&m).last().unwrap(), "Talk");
    assert_eq!(
        m.rename_group(&"nope".to_string(), "x"),
        Err(ModelError::UnknownGroup("nope".into()))
    );
}

#[test]
fn move_group_splices_to_the_target_index() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    m.move_group(&g, 1).unwrap();
    assert_eq!(page_labels(&m), ["Apps", "Chat", "Home", "Games"]);
    assert_eq!(page_orders(&m), [0, 1, 2, 3]);

    m.move_group(&g, 0).unwrap();
    assert_eq!(page_labels(&m), ["Chat", "Apps", "Home", "Games"]);
    assert_eq!(page_orders(&m), [0, 1, 2, 3]);
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn move_group_clamps_out_of_range_indices() {
    let mut m = Model::new_default();
    let g = m.add_group("Chat");
    m.move_group(&g, -5).unwrap();
    assert_eq!(page_labels(&m), ["Chat", "Apps", "Home", "Games"]);
    m.move_group(&g, 1000).unwrap();
    assert_eq!(page_labels(&m), ["Apps", "Home", "Games", "Chat"]);
    assert_eq!(page_orders(&m), [0, 1, 2, 3]);
}

#[test]
fn move_group_refuses_root_hidden_and_unknown() {
    let mut m = Model::new_default();
    let root = m.root_group_id.clone();
    let hidden = role_id(&m, GroupRole::Hidden);
    assert_eq!(m.move_group(&root, 0), Err(ModelError::NotAPage(root)));
    assert_eq!(m.move_group(&hidden, 0), Err(ModelError::NotAPage(hidden)));
    assert_eq!(
        m.move_group(&"nope".to_string(), 0),
        Err(ModelError::UnknownGroup("nope".into()))
    );
}

#[test]
fn assigning_home_to_another_group_moves_it_and_home_may_be_unset() {
    let mut m = Model::new_default();
    let old_home = role_id(&m, GroupRole::Home);
    let g = m.add_group("Chat");
    m.set_group_role(&g, Some(GroupRole::Home)).unwrap();
    assert_eq!(m.home_group_id(), Some(g.clone()));
    assert_eq!(
        m.groups.iter().find(|x| x.id == old_home).unwrap().role,
        None
    );
    m.set_group_role(&g, None).unwrap();
    assert_eq!(m.home_group_id(), None);
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn moving_the_hidden_role_makes_the_old_holder_the_last_page() {
    let mut m = Model::new_default();
    let old_hidden = role_id(&m, GroupRole::Hidden);
    let g = m.add_group("Chat");
    m.set_group_role(&g, Some(GroupRole::Hidden)).unwrap();
    assert_eq!(role_id(&m, GroupRole::Hidden), g);
    assert_eq!(page_labels(&m), ["Apps", "Home", "Games", "Hidden"]);
    assert_eq!(page_orders(&m), [0, 1, 2, 3]);
    assert_eq!(m.pages().last().unwrap().id, old_hidden);
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn root_role_cannot_be_assigned_or_changed() {
    let mut m = Model::new_default();
    let root = m.root_group_id.clone();
    let g = m.add_group("Chat");
    assert_eq!(
        m.set_group_role(&g, Some(GroupRole::Root)),
        Err(ModelError::RoleNotAssignable)
    );
    assert_eq!(
        m.set_group_role(&root, None),
        Err(ModelError::RoleNotAssignable)
    );
    assert_eq!(
        m.set_group_role(&root, Some(GroupRole::Apps)),
        Err(ModelError::RoleNotAssignable)
    );
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn set_group_role_rejects_unknown_group() {
    let mut m = Model::new_default();
    assert_eq!(
        m.set_group_role(&"nope".to_string(), Some(GroupRole::Home)),
        Err(ModelError::UnknownGroup("nope".into()))
    );
}

#[test]
fn deleting_a_group_closes_the_page_gap_and_appends_items_to_apps() {
    let mut m = Model::new_default();
    let chat = m.add_group("Chat");
    let music = m.add_group("Music");
    let apps = role_id(&m, GroupRole::Apps);
    m.add_item_to(&apps, "Existing");
    m.add_item_to(&chat, "Signal");
    m.add_item_to(&chat, "Matrix");

    m.delete_group(&chat).unwrap();

    assert_eq!(page_labels(&m), ["Apps", "Home", "Games", "Music"]);
    assert_eq!(page_orders(&m), [0, 1, 2, 3]);
    assert_eq!(item_labels(&m, &apps), ["Existing", "Signal", "Matrix"]);
    assert_eq!(item_orders(&m, &apps), [0, 1, 2]);
    assert!(m.groups.iter().all(|g| g.id != chat));
    assert!(m.groups.iter().any(|g| g.id == music));
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn delete_group_rejects_root_and_unknown() {
    let mut m = Model::new_default();
    let root = m.root_group_id.clone();
    assert_eq!(m.delete_group(&root), Err(ModelError::GroupHoldsRole(root)));
    assert_eq!(
        m.delete_group(&"nope".to_string()),
        Err(ModelError::UnknownGroup("nope".into()))
    );
}

#[test]
fn move_item_splices_within_and_across_groups_keeping_orders_dense() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let games = role_id(&m, GroupRole::Games);
    let a = m.add_item_to(&apps, "a");
    m.add_item_to(&apps, "b");
    m.add_item_to(&apps, "c");
    let x = m.add_item_to(&games, "x");
    m.add_item_to(&games, "y");

    m.move_item(&a, &apps, 2).unwrap();
    assert_eq!(item_labels(&m, &apps), ["b", "c", "a"]);
    assert_eq!(item_orders(&m, &apps), [0, 1, 2]);

    m.move_item(&a, &games, 1).unwrap();
    assert_eq!(item_labels(&m, &apps), ["b", "c"]);
    assert_eq!(item_orders(&m, &apps), [0, 1]);
    assert_eq!(item_labels(&m, &games), ["x", "a", "y"]);
    assert_eq!(item_orders(&m, &games), [0, 1, 2]);

    m.move_item(&x, &games, 99).unwrap();
    assert_eq!(item_labels(&m, &games), ["a", "y", "x"]);
    m.move_item(&x, &games, -1).unwrap();
    assert_eq!(item_labels(&m, &games), ["x", "a", "y"]);
    assert_eq!(item_orders(&m, &games), [0, 1, 2]);
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn move_item_refuses_the_root_and_unknown_ids_without_changing_anything() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let a = m.add_item_to(&apps, "a");
    let before = m.clone();
    let root = m.root_group_id.clone();
    assert_eq!(
        m.move_item(&a, &root, 0),
        Err(ModelError::ItemInRoot(a.clone()))
    );
    assert_eq!(
        m.move_item(&a, &"nope".to_string(), 0),
        Err(ModelError::UnknownGroup("nope".into()))
    );
    assert_eq!(
        m.move_item(&"nope".to_string(), &apps, 0),
        Err(ModelError::UnknownItem("nope".into()))
    );
    assert_eq!(m, before);
}

#[test]
fn rename_item_marks_the_label_as_overridden() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let a = m.add_item_to(&apps, "a");
    m.rename_item(&a, "Alpha").unwrap();
    let item = m.item(&a).unwrap();
    assert_eq!(item.label, "Alpha");
    assert!(item.label_overridden);
    assert_eq!(
        m.rename_item(&"nope".to_string(), "x"),
        Err(ModelError::UnknownItem("nope".into()))
    );
}

#[test]
fn remove_item_closes_the_gap() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    m.add_item_to(&apps, "a");
    let b = m.add_item_to(&apps, "b");
    m.add_item_to(&apps, "c");
    m.remove_item(&b).unwrap();
    assert_eq!(item_labels(&m, &apps), ["a", "c"]);
    assert_eq!(item_orders(&m, &apps), [0, 1]);
    assert!(m.item(&b).is_none());
    assert_eq!(m.remove_item(&b), Err(ModelError::UnknownItem(b.clone())));
}

#[test]
fn sort_group_is_stable_on_ties_and_rewrites_dense_orders() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    for l in ["b", "B", "a", "b"] {
        m.add_item_to(&apps, l);
    }
    m.sort_group(&apps).unwrap();
    assert_eq!(item_labels(&m, &apps), ["a", "B", "b", "b"]);
    assert_eq!(item_orders(&m, &apps), [0, 1, 2, 3]);
    assert_eq!(
        m.sort_group(&"nope".to_string()),
        Err(ModelError::UnknownGroup("nope".into()))
    );
}

#[test]
fn add_app_places_in_apps_and_never_duplicates_an_app_ref() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let app = AppRef {
        package: "org.mozilla.firefox".into(),
        activity: "Main".into(),
    };
    let first = m.add_app(app.clone(), "Firefox");
    let again = m.add_app(app.clone(), "Other Label");
    assert_eq!(first, again);
    let item = m.item(&first).unwrap();
    assert_eq!(item.group_id, apps);
    assert_eq!(item.label, "Firefox");
    assert_eq!(item.app, Some(app));
    assert!(!item.label_overridden);
    assert!(!item.uninstalled);
    assert_eq!(m.items.len(), 1);
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn add_item_to_an_unknown_or_root_group_falls_back_to_apps() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let root = m.root_group_id.clone();
    let a = m.add_item_to(&root, "a");
    let b = m.add_item_to(&"nope".to_string(), "b");
    assert_eq!(m.item(&a).unwrap().group_id, apps);
    assert_eq!(m.item(&b).unwrap().group_id, apps);
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn home_group_id_is_none_without_a_home_group() {
    let mut m = Model::new_default();
    assert_eq!(m.home_group_id(), Some(role_id(&m, GroupRole::Home)));
    m.groups.retain(|g| g.role != Some(GroupRole::Home));
    assert_eq!(m.home_group_id(), None);
}

#[test]
fn a_required_role_holder_cannot_trade_its_role_for_another() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let before = m.clone();
    assert_eq!(
        m.set_group_role(&apps, Some(GroupRole::Games)),
        Err(ModelError::RoleNotAssignable)
    );
    assert_eq!(m, before);
}

#[test]
fn the_home_group_can_take_over_apps_leaving_home_unset() {
    let mut m = Model::new_default();
    let home = role_id(&m, GroupRole::Home);
    let old_apps = role_id(&m, GroupRole::Apps);
    m.set_group_role(&home, Some(GroupRole::Apps)).unwrap();
    assert_eq!(role_id(&m, GroupRole::Apps), home);
    assert_eq!(m.home_group_id(), None);
    assert_eq!(
        m.groups.iter().find(|g| g.id == old_apps).unwrap().role,
        None
    );
    assert_eq!(m.validate(), Ok(()));
}

fn appref(package: &str, activity: &str) -> AppRef {
    AppRef {
        package: package.into(),
        activity: activity.into(),
    }
}

#[test]
fn set_item_app_sets_the_app_and_allows_the_same_value_again() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let it = m.add_item_to(&apps, "Signal");
    m.set_item_app(&it, appref("org.signal", "Main")).unwrap();
    assert_eq!(m.item(&it).unwrap().app, Some(appref("org.signal", "Main")));
    m.set_item_app(&it, appref("org.signal", "Main")).unwrap();
    m.set_item_app(&it, appref("org.signal", "Other")).unwrap();
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn set_item_app_refuses_unknown_items_duplicates_and_shortcut_items() {
    let mut m = Model::new_default();
    let apps = role_id(&m, GroupRole::Apps);
    let a = m.add_app(appref("com.a", "Main"), "A");
    let b = m.add_item_to(&apps, "B");
    let sc = m.add_shortcut(
        &apps,
        "S",
        ShortcutPayload::Command {
            command: "x".into(),
        },
    );
    let before = m.clone();

    let unknown: Id = "nope".into();
    assert_eq!(
        m.set_item_app(&unknown, appref("com.z", "Main")),
        Err(ModelError::UnknownItem(unknown.clone()))
    );
    assert_eq!(
        m.set_item_app(&b, appref("com.a", "Main")),
        Err(ModelError::DuplicateAppRef(appref("com.a", "Main")))
    );
    assert_eq!(
        m.set_item_app(&sc, appref("com.z", "Main")),
        Err(ModelError::ItemHasShortcut(sc.clone()))
    );
    assert_eq!(m, before);
    assert_eq!(m.item(&a).unwrap().app, Some(appref("com.a", "Main")));
    assert_eq!(m.validate(), Ok(()));
}
