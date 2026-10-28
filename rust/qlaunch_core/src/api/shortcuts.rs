use super::{api::ModelSnapshot, events::CoreEvent, state};
use crate::model::{ExtraKind, ShortcutPayload};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutProvider {
    pub id: String,
    pub label: String,
}

#[derive(Deserialize)]
struct ShortcutResult {
    label: String,
    payload: ShortcutPayload,
    icon: Option<Vec<u8>>,
    #[serde(default)]
    icon_ref: Option<String>,
}

#[cfg_attr(feature = "flutter", flutter_rust_bridge::frb(ignore))]
pub fn check_group(group: &str) -> Result<(), String> {
    state::with_app(|app, _| {
        if app
            .model
            .groups
            .iter()
            .any(|g| g.id == group && g.id != app.model.root_group_id)
        {
            Ok(())
        } else {
            Err("The shortcut's destination group no longer exists".into())
        }
    })
}

#[cfg_attr(feature = "flutter", flutter_rust_bridge::frb(ignore))]
pub fn add(
    group: &str,
    label: &str,
    payload: ShortcutPayload,
    icon: Option<&[u8]>,
    icon_ref: Option<&str>,
) -> Result<ModelSnapshot, String> {
    if label.trim().is_empty() {
        return Err("A shortcut needs a name".into());
    }
    if icon_ref
        .is_some_and(|reference| crate::platform::android::parse_icon_ref(reference).is_none())
    {
        return Err("Invalid shortcut icon reference".into());
    }
    validate(&payload)?;
    state::with_app(|app, bus| {
        if !app
            .model
            .groups
            .iter()
            .any(|g| g.id == group && g.id != app.model.root_group_id)
        {
            return Err("The shortcut's destination group no longer exists".into());
        }
        let mut next = app.model.clone();
        let id = next.add_shortcut(&group.to_string(), label.trim(), payload);
        let icon_override = if let Some(reference) = icon_ref {
            Some(reference.to_string())
        } else if let Some(bytes) = icon {
            if bytes.len() > 2 * 1024 * 1024 {
                return Err("Shortcut icon is too large".into());
            }
            let store = app
                .store
                .as_ref()
                .ok_or("No configuration directory for the shortcut icon")?;
            Some(
                store
                    .write_icon(&id, bytes)
                    .map_err(|e| state::store_error(&e))?,
            )
        } else {
            None
        };
        if let Some(icon_override) = icon_override {
            next.items
                .iter_mut()
                .find(|i| i.id == id)
                .unwrap()
                .icon_override = Some(icon_override);
        }
        let store = app
            .store
            .as_ref()
            .ok_or("No configuration directory for the shortcut")?;
        store
            .save(&next, &app.settings, &app.usage)
            .map_err(|e| state::store_error(&e))?;
        app.model = next;
        bus.emit(&CoreEvent::ShortcutCreated { item_id: id });
        bus.emit(&CoreEvent::ModelChanged);
        Ok(state::snapshot_of(app))
    })
}

fn validate(payload: &ShortcutPayload) -> Result<(), String> {
    match payload {
        ShortcutPayload::Command { command } if command.trim().is_empty() => {
            Err("A shortcut needs a command".into())
        }
        ShortcutPayload::Android {
            categories,
            extras,
            action,
            component,
            ..
        } => {
            if categories
                .iter()
                .any(|c| c == "android.intent.category.LAUNCHER")
            {
                return Err("Launcher app duplicates are not added as shortcuts".into());
            }
            if action.as_ref().is_none_or(|s| s.is_empty())
                && component.as_ref().is_none_or(|s| s.is_empty())
            {
                return Err("Shortcut has no action or component".into());
            }
            for extra in extras {
                let valid = !extra.key.is_empty()
                    && match extra.kind {
                        ExtraKind::String => true,
                        ExtraKind::Bool => extra.value.parse::<bool>().is_ok(),
                        ExtraKind::Int => extra.value.parse::<i32>().is_ok(),
                    };
                if !valid {
                    return Err(format!("Invalid shortcut extra: {}", extra.key));
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg_attr(feature = "flutter", flutter_rust_bridge::frb(ignore))]
pub fn accept_result(group: &str, json: &str) -> Result<ModelSnapshot, String> {
    let result: ShortcutResult =
        serde_json::from_str(json).map_err(|e| format!("Invalid shortcut result: {e}"))?;
    add(
        group,
        &result.label,
        result.payload,
        result.icon.as_deref(),
        result.icon_ref.as_deref(),
    )
}

#[cfg_attr(feature = "flutter", flutter_rust_bridge::frb(ignore))]
pub fn lock() -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        crate::platform::android::jni_host::JniHost::call_void("lockScreen", &[])
    }
    #[cfg(not(target_os = "android"))]
    {
        let status = std::process::Command::new("loginctl")
            .arg("lock-session")
            .status()
            .map_err(|e| format!("Could not lock this session: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("This session does not support screen locking".into())
        }
    }
}

#[cfg_attr(feature = "flutter", flutter_rust_bridge::frb(ignore))]
pub fn notifications() -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        crate::platform::android::jni_host::JniHost::call_void("openNotifications", &[])
    }
    #[cfg(not(target_os = "android"))]
    {
        Err("The notification shade is only available on Android".into())
    }
}

#[cfg_attr(feature = "flutter", flutter_rust_bridge::frb(ignore))]
pub fn launch(payload: &ShortcutPayload) -> Result<(), String> {
    match payload {
        ShortcutPayload::LockScreen => lock(),
        ShortcutPayload::Notifications => notifications(),
        ShortcutPayload::Command { command } => {
            #[cfg(target_os = "linux")]
            {
                let args = crate::platform::linux::build_command(command, false, None)
                    .map_err(|e| e.to_string())?;
                crate::platform::linux::launch::spawn(&args).map_err(|e| e.to_string())
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = command;
                Err("Command shortcuts are only available on Linux".into())
            }
        }
        ShortcutPayload::Android { .. } => {
            #[cfg(target_os = "android")]
            {
                crate::platform::android::jni_host::JniHost::call_void(
                    "launchShortcut",
                    &[serde_json::to_string(payload).map_err(|e| e.to_string())?],
                )
            }
            #[cfg(not(target_os = "android"))]
            {
                Err("Android shortcuts are only available on Android".into())
            }
        }
    }
}
