//! The Advanced page and its operation-result copy.

use crate::{App, Message, components, theme};
use iced::Element;
use iced::widget::column;
use qlaunch_core::api::api::ModelSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Row {
    Restart,
    Backup,
    Restore,
    FactoryReset,
}

impl Row {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Row::Restart => "Restart Launcher",
            Row::Backup => "Backup config",
            Row::Restore => "Restore config",
            Row::FactoryReset => "Factory Reset",
        }
    }
    pub(crate) fn subtitle(self) -> &'static str {
        match self {
            Row::Restart => "Re-reads the saved layout and the installed applications.",
            Row::Backup => "Saves the whole layout to a dated file in Downloads.",
            Row::Restore => "Replaces the layout with a backup from Downloads.",
            Row::FactoryReset => "Wipes every group, item and setting, after confirmation.",
        }
    }
}

pub(crate) fn rows() -> Vec<Row> {
    vec![Row::Restart, Row::Backup, Row::Restore, Row::FactoryReset]
}

/// What Restore does once the All Files Access check answers: list the backups,
/// or ask the system for the permission first. Android attributes a Downloads
/// file to the install that wrote it, so after a reflash the scoped query
/// cannot see an earlier install's backups without this permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestoreStep {
    List,
    RequestAccess,
}

/// The step for the permission state `granted`.
pub(crate) fn restore_step(granted: bool) -> RestoreStep {
    if granted {
        RestoreStep::List
    } else {
        RestoreStep::RequestAccess
    }
}

/// Shown after sending the user to the system "All files access" screen, so a
/// second press of Restore lists the backups the previous install wrote.
pub(crate) const NEEDS_ALL_FILES_ACCESS: &str = "QLaunch needs \"All files access\" to read backups from Downloads. Enable it for QLaunch, then press Restore again.";

pub(crate) fn backup_message(result: &Result<String, String>) -> String {
    match result {
        Ok(name) => format!("Backup saved to Downloads/{name}."),
        Err(error) => format!("Backup failed: {error}"),
    }
}

pub(crate) fn restore_message(result: &Result<ModelSnapshot, String>, name: &str) -> String {
    match result {
        Ok(_) => format!("Config restored from {name}."),
        Err(error) => format!("Could not restore: {error}"),
    }
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let typography = theme::Text::new();
    let motion = app.motion(crate::Region::Screen);
    let mut content = column![];
    for row in rows() {
        content = content.push(crate::elastic_scroll::row(
            components::action_tile(
                &typography,
                row.label(),
                row.subtitle(),
                Message::Advanced(row),
            ),
            &motion,
        ));
    }
    super::scaffold(app, "Advanced", vec![], content.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog;

    #[test]
    fn rows_match_order() {
        assert_eq!(
            rows(),
            vec![Row::Restart, Row::Backup, Row::Restore, Row::FactoryReset]
        );
    }

    #[test]
    fn backup_message_names_the_downloads_file() {
        assert_eq!(
            backup_message(&Ok("qlaunch-2026.json".into())),
            "Backup saved to Downloads/qlaunch-2026.json."
        );
        assert_eq!(
            backup_message(&Err("disk full".into())),
            "Backup failed: disk full"
        );
    }

    #[test]
    fn restore_message_reports_success_and_failure() {
        assert_eq!(
            restore_message(&Err("bad file".into()), "a.json"),
            "Could not restore: bad file"
        );
    }

    #[test]
    fn restore_success_names_the_file() {
        let snapshot = crate::tests::app().snapshot.unwrap();
        assert_eq!(
            restore_message(&Ok(snapshot), "a.json"),
            "Config restored from a.json."
        );
    }

    #[test]
    fn factory_reset_requires_confirmation() {
        let mut app = crate::tests::app();
        let _ = app.update(Message::Advanced(Row::FactoryReset));
        assert!(matches!(
            app.dialog,
            Some(dialog::Dialog::Confirm(dialog::ConfirmState {
                action: dialog::ConfirmAction::FactoryReset,
                ..
            }))
        ));
    }

    #[test]
    fn restore_lists_only_with_all_files_access() {
        assert_eq!(restore_step(true), RestoreStep::List);
        assert_eq!(restore_step(false), RestoreStep::RequestAccess);
    }

    #[test]
    fn missing_restore_access_asks_for_it_without_a_dialog() {
        let mut app = crate::tests::app();
        let _ = app.update(Message::Push(crate::Screen::Advanced));
        // The check answers "not granted"; the request task is not run here.
        let _ = app.update(Message::RestoreAccessChecked(false));
        let _ = app.update(Message::RestoreAccessRequested(Ok(())));
        assert!(app.dialog.is_none());
        let shown = &app.snackbar.as_ref().unwrap().1;
        assert_eq!(shown, NEEDS_ALL_FILES_ACCESS);
        assert!(shown.contains("All files access"));
    }

    #[test]
    fn empty_backups_shows_a_snackbar_not_an_empty_dialog() {
        let mut app = crate::tests::app();
        let _ = app.update(Message::Push(crate::Screen::Advanced));
        let _ = app.update(Message::BackupsListed(Ok(vec![])));
        assert!(app.dialog.is_none());
        assert_eq!(
            app.snackbar.as_ref().unwrap().1,
            "No backups found in Downloads."
        );
    }
}
