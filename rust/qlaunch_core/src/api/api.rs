use std::path::PathBuf;

use flutter_rust_bridge::frb;

use crate::frb_generated::StreamSink;
use crate::model::{AppRef, Group, GroupRole, Id, Item};
use crate::platform::AppSource;
use crate::platform::linux::LinuxSource;
use crate::search::{SearchCandidate, search};
use crate::settings::{Settings, SettingsPatch};
use crate::store;

pub use crate::search::SearchHit;

use super::events::CoreEvent;
use super::state;

pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Everything Dart needs to draw the launcher.
///
/// `page_ids` is the page order, computed by Rust from `Model::pages()`; Dart
/// must not recompute it. `items` is in display order within each group (see
/// the field). `warnings` is what is wrong right now (for example
/// that changes are not being saved), and does not grow from call to call.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSnapshot {
    pub root_group_id: Id,
    /// The group holding the Home role, or `None` if no group does. Dart must
    /// use this and must not look for a group with the Home role itself: how
    /// the Home group is found is a model rule, and it lives in Rust.
    pub home_group_id: Option<Id>,
    pub page_ids: Vec<Id>,
    pub groups: Vec<Group>,
    /// Every item, in **display order within its group**: filtering by
    /// `group_id` yields exactly what `Model::items_in` returns for that group.
    /// Consumers filter by group and must not sort again, since that would
    /// duplicate the model's ordering rule. The order *between* groups is
    /// unspecified.
    pub items: Vec<Item>,
    pub settings: Settings,
    pub warnings: Vec<String>,
}

/// Starts the core: loads the saved layout (or creates it), reconciles it with
/// the installed apps, and starts watching for changes.
///
/// The configuration directory is `config_root_override` if given, else the
/// user's (`store::config_root`). If neither is usable this still succeeds,
/// with the defaults and a warning that nothing will be saved. Calling it again
/// replaces the running state.
pub async fn init(config_root_override: Option<String>) -> Result<ModelSnapshot, String> {
    let (root, warnings) = match config_root_override {
        Some(path) => (Some(PathBuf::from(path)), Vec::new()),
        None => match store::config_root() {
            Ok(root) => (Some(root), Vec::new()),
            Err(e) => (
                None,
                vec![format!(
                    "The configuration directory could not be found ({e})."
                )],
            ),
        },
    };
    state::install(root, Box::new(LinuxSource::new()), warnings)
}

/// The testable core of [`init`], not part of the bridge surface.
///
/// `config_root: None` means there is no usable configuration directory.
/// `warnings` is whatever the caller already knows (a failed root resolution).
/// Everything after that is what `init` does.
#[doc(hidden)]
#[frb(ignore)]
pub fn init_with_source(
    config_root: Option<PathBuf>,
    source: Box<dyn AppSource>,
    warnings: Vec<String>,
) -> Result<ModelSnapshot, String> {
    state::install(config_root, source, warnings)
}

/// Empties the state and the event subscribers. For tests only.
#[doc(hidden)]
#[frb(ignore)]
pub fn reset_for_test() {
    state::clear();
}

/// Registers a plain Rust subscriber, which returns `false` once it is gone.
/// For tests only.
#[doc(hidden)]
#[frb(ignore)]
pub fn subscribe_for_test(
    deliver: Box<dyn Fn(CoreEvent) -> bool + Send + Sync>,
) -> Result<(), String> {
    state::subscribe(Box::new(move |event| deliver(event.clone())));
    Ok(())
}

pub async fn snapshot() -> Result<ModelSnapshot, String> {
    state::snapshot_now()
}

/// Re-reads the installed apps and brings the model up to date. If the apps
/// cannot be listed the model is left as it is and the snapshot carries a
/// warning.
pub async fn refresh_apps() -> Result<ModelSnapshot, String> {
    state::refresh_apps_now()
}

pub async fn add_group(label: String) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| {
        m.add_group(&label);
        Ok(())
    })
}

pub async fn rename_group(id: Id, label: String) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| m.rename_group(&id, &label))
}

pub async fn move_group(id: Id, new_order: i32) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| m.move_group(&id, new_order))
}

pub async fn set_group_role(id: Id, role: Option<GroupRole>) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| m.set_group_role(&id, role))
}

pub async fn delete_group(id: Id) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| m.delete_group(&id))
}

pub async fn move_item(id: Id, to_group: Id, new_order: i32) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| m.move_item(&id, &to_group, new_order))
}

pub async fn rename_item(id: Id, label: String) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| m.rename_item(&id, &label))
}

/// Stores `png` as the custom icon of the item `id`. An error if there is no
/// configuration directory to keep it in.
pub async fn set_item_icon(id: Id, png: Vec<u8>) -> Result<ModelSnapshot, String> {
    state::set_icon(&id, &png)
}

pub async fn remove_item(id: Id) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| m.remove_item(&id))
}

pub async fn sort_group(id: Id) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| m.sort_group(&id))
}

pub async fn update_settings(patch: SettingsPatch) -> Result<ModelSnapshot, String> {
    state::mutate_settings(|s| {
        s.apply(&patch);
        s.clamp();
    })
}

/// `None` before `init`, or when no group holds the Home role.
pub async fn home_group_id() -> Option<Id> {
    state::with_app(|app, _| Ok(app.model.home_group_id()))
        .ok()
        .flatten()
}

/// Searches every app in the launcher, by the label shown for it. Apps that
/// are marked uninstalled are not included. Empty before `init`.
pub async fn search_all_apps(query: String) -> Vec<SearchHit> {
    let candidates = state::with_app(|app, _| {
        Ok(app
            .model
            .items
            .iter()
            .filter(|i| !i.uninstalled)
            .filter_map(|i| {
                i.app.as_ref().map(|a| SearchCandidate {
                    app: a.clone(),
                    label: i.label.clone(),
                })
            })
            .collect::<Vec<_>>())
    })
    .unwrap_or_default();
    search(&candidates, &query)
}

/// The platform's PNG icon for an app, or `None` if there is none, it cannot
/// be read, or `init` has not been called.
pub async fn icon_for(package: String, activity: String, size_px: u32) -> Option<Vec<u8>> {
    let source = state::with_app(|app, _| Ok(app.source.clone())).ok()?;
    source
        .icon_png(&AppRef { package, activity }, size_px)
        .ok()
        .flatten()
}

pub async fn launch_app(package: String, activity: String) -> Result<(), String> {
    let source = state::with_app(|app, _| Ok(app.source.clone()))?;
    source
        .launch(&AppRef { package, activity })
        .map_err(|e| state::launch_error(&e))
}

/// Streams the core's events to Dart. A subscriber that misses an event can
/// always re-read `snapshot()`. The subscription ends when the stream is
/// closed.
pub fn events(sink: StreamSink<CoreEvent>) -> Result<(), String> {
    state::subscribe(Box::new(move |event| sink.add(event.clone()).is_ok()));
    Ok(())
}
