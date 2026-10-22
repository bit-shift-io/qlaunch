use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::icons::IconDiskCache;
use crate::model::{AppEntry, AppRef, Item, Model, ModelError};
use crate::platform::backup::{backup_name, is_backup_name};
use crate::platform::{AppSource, LaunchError, PlatformError, WatchHandle};
use crate::reconcile::reconcile;
use crate::settings::Settings;
use crate::store::{Store, StoreError};
use crate::usage::{Usage, UsageHit};

use super::api::{IconRevision, ModelSnapshot};
use super::events::{CoreEvent, EventBus};

/// How much the on-disk icon cache may hold before its oldest entries go.
const DEFAULT_ICON_CACHE_BYTES: u64 = 32 * 1024 * 1024;

/// Everything the bridge keeps between calls.
///
/// The event bus lives beside the `App`, not in it, so subscribers survive a
/// second `init`.
pub(crate) struct Global {
    pub(crate) app: Option<App>,
    pub(crate) bus: EventBus,
}

pub(crate) struct App {
    /// `None` when there is no usable configuration directory: nothing is
    /// saved, and the app still runs on the defaults.
    pub(crate) store: Option<Store>,
    pub(crate) model: Model,
    pub(crate) settings: Settings,
    /// What has been launched, and when; saved beside the model and settings.
    pub(crate) usage: Usage,
    /// Icons already fetched, kept between runs under the config root. `None`
    /// when there is no usable configuration directory.
    pub(crate) icon_cache: Option<IconDiskCache>,
    /// Each installed app's last update time, so an app update can be told
    /// apart from a plain re-list. Not persisted.
    pub(crate) icon_tokens: HashMap<(String, String), i64>,
    pub(crate) source: Arc<dyn AppSource>,
    /// Owns the platform watcher. Dropping it joins a thread that calls back
    /// into this module and takes the state lock, so an `App` must never be
    /// dropped while that lock is held. See [`install`] and [`clear`].
    pub(crate) _watch: Option<WatchHandle>,
    /// What `Store::load` and the caller reported at startup. A one-off event,
    /// so it is fixed from then on.
    base_warnings: Vec<String>,
    /// The state of the last save: `Some` while changes are not being kept.
    save_problem: Option<String>,
    /// The state of the last `list_apps`: `Some` while the platform failed.
    platform_problem: Option<String>,
    watch_problem: Option<String>,
}

static STATE: Mutex<Global> = Mutex::new(Global {
    app: None,
    bus: EventBus::new(),
});

fn lock() -> MutexGuard<'static, Global> {
    // A panic while the lock was held leaves the state as it was at the last
    // complete change, because every mutation is applied to a copy first.
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn not_initialised() -> String {
    "NotInitialised: init must be called before anything else".into()
}

pub(crate) fn model_error(e: &ModelError) -> String {
    let tag = match e {
        ModelError::MissingRoot => "MissingRoot",
        ModelError::DuplicateRole(_) => "DuplicateRole",
        ModelError::MissingRole(_) => "MissingRole",
        ModelError::UnknownGroup(_) => "UnknownGroup",
        ModelError::UnknownItem(_) => "UnknownItem",
        ModelError::DuplicateAppRef(_) => "DuplicateAppRef",
        ModelError::RoleNotAssignable => "RoleNotAssignable",
        ModelError::GroupHoldsRole(_) => "GroupHoldsRole",
        ModelError::NotAPage(_) => "NotAPage",
        ModelError::DuplicateGroupId(_) => "DuplicateGroupId",
        ModelError::DuplicateItemId(_) => "DuplicateItemId",
        ModelError::RootHasParent(_) => "RootHasParent",
        ModelError::SecondRoot(_) => "SecondRoot",
        ModelError::NotChildOfRoot(_) => "NotChildOfRoot",
        ModelError::ItemInRoot(_) => "ItemInRoot",
        ModelError::ItemHasAppAndShortcut(_) => "ItemHasAppAndShortcut",
        ModelError::ItemHasShortcut(_) => "ItemHasShortcut",
    };
    format!("{tag}: {e}")
}

pub(crate) fn store_error(e: &StoreError) -> String {
    let tag = match e {
        StoreError::Io(_) => "Io",
        StoreError::Parse(_) => "Parse",
        StoreError::UnsupportedSchema(_) => "UnsupportedSchema",
        StoreError::Invalid(_) => "Invalid",
        StoreError::InvalidIconName(_) => "InvalidIconName",
        StoreError::WritesBlocked => "WritesBlocked",
    };
    format!("{tag}: {e}")
}

