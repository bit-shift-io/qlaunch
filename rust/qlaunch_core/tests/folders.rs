use qlaunch_core::model::{AppRef, Group, GroupRole, Item, Model, ModelError};

fn folder_group(m: &Model, id: &str, label: &str) -> Group {
    Group {
        id: id.into(),
        label: label.into(),
        parent_id: Some(m.root_group_id.clone()),
        role: None,
        order: 0,
        hidden: false,
        folder: true,
    }
}

fn tile_item(id: &str, group: &str, folder_id: &str) -> Item {
    Item {
        id: id.into(),
        group_id: group.into(),
        label: id.into(),
        label_overridden: false,
        app: None,
        shortcut: None,
        uninstalled: false,
        order: 0,
        icon_override: None,
        folder_group_id: Some(folder_id.into()),
    }
}

fn apps_id(m: &Model) -> String {
    m.role_holder(GroupRole::Apps).unwrap().id.clone()
}

#[test]
fn a_folder_group_without_a_tile_is_invalid() {
    let mut m = Model::new_default();
    m.groups.push(folder_group(&m, "f", "Folder"));

    assert!(matches!(
        m.validate(),
        Err(ModelError::FolderGroupHasNoTile(g)) if g == "f"
    ));
}

#[test]
fn a_tile_item_with_an_app_is_invalid() {
    let mut m = Model::new_default();
    let apps = apps_id(&m);
    m.groups.push(folder_group(&m, "f", "Folder"));
    let mut tile = tile_item("t", &apps, "f");
    tile.app = Some(AppRef {
        package: "pkg".into(),
        activity: "act".into(),
    });
    m.items.push(tile);

    assert!(matches!(
        m.validate(),
        Err(ModelError::FolderTileHasContent(id)) if id == "t"
    ));
}

fn games_id(m: &Model) -> String {
    m.role_holder(GroupRole::Games).unwrap().id.clone()
}

fn tile_id_in(m: &Model, folder: &str) -> String {
    m.items
        .iter()
        .find(|i| i.folder_group_id.as_deref() == Some(folder))
        .unwrap()
        .id
        .clone()
}

fn folder_model() -> (Model, String, String) {
    let mut m = Model::new_default();
    let apps = apps_id(&m);
    let folder = m.add_folder(&apps, "Chat").unwrap();
    m.add_item_to(&folder, "Alice");
    m.add_item_to(&folder, "Bob");
    let tile = tile_id_in(&m, &folder);
    (m, folder, tile)
}

