//! The unified Material dialog model: state, rows, pure layout and the core
//! change plans. Rendering lives in `view`, accessibility reads the layout.

use iced::widget::{container, mouse_area, scrollable, space, text};
use iced::{Element, Fill, Rectangle, Size};

use qlaunch_core::api::api::ModelSnapshot;
use qlaunch_core::api::shortcuts::ShortcutProvider;
use qlaunch_core::model::{Group, GroupRole, Id, Item};

use crate::theme::{self, Text, TextRole};
use crate::{App, Message as AppMessage, components};

/// Every action a dialog can take. Rendering maps these to the same core calls
/// the old overlay used; no model rule lives here.
#[derive(Debug, Clone)]
pub(crate) enum Message {
    Close,
    Noop,
    OpenGroupSelect,
    GroupChosen(Id),
    CloseSelect,
    AppName(String),
    AppSave,
    AppUninstall,
    AppRemove,
    FolderName(String),
    FolderSave,
    FolderDelete,
    LauncherSettings,
    LauncherAddShortcut,
    LauncherCreateFolder,
    LauncherEditGroups,
    LauncherWallpaper,
    Providers(Result<Vec<ShortcutProvider>, String>),
    ShortcutLock,
    ShortcutNotifications,
    ShortcutProvider(String),
    ShortcutCommand,
    CommandName(String),
    CommandText(String),
    CommandAdd,
    AddGroupName(String),
    AddGroupConfirm,
    GroupName(String),
    GroupRole(Option<GroupRole>),
    GroupSave,
    GroupDelete,
    GroupChooseApps,
    RestoreChosen(String),
    ConfirmYes,
}

/// One popup. Every variant draws through the same card; only its rows and
/// actions differ.
#[allow(clippy::large_enum_variant)] // App holds an owned Item while forms are small.
pub(crate) enum Dialog {
    App(AppEditor),
    Folder(FolderEditor),
    Launcher,
    AddShortcut(AddShortcutState),
    CommandShortcut(CommandShortcutState),
    AddGroup(AddGroupState),
    GroupEditor(GroupEditorState),
    Confirm(ConfirmState),
    RestorePicker(RestorePickerState),
}

pub(crate) struct AppEditor {
    pub item: Item,
    pub name: String,
    pub group_id: Id,
    pub busy: bool,
    pub error: Option<String>,
}

impl AppEditor {
    pub(crate) fn new(item: Item) -> Self {
        AppEditor {
            name: item.label.clone(),
            group_id: item.group_id.clone(),
            item,
            busy: false,
            error: None,
        }
    }

    pub(crate) fn can_uninstall(&self) -> bool {
        self.item.app.is_some() && !self.item.uninstalled
    }

    pub(crate) fn is_shortcut(&self) -> bool {
        self.item.shortcut.is_some()
    }

    pub(crate) fn can_save(&self) -> bool {
        !self.busy && !self.name.trim().is_empty()
    }
}

pub(crate) enum FolderEditor {
    Create {
        name: String,
        group_id: Id,
        busy: bool,
        error: Option<String>,
    },
    Edit {
        folder: Id,
        label: String,
        name: String,
        group_id: Id,
        busy: bool,
        error: Option<String>,
    },
}

impl FolderEditor {
    pub(crate) fn create(group_id: Id) -> Self {
        FolderEditor::Create {
            name: "Folder".into(),
            group_id,
            busy: false,
            error: None,
        }
    }

    pub(crate) fn edit(folder: Id, label: String, group_id: Id) -> Self {
        FolderEditor::Edit {
            folder,
            name: label.clone(),
            label,
            group_id,
            busy: false,
            error: None,
        }
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            FolderEditor::Create { name, .. } | FolderEditor::Edit { name, .. } => name,
        }
    }

    pub(crate) fn set_name(&mut self, value: String) {
        match self {
            FolderEditor::Create { name, .. } | FolderEditor::Edit { name, .. } => *name = value,
        }
    }

    pub(crate) fn group_id(&self) -> &Id {
        match self {
            FolderEditor::Create { group_id, .. } | FolderEditor::Edit { group_id, .. } => group_id,
        }
    }

    pub(crate) fn set_group(&mut self, id: Id) {
        match self {
            FolderEditor::Create { group_id, .. } | FolderEditor::Edit { group_id, .. } => {
                *group_id = id
            }
        }
    }

    pub(crate) fn is_edit(&self) -> bool {
        matches!(self, FolderEditor::Edit { .. })
    }

    pub(crate) fn can_save(&self) -> bool {
        !self.busy() && !self.name().trim().is_empty() && !self.group_id().is_empty()
    }

    pub(crate) fn busy(&self) -> bool {
        match self {
            FolderEditor::Create { busy, .. } | FolderEditor::Edit { busy, .. } => *busy,
        }
    }

    pub(crate) fn set_busy(&mut self, value: bool) {
        match self {
            FolderEditor::Create { busy, .. } | FolderEditor::Edit { busy, .. } => *busy = value,
        }
    }

    pub(crate) fn error(&self) -> Option<&String> {
        match self {
            FolderEditor::Create { error, .. } | FolderEditor::Edit { error, .. } => error.as_ref(),
        }
    }

    pub(crate) fn set_error(&mut self, value: Option<String>) {
        match self {
            FolderEditor::Create { error, .. } | FolderEditor::Edit { error, .. } => *error = value,
        }
    }
}

pub(crate) struct AddShortcutState {
    pub target_group: Id,
    pub providers: Option<Result<Vec<ShortcutProvider>, String>>,
    pub busy: bool,
}

impl AddShortcutState {
    pub(crate) fn new(target_group: Id) -> Self {
        AddShortcutState {
            target_group,
            providers: None,
            busy: false,
        }
    }
}

pub(crate) struct CommandShortcutState {
    pub target_group: Id,
    pub name: String,
    pub command: String,
    pub busy: bool,
}

impl CommandShortcutState {
    pub(crate) fn new(target_group: Id) -> Self {
        CommandShortcutState {
            target_group,
            name: String::new(),
            command: String::new(),
            busy: false,
        }
    }

    pub(crate) fn ready(&self) -> bool {
        !self.busy && !self.name.trim().is_empty() && !self.command.trim().is_empty()
    }
}

pub(crate) struct AddGroupState {
    pub name: String,
    pub busy: bool,
}

impl AddGroupState {
    pub(crate) fn new() -> Self {
        AddGroupState {
            name: String::new(),
            busy: false,
        }
    }

    pub(crate) fn can_confirm(&self) -> bool {
        !self.busy && !self.name.trim().is_empty()
    }
}