pub(crate) fn platform_error(e: &PlatformError) -> String {
    let tag = match e {
        PlatformError::Io(_) => "Io",
        PlatformError::NotFound => "NotFound",
        PlatformError::Unsupported(_) => "Unsupported",
    };
    format!("{tag}: {e}")
}

pub(crate) fn launch_error(e: &LaunchError) -> String {
    let tag = match e {
        LaunchError::NotFound(_) => "NotFound",
        LaunchError::Failed(_) => "Failed",
        LaunchError::Unsupported(_) => "Unsupported",
    };
    format!("{tag}: {e}")
}

/// Every item, ordered by `(group_id, order)`. The sort is stable, so items
/// with equal `order` keep their `Vec` order, which is exactly how
/// `Model::items_in` breaks ties. Filtering the result by a group id therefore
/// gives `items_in(group)`. The model's own `Vec` is not in display order,
/// because the mutations rewrite `order` and never move anything.
fn items_in_display_order(model: &Model) -> Vec<Item> {
    let mut items = model.items.clone();
    items.sort_by(|a, b| a.group_id.cmp(&b.group_id).then(a.order.cmp(&b.order)));
    items
}

/// Builds the snapshot Dart sees. The warnings are the startup ones plus
/// whatever is wrong right now, worked out afresh from the flags on `app`, so
/// the list does not grow with every call.
pub(crate) fn snapshot_of(app: &App) -> ModelSnapshot {
    let mut warnings = app.base_warnings.clone();
    if app.store.is_none() {
        warnings.push("There is no configuration directory, so nothing will be saved.".into());
    }
    warnings.extend(app.save_problem.clone());
    warnings.extend(app.platform_problem.clone());
    warnings.extend(app.watch_problem.clone());
    let mut icon_revisions: Vec<_> = app
        .icon_tokens
        .iter()
        .map(|((package, activity), revision)| IconRevision {
            app: AppRef {
                package: package.clone(),
                activity: activity.clone(),
            },
            revision: *revision,
        })
        .collect();
    icon_revisions
        .sort_by(|a, b| (&a.app.package, &a.app.activity).cmp(&(&b.app.package, &b.app.activity)));
    ModelSnapshot {
        root_group_id: app.model.root_group_id.clone(),
        home_group_id: app.model.home_group_id(),
        page_ids: app.model.pages().iter().map(|g| g.id.clone()).collect(),
        groups: app.model.groups.clone(),
        items: items_in_display_order(&app.model),
        settings: app.settings.clone(),
        warnings,
        icon_revisions,
    }
}

/// Saves `model` and `settings`, and says what that means.
///
/// `Ok(None)`: saved, or there is no store to save to (which `snapshot_of`
/// reports separately). `Ok(Some(warning))`: the change stands in memory but is
/// not being kept, because the write failed or the document is being left alone
/// (`WritesBlocked`). `Err`: the model was refused, which is a bug in this
/// crate and not a degraded environment, so it is not papered over.
pub(crate) fn persist(
    store: Option<&Store>,
    model: &Model,
    settings: &Settings,
    usage: &Usage,
) -> Result<Option<String>, String> {
    let Some(store) = store else {
        return Ok(None);
    };
    match store.save(model, settings, usage) {
        Ok(()) => Ok(None),
        Err(e @ (StoreError::WritesBlocked | StoreError::Io(_))) => Ok(Some(format!(
            "Changes are not being saved ({}).",
            store_error(&e)
        ))),
        Err(
            e @ (StoreError::Invalid(_)
            | StoreError::Parse(_)
            | StoreError::UnsupportedSchema(_)
            | StoreError::InvalidIconName(_)),
        ) => Err(store_error(&e)),
    }
}

/// Unix milliseconds, or 0 if the clock is somehow before the epoch. Only ever
/// compared with another value from this clock, so that floor is harmless.
fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

/// Records a successful launch of `app` and saves it. The app has already
/// opened, so a failed save is only a warning on the state, never an error
/// here, and it is reported the same way as any other failed save.
pub(crate) fn record_launch(app: &AppRef) {
    let _ = with_app(|state, _bus| {
        state.usage.record(app, now_ms());
        let problem = persist(
            state.store.as_ref(),
            &state.model,
            &state.settings,
            &state.usage,
        )?;
        state.save_problem = problem;
        Ok(())
    });
}

