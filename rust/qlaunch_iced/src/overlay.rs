//! Menus rendered above the launcher. Actions call the existing core API;
//! no model rule is reimplemented here.

use iced::widget::{button, column, container, mouse_area, scrollable, space, text, text_input};
use iced::{Color, Element, Fill};
use qlaunch_core::api::api::ModelSnapshot;
use qlaunch_core::api::shortcuts::ShortcutProvider;
use qlaunch_core::model::{Group, Id, Item};

use crate::{App, Message as AppMessage, view};

// `AppMenu` carries a full `Item`, so the variants differ in size.
#[allow(clippy::large_enum_variant)]
pub enum Overlay {
    App(AppMenu),
    Launcher(LauncherMenu),
}

pub struct AppMenu {
    pub item: Item,
    pub step: AppStep,
    pub error: Option<String>,
}

pub enum AppStep {
    Actions,
    Rename(String),
    Move,
}

pub struct LauncherMenu {
    pub step: LauncherStep,
    pub providers: Vec<ShortcutProvider>,
    pub providers_loading: bool,
    pub error: Option<String>,
}

pub enum LauncherStep {
    Actions,
    FolderName(String),
    Shortcuts,
}

#[derive(Debug, Clone)]
pub enum Message {
    Close,
    Noop,
    Back,
    AppRenameStart,
    AppRename(String),
    AppRenameSubmit,
    AppMove,
    AppMoveTo(Id),
    AppUninstall,
    AppRemove,
    LauncherSettings,
    LauncherAddShortcut,
    LauncherCreateFolder,
    LauncherFolderName(String),
    LauncherFolderSubmit,
    LauncherEditGroups,
    LauncherWallpaper,
    ShortcutLock,
    ShortcutNotifications,
    ShortcutProvider(String),
    Providers(Result<Vec<ShortcutProvider>, String>),
}

impl Overlay {
    /// Steps a sub-menu back to its action list; returns true when the
    /// overlay itself should close.
    pub fn back(&mut self) -> bool {
        match self {
            Overlay::App(menu) => {
                if matches!(menu.step, AppStep::Actions) {
                    true
                } else {
                    menu.step = AppStep::Actions;
                    menu.error = None;
                    false
                }
            }
            Overlay::Launcher(menu) => {
                if matches!(menu.step, LauncherStep::Actions) {
                    true
                } else {
                    menu.step = LauncherStep::Actions;
                    menu.error = None;
                    false
                }
            }
        }
    }

    pub fn set_error(&mut self, error: String) {
        match self {
            Overlay::App(menu) => menu.error = Some(error),
            Overlay::Launcher(menu) => menu.error = Some(error),
        }
    }
}

/// Page groups an item can move to, in the snapshot's group order.
pub fn move_targets<'a>(snapshot: &'a ModelSnapshot, item: &Item) -> Vec<&'a Group> {
    snapshot
        .groups
        .iter()
        .filter(|group| {
            group.id != snapshot.root_group_id && !group.folder && group.id != item.group_id
        })
        .collect()
}

/// The order value that appends an item to the end of `group`.
pub fn append_order(snapshot: &ModelSnapshot, group: &Id) -> i32 {
    snapshot
        .items
        .iter()
        .filter(|item| &item.group_id == group)
        .count() as i32
}

pub fn view<'a>(app: &'a App, overlay: &'a Overlay) -> Element<'a, AppMessage> {
    let scrim =
        mouse_area(
            container(space())
                .width(Fill)
                .height(Fill)
                .style(|_| container::Style {
                    background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
                    ..Default::default()
                }),
        )
        .on_press(AppMessage::Overlay(Message::Close));

    let inner: Element<'a, AppMessage> = match overlay {
        Overlay::App(menu) => app_card(app, menu),
        Overlay::Launcher(menu) => launcher_card(app, menu),
    };
    let card = mouse_area(
        container(inner)
            .width(320)
            .padding(12)
            .style(|_| container::Style {
                background: Some(Color::from_rgba8(20, 20, 24, 0.96).into()),
                border: iced::border::rounded(14),
                ..Default::default()
            }),
    )
    .on_press(AppMessage::Overlay(Message::Noop));

    iced::widget::stack![scrim, container(card).center(Fill)].into()
}