pub(crate) struct GroupEditorState {
    pub group: Id,
    pub label: String,
    pub name: String,
    pub role: Option<GroupRole>,
    pub busy: bool,
    pub error: Option<String>,
}

impl GroupEditorState {
    pub(crate) fn new(group: Id, label: String, role: Option<GroupRole>) -> Self {
        GroupEditorState {
            name: label.clone(),
            group,
            label,
            role,
            busy: false,
            error: None,
        }
    }

    /// A role the core will not let go of. `Hidden` is never a page.
    pub(crate) fn role_locked(&self) -> bool {
        matches!(
            self.role,
            Some(GroupRole::Apps) | Some(GroupRole::Games) | Some(GroupRole::Hidden)
        )
    }

    pub(crate) fn can_delete(&self) -> bool {
        !self.busy && self.role.is_none()
    }

    pub(crate) fn can_save(&self) -> bool {
        !self.busy && !self.name.trim().is_empty()
    }
}

pub(crate) struct ConfirmState {
    pub title: String,
    pub body: String,
    pub confirm_label: String,
    pub destructive: bool,
    pub busy: bool,
    pub action: ConfirmAction,
}

pub(crate) enum ConfirmAction {
    DeleteGroup(Id),
    Restore(String),
    FactoryReset,
}

pub(crate) struct RestorePickerState {
    pub backups: Option<Result<Vec<String>, String>>,
}

pub(crate) struct Select {
    pub field: SelectField,
    pub options: Vec<SelectOption>,
    pub selected: Id,
}

pub(crate) enum SelectField {
    AppGroup,
    FolderGroup,
}

pub(crate) struct SelectOption {
    pub id: Id,
    pub label: String,
    pub folder: bool,
}

/// One line in a dialog's content, in draw order.
pub(crate) enum Row {
    Field {
        label: String,
        value: String,
        widget_id: &'static str,
    },
    Text {
        label: String,
        error: bool,
    },
    Select {
        label: String,
        value: String,
    },
    Chip {
        label: String,
        selected: bool,
        enabled: bool,
    },
    Tile {
        label: String,
        subtitle: Option<String>,
    },
    Note {
        label: String,
    },
}

pub(crate) fn row_height(row: &Row) -> f32 {
    match row {
        Row::Field { .. } | Row::Select { .. } => theme::metrics::FIELD_HEIGHT,
        Row::Text { .. } | Row::Note { .. } => 20.0,
        Row::Chip { .. } => theme::metrics::CHIP_HEIGHT,
        Row::Tile { subtitle, .. } => {
            if subtitle.is_some() {
                theme::metrics::LIST_TWO_LINE
            } else {
                theme::metrics::LIST_ONE_LINE
            }
        }
    }
}

/// Rough wrapped-line count for a label in `width`, used to give text rows the
/// height they actually draw at the current font scale.
fn wrapped_lines(label: &str, size: f32, width: f32) -> usize {
    if width <= 0.0 || size <= 0.0 {
        return 1;
    }
    let per_line = (width / (size * 0.55)).floor().max(1.0) as usize;
    label.chars().count().div_ceil(per_line).max(1)
}

pub(crate) fn heading(dialog: &Dialog) -> Option<String> {
    match dialog {
        Dialog::App(_) => None,
        Dialog::Folder(FolderEditor::Create { .. }) => Some("New Folder".into()),
        Dialog::Folder(FolderEditor::Edit { label, .. }) => Some(label.clone()),
        Dialog::Launcher => None,
        Dialog::AddShortcut(_) => Some("Add Shortcut".into()),
        Dialog::CommandShortcut(_) => Some("Command Shortcut".into()),
        Dialog::AddGroup(_) => Some("Add group".into()),
        Dialog::GroupEditor(state) => Some(state.label.clone()),
        Dialog::Confirm(state) => Some(state.title.clone()),
        Dialog::RestorePicker(_) => Some("Restore config".into()),
    }
}

fn group_label(app: &App, id: &Id) -> String {
    app.snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.groups.iter().find(|group| &group.id == id))
        .map(|group| group.label.clone())
        .unwrap_or_default()
}

/// Page groups and their folders an item can move to, in snapshot order.
pub(crate) fn destinations(snapshot: &ModelSnapshot) -> Vec<SelectOption> {
    let mut options = Vec::new();
    for group in &snapshot.groups {
        if group.id == snapshot.root_group_id || group.folder {
            continue;
        }
        options.push(SelectOption {
            id: group.id.clone(),
            label: group.label.clone(),
            folder: false,
        });
        for folder in folders_in(snapshot, &group.id) {
            options.push(SelectOption {
                id: folder.id.clone(),
                label: folder.label.clone(),
                folder: true,
            });
        }
    }
    options
}

/// The folder groups whose tile lives in `group_id`.
pub(crate) fn folders_in<'a>(snapshot: &'a ModelSnapshot, group_id: &Id) -> Vec<&'a Group> {
    snapshot
        .groups
        .iter()
        .filter(|candidate| candidate.folder)
        .filter(|candidate| {
            snapshot.items.iter().any(|item| {
                &item.group_id == group_id
                    && item.folder_group_id.as_deref() == Some(candidate.id.as_str())
            })
        })
        .collect()
}

/// The group that holds a folder's tile.
pub(crate) fn folder_of(snapshot: &ModelSnapshot, folder_id: &Id) -> Option<Id> {
    snapshot
        .items
        .iter()
        .find(|item| item.folder_group_id.as_deref() == Some(folder_id.as_str()))
        .map(|item| item.group_id.clone())
}

/// The order value that appends an item to the end of `group`.
pub(crate) fn append_order(snapshot: &ModelSnapshot, group: &Id) -> i32 {
    snapshot
        .items
        .iter()
        .filter(|item| &item.group_id == group)
        .count() as i32
}