/// The apps to show under Frequent (`frequent` true) or Recent, up to `limit`,
/// resolved to the label the launcher currently shows for each. Empty before
/// `init`.
pub(crate) fn usage_hits(limit: u32, frequent: bool) -> Vec<UsageHit> {
    with_app(|state, _bus| {
        let labels: HashMap<(String, String), String> = state
            .model
            .items
            .iter()
            .filter(|i| !i.uninstalled)
            .filter_map(|i| {
                i.app
                    .as_ref()
                    .map(|a| ((a.package.clone(), a.activity.clone()), i.label.clone()))
            })
            .collect();
        let label_of = |a: &AppRef| {
            labels
                .get(&(a.package.clone(), a.activity.clone()))
                .cloned()
        };
        let limit = limit as usize;
        Ok(if frequent {
            state.usage.frequent(limit, label_of)
        } else {
            state.usage.recent(limit, label_of)
        })
    })
    .unwrap_or_default()
}

fn list_problem(e: &PlatformError) -> String {
    format!(
        "The installed apps could not be listed, so the app list was left as it was ({}).",
        platform_error(e)
    )
}

/// Each listed app's update time, keyed by `(package, activity)`.
fn tokens_of(entries: &[AppEntry]) -> HashMap<(String, String), i64> {
    entries
        .iter()
        .map(|e| ((e.package.clone(), e.activity.clone()), e.last_update_time))
        .collect()
}

/// The testable core of `init`; see `api::init_with_source`.
///
/// Order, on every path: `load` (the only thing that arms the store's write
/// block, so nothing may save before it), then `reconcile`, then one save.
pub(crate) fn install(
    config_root: Option<PathBuf>,
    source: Box<dyn AppSource>,
    mut warnings: Vec<String>,
) -> Result<ModelSnapshot, String> {
    let source: Arc<dyn AppSource> = Arc::from(source);

    // Keep icons beside the document so a restart starts warm. With no config
    // root there is nowhere to keep them.
    let icon_cache = config_root
        .as_ref()
        .map(|root| IconDiskCache::new(root.join("icon-cache"), DEFAULT_ICON_CACHE_BYTES));

    // The lock is held from the load to the swap, so a watcher callback from
    // the app being replaced cannot save over what is loaded here.
    let mut global = lock();

    let (store, mut model, settings, usage) = match config_root {
        Some(root) => {
            let store = Store::new(root);
            let loaded = store.load().map_err(|e| store_error(&e))?;
            warnings.extend(loaded.warnings);
            (Some(store), loaded.model, loaded.settings, loaded.usage)
        }
        None => (
            None,
            Model::new_default(),
            Settings::default(),
            Usage::default(),
        ),
    };

    // A failed listing must not reach `reconcile`: an empty list would remove
    // every app item.
    let mut icon_tokens = HashMap::new();
    let platform_problem = match source.list_apps() {
        Ok(entries) => {
            icon_tokens = tokens_of(&entries);
            reconcile(&mut model, &entries);
            None
        }
        Err(e) => Some(list_problem(&e)),
    };

    let save_problem = persist(store.as_ref(), &model, &settings, &usage)?;

    // Registered last, with nothing after it that can return early: from here
    // on an `App` exists that owns a running watcher, and it must only ever
    // be dropped outside the lock.
    let (watch, watch_problem) = match source.watch(Box::new(|| {
        let _ = refresh_apps_now();
    })) {
        Ok(handle) => (Some(handle), None),
        Err(PlatformError::Unsupported(_)) => (None, None),
        Err(e) => (
            None,
            Some(format!(
                "Installed apps are not being watched for changes ({}).",
                platform_error(&e)
            )),
        ),
    };

    let app = App {
        store,
        model,
        settings,
        usage,
        icon_cache,
        icon_tokens,
        source,
        _watch: watch,
        base_warnings: warnings,
        save_problem,
        platform_problem,
        watch_problem,
    };
    let snapshot = snapshot_of(&app);

    // Never drop an `App`, or a `WatchHandle`, while holding the state lock.
    // Dropping a `WatchHandle` joins the debounce thread, and that thread's job
    // is to call `refresh_apps_now`, which takes this lock. If it is waiting
    // for the lock while we wait for the join, neither ever returns. So the
    // old value is taken out under the lock, the guard is released, and only
    // then is the old value dropped.
    let old = global.app.take();
    global.app = Some(app);
    drop(global);
    drop(old);

    Ok(snapshot)
}

