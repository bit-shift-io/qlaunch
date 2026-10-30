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
    let settings = app
        .snapshot
        .as_ref()
        .map(|s| s.settings.clone())
        .unwrap_or_default();
    let typography = theme::Text::from_settings(&settings);
    let mut content = column![];
    for row in rows() {
        content = content.push(components::action_tile(
            &typography,
            row.label(),
            row.subtitle(),
            Message::Advanced(row),
        ));
    }
    super::scaffold(app, "Advanced", vec![], content.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog;

    #[test]
    fn rows_match_flutter_order() {
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