pub(crate) fn rows(app: &App, dialog: &Dialog) -> Vec<Row> {
    let mut rows = Vec::new();
    match dialog {
        Dialog::App(editor) => {
            rows.push(Row::Field {
                label: "Name".into(),
                value: editor.name.clone(),
                widget_id: APP_NAME_FIELD_ID,
            });
            if let Some(app_ref) = &editor.item.app {
                rows.push(Row::Text {
                    label: app_ref.package.clone(),
                    error: false,
                });
                if editor.item.uninstalled {
                    rows.push(Row::Text {
                        label: "Uninstalled".into(),
                        error: false,
                    });
                }
            }
            rows.push(Row::Select {
                label: "Group".into(),
                value: group_label(app, &editor.group_id),
            });
            if let Some(error) = &editor.error {
                rows.push(Row::Text {
                    label: error.clone(),
                    error: true,
                });
            }
        }
        Dialog::Folder(editor) => {
            rows.push(Row::Field {
                label: "Name".into(),
                value: editor.name().to_string(),
                widget_id: FOLDER_NAME_FIELD_ID,
            });
            rows.push(Row::Select {
                label: "Group".into(),
                value: group_label(app, editor.group_id()),
            });
            if let Some(error) = editor.error() {
                rows.push(Row::Text {
                    label: error.clone(),
                    error: true,
                });
            }
        }
        Dialog::Launcher => {
            for label in [
                "Launcher Settings",
                "Add Shortcut",
                "Create Folder",
                "Edit Groups",
                "Choose Wallpaper",
            ] {
                rows.push(Row::Tile {
                    label: label.into(),
                    subtitle: None,
                });
            }
        }
        Dialog::AddShortcut(state) => match &state.providers {
            None => rows.push(Row::Note {
                label: "Loading…".into(),
            }),
            Some(Err(error)) => rows.push(Row::Text {
                label: error.clone(),
                error: true,
            }),
            Some(Ok(providers)) => {
                rows.push(Row::Tile {
                    label: "Lock Screen".into(),
                    subtitle: None,
                });
                if cfg!(target_os = "android") {
                    rows.push(Row::Tile {
                        label: "Notifications".into(),
                        subtitle: None,
                    });
                }
                for provider in providers {
                    rows.push(Row::Tile {
                        label: provider.label.clone(),
                        subtitle: None,
                    });
                }
            }
        },
        Dialog::CommandShortcut(state) => {
            rows.push(Row::Field {
                label: "Name".into(),
                value: state.name.clone(),
                widget_id: COMMAND_NAME_FIELD_ID,
            });
            rows.push(Row::Field {
                label: "Command".into(),
                value: state.command.clone(),
                widget_id: COMMAND_TEXT_FIELD_ID,
            });
            rows.push(Row::Note {
                label: "No shell is involved.".into(),
            });
        }
        Dialog::AddGroup(state) => rows.push(Row::Field {
            label: "Name".into(),
            value: state.name.clone(),
            widget_id: ADD_GROUP_FIELD_ID,
        }),
        Dialog::GroupEditor(state) => {
            rows.push(Row::Field {
                label: "Name".into(),
                value: state.name.clone(),
                widget_id: GROUP_NAME_FIELD_ID,
            });
            rows.push(Row::Note {
                label: "Role".into(),
            });
            for (label, role) in [
                ("Home", Some(GroupRole::Home)),
                ("Apps", Some(GroupRole::Apps)),
                ("Games", Some(GroupRole::Games)),
                ("None", None),
            ] {
                rows.push(Row::Chip {
                    label: label.into(),
                    selected: state.role == role,
                    enabled: !state.role_locked() || state.role == role,
                });
            }
            rows.push(Row::Tile {
                label: "Choose apps".into(),
                subtitle: None,
            });
            if let Some(error) = &state.error {
                rows.push(Row::Text {
                    label: error.clone(),
                    error: true,
                });
            }
        }
        Dialog::Confirm(state) => rows.push(Row::Note {
            label: state.body.clone(),
        }),
        Dialog::RestorePicker(state) => match &state.backups {
            None => rows.push(Row::Note {
                label: "Loading…".into(),
            }),
            Some(Err(error)) => rows.push(Row::Text {
                label: error.clone(),
                error: true,
            }),
            Some(Ok(names)) if names.is_empty() => rows.push(Row::Note {
                label: "No backups found in Downloads.".into(),
            }),
            Some(Ok(names)) => {
                for name in names {
                    rows.push(Row::Tile {
                        label: name.clone(),
                        subtitle: None,
                    });
                }
            }
        },
    }
    rows
}

/// The action row's labels, left to right.
pub(crate) fn action_labels(dialog: &Dialog) -> Vec<String> {
    match dialog {
        Dialog::App(editor) => {
            let mut labels = Vec::new();
            if editor.can_uninstall() {
                labels.push("Uninstall".into());
            }
            if editor.is_shortcut() {
                labels.push("Remove".into());
            }
            labels.push("Cancel".into());
            labels.push("Save".into());
            labels
        }
        Dialog::Folder(editor) => {
            let mut labels = Vec::new();
            if editor.is_edit() {
                labels.push("Delete".into());
            }
            labels.push("Cancel".into());
            labels.push("Save".into());
            labels
        }
        Dialog::Launcher | Dialog::RestorePicker(_) => Vec::new(),
        Dialog::AddShortcut(_) => vec!["Cancel".into()],
        Dialog::CommandShortcut(_) => vec!["Cancel".into(), "Add".into()],
        Dialog::AddGroup(_) => vec!["Cancel".into(), "Add".into()],
        Dialog::GroupEditor(_) => vec!["Delete".into(), "Cancel".into(), "Save".into()],
        Dialog::Confirm(state) => vec!["Cancel".into(), state.confirm_label.clone()],
    }
}

pub(crate) struct DialogLayout {
    pub card: Rectangle,
    pub title: Option<Rectangle>,
    pub content: Rectangle,
    /// The content's natural height, which may exceed `content`.
    pub content_height: f32,
    pub actions: Vec<(Rectangle, String)>,
    pub rows: Vec<(Rectangle, usize)>,
}

pub(crate) const APP_NAME_FIELD_ID: &str = "dialog-app-name";
pub(crate) const FOLDER_NAME_FIELD_ID: &str = "dialog-folder-name";
pub(crate) const COMMAND_NAME_FIELD_ID: &str = "dialog-command-name";
pub(crate) const COMMAND_TEXT_FIELD_ID: &str = "dialog-command-text";
pub(crate) const ADD_GROUP_FIELD_ID: &str = "dialog-add-group";
pub(crate) const GROUP_NAME_FIELD_ID: &str = "dialog-group-name";