fn app_card<'a>(app: &'a App, menu: &'a AppMenu) -> Element<'a, AppMessage> {
    let mut card = column![].spacing(6).push(view::label(
        app,
        menu.item.label.clone(),
        Color::WHITE,
        false,
    ));
    match &menu.step {
        AppStep::Actions => {
            card = card
                .push(action("Rename", Message::AppRenameStart))
                .push(action("Move", Message::AppMove));
            if menu.item.app.is_some() {
                card = card.push(action("Uninstall", Message::AppUninstall));
            }
            card = card
                .push(action("Remove", Message::AppRemove))
                .push(action("Cancel", Message::Close));
        }
        AppStep::Rename(name) => {
            card = card
                .push(
                    text_input("App name", name)
                        .on_input(|value| AppMessage::Overlay(Message::AppRename(value)))
                        .on_submit(AppMessage::Overlay(Message::AppRenameSubmit))
                        .padding(8),
                )
                .push(action("Save", Message::AppRenameSubmit))
                .push(action("Back", Message::Back));
        }
        AppStep::Move => {
            let targets = app
                .snapshot
                .as_ref()
                .map(|snapshot| move_targets(snapshot, &menu.item))
                .unwrap_or_default();
            if targets.is_empty() {
                card = card.push(text("No other pages").size(14));
            } else {
                let mut list = column![].spacing(2);
                for group in targets {
                    list = list.push(action(&group.label, Message::AppMoveTo(group.id.clone())));
                }
                card = card.push(scrollable(list).id("overlay-list").height(240.0));
            }
            card = card.push(action("Back", Message::Back));
        }
    }
    card = error_line(card, &menu.error);
    card.into()
}

fn launcher_card<'a>(app: &'a App, menu: &'a LauncherMenu) -> Element<'a, AppMessage> {
    let mut card =
        column![]
            .spacing(6)
            .push(view::label(app, "Launcher".into(), Color::WHITE, false));
    match &menu.step {
        LauncherStep::Actions => {
            card = card
                .push(action("Create Folder", Message::LauncherCreateFolder))
                .push(action("Add Shortcut", Message::LauncherAddShortcut))
                .push(action("Lock Screen Shortcut", Message::ShortcutLock))
                .push(action(
                    "Notifications Shortcut",
                    Message::ShortcutNotifications,
                ))
                .push(action("Choose Wallpaper", Message::LauncherWallpaper))
                .push(action("Settings", Message::LauncherSettings))
                .push(action("Edit Groups", Message::LauncherEditGroups))
                .push(action("Close", Message::Close));
        }
        LauncherStep::FolderName(name) => {
            card = card
                .push(
                    text_input("Folder name", name)
                        .on_input(|value| AppMessage::Overlay(Message::LauncherFolderName(value)))
                        .on_submit(AppMessage::Overlay(Message::LauncherFolderSubmit))
                        .padding(8),
                )
                .push(action("Create", Message::LauncherFolderSubmit))
                .push(action("Back", Message::Back));
        }
        LauncherStep::Shortcuts => {
            if menu.providers_loading {
                card = card.push(text("Loading shortcuts…").size(14));
            } else if menu.providers.is_empty() {
                card = card.push(text("No shortcut providers").size(14));
            } else {
                let mut list = column![].spacing(2);
                for provider in &menu.providers {
                    list = list.push(action(
                        &provider.label,
                        Message::ShortcutProvider(provider.id.clone()),
                    ));
                }
                card = card.push(scrollable(list).id("overlay-list").height(240.0));
            }
            card = card.push(action("Back", Message::Back));
        }
    }
    card = error_line(card, &menu.error);
    card.into()
}

fn error_line<'a>(
    card: iced::widget::Column<'a, AppMessage>,
    error: &'a Option<String>,
) -> iced::widget::Column<'a, AppMessage> {
    match error {
        Some(error) => card.push(text(error).size(13).color(Color::from_rgb8(255, 140, 140))),
        None => card,
    }
}

