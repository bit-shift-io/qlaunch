//! Menus rendered above the launcher. Actions call the existing core API;
//! no model rule is reimplemented here.
//!
//! Dormant since the dialog migration; kept until the old overlay layer is
//! removed in a cleanup pass.
#![allow(dead_code)]

use iced::widget::{
    button, column, container, mouse_area, opaque, scrollable, space, text, text_input,
};
use iced::{Element, Fill};
use qlaunch_core::api::api::ModelSnapshot;
use qlaunch_core::api::shortcuts::ShortcutProvider;
use qlaunch_core::model::{Group, Id, Item};

use crate::{App, Message as AppMessage, Region, theme};

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

pub(crate) const CARD_WIDTH: f32 = 320.0;
pub(crate) const CARD_PADDING: f32 = 24.0;
pub(crate) const CARD_GAP: f32 = 8.0;
pub(crate) const ACTION_HEIGHT: f32 = 48.0;
pub(crate) const INPUT_HEIGHT: f32 = 56.0;
pub(crate) const TEXT_HEIGHT: f32 = 18.0;
pub(crate) const LIST_ROW_GAP: f32 = 2.0;
pub(crate) const APP_NAME_FIELD_ID: &str = "overlay-app-name";
pub(crate) const FOLDER_NAME_FIELD_ID: &str = "overlay-folder-name";

/// Material dialog title typography is independent of launcher icon labels.
pub(crate) fn heading_height(_: &qlaunch_core::settings::Settings) -> f32 {
    28.0
}

pub(crate) struct CardLayout {
    pub bounds: iced::Rectangle,
    pub title: iced::Rectangle,
    pub viewport: iced::Rectangle,
    pub content_height: f32,
    pub offset: f32,
    pub entries: Vec<EntryLayout>,
}