pub(crate) fn layout(app: &App, dialog: &Dialog, window: Size) -> DialogLayout {
    let rows = rows(app, dialog);
    let labels = action_labels(dialog);
    let typography = dialog_text();
    let title_height = typography.role(TextRole::HeadlineSmall).line_height;
    // Menu rows supply their own horizontal padding. Only form dialogs need
    // the additional inset; the launcher menu keeps a small vertical edge gap.
    let launcher_menu = matches!(dialog, Dialog::Launcher);
    let padding = if launcher_menu {
        0.0
    } else {
        theme::metrics::DIALOG_PADDING.min(window.width / 4.0)
    };
    let edge_padding = if launcher_menu {
        theme::metrics::MENU_VERTICAL_PADDING
    } else {
        padding
    };
    let title_block = if heading(dialog).is_some() {
        padding + title_height + theme::metrics::CONTENT_TOP_GAP
    } else {
        edge_padding
    };
    let card_width = (window.width - 2.0 * theme::metrics::DIALOG_SIDE_MARGIN)
        .clamp(
            theme::metrics::DIALOG_MIN_WIDTH,
            theme::metrics::DIALOG_MAX_WIDTH,
        )
        .min(window.width.max(0.0));
    let inner_width = (card_width - 2.0 * padding).max(0.0);
    let widths: Vec<f32> = labels
        .iter()
        .map(|s| {
            (s.chars().count() as f32 * 8.0 + 24.0)
                .max(64.0)
                .min(inner_width)
        })
        .collect();
    let total = widths.iter().sum::<f32>()
        + theme::metrics::ACTION_GAP * widths.len().saturating_sub(1) as f32;
    let overflow = total > inner_width;
    let action_height =
        theme::metrics::ACTION_HEIGHT.max(typography.role(TextRole::LabelLarge).line_height + 12.0);
    let action_rows = if labels.is_empty() {
        0
    } else if overflow {
        labels.len()
    } else {
        1
    };
    let actions_height = edge_padding
        + if action_rows == 0 {
            0.0
        } else {
            padding + action_height * action_rows as f32
        };
    let list = matches!(
        dialog,
        Dialog::Launcher | Dialog::AddShortcut(_) | Dialog::RestorePicker(_)
    );
    let gap = if list { 0.0 } else { 16.0 };
    let mut row_rects = Vec::with_capacity(rows.len());
    let mut cursor = 0.0;
    let body = typography.role(TextRole::BodyMedium);
    let chip_text = typography.role(TextRole::LabelLarge);
    // Chips flow left to right and wrap only when the next one would not fit,
    // matching Flutter's `Wrap` of choice chips. The vertical cursor advances
    // once per wrapped line, not once per chip.
    let mut chip_line_used = 0.0f32;
    let mut chip_line_open = false;
    let mut chip_height = 0.0f32;
    for (index, row) in rows.iter().enumerate() {
        let height = row_height(row).max(match row {
            Row::Field { .. } | Row::Select { .. } => {
                typography.role(TextRole::BodyLarge).line_height
                    + typography.role(TextRole::LabelMedium).line_height
                    + 16.0
            }
            Row::Text { label, .. } | Row::Note { label } => {
                wrapped_lines(label, body.size, inner_width) as f32 * body.line_height
            }
            Row::Chip { .. } => chip_text.line_height + 8.0,
            Row::Tile { subtitle, .. } => {
                typography.role(TextRole::BodyLarge).line_height
                    + subtitle
                        .as_ref()
                        .map_or(0.0, |_| typography.role(TextRole::BodyMedium).line_height)
                    + 16.0
            }
        });
        if let Row::Chip {
            label, selected, ..
        } = row
        {
            let width = components::chip_width(label, chip_text.size, *selected);
            if chip_line_open && chip_line_used + components::CHIP_SPACING + width > inner_width {
                cursor += height + gap;
                chip_line_used = 0.0;
                chip_line_open = false;
            }
            let x = if chip_line_open {
                chip_line_used + components::CHIP_SPACING
            } else {
                0.0
            };
            chip_line_used = x + width;
            chip_line_open = true;
            chip_height = height;
            row_rects.push((
                Rectangle {
                    x,
                    y: cursor,
                    width,
                    height,
                },
                index,
            ));
            continue;
        }
        if chip_line_open {
            cursor += chip_height + gap;
            chip_line_used = 0.0;
            chip_line_open = false;
        }
        row_rects.push((
            Rectangle {
                x: 0.0,
                y: cursor,
                width: inner_width,
                height,
            },
            index,
        ));
        cursor += height + gap;
    }
    if chip_line_open {
        cursor += chip_height + gap;
    }
    let content_natural = (cursor - if rows.is_empty() { 0.0 } else { gap }).max(0.0);
    let max_card_height = (window.height - 2.0 * theme::metrics::DIALOG_VERTICAL_MARGIN).max(0.0);
    let available_content = (max_card_height - title_block - actions_height).max(0.0);
    let content_height = content_natural.min(available_content);
    let card_height = title_block + content_height + actions_height;

    let card = Rectangle {
        x: (window.width - card_width) / 2.0,
        y: (window.height - card_height) / 2.0,
        width: card_width,
        height: card_height,
    };

    let title = heading(dialog).map(|_| Rectangle {
        x: card.x + padding,
        y: card.y + padding,
        width: inner_width,
        height: title_height,
    });

    let content = Rectangle {
        x: card.x + padding,
        y: card.y + title_block,
        width: inner_width,
        height: content_height,
    };

    for (bounds, _) in &mut row_rects {
        bounds.x += content.x;
        bounds.y += content.y;
    }

    let mut actions = Vec::new();
    if !labels.is_empty() {
        let mut y = card.y + card_height - padding - action_height * action_rows as f32;
        let mut x = card.x + card.width - padding - total;
        for (label, width) in labels.iter().zip(widths) {
            if overflow {
                x = card.x + card.width - padding - width;
            }
            actions.push((
                Rectangle {
                    x,
                    y,
                    width,
                    height: action_height,
                },
                label.clone(),
            ));
            x += width + theme::metrics::ACTION_GAP;
            if overflow {
                y += action_height;
            }
        }
    }

    DialogLayout {
        card,
        title,
        content,
        content_height: content_natural,
        actions,
        rows: row_rects,
    }
}

/// One core mutation a Save will perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Change {
    RenameItem(Id, String),
    MoveItem(Id, Id, i32),
    RenameGroup(Id, String),
    AddFolder(Id, String),
}

/// Rename then move, exactly as Flutter's `AppSettingsDialog._save`.
pub(crate) fn app_save_plan(
    item: &Item,
    name: &str,
    group: &Id,
    snapshot: &ModelSnapshot,
) -> Vec<Change> {
    let mut plan = Vec::new();
    let name = name.trim();
    if name != item.label {
        plan.push(Change::RenameItem(item.id.clone(), name.to_string()));
    }
    if group != &item.group_id {
        plan.push(Change::MoveItem(
            item.id.clone(),
            group.clone(),
            append_order(snapshot, group),
        ));
    }
    plan
}

fn dialog_text() -> Text {
    Text::new()
}

fn plain_line<'a>(
    text_settings: &Text,
    label: &str,
    variant: bool,
    error: bool,
) -> Element<'a, AppMessage> {
    let style = if variant {
        text_settings.role(TextRole::BodySmall)
    } else {
        text_settings.role(TextRole::BodyMedium)
    };
    let color = if error {
        theme::color::ERROR
    } else if variant {
        theme::color::ON_SURFACE_VARIANT
    } else {
        theme::color::ON_SURFACE
    };
    text(label.to_string())
        .size(style.size)
        .line_height(style.line_height())
        .font(style.font)
        .color(color)
        .into()
}

