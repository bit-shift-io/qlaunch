//! Throwaway Android/desktop comparison UI. The core remains the source of truth.
mod accessibility;
pub mod gesture;
pub mod grid;
mod metrics;
mod overlay;
pub mod pager;
mod placeholder;
mod slots;
mod view;

use accessibility::{ActionRequest, Target};
use gesture::{Gesture, Outcome, Start};
use pager::Pager;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{Action, Target};
    use qlaunch_core::{model::Model, settings::Settings};

    fn app() -> App {
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
    fn scroll_offsets_are_tracked_per_region() {
        let mut app = app();
        let _ = app.update(Message::Scrolled(Region::Group("g".into()), 42.0));
        let _ = app.update(Message::Scrolled(Region::Search, 12.0));
        let _ = app.update(Message::Scrolled(Region::OverlayList, 7.0));
        assert_eq!(app.scroll.get(&Region::Group("g".into())), Some(&42.0));
        assert_eq!(app.scroll.get(&Region::Search), Some(&12.0));
        assert_eq!(app.scroll.get(&Region::OverlayList), Some(&7.0));
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
        let _ = app.update(Message::Scrolled(Region::OverlayList, 42.0));
        assert_eq!(app.scroll.get(&Region::OverlayList), Some(&42.0));
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

    fn item(app: &App, id: &str) -> Item {
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

    fn folder_item(app: &App) -> Item {
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
    fn long_press_opens_the_app_menu_and_clears_the_pressed_cell() {
        let mut app = app();
        let app_item = item(&app, "calculator");
        app.snapshot.as_mut().unwrap().items.push(app_item.clone());
        app.pressed = Some(app_item.id.clone());
        let _ = app.apply_gesture(Outcome::LongPress(Start::Item(app_item.id.clone())));
        assert!(matches!(app.overlay, Some(overlay::Overlay::App(_))));
        assert!(app.pressed.is_none());
    }

    #[test]
    fn long_press_on_blank_opens_the_launcher_menu() {
        let mut app = app();
        let _ = app.apply_gesture(Outcome::LongPress(Start::Blank));
        assert!(matches!(app.overlay, Some(overlay::Overlay::Launcher(_))));
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
        .title(|_: &App, _| "QLaunch · iced comparison".into())
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
    app.run()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Region {
    Group(Id),
    Search,
    OverlayList,
}

impl Region {
    pub(crate) fn widget_id(&self) -> &'static str {
        match self {
            Region::Group(_) => "apps",
            Region::Search => "search-results",
            Region::OverlayList => "overlay-list",
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
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
    LongPressTimer(u64),
    Mouse(iced::mouse::Event),
    Launched(Result<(), String>),
    Back,
    Icons(Vec<(AppRef, image::Handle)>),
    Touch(iced::touch::Event),
    Resized(iced::Size),
    Density(f32),
    Scrolled(Region, f32),
    Frame(Instant),
    Overlay(overlay::Message),
    Mutated(Result<core::ModelSnapshot, String>),
    ActionDone(Result<(), String>),
    Accessibility(ActionRequest),
    AccessibilitySettled(u64),
}

struct App {
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
    pending: HashSet<AppRef>,
    pub(crate) error: Option<String>,
    pub(crate) size: iced::Size,
    scroll: HashMap<Region, f32>,
    overlay: Option<overlay::Overlay>,
    accessibility: accessibility::State,
    metrics: Mutex<metrics::Samples>,
    gesture: Gesture,
    pending_target: Option<Start>,
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
            pending: HashSet::new(),
            error: None,
            size: iced::Size::new(420.0, 800.0),
            scroll: HashMap::new(),
            overlay: None,
            accessibility: accessibility::State::default(),
            metrics: Mutex::new(metrics::Samples::default()),
            gesture: Gesture::new(),
            pending_target: None,
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
        Subscription::batch(subscriptions)
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
            region.widget_id(),
            iced::widget::operation::AbsoluteOffset {
                x: Some(0.0),
                y: Some(0.0),
            },
        )
    }

    fn go_home(&mut self) -> Task<Message> {
        let home = self.home_index();
        let transition = self.transition();
        self.pager.settle_to(home, Instant::now(), transition);
        Task::none()
    }

    /// Installs a new snapshot. Reloads keep the group on screen when it still
    /// exists; the first load starts on the core's home group.
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

    fn overlay_error(&mut self, error: String) {
        match &mut self.overlay {
            Some(overlay) => overlay.set_error(error),
            None => self.error = Some(error),
        }
    }

    fn overlay_message(&mut self, message: overlay::Message) -> Task<Message> {
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
                    overlay.set_error("Not ported in this spike".into());
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
        let started = Instant::now();
        let task = self.update_inner(message);
        let elapsed = started.elapsed().as_secs_f32() * 1000.0;
        let moving = self.pager.is_animating() || self.pager.is_dragging();
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
                | Message::AccessibilitySettled(_)
                | Message::Accessibility(_)
                | Message::Icons(_)
                | Message::Frame(_)
        ) {
            self.accessibility.cancel_scroll();
        }
        match message {
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
            Message::ActionDone(result) => match result {
                Ok(()) => self.overlay = None,
                Err(error) => self.overlay_error(error),
            },
            Message::Overlay(message) => return self.overlay_message(message),
            Message::Event(CoreEvent::HomePressed) => {
                if self.overlay.is_some() {
                    self.overlay = None;
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
                self.overlay = None;
                self.searching = false;
                self.folder = None;
                return self.go_home();
            }
            Message::Event(CoreEvent::BackPressed) | Message::Back => {
                if let Some(overlay) = &mut self.overlay {
                    if overlay.back() {
                        self.overlay = None;
                    }
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
                if self.overlay.is_some() {
                    return Task::none();
                }
                let target = Start::Item(item.id.clone());
                self.pressed = Some(item.id);
                self.pending_target = Some(target.clone());
                let outcome = self.gesture.identify(target);
                return self.apply_gesture(outcome);
            }
            Message::BlankPressed => {
                if self.overlay.is_some() {
                    return Task::none();
                }
                self.pressed = None;
                self.pending_target = Some(Start::Blank);
                let outcome = self.gesture.identify(Start::Blank);
                return self.apply_gesture(outcome);
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
                self.scroll.insert(region, offset);
                let generation = self.accessibility.scroll_started();
                return Task::perform(
                    async move { tokio::time::sleep(accessibility::QUIESCENCE).await },
                    move |_| Message::AccessibilitySettled(generation),
                );
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
            Action::Dismiss => return self.overlay_message(overlay::Message::Back),
            _ => {}
        }

        match node.target {
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
            Target::Back => self.update(Message::Back),
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
                    self.open_launcher_menu();
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
            region.widget_id(),
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
            (position.y >= self.body_top() && position.y <= self.size.height - self.insets.bottom)
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
                    self.open_app_menu(item);
                }
                self.pressed = None;
            }
            Outcome::LongPress(Start::Blank) => self.open_launcher_menu(),
            Outcome::Tap(Start::Item(id)) => {
                let item = self
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.items.iter().find(|item| item.id == id))
                    .cloned();
                if let Some(item) = item {
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
            return self.warm_icons();
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

    fn open_app_menu(&mut self, item: Item) {
        self.overlay = Some(overlay::Overlay::App(overlay::AppMenu {
            item,
            step: overlay::AppStep::Actions,
            error: None,
        }));
        self.pressed = None;
        self.pending_target = None;
    }

    fn open_launcher_menu(&mut self) {
        self.overlay = Some(overlay::Overlay::Launcher(overlay::LauncherMenu {
            step: overlay::LauncherStep::Actions,
            providers: vec![],
            providers_loading: false,
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
                .filter_map(|app| s.items.iter().find(|i| i.app.as_ref() == Some(app)))
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

    pub(crate) fn columns(&self) -> usize {
        self.snapshot
            .as_ref()
            .map(|s| {
                if s.settings.layout == LayoutStyle::List {
                    1
                } else if self.size.width > self.size.height {
                    s.settings.grid_columns_landscape as usize
                } else {
                    s.settings.grid_columns_portrait as usize
                }
            })
            .unwrap_or(4)
            .max(1)
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