/// Empties the state and the event subscribers, following the same rule as
/// [`install`]: the `App` is dropped after the lock is released.
pub(crate) fn clear() {
    let (old_app, old_bus) = {
        let mut global = lock();
        let bus = std::mem::take(&mut global.bus);
        (global.app.take(), bus)
    };
    drop(old_app);
    drop(old_bus);
}

pub(crate) fn subscribe(deliver: Box<dyn Fn(&CoreEvent) -> bool + Send + Sync>) -> u64 {
    lock().bus.subscribe(deliver)
}

/// Emits an event without touching the model. Used by the platform host to
/// report something the OS did, such as the Home or Back button.
pub(crate) fn emit(event: CoreEvent) {
    lock().bus.emit(&event);
}

/// Runs `f` on the initialised state with the lock held. `f` must not drop
/// the `App` or call back into this module.
pub(crate) fn with_app<T>(
    f: impl FnOnce(&mut App, &mut EventBus) -> Result<T, String>,
) -> Result<T, String> {
    let mut global = lock();
    let Global { app, bus } = &mut *global;
    let app = app.as_mut().ok_or_else(not_initialised)?;
    f(app, bus)
}

pub(crate) fn snapshot_now() -> Result<ModelSnapshot, String> {
    with_app(|app, _| Ok(snapshot_of(app)))
}

pub(crate) fn factory_reset() -> Result<ModelSnapshot, String> {
    with_app(|app, bus| {
        let entries = app.source.list_apps().map_err(|e| platform_error(&e))?;
        let mut model = Model::new_default();
        reconcile(&mut model, &entries);
        let settings = Settings::default();
        let usage = Usage::default();
        let store = app
            .store
            .as_ref()
            .ok_or("No configuration directory; reset cannot be saved")?;
        store
            .save(&model, &settings, &usage)
            .map_err(|e| store_error(&e))?;
        // Only discard referenced launcher-owned files after the replacement is durable.
        for item in &app.model.items {
            if let Some(path) = item
                .icon_override
                .as_ref()
                .and_then(|name| store.icon_path(name).ok())
            {
                let _ = std::fs::remove_file(path);
            }
        }
        if let Some(cache) = &app.icon_cache {
            cache.clear();
        }
        app.model = model;
        app.settings = settings;
        app.usage = usage;
        app.icon_tokens = tokens_of(&entries);
        app.save_problem = None;
        app.platform_problem = None;
        app.base_warnings.clear();
        bus.emit(&CoreEvent::AppsChanged);
        Ok(snapshot_of(app))
    })
}

/// Writes the whole document to a dated file in the platform's Downloads
/// folder and returns the file name. The current document is what is saved,
/// so a failed save is not silently left out.
pub(crate) fn backup_config() -> Result<String, String> {
    with_app(|app, _| {
        app.store.as_ref().ok_or_else(|| {
            "NoStore: there is no configuration directory to read a backup from".to_string()
        })?;
        let bytes = Store::document_bytes(&app.model, &app.settings, &app.usage)
            .map_err(|e| store_error(&e))?;
        let name = backup_name(now_ms());
        app.source
            .write_backup(&name, &bytes)
            .map_err(|e| platform_error(&e))?;
        Ok(name)
    })
}

/// The backup files in the platform's Downloads folder, newest first.
pub(crate) fn list_backups() -> Result<Vec<String>, String> {
    with_app(|app, _| app.source.list_backups().map_err(|e| platform_error(&e)))
}