fn tile_with_enabled<'a>(
    text_settings: &Text,
    label: &str,
    leading: Option<char>,
    message: Message,
    enabled: bool,
) -> Element<'a, AppMessage> {
    components::list_tile(
        text_settings,
        label,
        None,
        leading,
        None,
        Some(AppMessage::Dialog(message)),
        enabled,
    )
}

fn tile<'a>(
    text_settings: &Text,
    label: &str,
    leading: Option<char>,
    message: Message,
) -> Element<'a, AppMessage> {
    tile_with_enabled(text_settings, label, leading, message, true)
}

fn field<'a>(
    text_settings: &Text,
    label: &str,
    value: &str,
    widget_id: &'static str,
    build: fn(String) -> Message,
    submit: Message,
) -> Element<'a, AppMessage> {
    components::text_field(
        text_settings,
        label,
        value,
        "",
        widget_id,
        move |value| AppMessage::Dialog(build(value)),
        AppMessage::Dialog(submit),
    )
}

struct BodyLayout<'a> {
    layout: DialogLayout,
    layers: iced::widget::Stack<'a, AppMessage>,
    index: usize,
}

impl<'a> BodyLayout<'a> {
    fn push(mut self, child: impl Into<Element<'a, AppMessage>>) -> Self {
        if let Some((bounds, _)) = self.layout.rows.get(self.index) {
            self.layers = self.layers.push(
                iced::widget::pin(
                    container(child)
                        .width(bounds.width)
                        .height(bounds.height)
                        .clip(true),
                )
                .x(bounds.x - self.layout.content.x)
                .y(bounds.y - self.layout.content.y),
            );
        }
        self.index += 1;
        self
    }
    fn finish(self) -> Element<'a, AppMessage> {
        self.layers.into()
    }
}

fn body<'a>(app: &'a App, dialog: &'a Dialog, text_settings: &Text) -> Element<'a, AppMessage> {
    let layout = layout(app, dialog, app.size);
    let mut rows = BodyLayout {
        layers: iced::widget::stack![
            space()
                .width(layout.content.width)
                .height(layout.content_height)
        ],
        layout,
        index: 0,
    };
    match dialog {
        Dialog::App(editor) => {
            rows = rows.push(field(
                text_settings,
                "Name",
                &editor.name,
                APP_NAME_FIELD_ID,
                Message::AppName,
                Message::AppSave,
            ));
            if let Some(app_ref) = &editor.item.app {
                rows = rows.push(plain_line(text_settings, &app_ref.package, true, false));
                if editor.item.uninstalled {
                    rows = rows.push(plain_line(text_settings, "Uninstalled", true, false));
                }
            }
            rows = rows.push(components::select_field(
                text_settings,
                "Group",
                &group_label(app, &editor.group_id),
                AppMessage::Dialog(Message::OpenGroupSelect),
            ));
            if let Some(error) = &editor.error {
                rows = rows.push(plain_line(text_settings, error, false, true));
            }
        }
        Dialog::Folder(editor) => {
            rows = rows.push(field(
                text_settings,
                "Name",
                editor.name(),
                FOLDER_NAME_FIELD_ID,
                Message::FolderName,
                Message::FolderSave,
            ));
            rows = rows.push(components::select_field(
                text_settings,
                "Group",
                &group_label(app, editor.group_id()),
                AppMessage::Dialog(Message::OpenGroupSelect),
            ));
            if let Some(error) = editor.error() {
                rows = rows.push(plain_line(text_settings, error, false, true));
            }
        }
        Dialog::Launcher => {
            for (label, glyph, message) in [
                (
                    "Launcher Settings",
                    crate::icons::SETTINGS,
                    Message::LauncherSettings,
                ),
                (
                    "Add Shortcut",
                    crate::icons::ADD_LINK,
                    Message::LauncherAddShortcut,
                ),
                (
                    "Create Folder",
                    crate::icons::CREATE_NEW_FOLDER,
                    Message::LauncherCreateFolder,
                ),
                (
                    "Edit Groups",
                    crate::icons::DASHBOARD_CUSTOMIZE,
                    Message::LauncherEditGroups,
                ),
                (
                    "Choose Wallpaper",
                    crate::icons::WALLPAPER,
                    Message::LauncherWallpaper,
                ),
            ] {
                rows = rows.push(tile(text_settings, label, Some(glyph), message));
            }
        }
        Dialog::AddShortcut(state) => match &state.providers {
            None => rows = rows.push(plain_line(text_settings, "Loading…", false, false)),
            Some(Err(error)) => rows = rows.push(plain_line(text_settings, error, false, true)),
            Some(Ok(providers)) => {
                rows = rows.push(tile_with_enabled(
                    text_settings,
                    "Lock Screen",
                    Some(crate::icons::LOCK_OUTLINE),
                    Message::ShortcutLock,
                    !state.busy,
                ));
                if cfg!(target_os = "android") {
                    rows = rows.push(tile_with_enabled(
                        text_settings,
                        "Notifications",
                        Some(crate::icons::NOTIFICATIONS_NONE),
                        Message::ShortcutNotifications,
                        !state.busy,
                    ));
                }
                for provider in providers {
                    let message = if provider.id == "command" {
                        Message::ShortcutCommand
                    } else {
                        Message::ShortcutProvider(provider.id.clone())
                    };
                    rows = rows.push(tile_with_enabled(
                        text_settings,
                        &provider.label,
                        Some(crate::icons::LINK),
                        message,
                        !state.busy,
                    ));
                }
            }
        },
        Dialog::CommandShortcut(state) => {
            rows = rows.push(field(
                text_settings,
                "Name",
                &state.name,
                COMMAND_NAME_FIELD_ID,
                Message::CommandName,
                Message::CommandAdd,
            ));
            rows = rows.push(field(
                text_settings,
                "Command",
                &state.command,
                COMMAND_TEXT_FIELD_ID,
                Message::CommandText,
                Message::CommandAdd,
            ));
            rows = rows.push(plain_line(
                text_settings,
                "No shell is involved.",
                true,
                false,
            ));
        }
        Dialog::AddGroup(state) => {
            rows = rows.push(field(
                text_settings,
                "Name",
                &state.name,
                ADD_GROUP_FIELD_ID,
                Message::AddGroupName,
                Message::AddGroupConfirm,
            ));
        }
        Dialog::GroupEditor(state) => {
            rows = rows.push(field(
                text_settings,
                "Name",
                &state.name,
                GROUP_NAME_FIELD_ID,
                Message::GroupName,
                Message::GroupSave,
            ));
            let role_style = text_settings.role(TextRole::LabelLarge);
            rows = rows.push(
                text("Role")
                    .size(role_style.size)
                    .line_height(role_style.line_height())
                    .font(role_style.font)
                    .color(theme::color::ON_SURFACE),
            );
            for (label, role) in [
                ("Home", Some(GroupRole::Home)),
                ("Apps", Some(GroupRole::Apps)),
                ("Games", Some(GroupRole::Games)),
                ("None", None),
            ] {
                let enabled = !state.role_locked() || state.role == role;
                rows = rows.push(components::chip(
                    text_settings,
                    label,
                    state.role == role,
                    enabled,
                    AppMessage::Dialog(Message::GroupRole(role)),
                ));
            }
            rows = rows.push(tile(
                text_settings,
                "Choose apps",
                Some(crate::icons::CHECKLIST),
                Message::GroupChooseApps,
            ));
            if let Some(error) = &state.error {
                rows = rows.push(plain_line(text_settings, error, false, true));
            }
        }
        Dialog::Confirm(state) => {
            rows = rows.push(plain_line(text_settings, &state.body, false, false));
        }
        Dialog::RestorePicker(state) => match &state.backups {
            None => rows = rows.push(plain_line(text_settings, "Loading…", false, false)),
            Some(Err(error)) => rows = rows.push(plain_line(text_settings, error, false, true)),
            Some(Ok(names)) if names.is_empty() => {
                rows = rows.push(plain_line(
                    text_settings,
                    "No backups found in Downloads.",
                    false,
                    false,
                ))
            }
            Some(Ok(names)) => {
                for name in names {
                    rows = rows.push(tile(
                        text_settings,
                        name,
                        None,
                        Message::RestoreChosen(name.clone()),
                    ));
                }
            }
        },
    }
    rows.finish()
}