fn action<'a>(label: &str, message: Message) -> Element<'a, AppMessage> {
    button(text(label.to_string()).size(15))
        .width(Fill)
        .padding([8, 6])
        .style(button::text)
        .on_press(AppMessage::Overlay(message))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qlaunch_core::model::{AppRef, Model};
    use qlaunch_core::settings::Settings;

    fn snapshot() -> ModelSnapshot {
        let model = Model::new_default();
        let root = model.root_group_id.clone();
        let home = model.home_group_id();
        let pages: Vec<Id> = model.pages().iter().map(|g| g.id.clone()).collect();
        let apps = pages[0].clone();
        let games = pages[2].clone();
        let mut groups = model.groups;
        groups.push(Group {
            id: "folder".into(),
            label: "Folder".into(),
            parent_id: Some(root.clone()),
            role: None,
            order: 3,
            hidden: false,
            folder: true,
        });
        let item = |id: &str, group: &Id, order: i32| Item {
            id: id.into(),
            group_id: group.clone(),
            label: id.into(),
            label_overridden: false,
            app: Some(AppRef {
                package: id.into(),
                activity: String::new(),
            }),
            shortcut: None,
            uninstalled: false,
            order,
            icon_override: None,
            folder_group_id: None,
        };
        ModelSnapshot {
            root_group_id: root,
            home_group_id: home,
            page_ids: pages,
            groups,
            items: vec![
                item("one", &apps, 0),
                item("two", &apps, 1),
                item("three", &games, 0),
            ],
            settings: Settings::default(),
            warnings: vec![],
            icon_revisions: vec![],
            folder_icons: vec![],
        }
    }

    #[test]
    fn move_targets_exclude_root_folders_and_current_group() {
        let snapshot = snapshot();
        let item = &snapshot.items[0];
        let targets = move_targets(&snapshot, item);
        let expected: Vec<&Id> = snapshot
            .groups
            .iter()
            .filter(|group| {
                group.id != snapshot.root_group_id && !group.folder && group.id != item.group_id
            })
            .map(|group| &group.id)
            .collect();
        let actual: Vec<&Id> = targets.iter().map(|group| &group.id).collect();
        assert_eq!(actual, expected);
        assert!(!actual.contains(&&snapshot.root_group_id));
        assert!(!actual.contains(&&"folder".to_string()));
        assert!(!actual.contains(&&item.group_id));
    }

    #[test]
    fn append_order_counts_items_in_the_destination() {
        let snapshot = snapshot();
        let apps = snapshot.items[0].group_id.clone();
        let games = snapshot.items[2].group_id.clone();
        assert_eq!(append_order(&snapshot, &apps), 2);
        assert_eq!(append_order(&snapshot, &games), 1);
    }

    fn app_menu(step: AppStep) -> Overlay {
        Overlay::App(AppMenu {
            item: snapshot().items[0].clone(),
            step,
            error: None,
        })
    }

    fn launcher_menu(step: LauncherStep) -> Overlay {
        Overlay::Launcher(LauncherMenu {
            step,
            providers: vec![],
            providers_loading: false,
            error: None,
        })
    }

    #[test]
    fn app_sub_menu_back_returns_to_actions_then_closes() {
        let mut overlay = app_menu(AppStep::Rename("Calculator".into()));
        assert!(!overlay.back());
        assert!(matches!(
            overlay,
            Overlay::App(AppMenu {
                step: AppStep::Actions,
                ..
            })
        ));
        assert!(overlay.back());
    }

    #[test]
    fn launcher_sub_menu_back_returns_to_actions_then_closes() {
        let mut overlay = launcher_menu(LauncherStep::Shortcuts);
        assert!(!overlay.back());
        assert!(matches!(
            overlay,
            Overlay::Launcher(LauncherMenu {
                step: LauncherStep::Actions,
                ..
            })
        ));
        assert!(overlay.back());
    }

    #[test]
    fn back_clears_a_sub_menu_error() {
        let mut overlay = app_menu(AppStep::Move);
        overlay.set_error("no such page".into());
        assert!(!overlay.back());
        match overlay {
            Overlay::App(menu) => assert!(menu.error.is_none()),
            Overlay::Launcher(_) => panic!("expected an app menu"),
        }
    }

    #[test]
    fn errors_land_in_the_open_menu() {
        let mut app = app_menu(AppStep::Actions);
        app.set_error("boom".into());
        match app {
            Overlay::App(menu) => assert_eq!(menu.error.as_deref(), Some("boom")),
            Overlay::Launcher(_) => panic!("expected an app menu"),
        }
        let mut launcher = launcher_menu(LauncherStep::Actions);
        launcher.set_error("bang".into());
        match launcher {
            Overlay::Launcher(menu) => assert_eq!(menu.error.as_deref(), Some("bang")),
            Overlay::App(_) => panic!("expected a launcher menu"),
        }
    }
}
