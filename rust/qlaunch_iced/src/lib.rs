//! Iced Android/desktop launcher UI. The core remains the source of truth.
mod accessibility;
mod components;
mod dialog;
mod elastic_scroll;
mod field;
mod stretch;
mod folder;
mod fonts;
pub mod gesture;
pub mod grid;
mod icons;
mod label;
mod metrics;
mod overlay;
pub mod pager;
mod placeholder;
mod probe;
mod reorder_list;
mod screens;
mod scroll_fade;
mod slots;
mod theme;
mod view;

use accessibility::{ActionRequest, Target};
use gesture::{Gesture, Outcome, Start};
use pager::Pager;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iced::futures::{SinkExt, executor::block_on};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::image;
use iced::{Color, Element, Point, Subscription, Task, Theme, window};
use iced_android::{self as platform, Insets};
use qlaunch_core::api::{api as core, events::CoreEvent};
use qlaunch_core::model::{AppRef, IconFormat, Id, Item};
use qlaunch_core::settings::LayoutStyle;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: platform::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("qlaunch-iced")
            .with_max_level(log::LevelFilter::Info),
    );
    platform::init(app);
    if let Err(error) = run() {
        log::error!("iced failed: {error}");
    }
}

/// How long a scrollbar stays visible after the last scroll movement.
const SCROLLBAR_IDLE: Duration = Duration::from_millis(800);

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::accessibility::{Action, Target};
    use qlaunch_core::{model::Model, settings::Settings};

    #[test]
    fn back_order_closes_select_then_dialog_then_screen() {
        let mut app = app();
        app.screens.push(Screen::Settings);
        app.dialog = Some(dialog::Dialog::Launcher);
        app.select = Some(dialog::Select {
            field: dialog::SelectField::AppGroup,
            options: vec![],
            selected: "apps".into(),
        });
        let _ = app.update(Message::Back);
        assert!(app.select.is_none() && app.dialog.is_some() && !app.screens.is_empty());
        let _ = app.update(Message::Back);
        assert!(app.dialog.is_none() && !app.screens.is_empty());
        let _ = app.update(Message::Back);
        assert!(app.screens.is_empty());
    }

    #[test]
    fn home_clears_dialog_screens_and_select() {
        let mut app = app();
        app.screens.push(Screen::Groups);
        app.dialog = Some(dialog::Dialog::Launcher);
        let _ = app.update(Message::Event(CoreEvent::HomePressed));
        assert!(app.dialog.is_none() && app.screens.is_empty());
    }

    #[test]
    fn blank_press_is_ignored_with_a_screen_open() {
        let mut app = app();
        app.screens.push(Screen::Settings);
        let _ = app.update(Message::BlankPressed);
        assert!(app.pending_target.is_none());
    }

    #[test]
    fn search_gear_opens_settings() {
        let mut app = app();
        app.searching = true;
        let _ = app.update(Message::Push(Screen::Settings));
        assert_eq!(app.screens.last(), Some(&Screen::Settings));
        assert!(app.searching);
    }

    #[test]
    fn role_completion_releases_busy_and_uses_the_saved_role() {
        let mut app = app();
        let snapshot = app.snapshot.clone().unwrap();
        let group = snapshot
            .groups
            .iter()
            .find(|g| g.role == Some(qlaunch_core::model::GroupRole::Home))
            .unwrap();
        let mut editor = dialog::GroupEditorState::new(group.id.clone(), group.label.clone(), None);
        editor.busy = true;
        app.dialog = Some(dialog::Dialog::GroupEditor(editor));
        let _ = app.update(Message::ModelDone {
            result: Ok(snapshot),
            close: false,
        });
        let Some(dialog::Dialog::GroupEditor(editor)) = &app.dialog else {
            panic!()
        };
        assert!(!editor.busy);
        assert_eq!(editor.role, Some(qlaunch_core::model::GroupRole::Home));
    }

    #[test]
    fn failed_screen_mutation_preserves_error_in_snackbar() {
        let mut app = app();
        app.screens.push(Screen::Groups);
        let _ = app.update(Message::ModelDone {
            result: Err("disk is full".into()),
            close: false,
        });
        assert_eq!(
            app.snackbar.as_ref().map(|(_, s)| s.as_str()),
            Some("disk is full")
        );
    }

    #[test]
    fn accessibility_long_click_on_blank_opens_the_dialog_not_the_overlay() {
        let mut app = app();
        publish(&mut app);
        let request = request(&app, "grid:container", Action::LongClick);
        let _ = app.update(Message::Accessibility(request));
        assert!(matches!(app.dialog, Some(dialog::Dialog::Launcher)));
        assert!(app.overlay.is_none());
    }

    #[test]
    fn step_is_ignored_while_a_screen_is_open() {
        let mut app = app();
        let before = app.pager.committed();
        app.screens.push(Screen::Settings);
        let _ = app.update(Message::Step(1));
        assert_eq!(app.pager.committed(), before);
    }

    #[test]
    fn late_backup_list_does_not_open_over_another_screen() {
        let mut app = app();
        let _ = app.update(Message::Push(Screen::Settings));
        let _ = app.update(Message::BackupsListed(Ok(vec!["a.json".into()])));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn snackbar_text_is_announced() {
        let mut app = app();
        let _ = app.update(Message::Snackbar("Backup saved.".into()));
        assert_eq!(accessibility::announcement(&app), "Backup saved.");
    }

    #[test]
    fn confirmation_ignores_a_second_activation_while_busy() {
        let mut app = app();
        app.dialog = Some(dialog::Dialog::Confirm(dialog::ConfirmState {
            title: "Factory Reset".into(),
            body: "Wipe?".into(),
            confirm_label: "Reset".into(),
            destructive: true,
            busy: false,
            action: dialog::ConfirmAction::FactoryReset,
        }));
        let _ = app.dialog_message(dialog::Message::ConfirmYes);
        let Some(dialog::Dialog::Confirm(state)) = &app.dialog else {
            panic!("confirmation should stay open while the request runs")
        };
        assert!(state.busy);
        assert_eq!(app.dialog_message(dialog::Message::ConfirmYes).units(), 0);
    }

    #[test]
    fn late_completion_does_not_close_another_dialog() {
        let mut app = app();
        app.dialog = Some(dialog::Dialog::Launcher);
        let generation = app.dialog_generation;
        let snapshot = app.snapshot.clone().unwrap();
        let _ = app.update(Message::Back);
        let _ = app.update(Message::GroupAdd);
        let _ = app.update(Message::ScopedDialog(
            generation,
            Box::new(Message::ModelDone {
                result: Ok(snapshot),
                close: true,
            }),
        ));
        assert!(matches!(app.dialog, Some(dialog::Dialog::AddGroup(_))));
    }

    #[test]
    fn cancelling_delete_confirmation_restores_group_editor() {
        let mut app = app();
        app.dialog = Some(dialog::Dialog::GroupEditor(dialog::GroupEditorState::new(
            "fun".into(),
            "Fun".into(),
            None,
        )));
        let _ = app.update(Message::Dialog(dialog::Message::GroupDelete));
        assert!(matches!(app.dialog, Some(dialog::Dialog::Confirm(_))));
        let _ = app.update(Message::Dialog(dialog::Message::Close));
        assert!(matches!(app.dialog, Some(dialog::Dialog::GroupEditor(_))));
    }

    #[test]
    fn busy_app_remove_and_shortcut_actions_ignore_repeat_activation() {
        let mut app = app();
        let mut editor = dialog::AppEditor::new(item(&app, "busy"));
        editor.busy = true;
        app.dialog = Some(dialog::Dialog::App(editor));
        assert_eq!(app.dialog_message(dialog::Message::AppRemove).units(), 0);
        assert_eq!(app.dialog_message(dialog::Message::AppUninstall).units(), 0);
        let mut shortcuts = dialog::AddShortcutState::new("apps".into());
        shortcuts.busy = true;
        app.dialog = Some(dialog::Dialog::AddShortcut(shortcuts));
        assert_eq!(app.dialog_message(dialog::Message::ShortcutLock).units(), 0);
    }

    #[test]
    fn snackbar_expires_only_for_its_own_generation() {
        let mut app = app();
        let _ = app.update(Message::Snackbar("hello".into()));
        let (generation, _) = app.snackbar.clone().unwrap();
        let _ = app.update(Message::SnackbarExpired(generation + 1));
        assert!(app.snackbar.is_some());
        let _ = app.update(Message::SnackbarExpired(generation));
        assert!(app.snackbar.is_none());
    }

    #[test]
    fn choosing_apps_suspends_editor_until_screen_is_popped() {
        let mut app = app();
        app.screens.push(Screen::Groups);
        app.dialog = Some(dialog::Dialog::GroupEditor(dialog::GroupEditorState::new(
            "games".into(),
            "Games".into(),
            None,
        )));
        let _ = app.update(Message::Dialog(dialog::Message::GroupChooseApps));
        assert_eq!(app.screens.last(), Some(&Screen::GroupApps("games".into())));
        assert!(app.dialog.is_none());
        let _ = app.update(Message::Back);
        assert_eq!(app.screens.last(), Some(&Screen::Groups));
        assert!(matches!(app.dialog, Some(dialog::Dialog::GroupEditor(_))));
    }

    #[test]
    fn home_from_screen_without_dialog_returns_home_not_search() {
        let mut app = app();
        app.screens.push(Screen::Settings);
        let _ = app.update(Message::Event(CoreEvent::HomePressed));
        assert!(app.screens.is_empty());
        assert!(!app.searching);
    }

    #[test]
    fn expired_snackbar_cannot_dismiss_a_later_message() {
        let mut app = app();
        let _ = app.update(Message::Snackbar("first".into()));
        let first = app.snackbar.as_ref().unwrap().0;
        let _ = app.update(Message::SnackbarExpired(first));
        let _ = app.update(Message::Snackbar("second".into()));
        let _ = app.update(Message::SnackbarExpired(first));
        assert_eq!(app.snackbar.as_ref().unwrap().1, "second");
    }

    pub(crate) fn app() -> App {
        let (mut app, _) = App::new();
        let model = Model::new_default();
        app.snapshot = Some(core::ModelSnapshot {
            root_group_id: model.root_group_id.clone(),
            home_group_id: model.home_group_id(),
            page_ids: model.pages().iter().map(|g| g.id.clone()).collect(),
            groups: model.groups,
            items: model.items,
            settings: Settings::default(),
            warnings: vec![],
            icon_revisions: vec![],
            folder_icons: vec![],
        });
        app.reindex_items();
        let home = app.home_index();
        let count = app.snapshot.as_ref().unwrap().page_ids.len();
        app.pager = Pager::new(count, home);
        app
    }

    fn settle(app: &mut App) {
        let _ = app.update(Message::Frame(Instant::now() + Duration::from_secs(10)));
    }

    #[test]
    fn paging_wraps_and_home_uses_the_core_role() {
        let mut app = app();
        assert_eq!(app.pager.committed(), 1);
        app.step(-1);
        settle(&mut app);
        assert_eq!(app.pager.committed(), 0);
        app.step(-1);
        settle(&mut app);
        assert_eq!(app.pager.committed(), 2);
        let _ = app.update(Message::Event(CoreEvent::HomePressed));
        settle(&mut app);
        assert_eq!(app.pager.committed(), 1);
        assert!(!app.searching);
    }

    #[test]
    fn home_opens_then_closes_search_and_back_is_not_exit() {
        let mut app = app();
        let _ = app.update(Message::Event(CoreEvent::HomePressed));
        assert!(app.searching);
        let _ = app.update(Message::Event(CoreEvent::HomePressed));
        assert!(!app.searching);
        let _ = app.update(Message::Back);
        assert!(app.window.is_some());
    }

    #[test]
    fn overlay_home_never_opens_search() {
        let mut app = app();
        app.pager.set_count(3, 0);
        let _ = app.update(Message::Event(CoreEvent::HomePressedFromOverlay));
        settle(&mut app);
        assert_eq!(app.pager.committed(), 1);
        assert!(!app.searching);
    }

    #[test]
    fn stale_search_results_do_not_replace_the_current_query() {
        let mut app = app();
        app.query = "calendar".into();
        let _ = app.update(Message::Results(
            "calc".into(),
            vec![AppRef {
                package: "wrong".into(),
                activity: "".into(),
            }],
        ));
        assert!(app.results.is_empty());
    }

    #[test]
    fn search_items_follow_the_result_order_and_skip_unknown_apps() {
        use qlaunch_core::model::{AppRef, Item};
        let mut app = app();
        let snapshot = app.snapshot.as_mut().unwrap();
        let group = snapshot.page_ids[0].clone();
        let item = |id: &str, package: &str| Item {
            id: id.into(),
            group_id: group.clone(),
            label: id.into(),
            label_overridden: false,
            app: Some(AppRef {
                package: package.into(),
                activity: String::new(),
            }),
            shortcut: None,
            uninstalled: false,
            order: 0,
            icon_override: None,
            folder_group_id: None,
        };
        snapshot.items = vec![item("a", "one"), item("b", "two"), item("c", "three")];
        app.reindex_items();
        app.searching = true;
        app.query = "t".into();
        app.results = vec![
            AppRef {
                package: "three".into(),
                activity: String::new(),
            },
            AppRef {
                package: "missing".into(),
                activity: String::new(),
            },
            AppRef {
                package: "one".into(),
                activity: String::new(),
            },
        ];
        let ids: Vec<_> = app
            .visible_items()
            .into_iter()
            .map(|item| item.id.clone())
            .collect();
        // Results order wins; a result with no item in the model is skipped.
        assert_eq!(ids, ["c", "a"]);
    }

    #[test]
    fn scroll_offsets_are_tracked_per_region() {
        let mut app = app();
        let _ = app.update(Message::Scrolled(Region::Group("g".into()), 42.0));
        let _ = app.update(Message::Scrolled(Region::Search, 12.0));
        let _ = app.update(Message::Scrolled(Region::OverlayCard, 7.0));
        assert_eq!(app.scroll.get(&Region::Group("g".into())), Some(&42.0));
        assert_eq!(app.scroll.get(&Region::Search), Some(&12.0));
        assert_eq!(app.scroll.get(&Region::OverlayCard), Some(&7.0));
    }

    #[test]
    fn scrollbars_are_visible_only_while_scrolling() {
        let mut app = app();
        assert!(!app.scrollbar_visible_at(Instant::now()));

        let _ = app.update(Message::Scrolled(Region::Group("apps".into()), 12.0));
        let last = app.scroll_last.expect("scrolling marks activity");
        assert!(app.scrollbar_visible_at(last));
        assert!(app.scrollbar_visible_at(last + SCROLLBAR_IDLE - Duration::from_millis(1)));
        assert!(!app.scrollbar_visible_at(last + SCROLLBAR_IDLE));

        let _ = app.update(Message::ScrollTick);
        assert!(
            app.scroll_last.is_some(),
            "a tick before the idle delay keeps the bar"
        );

        app.scroll_last = Some(Instant::now() - SCROLLBAR_IDLE - Duration::from_millis(1));
        let _ = app.update(Message::ScrollTick);
        assert!(app.scroll_last.is_none());
        assert_eq!(app.scroll.get(&Region::Group("apps".into())), Some(&12.0));
    }

    #[test]
    fn starting_a_page_swipe_hides_the_scrollbar() {
        let mut app = app();
        let region = Region::Group("apps".into());
        let _ = app.update(Message::Scrolled(region.clone(), 12.0));
        assert!(app.scrollbars_visible());

        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 300.0),
        }));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerMoved {
            id: iced::touch::Finger(0),
            position: Point::new(200.0, 300.0),
        }));
        assert!(app.pager.is_dragging());
        assert!(!app.scrollbars_visible());
        assert_eq!(app.scroll.get(&region), Some(&12.0));

        // A pending scroll notification must not bring it back during paging.
        let _ = app.update(Message::Scrolled(region, 24.0));
        assert!(!app.scrollbars_visible());
    }

    #[test]
    fn page_button_hides_the_scrollbar_until_vertical_scrolling_resumes() {
        let mut app = app();
        let region = Region::Group("apps".into());
        let _ = app.update(Message::Scrolled(region.clone(), 12.0));
        assert!(app.scrollbars_visible());
        let _ = app.update(Message::Step(1));
        assert!(app.pager.is_animating());
        assert!(!app.scrollbars_visible());
        settle(&mut app);
        assert!(!app.scrollbars_visible());

        let _ = app.update(Message::Scrolled(region, 48.0));
        assert!(app.scrollbars_visible());
    }

    #[test]
    fn horizontal_flick_does_not_flash_a_hidden_scrollbar_before_drag_start() {
        let mut app = app();
        let region = Region::Group(app.current_group_id().unwrap().clone());
        app.scroll.insert(region.clone(), 100.0);
        assert!(!app.scrollbars_visible());
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 300.0),
        }));
        // The scroll widget may notify before the raw move reaches the app.
        let _ = app.update(Message::Scrolled(region.clone(), 99.7));
        assert!(!app.scrollbars_visible());
        let _ = app.update(Message::Touch(iced::touch::Event::FingerMoved {
            id: iced::touch::Finger(0),
            position: Point::new(290.0, 300.5),
        }));
        let _ = app.update(Message::Scrolled(region.clone(), 99.2));
        assert!(!app.scrollbars_visible());
        assert_eq!(app.scroll.get(&region), Some(&99.2));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerMoved {
            id: iced::touch::Finger(0),
            position: Point::new(200.0, 302.0),
        }));
        assert!(app.pager.is_dragging());
        assert!(!app.scrollbars_visible());
        let _ = app.update(Message::Touch(iced::touch::Event::FingerLifted {
            id: iced::touch::Finger(0),
            position: Point::new(200.0, 302.0),
        }));
        settle(&mut app);
        assert!(!app.scrollbars_visible());
    }

    #[test]
    fn vertical_scroll_shows_the_bar_only_after_the_gesture_leaves_touch_slop() {
        let mut app = app();
        let region = Region::Group(app.current_group_id().unwrap().clone());
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 300.0),
        }));
        let _ = app.update(Message::Scrolled(region.clone(), 2.0));
        assert!(!app.scrollbars_visible());
        let _ = app.update(Message::Touch(iced::touch::Event::FingerMoved {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 270.0),
        }));
        let _ = app.update(Message::Scrolled(region, 30.0));
        assert!(app.scrollbars_visible());
        assert!(!app.pager.is_dragging());
    }

    #[test]
    fn layout_notifications_do_not_flash_the_scrollbar() {
        let mut app = app();
        let _ = app.update(Message::Scrolled(Region::Group("apps".into()), 0.0));
        assert!(app.scroll_last.is_none());
        let _ = app.update(Message::Scrolled(Region::Group("apps".into()), 0.0));
        assert!(app.scroll_last.is_none());
        let _ = app.update(Message::Scrolled(Region::Group("apps".into()), 24.0));
        assert!(app.scroll_last.is_some());
    }

    #[test]
    fn search_results_report_their_scroll_offset() {
        let mut app = app();
        let _ = app.update(Message::Scrolled(Region::Search, 42.0));
        assert_eq!(app.scroll.get(&Region::Search), Some(&42.0));
    }

    #[test]
    fn overlay_list_reports_its_scroll_offset() {
        let mut app = app();
        let _ = app.update(Message::Scrolled(Region::OverlayCard, 42.0));
        assert_eq!(app.scroll.get(&Region::OverlayCard), Some(&42.0));
    }

    #[test]
    fn open_search_and_query_reset_the_search_offset() {
        let mut app = app();
        app.scroll.insert(Region::Search, 42.0);
        let _ = app.open_search();
        assert_eq!(app.scroll.get(&Region::Search), Some(&0.0));
        app.scroll.insert(Region::Search, 42.0);
        let _ = app.update(Message::Query("calc".into()));
        assert_eq!(app.scroll.get(&Region::Search), Some(&0.0));
    }

    #[test]
    fn switching_history_resets_the_search_offset() {
        let mut app = app();
        app.scroll.insert(Region::Search, 42.0);
        let _ = app.update(Message::History(true));
        assert_eq!(app.scroll.get(&Region::Search), Some(&0.0));
    }

    #[test]
    fn step_animates_then_commits() {
        let mut app = app();
        assert_eq!(app.pager.committed(), 1);
        let _ = app.update(Message::Step(1));
        assert_eq!(app.pager.committed(), 1);
        assert!(app.pager.is_animating());
        settle(&mut app);
        assert_eq!(app.pager.committed(), 2);
        assert!(!app.pager.is_animating());
    }

    #[test]
    fn home_from_a_neighbour_animates() {
        let mut app = app();
        app.pager.set_count(3, 2);
        let _ = app.update(Message::Event(CoreEvent::HomePressed));
        assert_eq!(app.pager.committed(), 2);
        assert!(app.pager.is_animating());
        settle(&mut app);
        assert_eq!(app.pager.committed(), 1);
    }

    #[test]
    fn home_from_a_far_page_is_immediate() {
        let mut app = app();
        app.snapshot.as_mut().unwrap().page_ids.push("extra".into());
        app.pager.set_count(4, 3);
        let _ = app.update(Message::Event(CoreEvent::HomePressed));
        assert_eq!(app.pager.committed(), 1);
        assert!(!app.pager.is_animating());
    }

    #[test]
    fn resize_mid_animation_keeps_the_target() {
        let mut app = app();
        let _ = app.update(Message::Step(1));
        let _ = app.update(Message::Resized(iced::Size::new(1080.0, 2400.0)));
        assert!(app.pager.is_animating());
        settle(&mut app);
        assert_eq!(app.pager.committed(), 2);
    }

    pub(crate) fn item(app: &App, id: &str) -> Item {
        Item {
            id: id.into(),
            group_id: app.snapshot.as_ref().unwrap().page_ids[0].clone(),
            label: id.into(),
            label_overridden: false,
            app: Some(AppRef {
                package: id.into(),
                activity: String::new(),
            }),
            shortcut: None,
            uninstalled: false,
            order: 0,
            icon_override: None,
            folder_group_id: None,
        }
    }

    fn app_overlay(app: &App, step: overlay::AppStep) -> overlay::Overlay {
        overlay::Overlay::App(overlay::AppMenu {
            item: item(app, "calculator"),
            step,
            error: None,
        })
    }

    #[test]
    fn back_closes_overlay_before_search() {
        let mut app = app();
        app.searching = true;
        app.overlay = Some(app_overlay(&app, overlay::AppStep::Actions));
        let _ = app.update(Message::Back);
        assert!(app.overlay.is_none());
        assert!(app.searching, "Back should only close the overlay");
    }

    #[test]
    fn empty_rename_keeps_menu_open_with_error() {
        let mut app = app();
        app.overlay = Some(app_overlay(&app, overlay::AppStep::Rename("   ".into())));
        let _ = app.update(Message::Overlay(overlay::Message::AppRenameSubmit));
        let Some(overlay::Overlay::App(menu)) = &app.overlay else {
            panic!("the menu should stay open");
        };
        assert_eq!(menu.error.as_deref(), Some("Name cannot be empty"));
        assert!(matches!(menu.step, overlay::AppStep::Rename(_)));
    }

    #[test]
    fn overlay_home_goes_home_and_closes() {
        let mut app = app();
        app.pager.set_count(3, 0);
        app.searching = true;
        app.overlay = Some(overlay::Overlay::Launcher(overlay::LauncherMenu {
            step: overlay::LauncherStep::Actions,
            providers: vec![],
            providers_loading: false,
            error: None,
        }));
        let _ = app.update(Message::Event(CoreEvent::HomePressed));
        settle(&mut app);
        assert!(app.overlay.is_none());
        assert_eq!(app.pager.committed(), 1);
        assert!(!app.searching);
    }

    #[test]
    fn home_from_overlay_also_clears_the_overlay() {
        let mut app = app();
        app.pager.set_count(3, 0);
        app.overlay = Some(app_overlay(&app, overlay::AppStep::Actions));
        let _ = app.update(Message::Event(CoreEvent::HomePressedFromOverlay));
        settle(&mut app);
        assert!(app.overlay.is_none());
        assert_eq!(app.pager.committed(), 1);
    }

    #[test]
    fn page_group_is_the_committed_page_even_inside_a_folder() {
        let mut app = app();
        app.pager.set_count(3, 1);
        app.folder = Some("folder".into());
        assert_eq!(
            app.page_group_id(),
            Some(app.snapshot.as_ref().unwrap().page_ids[1].clone())
        );
    }

    pub(crate) fn folder_item(app: &App) -> Item {
        let mut item = item(app, "games-folder");
        item.folder_group_id = Some("folder-group".into());
        item
    }

    #[test]
    fn overlay_blocks_touch_and_paging() {
        let mut app = app();
        app.pager.set_count(3, 0);
        app.overlay = Some(app_overlay(&app, overlay::AppStep::Actions));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 300.0),
        }));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerMoved {
            id: iced::touch::Finger(0),
            position: Point::new(40.0, 300.0),
        }));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerLifted {
            id: iced::touch::Finger(0),
            position: Point::new(40.0, 300.0),
        }));
        settle(&mut app);
        assert_eq!(app.pager.committed(), 0);
        assert!(app.overlay.is_some());
    }

    #[test]
    fn losing_a_finger_mid_drag_settles_back() {
        let mut app = app();
        app.pager.set_count(3, 0);
        let gesture = item(&app, "calculator");
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 300.0),
        }));
        let _ = app.update(Message::ItemPressed(gesture));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerMoved {
            id: iced::touch::Finger(0),
            position: Point::new(100.0, 300.0),
        }));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerMoved {
            id: iced::touch::Finger(0),
            position: Point::new(60.0, 300.0),
        }));
        assert!(app.pager.is_dragging());
        let _ = app.update(Message::Touch(iced::touch::Event::FingerLost {
            id: iced::touch::Finger(0),
            position: Point::new(60.0, 300.0),
        }));
        settle(&mut app);
        assert_eq!(app.pager.committed(), 0);
        assert!((app.pager.offset() - 0.0).abs() < 1e-4);
        assert!(!app.pager.is_dragging());
    }

    #[test]
    fn tapping_a_folder_opens_it() {
        let mut app = app();
        let folder = folder_item(&app);
        app.snapshot.as_mut().unwrap().items.push(folder.clone());
        let _ = app.apply_gesture(Outcome::Tap(Start::Item(folder.id.clone())));
        assert_eq!(app.folder, Some("folder-group".into()));
        assert!(app.overlay.is_none());
    }

    #[test]
    fn tapping_an_app_does_not_open_a_folder_or_menu() {
        let mut app = app();
        let app_item = item(&app, "calculator");
        app.snapshot.as_mut().unwrap().items.push(app_item.clone());
        let _ = app.apply_gesture(Outcome::Tap(Start::Item(app_item.id.clone())));
        assert!(app.folder.is_none());
        assert!(app.overlay.is_none());
    }

    #[test]
    fn a_press_below_the_title_strip_is_blank() {
        let mut app = app();
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 300.0),
        }));
        assert_eq!(app.gesture.target(), Some(&Start::Blank));
    }

    #[test]
    fn a_press_on_the_title_strip_has_no_target() {
        let mut app = app();
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 10.0),
        }));
        assert_eq!(app.gesture.target(), None);
    }

    #[test]
    fn a_press_on_the_folder_header_has_no_target() {
        let mut app = app();
        app.folder = Some("folder".into());
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(60.0, 10.0),
        }));
        assert_eq!(app.gesture.target(), None);
    }

    #[test]
    fn folder_blocks_background_paging_and_item_messages() {
        let mut app = app();
        app.folder = Some("folder-group".into());
        let page = app.pager.committed();
        let _ = app.update(Message::Step(1));
        assert_eq!(app.folder.as_deref(), Some("folder-group"));
        assert!(!app.pager.is_animating());
        let _ = app.apply_gesture(Outcome::DragStart);
        assert!(!app.pager.is_dragging());
        assert_eq!(app.pager.committed(), page);
        let background = item(&app, "background");
        let _ = app.update(Message::ItemPressed(background));
        assert!(app.pressed.is_none());
    }

    #[test]
    fn folder_background_does_not_become_blank_grid() {
        let mut app = app();
        app.folder = Some("folder-group".into());
        let _ = app.begin_gesture(
            iced::touch::Finger(0),
            Point::new(5.0, 300.0),
            Instant::now(),
        );
        assert!(app.gesture.target().is_none());
    }

    #[test]
    fn folder_dismiss_never_closes_a_nested_app_menu() {
        let mut app = app();
        app.folder = Some("folder-group".into());
        let mut member = item(&app, "member");
        member.group_id = "folder-group".into();
        app.snapshot.as_mut().unwrap().items.push(member.clone());
        let _ = app.apply_gesture(Outcome::LongPress(Start::Item(member.id)));
        assert!(app.dialog.is_some());
        let _ = app.update(Message::FolderDismiss);
        assert!(app.dialog.is_some());
        assert!(app.folder.is_some());
        let _ = app.update(Message::Back);
        assert!(app.dialog.is_none());
        assert!(app.folder.is_some());
        let _ = app.update(Message::FolderDismiss);
        assert!(app.folder.is_none());
    }

    #[test]
    fn menu_navigation_resets_card_scroll_before_showing_a_new_step() {
        let mut app = app();
        app.open_app_menu(item(&app, "Calculator"));
        app.scroll.insert(Region::OverlayCard, 150.0);
        let _ = app.update(Message::Overlay(overlay::Message::AppMove));
        assert_eq!(app.scroll.get(&Region::OverlayCard), Some(&0.0));
        app.scroll.insert(Region::OverlayCard, 100.0);
        let _ = app.update(Message::Back);
        assert_eq!(app.scroll.get(&Region::OverlayCard), Some(&0.0));
    }

    #[test]
    fn folder_semantics_dismiss_closes_only_the_folder() {
        let mut app = app();
        app.folder = Some("folder-group".into());
        publish(&mut app);
        let request = request(&app, "grid:container", Action::Dismiss);
        let _ = app.update(Message::Accessibility(request));
        assert!(app.folder.is_none());
        assert!(!app.searching);
    }

    #[test]
    fn folder_scrim_consumes_the_dismissal_pointer_until_release() {
        let mut app = app();
        app.folder = Some("folder-group".into());
        let finger = iced::touch::Finger(0);
        let _ = app.update(Message::FolderDismiss);
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: finger,
            position: Point::new(5.0, 300.0),
        }));
        assert_eq!(app.gesture.phase(), gesture::Phase::Idle);
        let _ = app.update(Message::Touch(iced::touch::Event::FingerMoved {
            id: finger,
            position: Point::new(300.0, 300.0),
        }));
        assert!(!app.pager.is_dragging());
        let _ = app.update(Message::Touch(iced::touch::Event::FingerLifted {
            id: finger,
            position: Point::new(300.0, 300.0),
        }));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: finger,
            position: Point::new(5.0, 300.0),
        }));
        assert_eq!(app.gesture.target(), Some(&Start::Blank));
    }

    fn widget_states(tree: &iced::advanced::widget::Tree) -> Vec<usize> {
        let mut states = Vec::new();
        if let iced::advanced::widget::tree::State::Some(state) = &tree.state {
            states.push(state.as_ref() as *const dyn std::any::Any as *const () as usize);
        }
        for child in &tree.children {
            states.extend(widget_states(child));
        }
        states
    }

    #[test]
    fn opening_folder_preserves_launcher_widget_state() {
        use iced::advanced::widget::Tree;
        let mut app = app();
        let mut tree = Tree::new(view::launcher(&app).as_widget());
        let launcher_states = widget_states(&tree);
        assert!(!launcher_states.is_empty());
        app.folder = Some("folder-group".into());
        tree.diff(view::launcher(&app).as_widget());
        let after = widget_states(&tree);
        assert!(
            launcher_states.iter().all(|state| after.contains(state)),
            "opening a folder recreated launcher widget state, including scroll position"
        );
    }

    #[test]
    fn member_menu_preserves_folder_widget_state() {
        use iced::advanced::widget::Tree;
        let mut app = app();
        app.folder = Some("folder-group".into());
        let mut member = item(&app, "member");
        member.group_id = "folder-group".into();
        app.snapshot.as_mut().unwrap().items.push(member.clone());
        let mut tree = Tree::new(view::launcher(&app).as_widget());
        let folder_states = widget_states(&tree);
        app.open_app_menu(member);
        tree.diff(view::launcher(&app).as_widget());
        let after = widget_states(&tree);
        assert!(
            folder_states.iter().all(|state| after.contains(state)),
            "opening a member menu recreated the folder scrollable"
        );
        let _ = app.update(Message::Back);
        tree.diff(view::launcher(&app).as_widget());
        let after = widget_states(&tree);
        assert!(folder_states.iter().all(|state| after.contains(state)));
    }

    #[test]
    fn gesture_body_top_still_ignores_search() {
        let mut app = app();
        app.searching = true;
        assert_eq!(app.body_top(), f32::INFINITY);
    }

    #[test]
    fn pull_down_from_blank_at_the_top_reaches_the_classifier() {
        let mut app = app();
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 500.0),
        }));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerMoved {
            id: iced::touch::Finger(0),
            position: Point::new(300.0, 600.0),
        }));
        assert_eq!(app.gesture.phase(), gesture::Phase::Pulled);
    }

    #[test]
    fn a_press_on_chrome_never_locks_or_pages() {
        let mut app = app();
        app.pager.set_count(3, 0);
        let _ = app.update(Message::Touch(iced::touch::Event::FingerPressed {
            id: iced::touch::Finger(0),
            position: Point::new(200.0, 12.0),
        }));
        let _ = app.update(Message::Touch(iced::touch::Event::FingerLifted {
            id: iced::touch::Finger(0),
            position: Point::new(200.0, 12.0),
        }));
        settle(&mut app);
        assert_eq!(app.pager.committed(), 0);
        assert!(app.overlay.is_none());
        assert_eq!(app.gesture.phase(), gesture::Phase::Idle);
    }

    #[test]
    fn mouse_events_use_the_same_classifier() {
        use iced::mouse::{Button, Event as MouseEvent};
        let mut app = app();
        let folder = folder_item(&app);
        app.snapshot.as_mut().unwrap().items.push(folder.clone());
        let _ = app.update(Message::Mouse(MouseEvent::CursorMoved {
            position: Point::new(80.0, 90.0),
        }));
        let _ = app.update(Message::Mouse(MouseEvent::ButtonPressed(Button::Left)));
        let _ = app.update(Message::ItemPressed(folder));
        let _ = app.update(Message::Mouse(MouseEvent::ButtonReleased(Button::Left)));
        assert_eq!(app.folder, Some("folder-group".into()));
    }

    #[test]
    fn long_press_opens_the_app_editor() {
        let mut app = app();
        let item = item(&app, "calculator");
        app.snapshot.as_mut().unwrap().items.push(item.clone());
        let _ = app.apply_gesture(Outcome::LongPress(Start::Item(item.id.clone())));
        assert!(matches!(app.dialog, Some(dialog::Dialog::App(_))));
    }

    #[test]
    fn long_press_on_folder_tile_opens_folder_editor() {
        let mut app = app();
        let folder = folder_item(&app);
        app.snapshot.as_mut().unwrap().items.push(folder.clone());
        let _ = app.apply_gesture(Outcome::LongPress(Start::Item(folder.id.clone())));
        assert!(matches!(
            app.dialog,
            Some(dialog::Dialog::Folder(dialog::FolderEditor::Edit { .. }))
        ));
    }

    #[test]
    fn blank_long_press_opens_the_launcher_dialog() {
        let mut app = app();
        let _ = app.apply_gesture(Outcome::LongPress(Start::Blank));
        assert!(matches!(app.dialog, Some(dialog::Dialog::Launcher)));
    }

    #[test]
    fn group_delete_opens_a_confirmation() {
        let mut app = app();
        app.dialog = Some(dialog::Dialog::GroupEditor(dialog::GroupEditorState::new(
            "fun".into(),
            "Fun".into(),
            None,
        )));
        let _ = app.update(Message::Dialog(dialog::Message::GroupDelete));
        assert!(matches!(app.dialog, Some(dialog::Dialog::Confirm(_))));
    }

    #[test]
    fn back_closes_the_dialog_before_search() {
        let mut app = app();
        app.searching = true;
        app.dialog = Some(dialog::Dialog::Launcher);
        let _ = app.update(Message::Back);
        assert!(app.dialog.is_none());
        assert!(app.searching, "Back should only close the dialog");
    }

    #[test]
    fn long_press_opens_the_app_editor_and_clears_the_pressed_cell() {
        let mut app = app();
        let app_item = item(&app, "calculator");
        app.snapshot.as_mut().unwrap().items.push(app_item.clone());
        app.pressed = Some(app_item.id.clone());
        let _ = app.apply_gesture(Outcome::LongPress(Start::Item(app_item.id.clone())));
        assert!(matches!(app.dialog, Some(dialog::Dialog::App(_))));
        assert!(app.pressed.is_none());
    }

    #[test]
    fn snapshot_reload_keeps_the_current_group() {
        let mut app = app();
        app.pager.set_count(3, 0);
        let current = app.snapshot.as_ref().unwrap().page_ids[0].clone();
        let mut snapshot = app.snapshot.clone().unwrap();
        let id = snapshot.page_ids.remove(0);
        snapshot.page_ids.push(id);
        let _ = app.update(Message::Loaded(Ok(snapshot)));
        assert_eq!(app.pager.committed(), 2);
        assert_eq!(app.snapshot.as_ref().unwrap().page_ids[2], current);
    }

    fn page_keys(app: &App) -> Vec<String> {
        crate::view::page_slot_plan(app)
            .iter()
            .map(|plan| plan.key.clone())
            .collect()
    }

    fn label_of(app: &App, group: &Id) -> String {
        app.snapshot
            .as_ref()
            .unwrap()
            .groups
            .iter()
            .find(|candidate| &candidate.id == group)
            .map(|candidate| candidate.label.clone())
            .unwrap_or_default()
    }

    #[test]
    fn page_and_title_slot_keys_follow_wrapped_groups() {
        let mut app = app();
        let page_ids = app.snapshot.as_ref().unwrap().page_ids.clone();
        // A middle page shows its neighbours in order.
        app.pager.set_count(page_ids.len(), 1);
        assert_eq!(
            page_keys(&app),
            vec![
                format!("page:{}", page_ids[0]),
                format!("page:{}", page_ids[1]),
                format!("page:{}", page_ids[2]),
            ],
        );

        // Page 0 wraps: the left slot is the last group, not an empty one.
        app.pager.set_count(page_ids.len(), 0);
        assert_eq!(
            page_keys(&app),
            vec![
                format!("page:{}", page_ids[2]),
                format!("page:{}", page_ids[0]),
                format!("page:{}", page_ids[1]),
            ],
        );
        let titles = crate::view::title_slot_plan(&app);
        assert_eq!(titles[0].key, format!("title:{}:0", page_ids[2]));
        assert_eq!(titles[0].label, label_of(&app, &page_ids[2]));
        assert_eq!(titles[1].key, format!("title:{}:1", page_ids[0]));
        assert_eq!(titles[1].label, label_of(&app, &page_ids[0]));
        assert_eq!(titles[2].key, format!("title:{}:2", page_ids[1]));
        assert_eq!(titles[2].label, label_of(&app, &page_ids[1]));

        // Fewer groups than slots leaves the empty slots keyed by position.
        app.snapshot.as_mut().unwrap().page_ids.truncate(1);
        app.pager.set_count(3, 0);
        assert_eq!(
            page_keys(&app),
            vec![
                "page:empty:0".to_string(),
                format!("page:{}", page_ids[0]),
                "page:empty:2".to_string(),
            ],
        );
    }

    #[test]
    fn only_the_centre_page_is_scrollable_and_named_apps() {
        let mut app = app();
        let page_ids = app.snapshot.as_ref().unwrap().page_ids.clone();
        app.pager.set_count(page_ids.len(), 1);
        let pages = crate::view::page_slot_plan(&app);
        assert!(!pages[0].scrollable);
        assert!(pages[1].scrollable);
        assert!(!pages[2].scrollable);
        assert_eq!(pages[1].group.as_ref(), Some(&page_ids[1]));
        assert_eq!(pages[0].group.as_ref(), Some(&page_ids[0]));

        // The scrollable page is the committed group even while it is moving.
        app.pager.set_count(page_ids.len(), 2);
        let moved = crate::view::page_slot_plan(&app);
        assert!(moved[1].scrollable);
        assert_eq!(moved[1].group.as_ref(), Some(&page_ids[2]));
    }

    #[test]
    fn title_buttons_keep_step_and_search_messages() {
        let app = app();
        let titles = crate::view::title_slot_plan(&app);
        assert!(matches!(titles[0].message, Message::Step(-1)));
        assert!(matches!(titles[1].message, Message::Search));
        assert!(matches!(titles[2].message, Message::Step(1)));
    }

    fn publish(app: &mut App) {
        app.publish_accessibility();
        assert!(app.accessibility.frame.is_some(), "frame was not published");
    }

    fn request(app: &App, key: &str, action: Action) -> crate::accessibility::ActionRequest {
        let frame = app.accessibility.frame.as_ref().expect("published frame");
        let node = frame
            .nodes
            .iter()
            .find(|node| node.key == key)
            .unwrap_or_else(|| panic!("no node {key}"));
        crate::accessibility::ActionRequest {
            node_id: node.id,
            action,
            value: String::new(),
            revision: app.accessibility.publisher.revision(),
        }
    }

    fn fill_page(app: &mut App, count: usize) {
        let group = app.current_group_id().cloned().expect("a current group");
        for index in 0..count {
            let mut item = item(app, &format!("fill-{index}"));
            item.group_id = group.clone();
            app.snapshot.as_mut().unwrap().items.push(item);
        }
    }

    fn visible_folder(app: &mut App) -> Item {
        let group = app.current_group_id().cloned().expect("a current group");
        let mut folder = folder_item(app);
        folder.group_id = group;
        app.snapshot.as_mut().unwrap().items.push(folder.clone());
        folder
    }

    #[test]
    fn item_click_is_dropped_while_paging() {
        let mut app = app();
        app.pager.set_count(3, 0);
        let folder = visible_folder(&mut app);
        publish(&mut app);
        app.pager.begin_drag();
        let request = request(&app, &format!("item:{}", folder.id), Action::Click);
        let _ = app.update(Message::Accessibility(request));
        assert!(app.folder.is_none());
    }

    #[test]
    fn valid_item_action_is_rejected_only_while_the_pager_is_settling() {
        let mut app = app();
        app.pager.set_count(3, 0);
        let folder = visible_folder(&mut app);
        publish(&mut app);
        let request = request(&app, &format!("item:{}", folder.id), Action::Click);
        let revision = app.accessibility.publisher.revision();
        app.pager
            .step(1, Instant::now(), Duration::from_millis(400));
        assert!(app.pager.is_animating());
        let _ = app.update(Message::Accessibility(request.clone()));
        assert_eq!(app.accessibility.publisher.revision(), revision);
        assert!(app.folder.is_none());
        // Cancel motion without publishing another frame: the exact same
        // request must work, proving the rejection was not stale-id/revision.
        app.pager.set_count(3, 0);
        let _ = app.update(Message::Accessibility(request));
        assert!(app.folder.is_some());
    }

    #[test]
    fn returning_to_foreground_republishes_the_tree() {
        let mut app = app();
        app.background = true;
        app.accessibility.frame = None;
        app.set_foreground(true);
        assert!(
            app.accessibility.frame.is_some(),
            "foreground did not republish"
        );
    }

    #[test]
    fn stale_revision_cannot_launch_or_open_a_menu() {
        let mut app = app();
        app.pager.set_count(3, 0);
        let folder = visible_folder(&mut app);
        publish(&mut app);
        let mut request = request(&app, &format!("item:{}", folder.id), Action::Click);
        request.revision -= 1;
        let _ = app.update(Message::Accessibility(request));
        assert!(app.folder.is_none());
    }

    #[test]
    fn title_action_is_still_allowed_while_paging() {
        let mut app = app();
        app.pager.set_count(3, 0);
        publish(&mut app);
        app.pager.begin_drag();
        let request = request(&app, "title:next", Action::Click);
        let _ = app.update(Message::Accessibility(request));
        assert!(app.pager.is_animating());
        settle(&mut app);
        assert_eq!(app.pager.committed(), 1);
    }

    #[test]
    fn accessibility_scroll_updates_the_region_offset() {
        let mut app = app();
        app.pager.set_count(3, 0);
        fill_page(&mut app, 60);
        publish(&mut app);
        let region = Region::Group(app.current_group_id().cloned().unwrap());
        let viewport = app
            .accessibility
            .frame
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|node| node.key == "grid:container")
            .and_then(|node| match node.target {
                Target::Scroll { viewport, .. } => Some(viewport),
                _ => None,
            })
            .expect("a scroll target");
        let request = request(&app, "grid:container", Action::ScrollForward);
        let _ = app.update(Message::Accessibility(request));
        let offset = app.scroll.get(&region).copied().unwrap_or_default();
        assert!(
            (offset - viewport * 0.8).abs() < 0.1,
            "offset {offset} != {}",
            viewport * 0.8
        );
    }

    #[test]
    fn page_scroll_actions_step_the_pager() {
        let mut app = app();
        app.pager.set_count(3, 0);
        publish(&mut app);
        let request = request(&app, "grid:container", Action::ScrollRight);
        let _ = app.update(Message::Accessibility(request));
        settle(&mut app);
        assert_eq!(app.pager.committed(), 1);
    }

    #[test]
    fn accessibility_search_text_reaches_the_query() {
        let mut app = app();
        app.searching = true;
        app.query.clear();
        publish(&mut app);
        let mut request = request(&app, "search:field", Action::SetText);
        request.value = "calc".into();
        let _ = app.update(Message::Accessibility(request));
        assert_eq!(app.query, "calc");
    }

    #[test]
    fn overlay_dismiss_uses_the_back_path() {
        let mut app = app();
        app.overlay = Some(app_overlay(&app, overlay::AppStep::Actions));
        publish(&mut app);
        let request = request(&app, "overlay:title", Action::Dismiss);
        let _ = app.update(Message::Accessibility(request));
        assert!(app.overlay.is_none());
    }

    #[test]
    fn unknown_node_or_action_is_ignored() {
        let mut app = app();
        app.pager.set_count(3, 0);
        publish(&mut app);
        let mut unknown = request(&app, "grid:container", Action::Click);
        unknown.node_id = 9999;
        let _ = app.update(Message::Accessibility(unknown));
        assert!(!app.pager.is_animating());

        let unadvertised = request(&app, "grid:container", Action::Click);
        let _ = app.update(Message::Accessibility(unadvertised));
        assert!(!app.pager.is_animating());

        assert!(Action::parse("teleport").is_none());
    }
}