fn actions<'a>(app: &App, dialog: &Dialog, text_settings: &Text) -> Vec<Element<'a, AppMessage>> {
    let mut actions: Vec<Element<'a, AppMessage>> = Vec::new();
    match dialog {
        Dialog::App(editor) => {
            if editor.can_uninstall() {
                actions.push(components::text_button(
                    text_settings,
                    "Uninstall",
                    AppMessage::Dialog(Message::AppUninstall),
                    !editor.busy,
                ));
            }
            if editor.is_shortcut() {
                actions.push(components::text_button(
                    text_settings,
                    "Remove",
                    AppMessage::Dialog(Message::AppRemove),
                    !editor.busy,
                ));
            }
            actions.push(components::text_button(
                text_settings,
                "Cancel",
                AppMessage::Dialog(Message::Close),
                !editor.busy,
            ));
            actions.push(components::filled_button(
                text_settings,
                "Save",
                AppMessage::Dialog(Message::AppSave),
                editor.can_save(),
            ));
        }
        Dialog::Folder(editor) => {
            if editor.is_edit() {
                actions.push(components::destructive_button(
                    text_settings,
                    "Delete",
                    AppMessage::Dialog(Message::FolderDelete),
                    !editor.busy(),
                ));
            }
            actions.push(components::text_button(
                text_settings,
                "Cancel",
                AppMessage::Dialog(Message::Close),
                !editor.busy(),
            ));
            actions.push(components::filled_button(
                text_settings,
                "Save",
                AppMessage::Dialog(Message::FolderSave),
                editor.can_save(),
            ));
        }
        Dialog::Launcher | Dialog::RestorePicker(_) => {}
        Dialog::AddShortcut(state) => actions.push(components::text_button(
            text_settings,
            "Cancel",
            AppMessage::Dialog(Message::Close),
            !state.busy,
        )),
        Dialog::CommandShortcut(state) => {
            actions.push(components::text_button(
                text_settings,
                "Cancel",
                AppMessage::Dialog(Message::Close),
                !state.busy,
            ));
            actions.push(components::filled_button(
                text_settings,
                "Add",
                AppMessage::Dialog(Message::CommandAdd),
                state.ready(),
            ));
        }
        Dialog::AddGroup(state) => {
            actions.push(components::text_button(
                text_settings,
                "Cancel",
                AppMessage::Dialog(Message::Close),
                !state.busy,
            ));
            actions.push(components::filled_button(
                text_settings,
                "Add",
                AppMessage::Dialog(Message::AddGroupConfirm),
                state.can_confirm(),
            ));
        }
        Dialog::GroupEditor(state) => {
            actions.push(components::text_button(
                text_settings,
                "Delete",
                AppMessage::Dialog(Message::GroupDelete),
                state.can_delete(),
            ));
            actions.push(components::text_button(
                text_settings,
                "Cancel",
                AppMessage::Dialog(Message::Close),
                !state.busy,
            ));
            actions.push(components::filled_button(
                text_settings,
                "Save",
                AppMessage::Dialog(Message::GroupSave),
                state.can_save(),
            ));
        }
        Dialog::Confirm(state) => {
            actions.push(components::text_button(
                text_settings,
                "Cancel",
                AppMessage::Dialog(Message::Close),
                !state.busy,
            ));
            let message = AppMessage::Dialog(Message::ConfirmYes);
            if state.destructive {
                actions.push(components::filled_destructive_button(
                    text_settings,
                    &state.confirm_label,
                    message,
                    !state.busy,
                ));
            } else {
                actions.push(components::filled_button(
                    text_settings,
                    &state.confirm_label,
                    message,
                    !state.busy,
                ));
            }
        }
    }
    let _ = app;
    actions
}

