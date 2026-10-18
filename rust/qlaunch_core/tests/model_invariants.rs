use qlaunch_core::model::{AppRef, Group, GroupRole, Item, Model, ModelError};

fn app_item(id: &str, group: &str, package: &str, activity: &str) -> Item {
    Item {
        id: id.into(),
        group_id: group.into(),
        label: id.into(),
        label_overridden: false,
        app: Some(AppRef {
            package: package.into(),
            activity: activity.into(),
        }),
        shortcut: None,
        uninstalled: false,
        order: 0,
        icon_override: None,
    }
}

fn holders(m: &Model, role: GroupRole) -> Vec<&Group> {
    m.groups.iter().filter(|g| g.role == Some(role)).collect()
}

fn page_labels(m: &Model) -> Vec<String> {
    let mut pages: Vec<&Group> = m
        .groups
        .iter()
        .filter(|g| {
            g.parent_id.as_deref() == Some(m.root_group_id.as_str())
                && g.role != Some(GroupRole::Hidden)
        })
        .collect();
    pages.sort_by_key(|g| g.order);
    pages.iter().map(|g| g.label.clone()).collect()
}

#[test]
fn default_model_validates() {
    assert_eq!(Model::new_default().validate(), Ok(()));
}

#[test]
fn exactly_one_holder_for_each_required_role() {
    let m = Model::new_default();
    for role in [
        GroupRole::Root,
        GroupRole::Hidden,
        GroupRole::Apps,
        GroupRole::Games,
    ] {
        assert_eq!(holders(&m, role).len(), 1, "{role:?}");
        assert_eq!(m.role_holder(role).unwrap().role, Some(role));
    }
}

#[test]
fn root_group_is_the_root_role_holder() {
    let m = Model::new_default();
    let root = m.role_holder(GroupRole::Root).unwrap();
    assert_eq!(root.id, m.root_group_id);
    assert_eq!(root.label, "_root_");
    assert_eq!(root.order, 0);
    assert_eq!(root.parent_id, None);
}

#[test]
fn home_holder_is_labelled_home() {
    let m = Model::new_default();
    assert_eq!(m.role_holder(GroupRole::Home).unwrap().label, "Home");
}

#[test]
fn default_page_order_is_apps_home_games_and_hidden_is_not_a_page() {
    let m = Model::new_default();
    assert_eq!(page_labels(&m), ["Apps", "Home", "Games"]);
    let hidden = m.role_holder(GroupRole::Hidden).unwrap();
    assert_eq!(hidden.order, 999);
    assert_eq!(hidden.parent_id.as_deref(), Some(m.root_group_id.as_str()));
}

#[test]
fn duplicate_apps_role_is_rejected() {
    let mut m = Model::new_default();
    let mut extra = m.role_holder(GroupRole::Apps).unwrap().clone();
    extra.id = "second-apps".into();
    m.groups.push(extra);
    assert_eq!(
        m.validate(),
        Err(ModelError::DuplicateRole(GroupRole::Apps))
    );
}

#[test]
fn duplicate_home_role_is_rejected() {
    let mut m = Model::new_default();
    let mut extra = m.role_holder(GroupRole::Home).unwrap().clone();
    extra.id = "second-home".into();
    m.groups.push(extra);
    assert_eq!(
        m.validate(),
        Err(ModelError::DuplicateRole(GroupRole::Home))
    );
}

#[test]
fn home_role_is_optional() {
    let mut m = Model::new_default();
    m.groups.retain(|g| g.role != Some(GroupRole::Home));
    assert_eq!(m.validate(), Ok(()));
    assert!(m.role_holder(GroupRole::Home).is_none());
}

#[test]
fn missing_required_role_is_rejected() {
    let mut m = Model::new_default();
    m.groups.retain(|g| g.role != Some(GroupRole::Games));
    assert_eq!(
        m.validate(),
        Err(ModelError::MissingRole(GroupRole::Games))
    );
}

#[test]
fn missing_root_is_rejected() {
    let mut m = Model::new_default();
    m.root_group_id = "no-such-group".into();
    assert_eq!(m.validate(), Err(ModelError::MissingRoot));

    let mut m = Model::new_default();
    let apps = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    m.root_group_id = apps;
    assert_eq!(m.validate(), Err(ModelError::MissingRoot));
}

#[test]
fn duplicate_app_ref_is_rejected() {
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    m.items.push(app_item("i1", &g, "a", "b"));
    m.items.push(app_item("i2", &g, "a", "b"));
    assert!(matches!(m.validate(), Err(ModelError::DuplicateAppRef(_))));
}

#[test]
fn duplicate_app_ref_is_rejected_across_groups() {
    let mut m = Model::new_default();
    let apps = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    let games = m.role_holder(GroupRole::Games).unwrap().id.clone();
    m.items.push(app_item("i1", &apps, "a", ""));
    m.items.push(app_item("i2", &games, "a", ""));
    assert_eq!(
        m.validate(),
        Err(ModelError::DuplicateAppRef(AppRef {
            package: "a".into(),
            activity: "".into()
        }))
    );
}

#[test]
fn same_package_with_different_activity_is_allowed() {
    let mut m = Model::new_default();
    let g = m.role_holder(GroupRole::Apps).unwrap().id.clone();
    m.items.push(app_item("i1", &g, "a", "x"));
    m.items.push(app_item("i2", &g, "a", "y"));
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn item_in_unknown_group_is_rejected() {
    let mut m = Model::new_default();
    m.items.push(app_item("i1", "nowhere", "a", "b"));
    assert_eq!(
        m.validate(),
        Err(ModelError::UnknownGroup("nowhere".into()))
    );
}

#[test]
fn group_with_unknown_parent_is_rejected() {
    let mut m = Model::new_default();
    m.groups.push(Group {
        id: "orphan".into(),
        label: "Orphan".into(),
        parent_id: Some("nowhere".into()),
        role: None,
        order: 5,
    });
    assert_eq!(
        m.validate(),
        Err(ModelError::UnknownGroup("nowhere".into()))
    );
}