pub fn run() -> iced::Result {
    let mut app = iced::daemon(App::new, App::update, App::view)
        .title(|_: &App, _| "QLaunch".into())
        .subscription(App::subscription)
        .theme(|_: &App, _| Theme::Dark)
        .style(|_, _| iced::theme::Style {
            background_color: if cfg!(target_os = "android") {
                Color::TRANSPARENT
            } else {
                Color::from_rgb8(27, 27, 31)
            },
            text_color: Color::WHITE,
        })
        .scale_factor(|app: &App, _| app.scale)
        .default_font(platform::DEFAULT_FONT);
    for font in platform::fonts() {
        app = app.font(font);
    }
    #[cfg(target_os = "android")]
    for font in fonts::android_extra_fonts() {
        app = app.font(font);
    }
    app = app.font(include_bytes!("../assets/MaterialIcons-Regular.otf").as_slice());
    app.run()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Region {
    Screen,
    Dialog,
    Select,
    Group(Id),
    Search,
    OverlayCard,
}

impl Region {
    pub(crate) fn widget_id(&self) -> &'static str {
        match self {
            Region::Screen => "screen-scroll",
            Region::Dialog => "dialog-scroll",
            Region::Select => "select-scroll",
            Region::Group(_) => "apps",
            Region::Search => "search-results",
            Region::OverlayCard => "overlay-card",
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    ScopedDialog(u64, Box<Message>),
    SimpleDone(Result<(), String>),
    GroupAppToggle(Id, Id, bool),
    GroupAdd,
    GroupEdit(Id),
    GroupHidden(Id, bool),
    GroupMove(Id, i32),
    GroupDragScroll(f32),
    Advanced(screens::advanced::Row),
    BackupDone(Result<String, String>),
    BackupsListed(Result<Vec<String>, String>),
    RestoreDone(String, Result<core::ModelSnapshot, String>),
    ResetDone(Result<core::ModelSnapshot, String>),
    SettingDrag(screens::settings::SettingField, f32),
    SettingRelease(screens::settings::SettingField),
    SettingPatch(qlaunch_core::settings::SettingsPatch),
    SettingsDone(Result<core::ModelSnapshot, String>),
    Push(Screen),
    PopScreen,
    Snackbar(String),
    SnackbarExpired(u64),
    Opened(window::Id),
    Closed,
    Foreground(bool),
    Insets(Insets),
    Scale(f32),
    Loaded(Result<core::ModelSnapshot, String>),
    Event(CoreEvent),
    Step(i32),
    Search,
    Query(String),
    Results(String, Vec<AppRef>),
    History(bool),
    HistoryLoaded(Vec<AppRef>, Vec<AppRef>),
    ItemPressed(Item),
    BlankPressed,
    FolderCardPressed,
    FolderDismiss,
    LongPressTimer(u64),
    Mouse(iced::mouse::Event),
    Launched(Result<(), String>),
    Back,
    Icons(Vec<(AppRef, image::Handle)>),
    Touch(iced::touch::Event),
    Resized(iced::Size),
    Density(f32),
    Scrolled(Region, f32),
    ScrollTick,
    Frame(Instant),
    Overlay(overlay::Message),
    Dialog(dialog::Message),
    ModelDone {
        result: Result<core::ModelSnapshot, String>,
        close: bool,
    },
    Mutated(Result<core::ModelSnapshot, String>),
    ActionDone(Result<(), String>),
    Accessibility(ActionRequest),
    AccessibilitySettled(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Screen {
    Settings,
    Advanced,
    Groups,
    GroupApps(Id),
}

struct App {
    dialog_generation: u64,
    dialog_parents: Vec<dialog::Dialog>,
    pub(crate) settings_drag: Option<(screens::settings::SettingField, f32)>,
    pub(crate) screens: Vec<Screen>,
    suspended_dialogs: Vec<(usize, dialog::Dialog)>,
    pub(crate) snackbar: Option<(u64, String)>,
    snackbar_generation: u64,
    window: Option<window::Id>,
    background: bool,
    pub(crate) insets: Insets,
    scale: f32,
    density: f32,
    pub(crate) snapshot: Option<core::ModelSnapshot>,
    pager: Pager,
    pub(crate) folder: Option<Id>,
    pub(crate) searching: bool,
    pub(crate) query: String,
    pub(crate) results: Vec<AppRef>,
    pub(crate) frequent: Vec<AppRef>,
    pub(crate) recent: Vec<AppRef>,
    pub(crate) show_recent: bool,
    pub(crate) icons: HashMap<AppRef, image::Handle>,
    /// Item position per app, rebuilt with the snapshot. Search results are
    /// looked up here instead of scanning every item per result.
    item_by_app: HashMap<AppRef, usize>,
    pending: HashSet<AppRef>,
    pub(crate) error: Option<String>,
    pub(crate) size: iced::Size,
    scroll: HashMap<Region, f32>,
    elastic: Mutex<HashMap<Region, elastic_scroll::Motion>>,
    scroll_last: Option<Instant>,
    overlay: Option<overlay::Overlay>,
    pub(crate) dialog: Option<dialog::Dialog>,
    pub(crate) select: Option<dialog::Select>,
    accessibility: accessibility::State,
    metrics: Arc<Mutex<metrics::Samples>>,
    gesture: Gesture,
    pending_target: Option<Start>,
    suppress_dismiss_pointer: bool,
    pressed: Option<Id>,
    cursor: Point,
    started: Instant,
}

fn window_settings() -> window::Settings {
    window::Settings {
        transparent: true,
        size: iced::Size::new(420.0, 800.0),
        ..Default::default()
    }
}

/// Item position per app, for `App::visible_items`. The first item for an app
/// wins, matching the linear scan it replaces.
fn index_items(snapshot: &core::ModelSnapshot) -> HashMap<AppRef, usize> {
    let mut index = HashMap::with_capacity(snapshot.items.len());
    for (position, item) in snapshot.items.iter().enumerate() {
        if let Some(app) = &item.app {
            index.entry(app.clone()).or_insert(position);
        }
    }
    index
}

/// Core calls perform JNI and disk IO. Never execute them on iced's UI thread.
fn work<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
    done: fn(T) -> Message,
) -> Task<Message> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(f)
                .await
                .expect("core worker panicked")
        },
        done,
    )
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let (id, open) = window::open(window_settings());
        let app = Self {
            dialog_generation: 0,
            dialog_parents: Vec::new(),
            settings_drag: None,
            screens: Vec::new(),
            suspended_dialogs: Vec::new(),
            snackbar: None,
            snackbar_generation: 0,
            window: Some(id),
            background: false,
            insets: Insets::default(),
            scale: 1.0,
            density: 1.0,
            snapshot: None,
            pager: Pager::new(0, 0),
            folder: None,
            searching: false,
            query: String::new(),
            results: vec![],
            frequent: vec![],
            recent: vec![],
            show_recent: false,
            icons: HashMap::new(),
            item_by_app: HashMap::new(),
            pending: HashSet::new(),
            error: None,
            size: iced::Size::new(420.0, 800.0),
            scroll: HashMap::new(),
            elastic: Mutex::new(HashMap::new()),
            scroll_last: None,
            overlay: None,
            dialog: None,
            select: None,
            accessibility: accessibility::State::default(),
            metrics: Arc::new(Mutex::new(metrics::Samples::default())),
            gesture: Gesture::new(),
            pending_target: None,
            suppress_dismiss_pointer: false,
            pressed: None,
            cursor: Point::ORIGIN,
            started: Instant::now(),
        };
        platform::set_system_bars_dark(true);
        (
            app,
            Task::batch([
                open.map(Message::Opened),
                work(|| block_on(core::init(None)), Message::Loaded),
            ]),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        let events = Subscription::run(|| {
            iced::stream::channel(32, async |mut output| {
                let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
                core::subscribe_native(move |event| sender.send(event).is_ok());
                while let Some(event) = receiver.recv().await {
                    if output.send(Message::Event(event)).await.is_err() {
                        break;
                    }
                }
            })
        });
        let mut subscriptions = vec![
            events,
            platform::insets().map(Message::Insets),
            platform::foreground().map(Message::Foreground),
            platform::system_scale().map(Message::Scale),
            window::close_events().map(|_| Message::Closed),
            window::resize_events().map(|(_, size)| Message::Resized(size)),
            keyboard::listen().filter_map(|event| match event {
                keyboard::Event::KeyPressed {
                    key: Key::Named(Named::Escape | Named::BrowserBack),
                    ..
                } => Some(Message::Back),
                keyboard::Event::KeyPressed {
                    key: Key::Named(Named::ArrowLeft),
                    ..
                } => Some(Message::Step(-1)),
                keyboard::Event::KeyPressed {
                    key: Key::Named(Named::ArrowRight),
                    ..
                } => Some(Message::Step(1)),
                _ => None,
            }),
            iced::event::listen_raw(|event, _, _| match event {
                iced::Event::Touch(event) => Some(Message::Touch(event)),
                iced::Event::Mouse(event) => Some(Message::Mouse(event)),
                _ => None,
            }),
        ];
        subscriptions.push(accessibility::subscribe().map(Message::Accessibility));
        if self.pager.is_animating() {
            subscriptions.push(window::frames().map(Message::Frame));
        }
        if self.scroll_last.is_some() {
            subscriptions
                .push(iced::time::every(Duration::from_millis(100)).map(|_| Message::ScrollTick));
        }
        Subscription::batch(subscriptions)
    }

    /// Scrollbars hide on their own; the view reads this while building.
    /// The shared visual motion for one scroll region. Wrapper widgets own
    /// touch and spring state in the tree; this handle only lets content and
    /// row decorators read the same values while the app rebuilds.
    pub(crate) fn motion(&self, region: Region) -> elastic_scroll::Motion {
        let mut motions = self.elastic.lock().expect("elastic motion poisoned");
        motions.entry(region).or_default().clone()
    }

    pub(crate) fn scrollbars_visible(&self) -> bool {
        self.scrollbar_visible_at(Instant::now())
    }

    fn scrollbar_visible_at(&self, now: Instant) -> bool {
        self.scroll_last
            .is_some_and(|last| now.saturating_duration_since(last) < SCROLLBAR_IDLE)
    }

    fn home_index(&self) -> usize {
        self.snapshot
            .as_ref()
            .and_then(|s| {
                s.home_group_id
                    .as_ref()
                    .and_then(|home| s.page_ids.iter().position(|id| id == home))
            })
            .unwrap_or(0)
    }

    /// The group currently on screen (a folder when one is open).
    pub(crate) fn current_group_id(&self) -> Option<&Id> {
        self.folder.as_ref().or_else(|| {
            self.snapshot
                .as_ref()
                .and_then(|s| s.page_ids.get(self.pager.committed()))
        })
    }

    fn step(&mut self, delta: i32) {
        if self.folder.is_some()
            || self.overlay.is_some()
            || self.dialog.is_some()
            || self.select.is_some()
            || !self.screens.is_empty()
        {
            return;
        }
        if self.searching && self.query.is_empty() {
            self.show_recent = !self.show_recent;
            return;
        }
        let transition = self.transition();
        self.pager.step(delta, Instant::now(), transition);
        self.folder = None;
    }

    /// The configured settle duration, clamped to the core's supported range.
    pub(crate) fn transition(&self) -> Duration {
        let ms = self
            .snapshot
            .as_ref()
            .map_or(300, |s| s.settings.page_transition_ms)
            .clamp(100, 800);
        Duration::from_millis(ms as u64)
    }

    /// Returns the page scroll to the top and forgets its old offset.
    fn reset_scroll(&mut self) -> Task<Message> {
        let region = self.current_group_id().cloned().map(Region::Group);
        match region {
            Some(region) => self.scroll_region_to_top(region),
            None => Task::none(),
        }
    }

    /// Resets one scroll region and moves its widget to the top.
    fn scroll_region_to_top(&mut self, region: Region) -> Task<Message> {
        self.scroll.insert(region.clone(), 0.0);
        iced::widget::operation::scroll_to(
            self.scroll_widget_id(&region),
            iced::widget::operation::AbsoluteOffset {
                x: Some(0.0),
                y: Some(0.0),
            },
        )
    }

    fn scroll_widget_id(&self, region: &Region) -> &'static str {
        if matches!(region, Region::Group(group) if self.folder.as_ref() == Some(group)) {
            "folder-apps"
        } else {
            region.widget_id()
        }
    }

    fn go_home(&mut self) -> Task<Message> {
        let home = self.home_index();
        let transition = self.transition();
        self.pager.settle_to(home, Instant::now(), transition);
        Task::none()
    }

    /// Installs a new snapshot. Reloads keep the group on screen when it still
    /// exists; the first load starts on the core's home group.
    /// Rebuilds the app-index used by `visible_items` from the current
    /// snapshot. Call after replacing `snapshot`.
    fn reindex_items(&mut self) {
        let index = self
            .snapshot
            .as_ref()
            .map_or_else(HashMap::new, index_items);
        self.item_by_app = index;
    }

    fn apply_snapshot(&mut self, snapshot: core::ModelSnapshot, first: bool) {
        let current = self.current_group_id().cloned();
        // A package update may change an icon without changing its AppRef.
        if self
            .snapshot
            .as_ref()
            .is_some_and(|old| old.icon_revisions != snapshot.icon_revisions)
        {
            self.icons.clear();
        }
        self.snapshot = Some(snapshot);
        self.reindex_items();
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        let index = if first {
            self.home_index()
        } else {
            current
                .as_ref()
                .and_then(|id| snapshot.page_ids.iter().position(|page| page == id))
                .unwrap_or(0)
        };
        self.pager.set_count(snapshot.page_ids.len(), index);
        if first {
            log::info!("core ready in {} ms", self.started.elapsed().as_millis());
        }
    }

    /// The group a shortcut or folder created from a menu belongs to. Always a
    /// page group, never an open folder.
    fn page_group_id(&self) -> Option<Id> {
        self.snapshot
            .as_ref()?
            .page_ids
            .get(self.pager.committed())
            .cloned()
    }

    /// Runs a dialog's core changes in order, then reloads the snapshot.
    fn run_changes(changes: Vec<dialog::Change>) -> Task<Message> {
        work(
            move || {
                block_on(async move {
                    for change in changes {
                        let result = match change {
                            dialog::Change::RenameItem(id, name) => {
                                core::rename_item(id, name).await
                            }
                            dialog::Change::MoveItem(id, group, order) => {
                                core::move_item(id, group, order).await
                            }
                            dialog::Change::RenameGroup(id, name) => {
                                core::rename_group(id, name).await
                            }
                            dialog::Change::AddFolder(group, name) => {
                                core::add_folder(group, name).await
                            }
                        };
                        result?;
                    }
                    core::snapshot().await
                })
            },
            |result| Message::ModelDone {
                result,
                close: true,
            },
        )
    }

    /// Shows an error on the open dialog. Returns false when no dialog can
    /// carry it, so the caller can fall back to the launcher error line.
    fn dialog_error(&mut self, error: String) -> bool {
        match &mut self.dialog {
            Some(dialog::Dialog::App(editor)) => {
                editor.busy = false;
                editor.error = Some(error);
                true
            }
            Some(dialog::Dialog::Folder(editor)) => {
                editor.set_busy(false);
                editor.set_error(Some(error));
                true
            }
            Some(dialog::Dialog::GroupEditor(state)) => {
                state.busy = false;
                state.error = Some(error);
                true
            }
            _ => false,
        }
    }

    fn dialog_message(&mut self, message: dialog::Message) -> Task<Message> {
        use dialog::Dialog;
        use dialog::Message as D;
        let busy = match &self.dialog {
            Some(Dialog::App(e)) => e.busy,
            Some(Dialog::Folder(e)) => e.busy(),
            Some(Dialog::AddShortcut(e)) => e.busy,
            Some(Dialog::CommandShortcut(e)) => e.busy,
            Some(Dialog::AddGroup(e)) => e.busy,
            Some(Dialog::GroupEditor(e)) => e.busy,
            _ => false,
        };
        if busy
            && !matches!(
                message,
                D::Close | D::CloseSelect | D::Noop | D::Providers(_)
            )
        {
            return Task::none();
        }
        match message {
            D::Noop => {}
            D::Close => {
                self.dialog = self.dialog_parents.pop();
                self.select = None;
            }
            D::OpenGroupSelect => {
                let field = match &self.dialog {
                    Some(Dialog::App(_)) => dialog::SelectField::AppGroup,
                    Some(Dialog::Folder(_)) => dialog::SelectField::FolderGroup,
                    _ => return Task::none(),
                };
                self.open_select(field);
            }
            D::GroupChosen(id) => {
                match &mut self.dialog {
                    Some(Dialog::App(editor)) => editor.group_id = id,
                    Some(Dialog::Folder(editor)) => editor.set_group(id),
                    _ => {}
                }
                self.select = None;
            }
            D::CloseSelect => self.select = None,
            D::AppName(value) => {
                if let Some(Dialog::App(editor)) = &mut self.dialog {
                    editor.name = value;
                    editor.error = None;
                }
            }
            D::AppSave => {
                let Some(Dialog::App(editor)) = &mut self.dialog else {
                    return Task::none();
                };
                if !editor.can_save() {
                    return Task::none();
                }
                let Some(snapshot) = self.snapshot.as_ref() else {
                    return Task::none();
                };
                let plan =
                    dialog::app_save_plan(&editor.item, &editor.name, &editor.group_id, snapshot);
                editor.busy = true;
                return Self::run_changes(plan);
            }
            D::AppUninstall => {
                let Some(Dialog::App(editor)) = &mut self.dialog else {
                    return Task::none();
                };
                let Some(app) = editor.item.app.clone() else {
                    return Task::none();
                };
                editor.busy = true;
                return work(
                    move || {
                        block_on(async move {
                            core::uninstall_app(app.package).await?;
                            core::snapshot().await
                        })
                    },
                    |result| Message::ModelDone {
                        result,
                        close: true,
                    },
                );
            }
            D::AppRemove => {
                let Some(Dialog::App(editor)) = &mut self.dialog else {
                    return Task::none();
                };
                let id = editor.item.id.clone();
                editor.busy = true;
                return work(
                    move || block_on(core::remove_item(id)),
                    |result| Message::ModelDone {
                        result,
                        close: true,
                    },
                );
            }
            D::FolderName(value) => {
                if let Some(Dialog::Folder(editor)) = &mut self.dialog {
                    editor.set_name(value);
                    editor.set_error(None);
                }
            }
            D::FolderSave => {
                let Some(snapshot) = self.snapshot.as_ref() else {
                    return Task::none();
                };
                let Some(Dialog::Folder(editor)) = &mut self.dialog else {
                    return Task::none();
                };
                if !editor.can_save() {
                    return Task::none();
                }
                let plan = dialog::folder_save_plan(editor, snapshot);
                editor.set_busy(true);
                return Self::run_changes(plan);
            }
            D::FolderDelete => {
                let Some(Dialog::Folder(editor)) = &mut self.dialog else {
                    return Task::none();
                };
                let Some(folder) = (match editor {
                    dialog::FolderEditor::Edit { folder, .. } => Some(folder.clone()),
                    dialog::FolderEditor::Create { .. } => None,
                }) else {
                    return Task::none();
                };
                editor.set_busy(true);
                return work(
                    move || block_on(core::delete_folder(folder)),
                    |result| Message::ModelDone {
                        result,
                        close: true,
                    },
                );
            }
            D::LauncherSettings => {
                self.dialog = None;
                return self.update(Message::Push(Screen::Settings));
            }
            D::LauncherEditGroups => {
                self.dialog = None;
                return self.update(Message::Push(Screen::Groups));
            }
            D::LauncherAddShortcut => {
                let Some(group) = self.page_group_id() else {
                    return Task::none();
                };
                self.dialog = Some(Dialog::AddShortcut(dialog::AddShortcutState::new(group)));
                return work(
                    || block_on(core::shortcut_providers()),
                    |result| Message::Dialog(D::Providers(result)),
                );
            }
            D::LauncherCreateFolder => {
                let Some(group) = self.page_group_id() else {
                    return Task::none();
                };
                self.dialog = Some(Dialog::Folder(dialog::FolderEditor::create(group)));
            }
            D::LauncherWallpaper => {
                self.dialog = None;
                return work(|| block_on(core::choose_wallpaper()), Message::SimpleDone);
            }
            D::Providers(result) => {
                if let Some(Dialog::AddShortcut(state)) = &mut self.dialog {
                    state.providers = Some(result);
                }
            }
            D::ShortcutLock => {
                let Some(Dialog::AddShortcut(state)) = &self.dialog else {
                    return Task::none();
                };
                let group = state.target_group.clone();
                self.mark_shortcut_busy();
                return work(
                    move || {
                        block_on(async move {
                            core::add_lock_shortcut(group).await?;
                            core::snapshot().await
                        })
                    },
                    |result| Message::ModelDone {
                        result,
                        close: true,
                    },
                );
            }
            D::ShortcutNotifications => {
                let Some(Dialog::AddShortcut(state)) = &self.dialog else {
                    return Task::none();
                };
                let group = state.target_group.clone();
                self.mark_shortcut_busy();
                return work(
                    move || {
                        block_on(async move {
                            core::add_notifications_shortcut(group).await?;
                            core::snapshot().await
                        })
                    },
                    |result| Message::ModelDone {
                        result,
                        close: true,
                    },
                );
            }
            D::ShortcutProvider(id) => {
                let Some(Dialog::AddShortcut(state)) = &self.dialog else {
                    return Task::none();
                };
                let group = state.target_group.clone();
                self.mark_shortcut_busy();
                return work(
                    move || {
                        block_on(async move {
                            core::begin_shortcut(id, group).await?;
                            core::snapshot().await
                        })
                    },
                    |result| Message::ModelDone {
                        result,
                        close: true,
                    },
                );
            }
            D::ShortcutCommand => {
                let Some(Dialog::AddShortcut(state)) = &self.dialog else {
                    return Task::none();
                };
                let group = state.target_group.clone();
                if let Some(parent) = self.dialog.take() {
                    self.dialog_parents.push(parent);
                }
                self.dialog = Some(Dialog::CommandShortcut(dialog::CommandShortcutState::new(
                    group,
                )));
            }
            D::CommandName(value) => {
                if let Some(Dialog::CommandShortcut(state)) = &mut self.dialog {
                    state.name = value;
                }
            }
            D::CommandText(value) => {
                if let Some(Dialog::CommandShortcut(state)) = &mut self.dialog {
                    state.command = value;
                }
            }
            D::CommandAdd => {
                let Some(Dialog::CommandShortcut(state)) = &mut self.dialog else {
                    return Task::none();
                };
                if !state.ready() {
                    return Task::none();
                }
                let group = state.target_group.clone();
                let name = state.name.trim().to_string();
                let command = state.command.trim().to_string();
                state.busy = true;
                return work(
                    move || {
                        block_on(async move {
                            core::add_command_shortcut(group, name, command).await?;
                            core::snapshot().await
                        })
                    },
                    |result| Message::ModelDone {
                        result,
                        close: true,
                    },
                );
            }
            D::AddGroupName(value) => {
                if let Some(Dialog::AddGroup(state)) = &mut self.dialog {
                    state.name = value;
                }
            }
            D::AddGroupConfirm => {
                let Some(Dialog::AddGroup(state)) = &mut self.dialog else {
                    return Task::none();
                };
                if !state.can_confirm() {
                    return Task::none();
                }
                let label = state.name.trim().to_string();
                state.busy = true;
                return work(
                    move || {
                        block_on(async move {
                            core::add_group(label).await?;
                            core::snapshot().await
                        })
                    },
                    |result| Message::ModelDone {
                        result,
                        close: true,
                    },
                );
            }
            D::GroupName(value) => {
                if let Some(Dialog::GroupEditor(state)) = &mut self.dialog {
                    state.name = value;
                    state.error = None;
                }
            }
            D::GroupRole(role) => {
                let Some(Dialog::GroupEditor(state)) = &mut self.dialog else {
                    return Task::none();
                };
                if state.role == role || state.role_locked() {
                    return Task::none();
                }
                let group = state.group.clone();
                state.busy = true;
                return work(
                    move || {
                        block_on(async move {
                            core::set_group_role(group, role).await?;
                            core::snapshot().await
                        })
                    },
                    |result| Message::ModelDone {
                        result,
                        close: false,
                    },
                );
            }
            D::GroupSave => {
                let Some(Dialog::GroupEditor(state)) = &mut self.dialog else {
                    return Task::none();
                };
                if !state.can_save() {
                    return Task::none();
                }
                let group = state.group.clone();
                let name = state.name.trim().to_string();
                let rename = (name != state.label)
                    .then_some(dialog::Change::RenameGroup(group.clone(), name));
                state.busy = true;
                let changes: Vec<dialog::Change> = rename.into_iter().collect();
                return Self::run_changes(changes);
            }
            D::GroupDelete => {
                let Some(Dialog::GroupEditor(state)) = &self.dialog else {
                    return Task::none();
                };
                if !state.can_delete() {
                    return Task::none();
                }
                let confirmation = dialog::ConfirmState {
                    title: format!("Delete {}?", state.label),
                    body: "The apps in this group move to the Apps group.".into(),
                    confirm_label: "Delete".into(),
                    destructive: true,
                    busy: false,
                    action: dialog::ConfirmAction::DeleteGroup(state.group.clone()),
                };
                if let Some(parent) = self.dialog.take() {
                    self.dialog_parents.push(parent);
                }
                self.dialog = Some(Dialog::Confirm(confirmation));
            }
            D::GroupChooseApps => {
                if let Some(Dialog::GroupEditor(state)) = &self.dialog {
                    return self.update(Message::Push(Screen::GroupApps(state.group.clone())));
                }
            }
            D::RestoreChosen(name) => {
                self.dialog = Some(Dialog::Confirm(dialog::ConfirmState {
                    title: "Restore config".into(),
                    body: format!("This replaces your current layout and settings with {name}."),
                    confirm_label: "Restore".into(),
                    destructive: false,
                    busy: false,
                    action: dialog::ConfirmAction::Restore(name),
                }));
            }
            D::ConfirmYes => {
                let Some(Dialog::Confirm(state)) = &self.dialog else {
                    return Task::none();
                };
                if state.busy {
                    return Task::none();
                }
                let Some(Dialog::Confirm(state)) = &mut self.dialog else {
                    return Task::none();
                };
                state.busy = true;
                match &state.action {
                    dialog::ConfirmAction::DeleteGroup(id) => {
                        let id = id.clone();
                        return work(
                            move || block_on(core::delete_group(id)),
                            |result| Message::ModelDone {
                                result,
                                close: true,
                            },
                        );
                    }
                    dialog::ConfirmAction::Restore(name) => {
                        let name = name.clone();
                        return work(
                            move || {
                                let result = block_on(core::restore_config(name.clone()));
                                (name, result)
                            },
                            |(name, result)| Message::RestoreDone(name, result),
                        );
                    }
                    dialog::ConfirmAction::FactoryReset => {
                        return work(|| block_on(core::factory_reset()), Message::ResetDone);
                    }
                }
            }
        }
        Task::none()
    }

    fn mark_shortcut_busy(&mut self) {
        if let Some(dialog::Dialog::AddShortcut(state)) = &mut self.dialog {
            state.busy = true;
        }
    }

    fn overlay_error(&mut self, error: String) {
        match &mut self.overlay {
            Some(overlay) => overlay.set_error(error),
            None => self.error = Some(error),
        }
    }

    fn overlay_message(&mut self, message: overlay::Message) -> Task<Message> {
        let navigating = matches!(
            message,
            overlay::Message::Back
                | overlay::Message::AppRenameStart
                | overlay::Message::AppMove
                | overlay::Message::LauncherAddShortcut
                | overlay::Message::LauncherCreateFolder
        );
        let task = self.overlay_message_inner(message);
        if navigating {
            Task::batch([task, self.scroll_region_to_top(Region::OverlayCard)])
        } else {
            task
        }
    }

    fn overlay_message_inner(&mut self, message: overlay::Message) -> Task<Message> {
        use overlay::Message as Menu;
        match message {
            Menu::Close => self.overlay = None,
            Menu::Noop => {}
            Menu::Back => {
                if self.overlay.as_mut().is_some_and(overlay::Overlay::back) {
                    self.overlay = None;
                }
            }
            Menu::AppRenameStart => {
                if let Some(overlay::Overlay::App(menu)) = &mut self.overlay
                    && matches!(menu.step, overlay::AppStep::Actions)
                {
                    let label = menu.item.label.clone();
                    menu.step = overlay::AppStep::Rename(label);
                    menu.error = None;
                }
            }
            Menu::AppRename(value) => {
                if let Some(overlay::Overlay::App(menu)) = &mut self.overlay
                    && let overlay::AppStep::Rename(buffer) = &mut menu.step
                {
                    *buffer = value;
                    menu.error = None;
                }
            }
            Menu::AppRenameSubmit => {
                let Some(overlay::Overlay::App(menu)) = &self.overlay else {
                    return Task::none();
                };
                let overlay::AppStep::Rename(name) = &menu.step else {
                    return Task::none();
                };
                let name = name.trim().to_string();
                let id = menu.item.id.clone();
                if name.is_empty() {
                    if let Some(overlay::Overlay::App(menu)) = &mut self.overlay {
                        menu.error = Some("Name cannot be empty".into());
                    }
                    return Task::none();
                }
                return work(
                    move || block_on(core::rename_item(id, name)),
                    Message::Mutated,
                );
            }
            Menu::AppMove => {
                if let Some(overlay::Overlay::App(menu)) = &mut self.overlay {
                    menu.step = overlay::AppStep::Move;
                    menu.error = None;
                }
            }
            Menu::AppMoveTo(group) => {
                let Some(overlay::Overlay::App(menu)) = &self.overlay else {
                    return Task::none();
                };
                let id = menu.item.id.clone();
                let order = self
                    .snapshot
                    .as_ref()
                    .map_or(0, |snapshot| overlay::append_order(snapshot, &group));
                return work(
                    move || block_on(core::move_item(id, group, order)),
                    Message::Mutated,
                );
            }
            Menu::AppUninstall => {
                let Some(overlay::Overlay::App(menu)) = &mut self.overlay else {
                    return Task::none();
                };
                let Some(app) = menu.item.app.clone() else {
                    menu.error = Some("This item has no app to uninstall".into());
                    return Task::none();
                };
                return work(
                    move || block_on(core::uninstall_app(app.package)),
                    Message::ActionDone,
                );
            }
            Menu::AppRemove => {
                let Some(overlay::Overlay::App(menu)) = &self.overlay else {
                    return Task::none();
                };
                let id = menu.item.id.clone();
                return work(move || block_on(core::remove_item(id)), Message::Mutated);
            }
            Menu::LauncherSettings | Menu::LauncherEditGroups => {
                if let Some(overlay) = &mut self.overlay {
                    overlay.set_error("Not yet available in the Iced port".into());
                }
            }
            Menu::LauncherAddShortcut => {
                if let Some(overlay::Overlay::Launcher(menu)) = &mut self.overlay {
                    menu.step = overlay::LauncherStep::Shortcuts;
                    menu.providers_loading = true;
                    menu.error = None;
                }
                return work(
                    || block_on(core::shortcut_providers()),
                    |result| Message::Overlay(overlay::Message::Providers(result)),
                );
            }
            Menu::Providers(result) => {
                if let Some(overlay::Overlay::Launcher(menu)) = &mut self.overlay {
                    menu.providers_loading = false;
                    match result {
                        Ok(providers) => menu.providers = providers,
                        Err(error) => menu.error = Some(error),
                    }
                }
            }
            Menu::LauncherCreateFolder => {
                if let Some(overlay::Overlay::Launcher(menu)) = &mut self.overlay {
                    menu.step = overlay::LauncherStep::FolderName(String::new());
                    menu.error = None;
                }
            }
            Menu::LauncherFolderName(value) => {
                if let Some(overlay::Overlay::Launcher(menu)) = &mut self.overlay
                    && let overlay::LauncherStep::FolderName(buffer) = &mut menu.step
                {
                    *buffer = value;
                    menu.error = None;
                }
            }
            Menu::LauncherFolderSubmit => {
                let Some(overlay::Overlay::Launcher(menu)) = &self.overlay else {
                    return Task::none();
                };
                let overlay::LauncherStep::FolderName(name) = &menu.step else {
                    return Task::none();
                };
                let name = name.trim().to_string();
                if name.is_empty() {
                    if let Some(overlay::Overlay::Launcher(menu)) = &mut self.overlay {
                        menu.error = Some("Name cannot be empty".into());
                    }
                    return Task::none();
                }
                let Some(group) = self.page_group_id() else {
                    return Task::none();
                };
                return work(
                    move || block_on(core::add_folder(group, name)),
                    Message::Mutated,
                );
            }
            Menu::LauncherWallpaper => {
                return work(|| block_on(core::choose_wallpaper()), Message::ActionDone);
            }
            Menu::ShortcutLock => {
                let Some(group) = self.page_group_id() else {
                    return Task::none();
                };
                return work(
                    move || block_on(core::add_lock_shortcut(group)),
                    Message::Mutated,
                );
            }
            Menu::ShortcutNotifications => {
                let Some(group) = self.page_group_id() else {
                    return Task::none();
                };
                return work(
                    move || block_on(core::add_notifications_shortcut(group)),
                    Message::Mutated,
                );
            }
            Menu::ShortcutProvider(provider) => {
                let Some(group) = self.page_group_id() else {
                    return Task::none();
                };
                return work(
                    move || block_on(core::begin_shortcut(provider, group)),
                    Message::ActionDone,
                );
            }
        }
        Task::none()
    }

    fn open_search(&mut self) -> Task<Message> {
        self.searching = true;
        self.folder = None;
        self.query.clear();
        Task::batch([
            self.scroll_region_to_top(Region::Search),
            iced::widget::operation::focus("search"),
            work(
                || {
                    let frequent = block_on(core::frequent_apps(36))
                        .into_iter()
                        .map(|h| h.app)
                        .collect();
                    let recent = block_on(core::recent_apps(36))
                        .into_iter()
                        .map(|h| h.app)
                        .collect();
                    (frequent, recent)
                },
                |(a, b)| Message::HistoryLoaded(a, b),
            ),
        ])
    }

    /// Times each update and opens or closes the frame-work window around a
    /// drag or settle, keeping the update path itself in `update_inner`.
    fn update(&mut self, message: Message) -> Task<Message> {
        if matches!(
            &message,
            Message::Back
                | Message::Push(_)
                | Message::PopScreen
                | Message::GroupAdd
                | Message::GroupEdit(_)
                | Message::Event(
                    CoreEvent::HomePressed
                        | CoreEvent::HomePressedFromOverlay
                        | CoreEvent::BackPressed
                )
                | Message::Dialog(
                    dialog::Message::Close
                        | dialog::Message::LauncherAddShortcut
                        | dialog::Message::LauncherCreateFolder
                        | dialog::Message::ShortcutCommand
                        | dialog::Message::GroupDelete
                        | dialog::Message::RestoreChosen(_)
                )
        ) {
            self.dialog_generation += 1;
        }
        let started = Instant::now();
        let task = self.update_inner(message);
        let generation = self.dialog_generation;
        let task = task.map(move |message| match message {
            Message::ModelDone { .. }
            | Message::RestoreDone(..)
            | Message::ResetDone(_)
            | Message::Dialog(dialog::Message::Providers(_)) => {
                Message::ScopedDialog(generation, Box::new(message))
            }
            _ => message,
        });
        let elapsed = started.elapsed().as_secs_f32() * 1000.0;
        let moving = self.pager.is_animating() || self.pager.is_dragging();
        if moving {
            // Dismiss the vertical scrollbar immediately on page takeover;
            // don't let pending scroll callbacks revive it during a transition.
            self.scroll_last = None;
        }
        let summary = {
            let mut samples = self.metrics.lock().unwrap();
            if moving {
                samples.start();
            }
            samples.record_update(elapsed);
            if !moving && samples.is_active() {
                samples.finish()
            } else {
                None
            }
        };
        if let Some(summary) = summary {
            log::info!("{}", summary.log_line());
        }
        self.publish_accessibility_if_idle();
        task
    }

    /// Publishes whenever the tree can have changed, unless a pager motion or
    /// a scroll fling is still in flight.
    fn publish_accessibility_if_idle(&mut self) {
        if self.background
            || self.pager.is_dragging()
            || self.pager.is_animating()
            || self.accessibility.pending_scroll
        {
            return;
        }
        self.publish_accessibility();
    }

    pub(crate) fn publish_accessibility(&mut self) {
        let Some(frame) = accessibility::frame(self) else {
            return;
        };
        let announcement = accessibility::announcement(self);
        if let Err(error) = self.accessibility.publish(frame, Some(announcement)) {
            log::debug!("accessibility publish failed: {error}");
        }
    }

    /// Coming back from another app has no update of its own, so republish
    /// explicitly; otherwise the overlay stays empty until the next input.
    fn set_foreground(&mut self, visible: bool) {
        self.background = !visible;
        if visible {
            self.publish_accessibility();
        }
    }

    fn update_inner(&mut self, message: Message) -> Task<Message> {
        if !matches!(
            &message,
            Message::Scrolled(..)
                | Message::ScrollTick
                | Message::AccessibilitySettled(_)
                | Message::Accessibility(_)
                | Message::Icons(_)
                | Message::Frame(_)
        ) {
            self.accessibility.cancel_scroll();
        }
        match message {
            Message::ScopedDialog(generation, message) => {
                if generation == self.dialog_generation {
                    return self.update(*message);
                }
            }
            Message::SimpleDone(result) => {
                if let Err(error) = result {
                    return self.update(Message::Snackbar(error));
                }
            }
            Message::GroupAppToggle(item_id, group_id, checked) => {
                let Some(snapshot) = &self.snapshot else {
                    return Task::none();
                };
                let Some(item) = snapshot.items.iter().find(|i| i.id == item_id) else {
                    return Task::none();
                };
                if let Some(dialog::Change::MoveItem(id, group, order)) =
                    screens::group_apps::move_plan(snapshot, item, &group_id, checked)
                {
                    return work(
                        move || block_on(core::move_item(id, group, order)),
                        |result| Message::ModelDone {
                            result,
                            close: false,
                        },
                    );
                }
            }
            Message::GroupAdd => {
                self.dialog = Some(dialog::Dialog::AddGroup(dialog::AddGroupState::new()))
            }
            Message::GroupEdit(id) => {
                if let Some(group) = self
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.groups.iter().find(|g| g.id == id))
                {
                    self.dialog = Some(dialog::Dialog::GroupEditor(dialog::GroupEditorState::new(
                        group.id.clone(),
                        group.label.clone(),
                        group.role,
                    )));
                }
            }
            Message::GroupHidden(id, hidden) => {
                return work(
                    move || block_on(core::set_group_hidden(id, hidden)),
                    |result| Message::ModelDone {
                        result,
                        close: false,
                    },
                );
            }
            Message::GroupMove(id, index) => {
                return work(
                    move || block_on(core::reorder_group(id, index)),
                    |result| Message::ModelDone {
                        result,
                        close: false,
                    },
                );
            }
            Message::GroupDragScroll(offset) => {
                if self.screens.last() == Some(&Screen::Groups) && self.dialog.is_none() {
                    self.scroll.insert(Region::Screen, offset);
                    self.scroll_last = Some(Instant::now());
                    return iced::widget::operation::scroll_to(
                        "screen-scroll",
                        iced::widget::operation::AbsoluteOffset {
                            x: None,
                            y: Some(offset),
                        },
                    );
                }
            }
            Message::Advanced(action) => {
                use screens::advanced::Row;
                match action {
                    Row::Restart => return work(|| block_on(core::init(None)), Message::Loaded),
                    Row::Backup => {
                        return work(|| block_on(core::backup_config()), Message::BackupDone);
                    }
                    Row::Restore => {
                        return work(|| block_on(core::list_backups()), Message::BackupsListed);
                    }
                    Row::FactoryReset => {
                        self.dialog = Some(dialog::Dialog::Confirm(dialog::ConfirmState {
                            title: "Factory Reset".into(),
                            body: "This wipes every group, item and setting. It cannot be undone."
                                .into(),
                            confirm_label: "Reset".into(),
                            destructive: true,
                            busy: false,
                            action: dialog::ConfirmAction::FactoryReset,
                        }));
                    }
                }
            }
            Message::BackupDone(result) => {
                return self.update(Message::Snackbar(screens::advanced::backup_message(
                    &result,
                )));
            }
            Message::BackupsListed(result) => {
                if self.screens.last() != Some(&Screen::Advanced) {
                    return Task::none();
                }
                match result {
                    Ok(names) if names.is_empty() => {
                        return self
                            .update(Message::Snackbar("No backups found in Downloads.".into()));
                    }
                    Ok(names) => {
                        self.dialog =
                            Some(dialog::Dialog::RestorePicker(dialog::RestorePickerState {
                                backups: Some(Ok(names)),
                            }))
                    }
                    Err(error) => return self.update(Message::Snackbar(error)),
                }
            }
            Message::RestoreDone(name, result) => {
                let message = screens::advanced::restore_message(&result, &name);
                if let Ok(snapshot) = result {
                    self.apply_snapshot(snapshot, false);
                }
                self.dialog = None;
                return self.update(Message::Snackbar(message));
            }
            Message::ResetDone(result) => {
                self.dialog = None;
                match result {
                    Ok(snapshot) => {
                        self.apply_snapshot(snapshot, false);
                        return self.update(Message::Snackbar("The launcher was reset.".into()));
                    }
                    Err(error) => return self.update(Message::Snackbar(error)),
                }
            }
            Message::SettingDrag(field, value) => self.settings_drag = Some((field, value)),
            Message::SettingRelease(field) => {
                if let Some((active, value)) = self.settings_drag
                    && active == field
                {
                    return self.update(Message::SettingPatch(screens::settings::message(
                        field,
                        value.round() as i64,
                    )));
                }
            }
            Message::SettingPatch(patch) => {
                return work(
                    move || block_on(core::update_settings(patch)),
                    Message::SettingsDone,
                );
            }
            Message::SettingsDone(result) => {
                self.settings_drag = None;
                match result {
                    Ok(snapshot) => self.apply_snapshot(snapshot, false),
                    Err(error) => return self.update(Message::Snackbar(error)),
                }
            }
            Message::Push(screen) => {
                if let Some(dialog) = self.dialog.take() {
                    self.suspended_dialogs.push((self.screens.len(), dialog));
                }
                self.select = None;
                self.screens.push(screen);
                self.pressed = None;
                self.pending_target = None;
                self.gesture = Gesture::new();
            }
            Message::PopScreen => {
                self.screens.pop();
                if self
                    .suspended_dialogs
                    .last()
                    .is_some_and(|(depth, _)| *depth == self.screens.len())
                {
                    self.dialog = self.suspended_dialogs.pop().map(|(_, dialog)| dialog);
                }
            }
            Message::Snackbar(message) => {
                self.snackbar_generation += 1;
                let generation = self.snackbar_generation;
                self.snackbar = Some((generation, message));
                return Task::perform(
                    async { tokio::time::sleep(Duration::from_secs(4)).await },
                    move |_| Message::SnackbarExpired(generation),
                );
            }
            Message::SnackbarExpired(generation) => {
                if self
                    .snackbar
                    .as_ref()
                    .is_some_and(|(current, _)| *current == generation)
                {
                    self.snackbar = None;
                }
            }
            Message::Opened(id) => {
                self.window = Some(id);
                return window::scale_factor(id).map(Message::Density);
            }
            Message::Closed => {
                self.window = None;
                if !self.background {
                    return iced::exit();
                }
            }
            Message::Foreground(visible) => {
                self.set_foreground(visible);
                if visible && self.window.is_none() {
                    let (id, open) = window::open(window_settings());
                    self.window = Some(id);
                    return open.map(Message::Opened);
                } else if !visible && let Some(id) = self.window {
                    return window::close(id);
                }
            }
            Message::Insets(insets) => {
                self.insets = insets;
                return platform::scroll_to_focused();
            }
            Message::Scale(scale) => self.scale = scale,
            Message::Density(density) => {
                if self.density != density {
                    self.icons.clear();
                    self.density = density;
                }
            }
            Message::Resized(size) => self.size = size,
            Message::Loaded(result) => match result {
                Ok(snapshot) => self.apply_snapshot(snapshot, self.snapshot.is_none()),
                Err(error) => self.error = Some(error),
            },
            Message::Mutated(result) => match result {
                Ok(snapshot) => {
                    self.apply_snapshot(snapshot, false);
                    self.overlay = None;
                }
                Err(error) => self.overlay_error(error),
            },
            Message::ModelDone { result, close } => match result {
                Ok(snapshot) => {
                    self.apply_snapshot(snapshot, false);
                    if let Some(dialog::Dialog::GroupEditor(editor)) = &mut self.dialog {
                        editor.busy = false;
                        if let Some(group) = self
                            .snapshot
                            .as_ref()
                            .and_then(|s| s.groups.iter().find(|g| g.id == editor.group))
                        {
                            editor.role = group.role;
                        }
                    }
                    if close {
                        self.dialog = None;
                        self.dialog_parents.clear();
                        self.select = None;
                    }
                }
                Err(error) => {
                    if !self.dialog_error(error.clone()) {
                        match &mut self.dialog {
                            Some(dialog::Dialog::AddShortcut(e)) => e.busy = false,
                            Some(dialog::Dialog::CommandShortcut(e)) => e.busy = false,
                            Some(dialog::Dialog::AddGroup(e)) => e.busy = false,
                            Some(dialog::Dialog::Confirm(e)) => e.busy = false,
                            _ => {}
                        }
                        return self.update(Message::Snackbar(error));
                    }
                }
            },
            Message::ActionDone(result) => match result {
                Ok(()) => self.overlay = None,
                Err(error) => self.overlay_error(error),
            },
            Message::Overlay(message) => return self.overlay_message(message),
            Message::Dialog(message) => return self.dialog_message(message),
            Message::Event(CoreEvent::HomePressed) => {
                if self.overlay.is_some()
                    || self.dialog.is_some()
                    || self.select.is_some()
                    || !self.screens.is_empty()
                {
                    self.screens.clear();
                    self.suspended_dialogs.clear();
                    self.dialog_parents.clear();
                    self.overlay = None;
                    self.dialog = None;
                    self.select = None;
                    self.searching = false;
                    self.folder = None;
                    return self.go_home();
                }
                self.folder = None;
                if self.searching {
                    self.searching = false;
                } else if self.pager.committed() != self.home_index() {
                    return self.go_home();
                } else {
                    return self.open_search();
                }
            }
            Message::Event(CoreEvent::HomePressedFromOverlay) => {
                self.screens.clear();
                self.suspended_dialogs.clear();
                self.dialog_parents.clear();
                self.overlay = None;
                self.dialog = None;
                self.select = None;
                self.searching = false;
                self.folder = None;
                return self.go_home();
            }
            Message::Event(CoreEvent::BackPressed) | Message::Back => {
                if self.select.is_some() {
                    self.select = None;
                } else if self.dialog.is_some() {
                    self.dialog = self.dialog_parents.pop();
                } else if !self.screens.is_empty() {
                    return self.update(Message::PopScreen);
                } else if self.overlay.is_some() {
                    return self.overlay_message(overlay::Message::Back);
                } else {
                    self.searching = false;
                    self.folder = None;
                    self.error = None;
                }
            }
            Message::Event(CoreEvent::Error(error)) => self.error = Some(error),
            Message::Event(_) => return work(|| block_on(core::snapshot()), Message::Loaded),
            Message::Step(delta) => self.step(delta),
            Message::Frame(now) => {
                if self.pager.tick(now) {
                    let reset = self.reset_scroll();
                    return Task::batch([reset, self.warm_icons()]);
                }
            }
            Message::Search => {
                if self.folder.is_some()
                    || self.overlay.is_some()
                    || self.dialog.is_some()
                    || self.select.is_some()
                    || !self.screens.is_empty()
                {
                    return Task::none();
                }
                if self.searching {
                    self.searching = false;
                } else {
                    return self.open_search();
                }
            }
            Message::Query(query) => {
                self.query = query.clone();
                let reset = self.scroll_region_to_top(Region::Search);
                let search = work(
                    move || {
                        let hits = block_on(core::search_all_apps(query.clone()))
                            .into_iter()
                            .map(|h| h.app)
                            .collect();
                        (query, hits)
                    },
                    |(query, hits)| Message::Results(query, hits),
                );
                return Task::batch([reset, search]);
            }
            Message::Results(query, hits) => {
                if query == self.query {
                    self.results = hits;
                }
            }
            Message::History(recent) => {
                self.show_recent = recent;
                let reset = self.scroll_region_to_top(Region::Search);
                return Task::batch([reset, self.warm_icons()]);
            }
            Message::HistoryLoaded(frequent, recent) => {
                self.frequent = frequent;
                self.recent = recent;
            }
            Message::ItemPressed(item) => {
                if self.overlay.is_some()
                    || !self.screens.is_empty()
                    || self.dialog.is_some()
                    || self.select.is_some()
                    || self
                        .folder
                        .as_ref()
                        .is_some_and(|folder| &item.group_id != folder)
                {
                    return Task::none();
                }
                let target = Start::Item(item.id.clone());
                self.pressed = Some(item.id);
                self.pending_target = Some(target.clone());
                let outcome = self.gesture.identify(target);
                return self.apply_gesture(outcome);
            }
            Message::BlankPressed => {
                if self.overlay.is_some()
                    || !self.screens.is_empty()
                    || self.dialog.is_some()
                    || self.select.is_some()
                    || self.folder.is_some()
                {
                    return Task::none();
                }
                self.pressed = None;
                self.pending_target = Some(Start::Blank);
                let outcome = self.gesture.identify(Start::Blank);
                return self.apply_gesture(outcome);
            }
            Message::FolderCardPressed => {}
            Message::FolderDismiss => {
                if self.overlay.is_none()
                    && self.screens.is_empty()
                    && self.dialog.is_none()
                    && self.select.is_none()
                    && self.folder.is_some()
                {
                    self.folder = None;
                    self.suppress_dismiss_pointer = true;
                    self.pressed = None;
                    self.pending_target = None;
                    self.gesture = Gesture::new();
                }
            }
            Message::LongPressTimer(sequence) => {
                let outcome = self.gesture.tick(sequence, Instant::now());
                return self.apply_gesture(outcome);
            }
            Message::Mouse(event) => return self.mouse_event(event),
            Message::Launched(result) => {
                if let Err(error) = result {
                    self.error = Some(error);
                }
            }
            Message::Icons(icons) => {
                for (app, handle) in icons {
                    self.pending.remove(&app);
                    self.icons.insert(app, handle);
                }
            }
            Message::Scrolled(region, offset) => {
                let previous = self.scroll.insert(region, offset).unwrap_or(0.0);
                if (previous - offset).abs() > f32::EPSILON && !self.gesture.is_pending() {
                    self.scroll_last = Some(Instant::now());
                }
                let generation = self.accessibility.scroll_started();
                return Task::perform(
                    async move { tokio::time::sleep(accessibility::QUIESCENCE).await },
                    move |_| Message::AccessibilitySettled(generation),
                );
            }
            Message::ScrollTick => {
                if self
                    .scroll_last
                    .is_some_and(|last| last.elapsed() >= SCROLLBAR_IDLE)
                {
                    self.scroll_last = None;
                }
            }
            Message::AccessibilitySettled(generation) => {
                self.accessibility.scroll_settled(generation);
            }
            Message::Accessibility(request) => {
                return self.accessibility_action(request);
            }
            Message::Touch(event) => return self.touch_event(event),
        }
        self.warm_icons()
    }

    /// Dispatches one TalkBack action against the last published frame. Stale
    /// revisions, unknown nodes and unadvertised actions are ignored, and page
    /// content is frozen while the pager moves.
    fn accessibility_action(&mut self, request: ActionRequest) -> Task<Message> {
        use accessibility::{Action, TextField};

        let Some(frame) = self.accessibility.frame.clone() else {
            return Task::none();
        };
        if request.revision != self.accessibility.publisher.revision() {
            return Task::none();
        }
        let Some(node) = frame
            .nodes
            .iter()
            .find(|node| node.id == request.node_id)
            .cloned()
        else {
            return Task::none();
        };
        if !node.actions.contains(&request.action) {
            return Task::none();
        }

        if self.pager.is_dragging() || self.pager.is_animating() {
            if matches!(
                request.action,
                Action::ScrollForward
                    | Action::ScrollBackward
                    | Action::ScrollLeft
                    | Action::ScrollRight
            ) {
                return Task::none();
            }
            match node.target {
                Target::Item(_) => return Task::none(),
                Target::TextField(TextField::Search) if request.action == Action::SetText => {
                    return Task::none();
                }
                _ => {}
            }
        }

        self.accessibility.cancel_scroll();
        match request.action {
            Action::ScrollLeft => return self.update(Message::Step(-1)),
            Action::ScrollRight => return self.update(Message::Step(1)),
            Action::Dismiss => return self.update(Message::Back),
            _ => {}
        }

        match node.target {
            Target::Message(message) => self.update(*message),
            Target::Slider { step_up, step_down } => match request.action {
                Action::ScrollForward => self.update(*step_up),
                Action::ScrollBackward => self.update(*step_down),
                _ => Task::none(),
            },
            Target::Input {
                widget_id,
                on_input,
            } => match request.action {
                Action::SetText => self.update(on_input(request.value)),
                Action::Click => iced::widget::operation::focus(widget_id),
                _ => Task::none(),
            },
            Target::Item(id) => match request.action {
                Action::LongClick => self.apply_gesture(Outcome::LongPress(Start::Item(id))),
                Action::Click => {
                    let item = self.snapshot.as_ref().and_then(|snapshot| {
                        snapshot.items.iter().find(|item| item.id == id).cloned()
                    });
                    match item {
                        Some(item) => self.activate(item),
                        None => Task::none(),
                    }
                }
                _ => Task::none(),
            },
            Target::Step(delta) => self.update(Message::Step(delta)),
            Target::Search => self.update(Message::Search),
            Target::SearchTab(recent) => self.update(Message::History(recent)),
            Target::TextField(field) => {
                self.accessibility_text_field(field, request.action, request.value)
            }
            Target::Scroll {
                region,
                offset,
                viewport,
                content,
            } => match request.action {
                Action::ScrollForward => {
                    self.accessibility_scroll(region, offset + 0.8 * viewport, viewport, content)
                }
                Action::ScrollBackward => {
                    self.accessibility_scroll(region, offset - 0.8 * viewport, viewport, content)
                }
                Action::LongClick => {
                    self.open_launcher_dialog();
                    Task::none()
                }
                _ => Task::none(),
            },
            Target::Overlay(message) => self.overlay_message(message),
            Target::Dismiss => self.overlay_message(overlay::Message::Back),
        }
    }

    fn accessibility_text_field(
        &mut self,
        field: accessibility::TextField,
        action: accessibility::Action,
        value: String,
    ) -> Task<Message> {
        use accessibility::{Action, TextField};
        use iced::widget::operation;

        match field {
            TextField::Search => {
                let focus = operation::focus("search");
                if action == Action::SetText {
                    Task::batch([focus, self.update(Message::Query(value))])
                } else {
                    focus
                }
            }
            TextField::AppName => {
                let focus = operation::focus(overlay::APP_NAME_FIELD_ID);
                if action == Action::SetText {
                    Task::batch([
                        focus,
                        self.overlay_message(overlay::Message::AppRename(value)),
                    ])
                } else {
                    focus
                }
            }
            TextField::FolderName => {
                let focus = operation::focus(overlay::FOLDER_NAME_FIELD_ID);
                if action == Action::SetText {
                    Task::batch([
                        focus,
                        self.overlay_message(overlay::Message::LauncherFolderName(value)),
                    ])
                } else {
                    focus
                }
            }
        }
    }

    fn accessibility_scroll(
        &mut self,
        region: Region,
        next: f32,
        viewport: f32,
        content: f32,
    ) -> Task<Message> {
        let maximum = (content - viewport).max(0.0);
        let target = next.clamp(0.0, maximum);
        self.scroll.insert(region.clone(), target);
        iced::widget::operation::scroll_to(
            self.scroll_widget_id(&region),
            iced::widget::operation::AbsoluteOffset {
                x: Some(0.0),
                y: Some(target),
            },
        )
    }

    fn at_top(&self) -> bool {
        self.current_group_id()
            .and_then(|group| self.scroll.get(&Region::Group(group.clone())))
            .copied()
            .unwrap_or(0.0)
            <= 0.5
    }

    fn long_press_timer(sequence: u64) -> Task<Message> {
        Task::perform(
            async move { tokio::time::sleep(gesture::LONG_PRESS).await },
            move |_| Message::LongPressTimer(sequence),
        )
    }

    /// Starts a gesture for the finger that went down and schedules the
    /// one-shot long-press deadline. A target that arrived before the raw
    /// event (desktop mouse areas) is applied now. Otherwise a press below the
    /// chrome is blank: the scrollable captures every touch before a wrapper
    /// `mouse_area` can report one, so the launcher infers it from geometry.
    fn begin_gesture(
        &mut self,
        finger: iced::touch::Finger,
        position: Point,
        now: Instant,
    ) -> Task<Message> {
        let sequence = self.gesture.pressed(finger, position, now);
        self.pressed = None;
        let mut tasks = vec![Self::long_press_timer(sequence)];
        let target = self.pending_target.take().or_else(|| {
            (self.folder.is_none()
                && position.y >= self.body_top()
                && position.y <= self.size.height - self.insets.bottom)
                .then_some(Start::Blank)
        });
        if let Some(target) = target {
            let outcome = self.gesture.identify(target);
            tasks.push(self.apply_gesture(outcome));
        }
        Task::batch(tasks)
    }

    /// Top edge of the tappable grid. Everything above it is chrome: the title
    /// strip, the folder header or the search bar. A press there is not blank,
    /// so it can neither lock the screen nor open the launcher menu.
    fn body_top(&self) -> f32 {
        let Some(snapshot) = &self.snapshot else {
            return f32::INFINITY;
        };
        if self.searching {
            return f32::INFINITY;
        }
        let settings = &snapshot.settings;
        let top = self.insets.top;
        if self.folder.is_some() {
            return top + grid::title_height(settings);
        }
        if grid::tabs_visible(settings, self.size, snapshot.page_ids.len()) {
            top + grid::title_height(settings)
        } else {
            top
        }
    }

    fn touch_event(&mut self, event: iced::touch::Event) -> Task<Message> {
        if !self.screens.is_empty() || self.dialog.is_some() || self.select.is_some() {
            self.gesture = Gesture::new();
            self.pending_target = None;
            self.pressed = None;
            return Task::none();
        }
        if self.suppress_dismiss_pointer {
            if matches!(
                event,
                iced::touch::Event::FingerLifted { .. } | iced::touch::Event::FingerLost { .. }
            ) {
                self.suppress_dismiss_pointer = false;
            }
            return Task::none();
        }
        match event {
            iced::touch::Event::FingerPressed { id, position } => {
                if self.overlay.is_some() {
                    self.pressed = None;
                    self.pending_target = None;
                    return Task::none();
                }
                self.begin_gesture(id, position, Instant::now())
            }
            iced::touch::Event::FingerMoved { id, position } => {
                if self.overlay.is_some() {
                    self.pressed = None;
                    return Task::none();
                }
                let at_top = self.at_top();
                let outcome = self.gesture.moved(id, position, Instant::now(), at_top);
                if !self.gesture.is_tap_candidate() {
                    self.pressed = None;
                }
                self.apply_gesture(outcome)
            }
            iced::touch::Event::FingerLifted { id, position } => {
                let outcome = self.gesture.released(id, position, Instant::now());
                self.pressed = None;
                self.pending_target = None;
                if self.overlay.is_some() {
                    Task::none()
                } else {
                    self.apply_gesture(outcome)
                }
            }
            iced::touch::Event::FingerLost { id, .. } => {
                let outcome = self.gesture.lost(id);
                self.pressed = None;
                self.pending_target = None;
                if self.overlay.is_some() {
                    Task::none()
                } else {
                    self.apply_gesture(outcome)
                }
            }
        }
    }

    /// Desktop preview: the mouse drives the same classifier the touchscreen
    /// does, with finger 0 standing in for the pointer.
    fn mouse_event(&mut self, event: iced::mouse::Event) -> Task<Message> {
        use iced::mouse::{Button, Event};
        if !self.screens.is_empty() || self.dialog.is_some() || self.select.is_some() {
            if let Event::CursorMoved { position } = event {
                self.cursor = position;
            }
            self.gesture = Gesture::new();
            self.pending_target = None;
            self.pressed = None;
            return Task::none();
        }
        if self.suppress_dismiss_pointer {
            if matches!(event, Event::ButtonReleased(Button::Left)) {
                self.suppress_dismiss_pointer = false;
            }
            return Task::none();
        }
        let finger = iced::touch::Finger(0);
        match event {
            Event::CursorMoved { position } => {
                self.cursor = position;
                if self.overlay.is_some() || self.gesture.phase() == gesture::Phase::Idle {
                    return Task::none();
                }
                let at_top = self.at_top();
                let outcome = self.gesture.moved(finger, position, Instant::now(), at_top);
                if !self.gesture.is_tap_candidate() {
                    self.pressed = None;
                }
                self.apply_gesture(outcome)
            }
            Event::ButtonPressed(Button::Left) => {
                if self.overlay.is_some() {
                    return Task::none();
                }
                self.begin_gesture(finger, self.cursor, Instant::now())
            }
            Event::ButtonReleased(Button::Left) => {
                let outcome = self.gesture.released(finger, self.cursor, Instant::now());
                self.pressed = None;
                self.pending_target = None;
                if self.overlay.is_some() {
                    Task::none()
                } else {
                    self.apply_gesture(outcome)
                }
            }
            _ => Task::none(),
        }
    }

    fn apply_gesture(&mut self, outcome: Outcome) -> Task<Message> {
        if !self.screens.is_empty() || self.dialog.is_some() || self.select.is_some() {
            self.pressed = None;
            self.pending_target = None;
            return Task::none();
        }
        if self.folder.is_some()
            && !matches!(
                &outcome,
                Outcome::Tap(Start::Item(_)) | Outcome::LongPress(Start::Item(_)) | Outcome::None
            )
        {
            self.pressed = None;
            return Task::none();
        }
        let width = grid::content_width(self.size, self.insets);
        match outcome {
            Outcome::None | Outcome::Tap(Start::Blank) => {}
            Outcome::DragStart => self.pager.begin_drag(),
            Outcome::Drag => {
                let dx = self.gesture.drag_dx();
                self.pager.drag(dx, width);
            }
            Outcome::DragEnd { velocity } => {
                let duration = self.transition();
                self.pager
                    .end_drag(velocity, width, Instant::now(), duration);
            }
            Outcome::Cancelled => {
                let duration = self.transition();
                self.pager.cancel_drag(Instant::now(), duration);
            }
            Outcome::PullDown => {
                self.pressed = None;
                return work(|| block_on(core::open_notifications()), Message::ActionDone);
            }
            Outcome::LongPress(Start::Item(id)) => {
                let item = self
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.items.iter().find(|item| item.id == id))
                    .cloned();
                if let Some(item) = item {
                    if item.folder_group_id.is_some() {
                        self.open_folder_dialog(item);
                    } else {
                        self.open_app_dialog(item);
                    }
                }
                self.pressed = None;
            }
            Outcome::LongPress(Start::Blank) => self.open_launcher_dialog(),
            Outcome::Tap(Start::Item(id)) => {
                let item = self
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.items.iter().find(|item| item.id == id))
                    .cloned();
                if let Some(item) = item.filter(|item| {
                    self.folder
                        .as_ref()
                        .is_none_or(|folder| &item.group_id == folder)
                }) {
                    return self.activate(item);
                }
            }
            Outcome::DoubleTap => {
                self.pressed = None;
                return work(|| block_on(core::lock_screen()), Message::ActionDone);
            }
        }
        Task::none()
    }

    fn activate(&mut self, item: Item) -> Task<Message> {
        if let Some(folder) = item.folder_group_id {
            self.searching = false;
            self.folder = Some(folder);
            let reset = self.reset_scroll();
            return Task::batch([reset, self.warm_icons()]);
        }
        work(
            move || {
                block_on(async {
                    if let Some(app) = item.app {
                        core::launch_app(app.package, app.activity).await
                    } else {
                        core::launch_shortcut(item.id).await
                    }
                })
            },
            Message::Launched,
        )
    }

    fn open_app_dialog(&mut self, item: Item) {
        self.dialog_generation += 1;
        self.dialog = Some(dialog::Dialog::App(dialog::AppEditor::new(item)));
        self.select = None;
        self.pressed = None;
        self.pending_target = None;
    }

    fn open_folder_dialog(&mut self, item: Item) {
        self.dialog_generation += 1;
        let Some(folder_id) = item.folder_group_id.clone() else {
            return;
        };
        self.dialog = Some(dialog::Dialog::Folder(dialog::FolderEditor::edit(
            folder_id,
            item.label.clone(),
            item.group_id.clone(),
        )));
        self.select = None;
        self.pressed = None;
        self.pending_target = None;
    }

    fn open_launcher_dialog(&mut self) {
        self.dialog_generation += 1;
        self.dialog = Some(dialog::Dialog::Launcher);
        self.select = None;
        self.pressed = None;
        self.pending_target = None;
    }

    /// The list of page groups and folders a dialog's select can choose.
    fn select_options(&self, field: &dialog::SelectField) -> Vec<dialog::SelectOption> {
        let Some(snapshot) = &self.snapshot else {
            return Vec::new();
        };
        match field {
            dialog::SelectField::AppGroup => dialog::destinations(snapshot),
            dialog::SelectField::FolderGroup => {
                let hidden = snapshot
                    .groups
                    .iter()
                    .filter(|group| group.role == Some(qlaunch_core::model::GroupRole::Hidden))
                    .map(|group| group.id.clone())
                    .collect::<Vec<_>>();
                dialog::destinations(snapshot)
                    .into_iter()
                    .filter(|option| !option.folder && !hidden.contains(&option.id))
                    .collect()
            }
        }
    }

    fn open_select(&mut self, field: dialog::SelectField) {
        let selected = match (&field, &self.dialog) {
            (dialog::SelectField::AppGroup, Some(dialog::Dialog::App(editor))) => {
                editor.group_id.clone()
            }
            (dialog::SelectField::FolderGroup, Some(dialog::Dialog::Folder(editor))) => {
                editor.group_id().clone()
            }
            _ => return,
        };
        let mut options = self.select_options(&field);
        if !options.iter().any(|option| option.id == selected)
            && let Some(group) = self
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.groups.iter().find(|group| group.id == selected))
        {
            options.push(dialog::SelectOption {
                id: group.id.clone(),
                label: group.label.clone(),
                folder: group.folder,
            });
        }
        self.select = Some(dialog::Select {
            field,
            options,
            selected,
        });
    }

    #[allow(dead_code)] // exercised by the legacy overlay tests only
    fn open_app_menu(&mut self, item: Item) {
        self.scroll.insert(Region::OverlayCard, 0.0);
        self.overlay = Some(overlay::Overlay::App(overlay::AppMenu {
            item,
            step: overlay::AppStep::Actions,
            error: None,
        }));
        self.pressed = None;
        self.pending_target = None;
    }

    pub(crate) fn visible_items(&self) -> Vec<&Item> {
        let Some(s) = &self.snapshot else {
            return vec![];
        };
        if self.searching {
            let refs = if !self.query.is_empty() {
                &self.results
            } else if self.show_recent {
                &self.recent
            } else {
                &self.frequent
            };
            refs.iter()
                .filter_map(|app| self.item_by_app.get(app).map(|&index| &s.items[index]))
                .take(if self.query.is_empty() {
                    self.columns() * 3
                } else {
                    usize::MAX
                })
                .collect()
        } else {
            let group = self
                .folder
                .as_ref()
                .or_else(|| s.page_ids.get(self.pager.committed()));
            s.items
                .iter()
                .filter(|i| Some(&i.group_id) == group)
                .collect()
        }
    }

    /// Shared with the frame-phase probe in the view tree.
    pub(crate) fn metrics_handle(&self) -> Arc<Mutex<metrics::Samples>> {
        self.metrics.clone()
    }

    pub(crate) fn columns(&self) -> usize {
        let Some(s) = self.snapshot.as_ref() else {
            return 4;
        };
        if s.settings.layout == LayoutStyle::List {
            return 1;
        }
        let configured = if self.size.width > self.size.height {
            s.settings.grid_columns_landscape
        } else {
            s.settings.grid_columns_portrait
        };
        // Reduce the column count when it cannot hold a full-size icon, so a
        // narrow window or a brief stale size never shrinks the icons.
        crate::grid::fit_columns(
            configured.max(1) as usize,
            crate::grid::content_width(self.size, self.insets),
            &s.settings,
        )
    }

    fn warm_icons(&mut self) -> Task<Message> {
        if self.background || !self.pending.is_empty() {
            return Task::none();
        }
        let Some(snapshot) = &self.snapshot else {
            return Task::none();
        };
        if !snapshot.settings.icons_visible {
            return Task::none();
        }
        // A bounded decoded working set (~24 MiB at 162px), not a second raster
        // cache of whole tiles. Clear at the cap; current page is loaded first.
        let size = (snapshot.settings.icon_size as f32 * self.density)
            .round()
            .clamp(1.0, 300.0) as u32;
        let limit = (24 * 1024 * 1024 / (size * size * 4)).max(1) as usize;
        let wanted: Vec<_> = if self.searching || self.folder.is_some() {
            self.visible_items()
                .into_iter()
                .filter_map(|i| i.app.clone())
                .take(limit)
                .filter(|a| !self.icons.contains_key(a))
                .collect()
        } else {
            // The current page first, then its neighbours, so a fling never
            // lands on placeholder icons.
            let slots = self.pager.slots();
            let order = [slots[1], slots[0], slots[2]];
            let mut groups: Vec<&Id> = Vec::new();
            for index in order {
                if let Some(group) = snapshot.page_ids.get(index)
                    && !groups.contains(&group)
                {
                    groups.push(group);
                }
            }
            groups
                .into_iter()
                .flat_map(|group| {
                    snapshot
                        .items
                        .iter()
                        .filter(move |item| &item.group_id == group)
                })
                .filter_map(|i| i.app.clone())
                .take(limit)
                .filter(|a| !self.icons.contains_key(a))
                .collect()
        };
        if wanted.is_empty() {
            return Task::none();
        }
        if self.icons.len() + wanted.len() > limit {
            self.icons.clear();
        }
        let wanted: Vec<_> = wanted
            .into_iter()
            .take(limit.saturating_sub(self.pending.len()))
            .collect();
        self.pending.extend(wanted.iter().cloned());
        work(
            move || {
                wanted
                    .into_iter()
                    .map(|app| {
                        let icon = block_on(core::icon_for(
                            app.package.clone(),
                            app.activity.clone(),
                            size,
                        ));
                        // Keep a cheap placeholder for a failed icon, so it isn't retried on
                        // every unrelated UI message.
                        let handle = match icon {
                            Some(icon) if icon.format == IconFormat::Rgba => {
                                image::Handle::from_rgba(icon.width, icon.height, icon.bytes)
                            }
                            Some(icon) => image::Handle::from_bytes(icon.bytes),
                            None => image::Handle::from_rgba(1, 1, vec![150, 150, 150, 255]),
                        };
                        (app, handle)
                    })
                    .collect()
            },
            Message::Icons,
        )
    }

    fn view(&self, _: window::Id) -> Element<'_, Message> {
        let started = Instant::now();
        let element = view::launcher(self);
        let elapsed = started.elapsed().as_secs_f32() * 1000.0;
        self.metrics.lock().unwrap().record_view(elapsed);
        element
    }
}