pub(crate) fn view<'a>(app: &'a App, dialog: &'a Dialog) -> Element<'a, AppMessage> {
    let text_settings = dialog_text();
    let layout = layout(app, dialog, app.size);

    let mut content =
        iced::widget::stack![space().width(layout.card.width).height(layout.card.height)];
    if let (Some(title), Some(bounds)) = (heading(dialog), layout.title) {
        let style = text_settings.role(TextRole::HeadlineSmall);
        content = content.push(
            iced::widget::pin(
                container(
                    text(title)
                        .size(style.size)
                        .line_height(style.line_height())
                        .font(style.font)
                        .color(theme::color::ON_SURFACE),
                )
                .width(bounds.width)
                .height(bounds.height)
                .clip(true),
            )
            .x(bounds.x - layout.card.x)
            .y(bounds.y - layout.card.y),
        );
    }
    content = content.push(
        iced::widget::pin(
            scrollable(body(app, dialog, &text_settings))
                .id("dialog-scroll")
                .on_scroll(|viewport| {
                    AppMessage::Scrolled(crate::Region::Dialog, viewport.absolute_offset().y)
                })
                .height(layout.content.height)
                .width(layout.content.width)
                .style(theme::scrollable),
        )
        .x(layout.content.x - layout.card.x)
        .y(layout.content.y - layout.card.y),
    );
    let action_views = actions(app, dialog, &text_settings);
    for (action, (bounds, _)) in action_views.into_iter().zip(&layout.actions) {
        content = content.push(
            iced::widget::pin(
                container(action)
                    .width(bounds.width)
                    .height(bounds.height)
                    .clip(true),
            )
            .x(bounds.x - layout.card.x)
            .y(bounds.y - layout.card.y),
        );
    }

    let card = mouse_area(
        container(content)
            .width(layout.card.width)
            .height(layout.card.height)
            .style(theme::card),
    )
    .on_press(AppMessage::Dialog(Message::Noop));

    let scrim = mouse_area(
        container(space())
            .width(Fill)
            .height(Fill)
            .style(theme::scrim),
    )
    .on_press(AppMessage::Dialog(Message::Close));

    iced::widget::opaque(iced::widget::stack![
        scrim,
        iced::widget::pin(card).x(layout.card.x).y(layout.card.y)
    ])
}

pub(crate) fn select_anchor(app: &App, select: &Select) -> Rectangle {
    if let Some(dialog) = &app.dialog
        && let Some(slot) = layout(app, dialog, app.size)
            .rows
            .iter()
            .find(|(_, index)| {
                matches!(rows(app, dialog).get(*index), Some(Row::Select { .. }))
                    && matches!(
                        select.field,
                        SelectField::AppGroup | SelectField::FolderGroup
                    )
            })
    {
        let scrolled = app
            .scroll
            .get(&crate::Region::Dialog)
            .copied()
            .unwrap_or(0.0);
        return Rectangle {
            y: slot.0.y - scrolled,
            ..slot.0
        };
    }
    Rectangle {
        x: app.size.width / 2.0 - 140.0,
        y: app.size.height / 3.0,
        width: 280.0,
        height: theme::metrics::FIELD_HEIGHT,
    }
}

pub(crate) fn select_view<'a>(app: &'a App, select: &'a Select) -> Element<'a, AppMessage> {
    let text_settings = dialog_text();
    let anchor = select_anchor(app, select);
    let bounds = components::select_menu_bounds(anchor, select.options.len(), app.size);
    let ids: Vec<String> = select
        .options
        .iter()
        .map(|option| option.id.clone())
        .collect();
    let options: Vec<(String, String, bool)> = select
        .options
        .iter()
        .map(|option| (option.id.clone(), option.label.clone(), option.folder))
        .collect();

    let barrier = mouse_area(container(space()).width(Fill).height(Fill))
        .on_press(AppMessage::Dialog(Message::CloseSelect));
    let menu = container(components::select_menu(
        &text_settings,
        options,
        Some(select.selected.as_str()),
        move |index| AppMessage::Dialog(Message::GroupChosen(ids[index].clone())),
    ))
    .width(bounds.width)
    .max_height(bounds.height)
    .style(theme::menu);

    iced::widget::opaque(iced::widget::stack![
        barrier,
        iced::widget::pin(menu).x(bounds.x).y(bounds.y)
    ])
}