#[test]
fn add_folder_creates_group_and_tile_in_order() {
    let mut m = Model::new_default();
    let apps = apps_id(&m);
    let folder = m.add_folder(&apps, "Chat").unwrap();

    let group = m.groups.iter().find(|g| g.id == folder).unwrap();
    assert!(group.folder);
    assert_eq!(group.parent_id.as_deref(), Some(m.root_group_id.as_str()));
    assert_eq!(group.role, None);
    assert_eq!(group.label, "Chat");

    let tile = m
        .items
        .iter()
        .find(|i| i.folder_group_id.is_some())
        .unwrap();
    assert_eq!(tile.group_id, apps);
    assert_eq!(tile.label, "Chat");
    assert!(tile.app.is_none());
    assert!(tile.shortcut.is_none());

    assert!(!m.pages().iter().any(|g| g.id == folder));
    assert_eq!(m.pages().len(), 3);
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn add_folder_refuses_the_root_and_another_folder() {
    let (mut m, folder, _) = folder_model();
    assert!(matches!(
        m.add_folder(&folder, "Nested"),
        Err(ModelError::NotAFolderDestination(id)) if id == folder
    ));
    let root = m.root_group_id.clone();
    assert!(matches!(
        m.add_folder(&root, "RootFolder"),
        Err(ModelError::NotAFolderDestination(id)) if id == root
    ));
    assert!(matches!(
        m.add_folder(&"nope".into(), "X"),
        Err(ModelError::UnknownGroup(_))
    ));
}

#[test]
fn delete_folder_returns_members_to_the_tiles_current_group() {
    let (mut m, folder, tile) = folder_model();
    let games = games_id(&m);
    m.move_item(&tile, &games, 0).unwrap();
    m.delete_folder(&folder).unwrap();

    assert!(m.groups.iter().all(|g| g.id != folder));
    assert!(m.items.iter().all(|i| i.id != tile));
    let labels: Vec<String> = m.items_in(&games).iter().map(|i| i.label.clone()).collect();
    assert_eq!(labels, ["Alice", "Bob"]);
    let orders: Vec<i32> = m.items_in(&games).iter().map(|i| i.order).collect();
    assert_eq!(orders, [0, 1]);
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn deleting_an_empty_folder_removes_only_the_tile_and_group() {
    let mut m = Model::new_default();
    let apps = apps_id(&m);
    let folder = m.add_folder(&apps, "Empty").unwrap();
    let tile = tile_id_in(&m, &folder);

    m.delete_folder(&folder).unwrap();

    assert!(m.groups.iter().all(|g| g.id != folder));
    assert!(m.items.iter().all(|i| i.id != tile));
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn deleting_a_non_folder_group_is_refused() {
    let (mut m, _, _) = folder_model();
    let apps = apps_id(&m);
    assert!(matches!(
        m.delete_folder(&apps),
        Err(ModelError::NotAFolderGroup(id)) if id == apps
    ));
}

#[test]
fn moving_a_folder_tile_into_a_folder_or_the_root_is_refused() {
    let mut m = Model::new_default();
    let apps = apps_id(&m);
    let a = m.add_folder(&apps, "A").unwrap();
    let b = m.add_folder(&apps, "B").unwrap();
    let tile = tile_id_in(&m, &a);

    assert!(matches!(
        m.move_item(&tile, &b, 0),
        Err(ModelError::FolderTileInFolder(id)) if id == tile
    ));
    let root = m.root_group_id.clone();
    assert!(matches!(
        m.move_item(&tile, &root, 0),
        Err(ModelError::ItemInRoot(id)) if id == tile
    ));
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn moving_an_app_into_and_out_of_a_folder_works() {
    let (mut m, folder, _) = folder_model();
    let apps = apps_id(&m);
    let app = m.add_app(
        AppRef {
            package: "pkg".into(),
            activity: "act".into(),
        },
        "App",
    );

    m.move_item(&app, &folder, 0).unwrap();
    assert!(m.items_in(&folder).iter().any(|i| i.id == app));
    m.move_item(&app, &apps, 0).unwrap();
    assert!(m.items_in(&apps).iter().any(|i| i.id == app));
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn renaming_a_folder_renames_its_tile() {
    let (mut m, folder, tile) = folder_model();
    m.rename_group(&folder, "Friends").unwrap();

    assert_eq!(
        m.groups.iter().find(|g| g.id == folder).unwrap().label,
        "Friends"
    );
    assert_eq!(m.item(&tile).unwrap().label, "Friends");
}

#[test]
fn folder_groups_are_refused_by_the_ordinary_group_operations() {
    let (mut m, folder, _) = folder_model();
    assert!(matches!(
        m.delete_group(&folder),
        Err(ModelError::FolderHoldsRole(id)) if id == folder
    ));
    assert!(matches!(
        m.set_group_role(&folder, Some(GroupRole::Home)),
        Err(ModelError::RoleNotAssignable)
    ));
    assert!(matches!(
        m.set_group_hidden(&folder, true),
        Err(ModelError::NotAPage(id)) if id == folder
    ));
}

#[test]
fn a_folder_written_before_the_field_loads_as_an_ordinary_group() {
    let group: Group = serde_json::from_value(serde_json::json!({
        "id": "g",
        "label": "G",
        "parent_id": null,
        "role": null,
        "order": 0,
        "hidden": false
    }))
    .unwrap();
    assert!(!group.folder);

    let item: Item = serde_json::from_value(serde_json::json!({
        "id": "i",
        "group_id": "g",
        "label": "I",
        "label_overridden": false,
        "app": null,
        "shortcut": null,
        "uninstalled": false,
        "order": 0,
        "icon_override": null
    }))
    .unwrap();
    assert_eq!(item.folder_group_id, None);
}