pub(crate) struct EntryLayout {
    pub key: String,
    pub bounds: iced::Rectangle,
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum EntryKind {
    Action {
        label: String,
        message: Message,
    },
    Input {
        hint: String,
        value: String,
        widget_id: &'static str,
        on_input: fn(String) -> AppMessage,
        on_submit: AppMessage,
    },
    Text {
        label: String,
        error: bool,
    },
    List {
        rows: Vec<ActionRow>,
    },
}

pub(crate) struct ActionRow {
    pub key: String,
    pub label: String,
    pub message: Message,
}

pub(crate) struct Entry {
    pub key: String,
    pub kind: EntryKind,
}

/// The card's title for the open menu.
pub(crate) fn heading(overlay: &Overlay) -> String {
    match overlay {
        Overlay::App(menu) => menu.item.label.clone(),
        Overlay::Launcher(_) => "Launcher".into(),
    }
}

/// The card's entries in draw order. This is the single source of row order for
/// both the view and the accessibility tree.
pub(crate) fn entries(app: &App, overlay: &Overlay) -> Vec<Entry> {
    let mut rows = Vec::new();
    match overlay {
        Overlay::App(menu) => {
            match &menu.step {
                AppStep::Actions => {
                    rows.push(action_entry("rename", "Rename", Message::AppRenameStart));
                    rows.push(action_entry("move", "Move", Message::AppMove));
                    if menu.item.app.is_some() {
                        rows.push(action_entry(
                            "uninstall",
                            "Uninstall",
                            Message::AppUninstall,
                        ));
                    }
                    rows.push(action_entry("remove", "Remove", Message::AppRemove));
                    rows.push(action_entry("cancel", "Cancel", Message::Close));
                }
                AppStep::Rename(name) => {
                    rows.push(Entry {
                        key: "name".into(),
                        kind: EntryKind::Input {
                            hint: "App name".into(),
                            value: name.clone(),
                            widget_id: APP_NAME_FIELD_ID,
                            on_input: |value| AppMessage::Overlay(Message::AppRename(value)),
                            on_submit: AppMessage::Overlay(Message::AppRenameSubmit),
                        },
                    });
                    rows.push(action_entry("save", "Save", Message::AppRenameSubmit));
                    rows.push(action_entry("back", "Back", Message::Back));
                }
                AppStep::Move => {
                    let targets = app
                        .snapshot
                        .as_ref()
                        .map(|snapshot| move_targets(snapshot, &menu.item))
                        .unwrap_or_default();
                    if targets.is_empty() {
                        rows.push(text_entry("message", "No other pages", false));
                    } else {
                        rows.push(Entry {
                            key: "list".into(),
                            kind: EntryKind::List {
                                rows: targets
                                    .into_iter()
                                    .map(|group| ActionRow {
                                        key: group.id.clone(),
                                        label: group.label.clone(),
                                        message: Message::AppMoveTo(group.id.clone()),
                                    })
                                    .collect(),
                            },
                        });
                    }
                    rows.push(action_entry("back", "Back", Message::Back));
                }
            }
            if let Some(error) = &menu.error {
                rows.push(text_entry("error", error, true));
            }
        }
        Overlay::Launcher(menu) => {
            match &menu.step {
                LauncherStep::Actions => {
                    rows.push(action_entry(
                        "create-folder",
                        "Create Folder",
                        Message::LauncherCreateFolder,
                    ));
                    rows.push(action_entry(
                        "add-shortcut",
                        "Add Shortcut",
                        Message::LauncherAddShortcut,
                    ));
                    rows.push(action_entry(
                        "lock",
                        "Lock Screen Shortcut",
                        Message::ShortcutLock,
                    ));
                    rows.push(action_entry(
                        "notifications",
                        "Notifications Shortcut",
                        Message::ShortcutNotifications,
                    ));
                    rows.push(action_entry(
                        "wallpaper",
                        "Choose Wallpaper",
                        Message::LauncherWallpaper,
                    ));
                    rows.push(action_entry(
                        "settings",
                        "Settings",
                        Message::LauncherSettings,
                    ));
                    rows.push(action_entry(
                        "edit-groups",
                        "Edit Groups",
                        Message::LauncherEditGroups,
                    ));
                    rows.push(action_entry("close", "Close", Message::Close));
                }
                LauncherStep::FolderName(name) => {
                    rows.push(Entry {
                        key: "name".into(),
                        kind: EntryKind::Input {
                            hint: "Folder name".into(),
                            value: name.clone(),
                            widget_id: FOLDER_NAME_FIELD_ID,
                            on_input: |value| {
                                AppMessage::Overlay(Message::LauncherFolderName(value))
                            },
                            on_submit: AppMessage::Overlay(Message::LauncherFolderSubmit),
                        },
                    });
                    rows.push(action_entry(
                        "create",
                        "Create",
                        Message::LauncherFolderSubmit,
                    ));
                    rows.push(action_entry("back", "Back", Message::Back));
                }
                LauncherStep::Shortcuts => {
                    if menu.providers_loading {
                        rows.push(text_entry("message", "Loading shortcuts…", false));
                    } else if menu.providers.is_empty() {
                        rows.push(text_entry("message", "No shortcut providers", false));
                    } else {
                        rows.push(Entry {
                            key: "list".into(),
                            kind: EntryKind::List {
                                rows: menu
                                    .providers
                                    .iter()
                                    .map(|provider| ActionRow {
                                        key: provider.id.clone(),
                                        label: provider.label.clone(),
                                        message: Message::ShortcutProvider(provider.id.clone()),
                                    })
                                    .collect(),
                            },
                        });
                    }
                    rows.push(action_entry("back", "Back", Message::Back));
                }
            }
            if let Some(error) = &menu.error {
                rows.push(text_entry("error", error, true));
            }
        }
    }
    rows
}

fn action_entry(key: &str, label: &str, message: Message) -> Entry {
    Entry {
        key: key.into(),
        kind: EntryKind::Action {
            label: label.into(),
            message,
        },
    }
}

fn text_entry(key: &str, label: &str, error: bool) -> Entry {
    Entry {
        key: key.into(),
        kind: EntryKind::Text {
            label: label.into(),
            error,
        },
    }
}

fn entry_height(kind: &EntryKind) -> f32 {
    match kind {
        EntryKind::Action { .. } => ACTION_HEIGHT,
        EntryKind::Input { .. } => INPUT_HEIGHT,
        EntryKind::Text { .. } => TEXT_HEIGHT,
        EntryKind::List { rows } => {
            rows.len() as f32 * ACTION_HEIGHT + rows.len().saturating_sub(1) as f32 * LIST_ROW_GAP
        }
    }
}

/// Deterministic card geometry used by the view and the accessibility tree.
pub(crate) fn card(app: &App, overlay: &Overlay) -> CardLayout {
    let default_settings = qlaunch_core::settings::Settings::default();
    let settings = app
        .snapshot
        .as_ref()
        .map_or(&default_settings, |snapshot| &snapshot.settings);
    let entries = entries(app, overlay);
    let title_height = heading_height(settings);
    let content_height = title_height
        + entries
            .iter()
            .map(|entry| entry_height(&entry.kind))
            .sum::<f32>()
        + entries.len() as f32 * CARD_GAP;
    let usable_width = (app.size.width - app.insets.left - app.insets.right).max(0.0);
    let usable_height = (app.size.height - app.insets.top - app.insets.bottom).max(0.0);
    let margin = 24.0_f32.min(usable_width / 8.0).min(usable_height / 8.0);
    let width = CARD_WIDTH.min((usable_width - 2.0 * margin).max(0.0));
    let available_height = (usable_height - 2.0 * margin).max(0.0);
    let padding = CARD_PADDING.min(width / 4.0).min(available_height / 4.0);
    let height = (content_height + 2.0 * padding).min(available_height);
    let x = app.insets.left + (usable_width - width) / 2.0;
    let y = app.insets.top + (usable_height - height) / 2.0;
    let bounds = iced::Rectangle {
        x,
        y,
        width,
        height,
    };
    let viewport = iced::Rectangle {
        x: x + padding,
        y: y + padding,
        width: (width - 2.0 * padding).max(0.0),
        height: (height - 2.0 * padding).max(0.0),
    };
    let offset = app
        .scroll
        .get(&Region::OverlayCard)
        .copied()
        .unwrap_or(0.0)
        .clamp(0.0, (content_height - viewport.height).max(0.0));
    let title = iced::Rectangle {
        x: viewport.x,
        y: viewport.y - offset,
        width: viewport.width,
        height: title_height,
    };
    let mut cursor = title.y + title.height + CARD_GAP;
    let entries = entries
        .into_iter()
        .map(|entry| {
            let height = entry_height(&entry.kind);
            let bounds = iced::Rectangle {
                x: viewport.x,
                y: cursor,
                width: viewport.width,
                height,
            };
            cursor += height + CARD_GAP;
            EntryLayout {
                key: entry.key,
                bounds,
            }
        })
        .collect();
    CardLayout {
        bounds,
        title,
        viewport,
        content_height,
        offset,
        entries,
    }
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
                    background: Some(theme::color::SCRIM.into()),
                    ..Default::default()
                }),
        )
        .on_press(AppMessage::Overlay(Message::Close));

    let layout = card(app, overlay);
    let motion = app.motion(Region::OverlayCard);
    let mut card = column![].spacing(CARD_GAP).push(crate::elastic_scroll::row(
        container(
            text(heading(overlay))
                .size(22)
                .line_height(1.27)
                .color(theme::color::ON_SURFACE),
        )
        .width(Fill)
        .height(layout.title.height)
        .clip(true)
        .into(),
        &motion,
    ));
    for entry in entries(app, overlay) {
        card = card.push(crate::elastic_scroll::row(entry_view(entry), &motion));
    }
    let padding = layout.viewport.x - layout.bounds.x;
    let scroller = scrollable(crate::elastic_scroll::content(card.into(), &motion))
        .id("overlay-card")
        .direction(theme::vertical_scrollbar(app.scrollbars_visible()))
        .style(theme::scrollable)
        .width(layout.viewport.width)
        .height(layout.viewport.height)
        .on_scroll(|viewport| {
            AppMessage::Scrolled(Region::OverlayCard, viewport.absolute_offset().y)
        });
    let card = opaque(
        mouse_area(
            container(crate::elastic_scroll::wrap(
                scroller.into(),
                motion,
                crate::elastic_scroll::Policy::Normal,
            ))
            .width(layout.bounds.width)
            .height(layout.bounds.height)
            .padding(padding)
            .style(theme::card),
        )
        .on_press(AppMessage::Overlay(Message::Noop)),
    );

    opaque(iced::widget::stack![
        scrim,
        iced::widget::pin(card)
            .x(layout.bounds.x)
            .y(layout.bounds.y)
    ])
}