/// Replaces the model, settings and usage with the contents of the backup
/// `file_name`, then reconciles the imported layout with the apps installed
/// here. The current document is kept beside it as `qlaunch-backup.pre-restore.json`.
///
/// Nothing is written until the backup has been read and parsed, so a corrupt
/// or unknown file leaves the launcher exactly as it was.
pub(crate) fn restore_config(file_name: String) -> Result<ModelSnapshot, String> {
    with_app(|app, bus| {
        if !is_backup_name(&file_name) {
            return Err(format!(
                "InvalidBackup: {file_name:?} is not a backup file name"
            ));
        }
        let store = app.store.as_ref().ok_or_else(|| {
            "NoStore: there is no configuration directory to restore into".to_string()
        })?;
        let bytes = app
            .source
            .read_backup(&file_name)
            .map_err(|e| platform_error(&e))?;
        let loaded = Store::parse_document(&bytes).map_err(|e| store_error(&e))?;

        // Kept only after the backup parses, so a bad file cannot cost the
        // current setup. A missing current document is normal on a fresh root.
        if let Ok(current) = std::fs::read(store.document_path()) {
            store
                .write_pre_restore(&current)
                .map_err(|e| store_error(&e))?;
        }

        let entries = app.source.list_apps().map_err(|e| platform_error(&e))?;
        let mut model = loaded.model;
        reconcile(&mut model, &entries);
        store
            .save(&model, &loaded.settings, &loaded.usage)
            .map_err(|e| store_error(&e))?;

        app.model = model;
        app.settings = loaded.settings;
        app.usage = loaded.usage;
        app.icon_tokens = tokens_of(&entries);
        app.save_problem = None;
        app.platform_problem = None;
        if let Some(cache) = &app.icon_cache {
            cache.clear();
        }
        bus.emit(&CoreEvent::AppsChanged);
        Ok(snapshot_of(app))
    })
}

/// Applies `change` to a copy of the model and keeps it only if it worked and
/// could be saved (or the save only failed in a way that leaves the app
/// running: see [`persist`]). On `Err` nothing has changed.
pub(crate) fn mutate_model(
    change: impl FnOnce(&mut Model) -> Result<(), ModelError>,
) -> Result<ModelSnapshot, String> {
    with_app(|app, bus| {
        let mut next = app.model.clone();
        change(&mut next).map_err(|e| model_error(&e))?;
        commit_model(app, bus, next)
    })
}

fn commit_model(app: &mut App, bus: &mut EventBus, next: Model) -> Result<ModelSnapshot, String> {
    let problem = persist(app.store.as_ref(), &next, &app.settings, &app.usage)?;
    app.model = next;
    app.save_problem = problem;
    bus.emit(&CoreEvent::ModelChanged);
    Ok(snapshot_of(app))
}

pub(crate) fn mutate_settings(change: impl FnOnce(&mut Settings)) -> Result<ModelSnapshot, String> {
    with_app(|app, bus| {
        let mut next = app.settings.clone();
        change(&mut next);
        let problem = persist(app.store.as_ref(), &app.model, &next, &app.usage)?;
        app.settings = next;
        app.save_problem = problem;
        bus.emit(&CoreEvent::ModelChanged);
        Ok(snapshot_of(app))
    })
}

/// Stores a custom icon for the item `id` and points the item at it.
pub(crate) fn set_icon(id: &str, png: &[u8]) -> Result<ModelSnapshot, String> {
    with_app(|app, bus| {
        let id = id.to_string();
        if app.model.item(&id).is_none() {
            return Err(model_error(&ModelError::UnknownItem(id)));
        }
        let store = app.store.as_ref().ok_or_else(|| {
            "NoStore: there is no configuration directory to keep a custom icon in".to_string()
        })?;
        let name = store.write_icon(&id, png).map_err(|e| store_error(&e))?;
        let mut next = app.model.clone();
        if let Some(item) = next.items.iter_mut().find(|i| i.id == id) {
            item.icon_override = Some(name);
        }
        commit_model(app, bus, next)
    })
}

/// Lists the installed apps and reconciles the model with them.
///
/// A failed listing leaves the model alone: it is recorded as a warning and
/// reported as an `Error` event, and nothing claims the apps changed.
pub(crate) fn refresh_apps_now() -> Result<ModelSnapshot, String> {
    with_app(|app, bus| {
        match app.source.list_apps() {
            Err(e) => {
                let problem = list_problem(&e);
                app.platform_problem = Some(problem.clone());
                bus.emit(&CoreEvent::Error(problem));
            }
            Ok(entries) => {
                app.platform_problem = None;
                let tokens = tokens_of(&entries);
                let icons_changed = tokens != app.icon_tokens;
                app.icon_tokens = tokens;

                let mut next = app.model.clone();
                let outcome = reconcile(&mut next, &entries);
                let set_changed = !(outcome.added.is_empty() && outcome.removed.is_empty());
                if set_changed {
                    let problem = persist(app.store.as_ref(), &next, &app.settings, &app.usage)?;
                    app.model = next;
                    app.save_problem = problem;
                }
                // Disk keys already contain update times. Preserve unaffected
                // icons; Dart compares snapshot revisions to invalidate selectively.
                if set_changed || icons_changed {
                    bus.emit(&CoreEvent::AppsChanged);
                }
            }
        }
        Ok(snapshot_of(app))
    })
}
