use std::path::PathBuf;

use flutter_rust_bridge::frb;

use crate::frb_generated::StreamSink;
use crate::model::{AppRef, Group, GroupRole, IconImage, Id, Item};
use crate::platform::AppSource;
#[cfg(not(target_os = "android"))]
use crate::platform::linux::LinuxSource;
#[cfg(target_os = "android")]
use crate::platform::android::{AndroidSource, jni_host::JniHost};
use crate::search::{SearchCandidate, search};
use crate::settings::{Settings, SettingsPatch};
#[cfg(not(target_os = "android"))]
use crate::store;

pub use crate::search::SearchHit;
pub use crate::usage::UsageHit;

use super::events::CoreEvent;
use super::state;

/// The platform's `AppSource`. Rust resolves this; Dart never chooses.
fn platform_source() -> Box<dyn AppSource> {
    #[cfg(target_os = "android")]
    {
        Box::new(AndroidSource::new(JniHost::new()))
    }
    #[cfg(not(target_os = "android"))]
    {
        Box::new(LinuxSource::new())
    }
}

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
    let (root, warnings) = choose_root(
        config_root_override.map(PathBuf::from),
        platform_config_root(),
    );
    state::install(root, platform_source(), warnings)
}

/// Chooses the configuration directory and the warnings `init` starts with: the
/// override if there is one, else the platform default, else nothing and a
/// warning that changes will not be saved.
fn choose_root(
    override_path: Option<PathBuf>,
    default: Result<PathBuf, String>,
) -> (Option<PathBuf>, Vec<String>) {
    match override_path {
        Some(path) => (Some(path), Vec::new()),
        None => match default {
            Ok(path) => (Some(path), Vec::new()),
            Err(e) => (
                None,
                vec![format!(
                    "The configuration directory could not be found ({e})."
                )],
            ),
        },
    }
}

/// Android hands its private files directory (`Context.filesDir`) through the
/// bridge; there is no usable XDG home there.
#[cfg(target_os = "android")]
fn platform_config_root() -> Result<PathBuf, String> {
    JniHost::files_dir().map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
fn platform_config_root() -> Result<PathBuf, String> {
    store::config_root().map_err(|e| e.to_string())
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

/// Emits a core event to subscribers. For tests only: the Android host emits
/// the Home and Back events itself, from the JNI callbacks.
#[doc(hidden)]
#[frb(ignore)]
pub fn emit_for_test(event: CoreEvent) {
    state::emit(event);
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

/// Shows or hides the group `id` from the pager and the tab strip.
pub async fn set_group_hidden(id: Id, hidden: bool) -> Result<ModelSnapshot, String> {
    state::mutate_model(|m| m.set_group_hidden(&id, hidden))
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

/// The platform's icon for an app, or `None` if there is none, it cannot
/// be read, or `init` has not been called.
pub async fn icon_for(package: String, activity: String, size_px: u32) -> Option<IconImage> {
    let target = AppRef { package, activity };
    let (source, cache, update_time) = state::with_app(|state, _| {
        // The update time is part of the cache key: an app that was updated
        // since the icon was stored must not keep the old one.
        let token = state
            .icon_tokens
            .get(&(target.package.clone(), target.activity.clone()))
            .copied()
            .unwrap_or(0);
        Ok((state.source.clone(), state.icon_cache.clone(), token))
    })
    .ok()?;

    // Check the persisted cache first: a restart should not re-run the
    // platform fetch for icons the previous run already stored.
    let generation = cache.as_ref().map(|cache| cache.generation());
    if let Some(image) = cache
        .as_ref()
        .and_then(|cache| cache.get(&target, size_px, update_time))
    {
        return Some(image);
    }

    let image = source.icon_image(&target, size_px).ok().flatten()?;
    if let (Some(cache), Some(generation)) = (cache.as_ref(), generation) {
        // A clear while this fetch was in flight means the icon is suspect, so
        // the write is dropped rather than resurrecting a stale entry.
        cache.put_if(generation, &target, size_px, update_time, &image);
    }
    Some(image)
}

/// The most-launched apps, in launch-count order, up to `limit`. Apps that are
/// not in the launcher, or are uninstalled, are not included. Empty before
/// `init`.
pub async fn frequent_apps(limit: u32) -> Vec<UsageHit> {
    state::usage_hits(limit, true)
}

/// The most-recently-launched apps, newest first, up to `limit`. The same
/// exclusions as [`frequent_apps`]. Empty before `init`.
pub async fn recent_apps(limit: u32) -> Vec<UsageHit> {
    state::usage_hits(limit, false)
}

/// Launches an app, and records the launch once it has opened. Only a launch
/// that succeeds is recorded, so Frequent and Recent never hold an app the user
/// did not actually open.
pub async fn launch_app(package: String, activity: String) -> Result<(), String> {
    let source = state::with_app(|app, _| Ok(app.source.clone()))?;
    let app = AppRef { package, activity };
    source.launch(&app).map_err(|e| state::launch_error(&e))?;
    state::record_launch(&app);
    Ok(())
}

/// Asks the system to uninstall `package_name`. On Android this opens the
/// system's confirmation; the app list is refreshed by the package-change
/// receiver, not here.
pub async fn uninstall_app(package_name: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        JniHost::uninstall(&package_name).map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = package_name;
        Err("uninstalling an app is only available on Android".into())
    }
}

/// Opens the system wallpaper chooser. Only Android has one; elsewhere this is
/// an error the settings screen reports like any other. There is no result: the
/// user chooses in the system UI, and the change arrives as
/// [`CoreEvent::WallpaperChanged`].
pub async fn choose_wallpaper() -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        JniHost::choose_wallpaper().map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "android"))]
    {
        Err("choosing a wallpaper is only available on Android".into())
    }
}

/// Streams the core's events to Dart. A subscriber that misses an event can
/// always re-read `snapshot()`. The subscription ends when the stream is
/// closed.
pub fn events(sink: StreamSink<CoreEvent>) -> Result<(), String> {
    state::subscribe(Box::new(move |event| sink.add(event.clone()).is_ok()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::choose_root;
    use std::path::PathBuf;

    #[test]
    fn choose_root_prefers_the_override() {
        let (root, warnings) =
            choose_root(Some(PathBuf::from("/tmp/x")), Err("no home".into()));
        assert_eq!(root, Some(PathBuf::from("/tmp/x")));
        assert!(warnings.is_empty());
    }

    #[test]
    fn choose_root_uses_the_default_when_there_is_no_override() {
        let (root, warnings) = choose_root(None, Ok(PathBuf::from("/tmp/y")));
        assert_eq!(root, Some(PathBuf::from("/tmp/y")));
        assert!(warnings.is_empty());
    }

    #[test]
    fn choose_root_warns_when_there_is_no_directory() {
        let (root, warnings) = choose_root(None, Err("no home".into()));
        assert_eq!(root, None);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("no home"));
    }
}