fn entry_view<'a>(entry: Entry) -> Element<'a, AppMessage> {
    match entry.kind {
        EntryKind::Action { label, message } => action(&label, message, ACTION_HEIGHT),
        EntryKind::Input {
            hint,
            value,
            widget_id,
            on_input,
            on_submit,
        } => container(crate::field::input(
            text_input(&hint, &value)
                .id(widget_id)
                .on_input(on_input)
                .on_submit(on_submit)
                .size(16)
                .padding([16, 0])
                .style(theme::input),
        ))
        .width(Fill)
        .center_y(INPUT_HEIGHT)
        .into(),
        EntryKind::Text { label, error } => {
            let line = text(label).size(if error { 13 } else { 14 });
            let line = if error {
                line.color(theme::color::ERROR)
            } else {
                line.color(theme::color::ON_SURFACE)
            };
            container(line)
                .width(Fill)
                .height(TEXT_HEIGHT)
                .clip(true)
                .into()
        }
        EntryKind::List { rows } => {
            let mut list = column![].spacing(LIST_ROW_GAP);
            for row in rows {
                list = list.push(action(&row.label, row.message, ACTION_HEIGHT));
            }
            list.into()
        }
    }
}

fn action<'a>(label: &str, message: Message, height: f32) -> Element<'a, AppMessage> {
    let style = match &message {
        Message::AppRenameSubmit | Message::LauncherFolderSubmit => theme::filled_button,
        Message::AppUninstall | Message::AppRemove => theme::destructive_button,
        _ => theme::text_button,
    };
    button(
        container(text(label.to_string()).size(14))
            .center_y(Fill)
            .clip(true),
    )
    .width(Fill)
    .height(height)
    .padding([0, 16])
    .style(style)
    .on_press(AppMessage::Overlay(message))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qlaunch_core::model::{AppRef, Model};
    use qlaunch_core::settings::Settings;

    #[test]
    fn input_entry_keeps_the_height_used_by_accessibility() {
        let entry = Entry {
            key: "name".into(),
            kind: EntryKind::Input {
                hint: "App name".into(),
                value: "Firefox".into(),
                widget_id: APP_NAME_FIELD_ID,
                on_input: |value| AppMessage::Overlay(Message::AppRename(value)),
                on_submit: AppMessage::Overlay(Message::AppRenameSubmit),
            },
        };
        assert_eq!(
            entry_view(entry).as_widget().size().height,
            iced::Length::Fixed(INPUT_HEIGHT),
        );
    }

    #[test]
    fn move_menu_has_one_scrollable_so_touch_cannot_be_trapped() {
        use iced::advanced::widget::Tree;
        let app = test_app();
        let overlay = app_menu(AppStep::Move);
        let tree = Tree::new(view(&app, &overlay).as_widget());
        let probe: Element<'_, AppMessage> = scrollable(space()).into();
        let scroll_tag = Tree::new(probe.as_widget()).tag;
        fn count(tree: &Tree, tag: iced::advanced::widget::tree::Tag) -> usize {
            usize::from(tree.tag == tag)
                + tree
                    .children
                    .iter()
                    .map(|child| count(child, tag))
                    .sum::<usize>()
        }
        assert_eq!(
            count(&tree, scroll_tag),
            1,
            "a nested list would capture swipes intended to reach Back"
        );
    }

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

    fn test_app() -> App {
        let (mut app, _) = App::new();
        app.snapshot = Some(snapshot());
        app.size = iced::Size::new(420.0, 800.0);
        app.pager = crate::Pager::new(3, 0);
        app
    }

    fn action_labels(overlay: &Overlay) -> Vec<String> {
        let app = test_app();
        entries(&app, overlay)
            .into_iter()
            .filter_map(|entry| match entry.kind {
                EntryKind::Action { label, .. } => Some(label),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn app_actions_follow_the_drawn_order() {
        let overlay = app_menu(AppStep::Actions);
        assert_eq!(
            action_labels(&overlay),
            ["Rename", "Move", "Uninstall", "Remove", "Cancel"]
        );
    }

    #[test]
    fn card_height_matches_rows_gaps_and_padding() {
        let app = test_app();
        let layout = card(&app, &app_menu(AppStep::Actions));
        assert_eq!(layout.entries.len(), 5);
        assert_eq!(layout.bounds.height, 356.0);
        assert_eq!(layout.title.height, 28.0);
        for entry in &layout.entries {
            assert_eq!(entry.bounds.height, ACTION_HEIGHT);
        }
    }

    #[test]
    fn material_cards_fit_narrow_windows_and_keyboard_insets() {
        for size in [
            iced::Size::new(280.0, 500.0),
            iced::Size::new(800.0, 360.0),
            iced::Size::new(100.0, 100.0),
        ] {
            let mut app = test_app();
            app.size = size;
            app.insets = iced_android::Insets {
                left: 8.0,
                right: 12.0,
                top: 10.0,
                bottom: 30.0,
            };
            app.snapshot.as_mut().unwrap().settings.label_size = 40;
            for overlay in [
                app_menu(AppStep::Actions),
                app_menu(AppStep::Move),
                app_menu(AppStep::Rename("Clock".into())),
            ] {
                let layout = card(&app, &overlay);
                assert!(layout.bounds.x >= 8.0);
                assert!(layout.bounds.y >= 10.0);
                assert!(layout.bounds.x + layout.bounds.width <= size.width - 12.0 + 0.01);
                assert!(layout.bounds.y + layout.bounds.height <= size.height - 30.0 + 0.01);
            }
        }
    }

    #[test]
    fn rename_card_replaces_actions_with_an_input() {
        let overlay = app_menu(AppStep::Rename("Calculator".into()));
        let app = test_app();
        let entries = entries(&app, &overlay);
        assert_eq!(entries.len(), 3);
        assert!(matches!(
            &entries[0].kind,
            EntryKind::Input { widget_id, .. } if *widget_id == APP_NAME_FIELD_ID
        ));
        assert!(matches!(
            &entries[1].kind,
            EntryKind::Action { label, message: Message::AppRenameSubmit } if label == "Save"
        ));
    }

    #[test]
    fn move_card_excludes_the_current_group() {
        let app = test_app();
        let overlay = app_menu(AppStep::Move);
        let current = match &overlay {
            Overlay::App(menu) => menu.item.group_id.clone(),
            Overlay::Launcher(_) => unreachable!(),
        };
        let rows = entries(&app, &overlay)
            .into_iter()
            .find_map(|entry| match entry.kind {
                EntryKind::List { rows } => Some(rows),
                _ => None,
            })
            .expect("the move card has a list");
        assert!(!rows.is_empty());
        assert!(!rows.iter().any(|row| row.key == current));
        assert!(!rows.iter().any(|row| row.key == "folder"));
    }

    #[test]
    fn shortcut_card_keeps_loading_and_empty_states() {
        let app = test_app();
        let loading = entries(
            &app,
            &Overlay::Launcher(LauncherMenu {
                step: LauncherStep::Shortcuts,
                providers: vec![],
                providers_loading: true,
                error: None,
            }),
        );
        assert!(matches!(
            &loading[0].kind,
            EntryKind::Text { error: false, .. }
        ));

        let empty = entries(
            &app,
            &Overlay::Launcher(LauncherMenu {
                step: LauncherStep::Shortcuts,
                providers: vec![],
                providers_loading: false,
                error: None,
            }),
        );
        assert!(matches!(
            &empty[0].kind,
            EntryKind::Text { error: false, .. }
        ));

        let loaded = entries(
            &app,
            &Overlay::Launcher(LauncherMenu {
                step: LauncherStep::Shortcuts,
                providers: vec![ShortcutProvider {
                    id: "pkg/.Provider".into(),
                    label: "Provider".into(),
                }],
                providers_loading: false,
                error: None,
            }),
        );
        assert!(matches!(&loaded[0].kind, EntryKind::List { .. }));
    }

    #[test]
    fn card_bounds_are_centred_and_inside_the_window() {
        let app = test_app();
        let layout = card(&app, &app_menu(AppStep::Actions));
        assert_eq!(layout.bounds.x, 50.0);
        assert_eq!(layout.bounds.y, 222.0);
        assert_eq!(layout.bounds.width, 320.0);
        assert_eq!(layout.bounds.height, 356.0);
        for entry in &layout.entries {
            assert_eq!(entry.bounds.x, 74.0);
            assert_eq!(entry.bounds.width, 272.0);
            assert!(entry.bounds.y + entry.bounds.height <= layout.bounds.y + layout.bounds.height);
        }
    }
}