/// Create a folder, or rename and move an existing one.
pub(crate) fn folder_save_plan(editor: &FolderEditor, snapshot: &ModelSnapshot) -> Vec<Change> {
    let mut plan = Vec::new();
    match editor {
        FolderEditor::Create { group_id, name, .. } => {
            let name = name.trim();
            if !name.is_empty() {
                plan.push(Change::AddFolder(group_id.clone(), name.to_string()));
            }
        }
        FolderEditor::Edit {
            folder,
            label,
            name,
            group_id,
            ..
        } => {
            let name = name.trim();
            if name != label {
                plan.push(Change::RenameGroup(folder.clone(), name.to_string()));
            }
            if folder_of(snapshot, folder).as_ref() != Some(group_id)
                && let Some(tile) = snapshot
                    .items
                    .iter()
                    .find(|item| item.folder_group_id.as_deref() == Some(folder.as_str()))
            {
                plan.push(Change::MoveItem(
                    tile.id.clone(),
                    group_id.clone(),
                    append_order(snapshot, group_id),
                ));
            }
        }
    }
    plan
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::App;
    use crate::tests::{app, item};

    fn dialog_app() -> (App, Item) {
        let mut app = app();
        let item = item(&app, "calculator");
        app.snapshot.as_mut().unwrap().items.push(item.clone());
        (app, item)
    }

    #[test]
    fn card_is_centered_and_width_capped() {
        let (app, _) = dialog_app();
        let layout = layout(&app, &Dialog::Launcher, Size::new(1280.0, 800.0));
        assert_eq!(layout.card.width, theme::metrics::DIALOG_MAX_WIDTH);
        assert!((layout.card.x - (1280.0 - layout.card.width) / 2.0).abs() < 0.5);
    }

    #[test]
    fn narrow_window_uses_side_margins() {
        let (app, _) = dialog_app();
        let layout = layout(&app, &Dialog::Launcher, Size::new(360.0, 640.0));
        assert_eq!(
            layout.card.width,
            360.0 - 2.0 * theme::metrics::DIALOG_SIDE_MARGIN
        );
    }

    #[test]
    fn dialog_bounds_and_actions_fit_a_narrow_window() {
        let (app, item) = dialog_app();
        let layout = layout(
            &app,
            &Dialog::App(AppEditor::new(item)),
            Size::new(240.0, 640.0),
        );
        assert!(layout.card.x >= 0.0);
        assert!(layout.card.x + layout.card.width <= 240.0);
        for (bounds, _) in &layout.actions {
            assert!(bounds.x >= layout.card.x);
            assert!(bounds.x + bounds.width <= layout.card.x + layout.card.width);
        }
    }

    #[test]
    fn long_confirmation_body_gets_more_than_one_line() {
        let (app, _) = dialog_app();
        let dialog = Dialog::Confirm(ConfirmState {
            title: "Delete Fun?".into(),
            body: "The apps in this group move to the Apps group.".into(),
            confirm_label: "Delete".into(),
            destructive: true,
            busy: false,
            action: ConfirmAction::DeleteGroup("fun".into()),
        });
        let layout = layout(&app, &dialog, Size::new(360.0, 800.0));
        let text = dialog_text();
        let (row, _) = layout
            .rows
            .iter()
            .find(|(_, index)| matches!(rows(&app, &dialog)[*index], Row::Note { .. }))
            .unwrap();
        assert!(row.height >= 2.0 * text.role(TextRole::BodyMedium).line_height);
    }

    #[test]
    fn launcher_popup_has_no_title_block() {
        let app = dialog_app().0;
        let layout = layout(&app, &Dialog::Launcher, Size::new(360.0, 800.0));
        assert!(layout.title.is_none(), "launcher actions need no heading");
    }

    #[test]
    fn launcher_popup_uses_row_padding_not_extra_dialog_padding() {
        let app = dialog_app().0;
        let layout = layout(&app, &Dialog::Launcher, Size::new(360.0, 800.0));
        assert_eq!(layout.content.x, layout.card.x);
        assert_eq!(layout.content.width, layout.card.width);
        assert_eq!(layout.content.y - layout.card.y, 8.0);
        assert_eq!(
            layout.card.y + layout.card.height - layout.content.y - layout.content.height,
            8.0
        );
        assert_eq!(layout.rows.len(), 5);
        for (row, _) in &layout.rows {
            assert_eq!(row.height, 56.0, "keep full-height menu touch targets");
        }
    }

    #[test]
    fn tile_rows_fit_the_fixed_body_large_line_height() {
        // Chrome rows follow the fixed Material scale, not label_size.
        let app = dialog_app().0;
        let layout = layout(&app, &Dialog::Launcher, Size::new(360.0, 800.0));
        let text = dialog_text();
        let (row, _) = layout
            .rows
            .iter()
            .find(|(_, index)| matches!(rows(&app, &Dialog::Launcher)[*index], Row::Tile { .. }))
            .unwrap();
        assert!(row.height >= text.role(TextRole::BodyLarge).line_height);
    }

    #[test]
    fn select_anchor_follows_the_dialog_scroll_offset() {
        let (mut app, item) = dialog_app();
        app.dialog = Some(Dialog::App(AppEditor::new(item)));
        let unscrolled = select_anchor(
            &app,
            &Select {
                field: SelectField::AppGroup,
                options: vec![],
                selected: "apps".into(),
            },
        );
        app.scroll.insert(crate::Region::Dialog, 40.0);
        let scrolled = select_anchor(
            &app,
            &Select {
                field: SelectField::AppGroup,
                options: vec![],
                selected: "apps".into(),
            },
        );
        assert!((unscrolled.y - scrolled.y - 40.0).abs() < 0.5);
    }

    #[test]
    fn group_role_chips_share_a_line_until_they_overflow() {
        let (app, _) = dialog_app();
        let dialog = Dialog::GroupEditor(GroupEditorState::new("g".into(), "G".into(), None));
        let chips = |size| {
            let layout = layout(&app, &dialog, size);
            layout
                .rows
                .iter()
                .filter(|(_, index)| matches!(rows(&app, &dialog)[*index], Row::Chip { .. }))
                .map(|(r, _)| (r.x, r.y, r.width))
                .collect::<Vec<_>>()
        };
        // Wide enough for all four: one line, left to right.
        let wide = chips(Size::new(600.0, 800.0));
        assert_eq!(wide.len(), 4);
        assert!(wide.iter().all(|(_, y, _)| (*y - wide[0].1).abs() < 0.5));
        assert!(wide[1].0 > wide[0].0 && wide[2].0 > wide[1].0);
        // Narrow enough that the last chip no longer fits: it wraps to the left.
        let narrow = chips(Size::new(360.0, 800.0));
        assert!(narrow[1].1 == narrow[0].1 && narrow[2].1 == narrow[0].1);
        assert!(narrow[3].1 > narrow[2].1, "the fourth chip wraps");
        assert!(narrow[3].0 < narrow[1].0);
    }

    #[test]
    fn app_save_plan_renames_then_moves() {
        let (app, item) = dialog_app();
        let snapshot = app.snapshot.as_ref().unwrap();
        let target = snapshot.page_ids[1].clone();
        let plan = app_save_plan(&item, "New name", &target, snapshot);
        assert!(matches!(plan[0], Change::RenameItem(_, ref n) if n == "New name"));
        assert!(matches!(plan[1], Change::MoveItem(_, ref g, _) if g == &target));
    }

    #[test]
    fn app_save_plan_is_empty_when_nothing_changed() {
        let (app, item) = dialog_app();
        let snapshot = app.snapshot.as_ref().unwrap();
        assert!(app_save_plan(&item, &item.label, &item.group_id, snapshot).is_empty());
    }

    #[test]
    fn new_folder_is_created_in_the_selected_group() {
        let (app, _) = dialog_app();
        let snapshot = app.snapshot.as_ref().unwrap();
        let mut editor = FolderEditor::create(snapshot.page_ids[0].clone());
        editor.set_group(snapshot.page_ids[1].clone());
        assert_eq!(
            folder_save_plan(&editor, snapshot),
            vec![Change::AddFolder(
                snapshot.page_ids[1].clone(),
                "Folder".into()
            )]
        );
    }

    #[test]
    fn content_scrolls_when_taller_than_the_window() {
        let (app, _) = dialog_app();
        let layout = layout(&app, &Dialog::Launcher, Size::new(360.0, 200.0));
        assert!(layout.content_height > layout.content.height);
    }

    #[test]
    fn app_editor_actions_hide_remove_for_plain_apps() {
        let (app, item) = dialog_app();
        let editor = AppEditor::new(item);
        let labels = action_labels(&Dialog::App(editor));
        assert_eq!(labels, vec!["Uninstall", "Cancel", "Save"]);
        assert!(!labels.contains(&"Remove".to_string()));
        let _ = app;
    }

    #[test]
    fn group_role_locked_for_one_holder_roles() {
        let mut state = GroupEditorState::new("g".into(), "G".into(), Some(GroupRole::Apps));
        assert!(state.role_locked());
        state.role = Some(GroupRole::Home);
        assert!(!state.role_locked());
        state.role = None;
        assert!(!state.role_locked());
    }

    #[test]
    fn add_group_confirm_disabled_until_named() {
        let mut state = AddGroupState::new();
        state.name = "  ".into();
        assert!(!state.can_confirm());
        state.name = " Fun ".into();
        assert!(state.can_confirm());
    }

    #[test]
    fn command_shortcut_ready_needs_both_fields() {
        let mut state = CommandShortcutState::new("apps".into());
        state.name = "X".into();
        assert!(!state.ready());
        state.command = "firefox".into();
        assert!(state.ready());
    }

    #[test]
    fn launcher_menu_has_five_tile_rows() {
        let (app, _) = dialog_app();
        let rows = rows(&app, &Dialog::Launcher);
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row, Row::Tile { .. }))
                .count(),
            5
        );
    }
}
