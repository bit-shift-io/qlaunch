//! Accessibility frame derived from launcher state and shared geometry.

use std::collections::HashMap;

use qlaunch_core::model::{Id, Item};
use serde::Serialize;

use crate::overlay::{self, EntryKind, Overlay};
use crate::{App, Region, grid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role {
    Button,
    Tab,
    Header,
    Text,
    TextField,
    Scroll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Action {
    Click,
    LongClick,
    ScrollForward,
    ScrollBackward,
    ScrollLeft,
    ScrollRight,
    SetText,
    Dismiss,
}

impl Action {
    #[cfg_attr(not(target_os = "android"), allow(dead_code))] // only Kotlin sends action strings.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "click" => Action::Click,
            "long_click" => Action::LongClick,
            "scroll_forward" => Action::ScrollForward,
            "scroll_backward" => Action::ScrollBackward,
            "scroll_left" => Action::ScrollLeft,
            "scroll_right" => Action::ScrollRight,
            "set_text" => Action::SetText,
            "dismiss" => Action::Dismiss,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextField {
    Search,
    AppName,
    FolderName,
}

#[derive(Debug, Clone)]
pub(crate) enum Target {
    Item(Id),
    Step(i32),
    Search,
    Back,
    SearchTab(bool),
    TextField(TextField),
    Scroll {
        region: Region,
        offset: f32,
        viewport: f32,
        content: f32,
    },
    Overlay(overlay::Message),
    Dismiss,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Node {
    #[serde(skip)]
    pub key: String,
    #[serde(skip)]
    pub target: Target,
    pub id: i32,
    pub role: Role,
    pub label: String,
    pub bounds: [f32; 4],
    pub actions: Vec<Action>,
    pub selected: bool,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Frame {
    pub screen: String,
    /// Identifies the announced context; same as `screen`, except
    /// `launcher:<group-id>` so a page step announces the new page.
    pub announcement_key: String,
    pub window: [f32; 2],
    pub nodes: Vec<Node>,
}

#[derive(Default)]
pub(crate) struct NodeIds {
    by_key: HashMap<String, i32>,
    next: i32,
}

impl NodeIds {
    /// Gives every node its stable id. A semantic key keeps its id for the
    /// lifetime of the process; ids are never reused.
    pub(crate) fn assign(&mut self, frame: &mut Frame) {
        for node in &mut frame.nodes {
            let next = &mut self.next;
            let id = *self.by_key.entry(node.key.clone()).or_insert_with(|| {
                *next += 1;
                *next
            });
            node.id = id;
        }
    }
}

pub(crate) const QUIESCENCE: std::time::Duration = std::time::Duration::from_millis(150);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActionRequest {
    pub node_id: i32,
    pub action: Action,
    pub value: String,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Payload {
    pub revision: u64,
    pub screen: String,
    pub window: Window,
    pub nodes: Vec<Node>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announcement: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct Window {
    pub width: f32,
    pub height: f32,
}

#[derive(Default)]
pub(crate) struct Publisher {
    revision: u64,
    fingerprint: Option<String>,
    announced: Option<String>,
}

impl Publisher {
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// Builds the next payload without committing it, so a failed JNI call
    /// does not consume a revision or suppress the retry.
    pub(crate) fn prepare(&self, frame: &Frame, announcement: Option<String>) -> Option<Payload> {
        let fingerprint = fingerprint(frame);
        if self.fingerprint.as_deref() == Some(fingerprint.as_str()) {
            return None;
        }
        let announcement = announcement
            .filter(|_| self.announced.as_deref() != Some(frame.announcement_key.as_str()));
        Some(Payload {
            revision: self.revision + 1,
            screen: frame.screen.clone(),
            window: Window {
                width: frame.window[0],
                height: frame.window[1],
            },
            nodes: frame.nodes.clone(),
            announcement,
        })
    }

    pub(crate) fn commit(&mut self, payload: &Payload, frame: &Frame) {
        self.revision = payload.revision;
        self.fingerprint = Some(fingerprint(frame));
        if payload.announcement.is_some() {
            self.announced = Some(frame.announcement_key.clone());
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    pub(crate) ids: NodeIds,
    pub(crate) publisher: Publisher,
    pub(crate) frame: Option<Frame>,
    pub(crate) generation: u64,
    pub(crate) pending_scroll: bool,
}

impl State {
    pub(crate) fn scroll_started(&mut self) -> u64 {
        self.generation += 1;
        self.pending_scroll = true;
        self.generation
    }

    pub(crate) fn scroll_settled(&mut self, generation: u64) -> bool {
        if generation != self.generation || !self.pending_scroll {
            return false;
        }
        self.pending_scroll = false;
        true
    }

    pub(crate) fn cancel_scroll(&mut self) {
        self.generation += 1;
        self.pending_scroll = false;
    }

    pub(crate) fn publish(
        &mut self,
        mut frame: Frame,
        announcement: Option<String>,
    ) -> Result<u64, String> {
        self.ids.assign(&mut frame);
        let Some(payload) = self.publisher.prepare(&frame, announcement) else {
            self.frame = Some(frame);
            return Ok(self.publisher.revision());
        };
        let json = serialize(&payload);
        publish_json(&json)?;
        self.publisher.commit(&payload, &frame);
        self.frame = Some(frame);
        Ok(payload.revision)
    }
}

fn fingerprint(frame: &Frame) -> String {
    let mut out = format!(
        "{}|{}|{}|{}",
        frame.screen,
        frame.announcement_key,
        frame.window[0].to_bits(),
        frame.window[1].to_bits()
    );
    for node in &frame.nodes {
        out.push_str(&format!(
            "|{}|{:?}|{}|{:?}|{:?}|{}|{}|{:?}",
            node.key,
            node.role,
            node.label,
            node.bounds.map(f32::to_bits),
            node.actions,
            node.selected,
            node.enabled,
            node.value
        ));
    }
    out
}

pub(crate) fn serialize(payload: &Payload) -> String {
    serde_json::to_string(payload).expect("the accessibility payload serializes")
}

fn sender() -> &'static tokio::sync::broadcast::Sender<ActionRequest> {
    static ACTIONS: std::sync::OnceLock<tokio::sync::broadcast::Sender<ActionRequest>> =
        std::sync::OnceLock::new();
    ACTIONS.get_or_init(|| tokio::sync::broadcast::channel(64).0)
}

pub(crate) fn subscribe() -> iced::Subscription<ActionRequest> {
    iced::Subscription::run(|| {
        iced::stream::channel(64, async |mut output| {
            use iced::futures::SinkExt;
            let mut receiver = sender().subscribe();
            while let Ok(request) = receiver.recv().await {
                if output.send(request).await.is_err() {
                    break;
                }
            }
        })
    })
}

#[cfg(target_os = "android")]
fn push_request(request: ActionRequest) {
    let _ = sender().send(request);
}

#[cfg(target_os = "android")]
pub(crate) fn publish_json(json: &str) -> Result<(), String> {
    qlaunch_core::platform::android::jni_host::JniHost::call_void(
        "publishAccessibility",
        &[json.to_owned()],
    )
}

#[cfg(not(target_os = "android"))]
pub(crate) fn publish_json(_json: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_bitshift_qlaunch_MainActivity_onAccessibilityAction(
    mut env: jni::JNIEnv,
    _this: jni::objects::JObject,
    node_id: jni::sys::jint,
    action: jni::objects::JString,
    value: jni::objects::JString,
    revision: jni::sys::jlong,
) {
    let Ok(action) = env.get_string(&action) else {
        return;
    };
    let action: String = action.into();
    let Some(action) = Action::parse(&action) else {
        return;
    };
    let value = env.get_string(&value).map(String::from).unwrap_or_default();
    push_request(ActionRequest {
        node_id,
        action,
        value,
        revision: revision.max(0) as u64,
    });
}

pub(crate) fn frame(app: &App) -> Option<Frame> {
    app.snapshot.as_ref()?;
    if let Some(overlay) = &app.overlay {
        return Some(overlay_frame(app, overlay));
    }
    if app.searching {
        return Some(search_frame(app));
    }
    if let Some(folder) = &app.folder {
        return Some(folder_frame(app, folder));
    }
    Some(launcher_frame(app))
}

pub(crate) fn announcement(app: &App) -> String {
    if let Some(overlay) = &app.overlay {
        return overlay::heading(overlay);
    }
    let Some(snapshot) = &app.snapshot else {
        return String::new();
    };
    if app.searching {
        return "Search".into();
    }
    if let Some(folder) = &app.folder {
        return group_label(snapshot.groups.as_slice(), folder)
            .unwrap_or("Folder")
            .to_string();
    }
    let page = app.pager.committed() + 1;
    format!(
        "{}, page {page} of {}",
        page_group_label(app),
        snapshot.page_ids.len()
    )
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> iced::Rectangle {
    iced::Rectangle {
        x,
        y,
        width,
        height,
    }
}

fn window(app: &App) -> iced::Rectangle {
    rect(0.0, 0.0, app.size.width, app.size.height)
}

fn group_label<'a>(groups: &'a [qlaunch_core::model::Group], id: &Id) -> Option<&'a str> {
    groups
        .iter()
        .find(|group| &group.id == id)
        .map(|group| group.label.as_str())
}

fn page_group_label(app: &App) -> &str {
    app.snapshot
        .as_ref()
        .and_then(|snapshot| {
            snapshot
                .page_ids
                .get(app.pager.committed())
                .and_then(|id| group_label(&snapshot.groups, id))
        })
        .unwrap_or("")
}

fn screen_with_key(app: &App, screen: String) -> Frame {
    let announcement_key = if screen == "launcher" {
        format!(
            "launcher:{}",
            app.current_group_id().cloned().unwrap_or_default()
        )
    } else {
        screen.clone()
    };
    Frame {
        screen,
        announcement_key,
        window: [app.size.width, app.size.height],
        nodes: vec![],
    }
}

#[allow(clippy::too_many_arguments)]
fn push(
    app: &App,
    nodes: &mut Vec<Node>,
    key: impl Into<String>,
    target: Target,
    role: Role,
    label: impl Into<String>,
    bounds: iced::Rectangle,
    actions: Vec<Action>,
    selected: bool,
    value: Option<String>,
    viewport: Option<iced::Rectangle>,
) {
    let Some(bounds) = bounds.intersection(&window(app)) else {
        return;
    };
    if let Some(viewport) = viewport
        && bounds.intersection(&viewport).is_none()
    {
        return;
    }
    let bounds = match viewport {
        Some(viewport) => bounds.intersection(&viewport).unwrap_or(bounds),
        None => bounds,
    };
    nodes.push(Node {
        key: key.into(),
        target,
        id: 0,
        role,
        label: label.into(),
        bounds: [bounds.x, bounds.y, bounds.width, bounds.height],
        actions,
        selected,
        enabled: true,
        value,
    });
}

fn scroll_offset(app: &App, region: &Region, content: f32, viewport: f32) -> f32 {
    let max = (content - viewport).max(0.0);
    app.scroll
        .get(region)
        .copied()
        .unwrap_or(0.0)
        .clamp(0.0, max)
}

fn scroll_actions(offset: f32, viewport: f32, content: f32) -> Vec<Action> {
    let max = (content - viewport).max(0.0);
    let mut actions = Vec::new();
    if offset > 0.5 {
        actions.push(Action::ScrollBackward);
    }
    if offset < max - 0.5 {
        actions.push(Action::ScrollForward);
    }
    actions
}

fn launcher_frame(app: &App) -> Frame {
    let snapshot = app.snapshot.as_ref().expect("frame requires a snapshot");
    let settings = &snapshot.settings;
    let mut frame = screen_with_key(app, "launcher".into());
    push_titles(app, &mut frame.nodes);

    let footer = grid::footer_height(app.error.is_some(), snapshot.warnings.len());
    let viewport = grid::viewport(
        app.size,
        app.insets,
        settings,
        false,
        true,
        false,
        snapshot.page_ids.len(),
        footer,
    );
    let items = app.visible_items();
    let label = if snapshot.page_ids.is_empty() {
        format!("{} apps", items.len())
    } else {
        format!(
            "{}, page {} of {}, {} apps",
            page_group_label(app),
            app.pager.committed() + 1,
            snapshot.page_ids.len(),
            items.len()
        )
    };
    let region = Region::Group(
        snapshot
            .page_ids
            .get(app.pager.committed())
            .cloned()
            .unwrap_or_default(),
    );
    push_grid(
        app,
        &mut frame.nodes,
        region,
        viewport,
        label,
        &items,
        true,
        snapshot.page_ids.len() > 1,
        false,
    );
    push_footer(app, &mut frame.nodes, viewport);
    frame
}

fn folder_frame(app: &App, folder: &Id) -> Frame {
    let snapshot = app.snapshot.as_ref().expect("frame requires a snapshot");
    let settings = &snapshot.settings;
    let mut frame = screen_with_key(app, format!("folder:{folder}"));
    let label = group_label(&snapshot.groups, folder).unwrap_or("Folder");

    push(
        app,
        &mut frame.nodes,
        "folder:back",
        Target::Back,
        Role::Button,
        "Back",
        grid::folder_back(app.size, app.insets, settings),
        vec![Action::Click],
        false,
        None,
        None,
    );
    push(
        app,
        &mut frame.nodes,
        "folder:title",
        Target::Dismiss,
        Role::Header,
        label,
        grid::folder_title(app.size, app.insets, settings),
        vec![],
        false,
        None,
        None,
    );

    let footer = grid::footer_height(app.error.is_some(), snapshot.warnings.len());
    let viewport = grid::viewport(
        app.size,
        app.insets,
        settings,
        false,
        true,
        true,
        snapshot.page_ids.len(),
        footer,
    );
    let items = app.visible_items();
    push_grid(
        app,
        &mut frame.nodes,
        Region::Group(folder.clone()),
        viewport,
        format!("{label}, {} apps", items.len()),
        &items,
        true,
        false,
        false,
    );
    push_footer(app, &mut frame.nodes, viewport);
    frame
}

fn search_frame(app: &App) -> Frame {
    let snapshot = app.snapshot.as_ref().expect("frame requires a snapshot");
    let settings = &snapshot.settings;
    let mut frame = screen_with_key(app, "search".into());

    push(
        app,
        &mut frame.nodes,
        "search:field",
        Target::TextField(TextField::Search),
        Role::TextField,
        "Search apps",
        grid::search_field(app.size, app.insets),
        vec![Action::Click, Action::SetText],
        false,
        Some(app.query.clone()),
        None,
    );
    push(
        app,
        &mut frame.nodes,
        "search:close",
        Target::Search,
        Role::Button,
        "Close search",
        grid::search_close(app.size, app.insets),
        vec![Action::Click],
        false,
        None,
        None,
    );
    if app.query.is_empty() {
        push(
            app,
            &mut frame.nodes,
            "search:frequent",
            Target::SearchTab(false),
            Role::Tab,
            "Frequent",
            grid::search_tab(app.size, app.insets, false),
            vec![Action::Click],
            !app.show_recent,
            None,
            None,
        );
        push(
            app,
            &mut frame.nodes,
            "search:recent",
            Target::SearchTab(true),
            Role::Tab,
            "Recent",
            grid::search_tab(app.size, app.insets, true),
            vec![Action::Click],
            app.show_recent,
            None,
            None,
        );
    }

    let footer = grid::footer_height(app.error.is_some(), snapshot.warnings.len());
    let viewport = grid::viewport(
        app.size,
        app.insets,
        settings,
        true,
        app.query.is_empty(),
        false,
        snapshot.page_ids.len(),
        footer,
    );
    let items = app.visible_items();
    let label = if app.query.is_empty() {
        format!(
            "{}, {} apps",
            if app.show_recent {
                "Recent"
            } else {
                "Frequent"
            },
            items.len()
        )
    } else {
        format!("{} results", items.len())
    };
    push_grid(
        app,
        &mut frame.nodes,
        Region::Search,
        viewport,
        label,
        &items,
        false,
        false,
        true,
    );
    push_footer(app, &mut frame.nodes, viewport);
    frame
}

fn push_titles(app: &App, nodes: &mut Vec<Node>) {
    let snapshot = app.snapshot.as_ref().expect("frame requires a snapshot");
    if !grid::tabs_visible(&snapshot.settings, app.size, snapshot.page_ids.len()) {
        return;
    }
    let strip = grid::title_strip(app.size, app.insets, &snapshot.settings);
    let width = strip.width / 3.0;
    let labels: Vec<String> = app
        .pager
        .slots()
        .iter()
        .map(|slot| {
            snapshot
                .page_ids
                .get(*slot)
                .and_then(|id| group_label(&snapshot.groups, id))
                .unwrap_or("")
                .to_string()
        })
        .collect();
    let specs = [
        (
            "title:prev",
            format!("Previous page, {}", labels[0]),
            Target::Step(-1),
            false,
        ),
        (
            "title:current",
            format!("{}, search apps", labels[1]),
            Target::Search,
            true,
        ),
        (
            "title:next",
            format!("Next page, {}", labels[2]),
            Target::Step(1),
            false,
        ),
    ];
    for (index, (key, label, target, selected)) in specs.into_iter().enumerate() {
        push(
            app,
            nodes,
            key,
            target,
            Role::Tab,
            label,
            rect(strip.x + index as f32 * width, strip.y, width, strip.height),
            vec![Action::Click],
            selected,
            None,
            None,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_grid(
    app: &App,
    nodes: &mut Vec<Node>,
    region: Region,
    viewport: iced::Rectangle,
    label: String,
    items: &[&Item],
    long_click: bool,
    page_actions: bool,
    search_results: bool,
) {
    let row_height = grid::row_height(
        &app.snapshot
            .as_ref()
            .expect("frame requires a snapshot")
            .settings,
    );
    let columns = app.columns();
    let rows = items.len().div_ceil(columns);
    let content = rows as f32 * row_height;
    let offset = scroll_offset(app, &region, content, viewport.height);
    let mut grid_actions = scroll_actions(offset, viewport.height, content);
    if long_click {
        grid_actions.push(Action::LongClick);
    }
    if page_actions {
        grid_actions.push(Action::ScrollLeft);
        grid_actions.push(Action::ScrollRight);
    }
    push(
        app,
        nodes,
        "grid:container",
        Target::Scroll {
            region: region.clone(),
            offset,
            viewport: viewport.height,
            content,
        },
        Role::Scroll,
        label,
        viewport,
        grid_actions,
        false,
        None,
        None,
    );

    let cell_width = viewport.width / columns as f32;
    for (index, item) in items.iter().enumerate() {
        let row = index / columns;
        let column = index % columns;
        let mut actions = vec![Action::Click];
        if !search_results {
            actions.push(Action::LongClick);
        }
        push(
            app,
            nodes,
            format!("item:{}", item.id),
            Target::Item(item.id.clone()),
            Role::Button,
            item.label.clone(),
            rect(
                viewport.x + column as f32 * cell_width,
                viewport.y + row as f32 * row_height - offset,
                cell_width,
                row_height,
            ),
            actions,
            false,
            None,
            Some(viewport),
        );
    }
}

fn push_footer(app: &App, nodes: &mut Vec<Node>, viewport: iced::Rectangle) {
    let Some(snapshot) = &app.snapshot else {
        return;
    };
    let width = viewport.width;
    let mut y = viewport.y + viewport.height;
    if let Some(error) = &app.error {
        push(
            app,
            nodes,
            "footer:error",
            Target::Dismiss,
            Role::Text,
            error.clone(),
            rect(viewport.x, y, width, grid::ERROR_LINE_HEIGHT),
            vec![],
            false,
            None,
            None,
        );
        y += grid::ERROR_LINE_HEIGHT;
    }
    for (index, warning) in snapshot.warnings.iter().enumerate() {
        push(
            app,
            nodes,
            format!("footer:warning:{index}"),
            Target::Dismiss,
            Role::Text,
            warning.clone(),
            rect(viewport.x, y, width, grid::WARNING_LINE_HEIGHT),
            vec![],
            false,
            None,
            None,
        );
        y += grid::WARNING_LINE_HEIGHT;
    }
}

fn overlay_frame(app: &App, overlay: &Overlay) -> Frame {
    let mut frame = screen_with_key(app, overlay_screen(overlay));
    let layout = overlay::card(app, overlay);
    let card_bounds = layout.bounds;
    push(
        app,
        &mut frame.nodes,
        "overlay:title",
        Target::Dismiss,
        Role::Header,
        overlay::heading(overlay),
        layout.title,
        vec![Action::Dismiss],
        false,
        None,
        Some(card_bounds),
    );

    let entries = overlay::entries(app, overlay);
    debug_assert_eq!(entries.len(), layout.entries.len());
    for (entry, layout) in entries.into_iter().zip(layout.entries) {
        debug_assert_eq!(entry.key, layout.key);
        match entry.kind {
            EntryKind::Action { label, message } => push(
                app,
                &mut frame.nodes,
                format!("overlay:action:{}", entry.key),
                Target::Overlay(message),
                Role::Button,
                label,
                layout.bounds,
                vec![Action::Click],
                false,
                None,
                Some(card_bounds),
            ),
            EntryKind::Input {
                hint,
                value,
                widget_id,
                ..
            } => {
                let field = if widget_id == overlay::APP_NAME_FIELD_ID {
                    TextField::AppName
                } else {
                    TextField::FolderName
                };
                let name = match field {
                    TextField::AppName => "app-name",
                    TextField::FolderName => "folder-name",
                    TextField::Search => "search",
                };
                push(
                    app,
                    &mut frame.nodes,
                    format!("overlay:field:{name}"),
                    Target::TextField(field),
                    Role::TextField,
                    hint,
                    layout.bounds,
                    vec![Action::Click, Action::SetText],
                    false,
                    Some(value),
                    Some(card_bounds),
                );
            }
            EntryKind::Text { label, error } => push(
                app,
                &mut frame.nodes,
                if error {
                    "overlay:error".to_string()
                } else {
                    format!("overlay:text:{}", entry.key)
                },
                Target::Dismiss,
                Role::Text,
                label,
                layout.bounds,
                vec![],
                false,
                None,
                Some(card_bounds),
            ),
            EntryKind::List { rows } => {
                let content = rows.len() as f32 * overlay::ACTION_HEIGHT
                    + rows.len().saturating_sub(1) as f32 * overlay::LIST_ROW_GAP;
                let offset =
                    scroll_offset(app, &Region::OverlayList, content, overlay::LIST_HEIGHT);
                let mut actions = scroll_actions(offset, overlay::LIST_HEIGHT, content);
                actions.push(Action::Dismiss);
                push(
                    app,
                    &mut frame.nodes,
                    "overlay:list",
                    Target::Scroll {
                        region: Region::OverlayList,
                        offset,
                        viewport: overlay::LIST_HEIGHT,
                        content,
                    },
                    Role::Scroll,
                    list_label(overlay),
                    layout.bounds,
                    actions,
                    false,
                    None,
                    Some(card_bounds),
                );
                for (index, row) in rows.into_iter().enumerate() {
                    push(
                        app,
                        &mut frame.nodes,
                        format!("overlay:item:{}", row.key),
                        Target::Overlay(row.message),
                        Role::Button,
                        row.label,
                        rect(
                            layout.bounds.x,
                            layout.bounds.y
                                + index as f32 * (overlay::ACTION_HEIGHT + overlay::LIST_ROW_GAP)
                                - offset,
                            layout.bounds.width,
                            overlay::ACTION_HEIGHT,
                        ),
                        vec![Action::Click],
                        false,
                        None,
                        Some(layout.bounds),
                    );
                }
            }
        }
    }
    frame
}

fn overlay_screen(overlay: &Overlay) -> String {
    match overlay {
        Overlay::App(menu) => format!(
            "overlay:app:{}",
            match menu.step {
                overlay::AppStep::Actions => "actions",
                overlay::AppStep::Rename(_) => "rename",
                overlay::AppStep::Move => "move",
            }
        ),
        Overlay::Launcher(menu) => format!(
            "overlay:launcher:{}",
            match menu.step {
                overlay::LauncherStep::Actions => "actions",
                overlay::LauncherStep::FolderName(_) => "folder-name",
                overlay::LauncherStep::Shortcuts => "shortcuts",
            }
        ),
    }
}

fn list_label(overlay: &Overlay) -> &'static str {
    match overlay {
        Overlay::App(menu) if matches!(menu.step, overlay::AppStep::Move) => "Move to page",
        Overlay::Launcher(menu) if matches!(menu.step, overlay::LauncherStep::Shortcuts) => {
            "Shortcut providers"
        }
        _ => "List",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::{self, AppMenu, AppStep, LauncherMenu, LauncherStep, Overlay};
    use crate::{App, Pager, Region};
    use iced::Size;
    use iced_android::Insets;
    use qlaunch_core::api::api::ModelSnapshot;
    use qlaunch_core::model::{AppRef, Group, GroupRole, Item};
    use qlaunch_core::settings::Settings;

    fn group(id: &str, label: &str, role: Option<GroupRole>, order: i32, folder: bool) -> Group {
        Group {
            id: id.into(),
            label: label.into(),
            parent_id: Some("root".into()),
            role,
            order,
            hidden: false,
            folder,
        }
    }

    fn item(id: &str, group: &str, label: &str, app: Option<AppRef>) -> Item {
        Item {
            id: id.into(),
            group_id: group.into(),
            label: label.into(),
            label_overridden: false,
            app,
            shortcut: None,
            uninstalled: false,
            order: 0,
            icon_override: None,
            folder_group_id: None,
        }
    }

    fn app_ref(id: &str) -> AppRef {
        AppRef {
            package: format!("pkg.{id}"),
            activity: ".Main".into(),
        }
    }

    /// Three pages, a folder, enough apps to scroll, search lists and spare
    /// move targets.
    fn sample_app() -> App {
        let (mut app, _) = App::new();
        app.size = Size::new(420.0, 800.0);
        app.insets = Insets {
            left: 10.0,
            right: 10.0,
            top: 50.0,
            bottom: 20.0,
        };

        let mut groups = vec![
            Group {
                id: "root".into(),
                label: "_root_".into(),
                parent_id: None,
                role: Some(GroupRole::Root),
                order: 0,
                hidden: false,
                folder: false,
            },
            group("apps", "Apps", Some(GroupRole::Apps), 0, false),
            group("home", "Home", Some(GroupRole::Home), 1, false),
            group("extra", "Extra", None, 2, false),
            group("hidden-group", "Hidden", None, 3, false),
            group("folder-group", "Games", None, 4, true),
        ];
        for index in 1..=8 {
            groups.push(group(
                &format!("spare-{index}"),
                &format!("Spare {index}"),
                None,
                10 + index,
                false,
            ));
        }

        let mut items = Vec::new();
        let mut refs = Vec::new();
        for index in 0..39 {
            let id = format!("app-{index}");
            let reference = app_ref(&id);
            items.push(item(
                &id,
                "apps",
                &format!("App {index}"),
                Some(reference.clone()),
            ));
            refs.push(reference);
        }
        let mut folder_tile = item("games-folder", "apps", "Games", None);
        folder_tile.folder_group_id = Some("folder-group".into());
        items.push(folder_tile);

        for index in 0..2 {
            let id = format!("home-{index}");
            items.push(item(
                &id,
                "home",
                &format!("Home {index}"),
                Some(app_ref(&id)),
            ));
            let id = format!("extra-{index}");
            items.push(item(
                &id,
                "extra",
                &format!("Extra {index}"),
                Some(app_ref(&id)),
            ));
        }
        for index in 0..3 {
            let id = format!("folder-{index}");
            items.push(item(
                &id,
                "folder-group",
                &format!("Folder {index}"),
                Some(app_ref(&id)),
            ));
        }
        items.push(item(
            "hidden-item",
            "hidden-group",
            "Hidden App",
            Some(app_ref("hidden-item")),
        ));

        app.snapshot = Some(ModelSnapshot {
            root_group_id: "root".into(),
            home_group_id: Some("home".into()),
            page_ids: vec!["apps".into(), "home".into(), "extra".into()],
            groups,
            items,
            settings: Settings::default(),
            warnings: vec![],
            icon_revisions: vec![],
            folder_icons: vec![],
        });
        app.pager = Pager::new(3, 0);
        app.frequent = refs[..12].to_vec();
        app.recent = refs[12..16].to_vec();
        app.results = refs;
        app
    }

    fn node<'a>(frame: &'a Frame, key: &str) -> &'a Node {
        frame
            .nodes
            .iter()
            .find(|node| node.key == key)
            .unwrap_or_else(|| panic!("missing node {key}; have {:?}", keys(frame)))
    }

    fn has(frame: &Frame, key: &str) -> bool {
        frame.nodes.iter().any(|node| node.key == key)
    }

    fn keys(frame: &Frame) -> Vec<&str> {
        frame.nodes.iter().map(|node| node.key.as_str()).collect()
    }

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {expected}, got {actual}"
        );
    }

    fn actions(frame: &Frame, key: &str) -> Vec<Action> {
        node(frame, key).actions.clone()
    }

    fn app_menu(label: &str, step: AppStep) -> Overlay {
        let mut item = item(label, "apps", label, Some(app_ref(label)));
        item.id = label.into();
        Overlay::App(AppMenu {
            item,
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
    fn launcher_frame_has_title_grid_items_and_footer() {
        let mut app = sample_app();
        app.error = Some("boom".into());
        app.snapshot.as_mut().unwrap().warnings = vec!["careful".into()];
        let frame = super::frame(&app).expect("a loaded app has a frame");

        assert_eq!(frame.screen, "launcher");
        assert_eq!(frame.window, [420.0, 800.0]);
        assert_eq!(announcement(&app), "Apps, page 1 of 3");
        assert_eq!(frame.announcement_key, "launcher:apps");

        assert_eq!(node(&frame, "title:prev").label, "Previous page, Extra");
        assert_eq!(node(&frame, "title:current").label, "Apps, search apps");
        assert_eq!(node(&frame, "title:next").label, "Next page, Home");
        assert!(node(&frame, "title:current").selected);
        assert!(!node(&frame, "title:prev").selected);
        assert_eq!(
            node(&frame, "grid:container").label,
            "Apps, page 1 of 3, 40 apps"
        );
        assert_eq!(node(&frame, "item:app-0").label, "App 0");
        assert_eq!(node(&frame, "item:app-0").role, Role::Button);
        assert!(actions(&frame, "item:app-0").contains(&Action::Click));
        assert!(actions(&frame, "item:app-0").contains(&Action::LongClick));
        assert_eq!(node(&frame, "footer:error").label, "boom");
        assert_eq!(node(&frame, "footer:warning:0").label, "careful");
    }

    #[test]
    fn launcher_frame_contains_only_the_current_page() {
        let app = sample_app();
        let frame = super::frame(&app).unwrap();
        assert!(has(&frame, "item:app-0"));
        assert!(!has(&frame, "item:home-0"));
        assert!(!has(&frame, "item:hidden-item"));

        let mut second = sample_app();
        second.pager = Pager::new(3, 1);
        let frame = super::frame(&second).unwrap();
        assert_eq!(announcement(&second), "Home, page 2 of 3");
        assert!(has(&frame, "item:home-0"));
        assert!(!has(&frame, "item:app-0"));
    }

    #[test]
    fn hidden_groups_and_closed_menus_are_absent() {
        let frame = super::frame(&sample_app()).unwrap();
        assert!(!has(&frame, "item:hidden-item"));
        assert!(!keys(&frame).iter().any(|key| key.starts_with("overlay:")));
        assert!(!keys(&frame).iter().any(|key| key.starts_with("search:")));

        let mut app = sample_app();
        app.overlay = Some(app_menu("Calculator", AppStep::Actions));
        let frame = super::frame(&app).unwrap();
        assert!(!has(&frame, "item:app-0"));
        assert!(!has(&frame, "title:current"));
    }

    #[test]
    fn item_bounds_are_row_major_and_clipped_to_the_viewport() {
        let app = sample_app();
        let frame = super::frame(&app).unwrap();
        let grid = node(&frame, "grid:container").bounds;
        let first = node(&frame, "item:app-0").bounds;
        let second = node(&frame, "item:app-1").bounds;
        let next_row = node(&frame, "item:app-4").bounds;

        close(first[0], 10.0);
        close(first[1], grid[1]);
        close(first[2], 100.0);
        close(first[3], qlaunch_iced_row_height());
        close(second[0], 110.0);
        close(second[1], first[1]);
        close(next_row[1], first[1] + qlaunch_iced_row_height());

        let mut scrolled = sample_app();
        scrolled.scroll.insert(Region::Group("apps".into()), 240.0);
        let frame = super::frame(&scrolled).unwrap();
        let grid = node(&frame, "grid:container").bounds;
        assert!(!has(&frame, "item:app-0"));
        assert!(!has(&frame, "item:app-4"));
        let clipped = node(&frame, "item:app-8").bounds;
        close(clipped[1], grid[1]);
        close(clipped[3], 3.0 * qlaunch_iced_row_height() - 240.0);
    }

    fn qlaunch_iced_row_height() -> f32 {
        crate::grid::row_height(&Settings::default())
    }

    #[test]
    fn title_slots_use_wrapped_labels_and_actions() {
        let app = sample_app();
        let frame = super::frame(&app).unwrap();
        assert!(matches!(
            node(&frame, "title:prev").target,
            Target::Step(-1)
        ));
        assert!(matches!(
            node(&frame, "title:current").target,
            Target::Search
        ));
        assert!(matches!(node(&frame, "title:next").target, Target::Step(1)));
        assert_eq!(node(&frame, "title:prev").role, Role::Tab);
        assert_eq!(node(&frame, "title:current").role, Role::Tab);

        let strip = crate::grid::title_strip(
            app.size,
            app.insets,
            &app.snapshot.as_ref().unwrap().settings,
        );
        let current = node(&frame, "title:current").bounds;
        close(current[0], strip.x + strip.width / 3.0);
        close(current[2], strip.width / 3.0);
    }

    #[test]
    fn grid_advertises_scroll_and_page_actions_only_when_available() {
        let mut app = sample_app();
        let frame = super::frame(&app).unwrap();
        let idle = actions(&frame, "grid:container");
        assert!(idle.contains(&Action::LongClick));
        assert!(idle.contains(&Action::ScrollForward));
        assert!(idle.contains(&Action::ScrollLeft));
        assert!(idle.contains(&Action::ScrollRight));
        assert!(!idle.contains(&Action::ScrollBackward));

        app.scroll.insert(Region::Group("apps".into()), 100.0);
        let frame = super::frame(&app).unwrap();
        assert!(actions(&frame, "grid:container").contains(&Action::ScrollBackward));

        app.snapshot.as_mut().unwrap().page_ids.truncate(1);
        app.pager = Pager::new(1, 0);
        let frame = super::frame(&app).unwrap();
        let single = actions(&frame, "grid:container");
        assert!(!single.contains(&Action::ScrollLeft));
        assert!(!single.contains(&Action::ScrollRight));

        let mut search = sample_app();
        search.searching = true;
        search.query = "app".into();
        let frame = super::frame(&search).unwrap();
        let results = actions(&frame, "grid:container");
        assert!(!results.contains(&Action::LongClick));
        assert!(!results.contains(&Action::ScrollLeft));
        assert!(!results.contains(&Action::ScrollRight));
    }

    #[test]
    fn folder_frame_has_back_title_and_folder_items() {
        let mut app = sample_app();
        app.folder = Some("folder-group".into());
        let frame = super::frame(&app).unwrap();
        let settings = &app.snapshot.as_ref().unwrap().settings;

        assert_eq!(frame.screen, "folder:folder-group");
        assert_eq!(announcement(&app), "Games");
        assert_eq!(node(&frame, "folder:back").label, "Back");
        assert!(matches!(node(&frame, "folder:back").target, Target::Back));
        assert_eq!(node(&frame, "folder:title").role, Role::Header);
        assert_eq!(node(&frame, "folder:title").label, "Games");
        assert_eq!(
            node(&frame, "folder:back").bounds,
            bounds_of(crate::grid::folder_back(app.size, app.insets, settings))
        );
        assert_eq!(node(&frame, "grid:container").label, "Games, 3 apps");
        assert!(has(&frame, "item:folder-0"));
        assert!(!has(&frame, "title:current"));
    }

    #[test]
    fn search_frame_has_field_tabs_results_and_tracked_offset() {
        let mut app = sample_app();
        app.size = Size::new(420.0, 450.0);
        app.searching = true;
        app.scroll.insert(Region::Search, 40.0);
        let frame = super::frame(&app).unwrap();

        assert_eq!(frame.screen, "search");
        assert_eq!(frame.announcement_key, "search");
        assert_eq!(announcement(&app), "Search");
        assert_eq!(node(&frame, "search:field").role, Role::TextField);
        assert_eq!(node(&frame, "search:field").value.as_deref(), Some(""));
        assert!(actions(&frame, "search:field").contains(&Action::SetText));
        assert_eq!(node(&frame, "search:close").label, "Close search");
        assert!(matches!(
            node(&frame, "search:close").target,
            Target::Search
        ));
        assert!(node(&frame, "search:frequent").selected);
        assert!(!node(&frame, "search:recent").selected);
        assert!(matches!(
            node(&frame, "search:recent").target,
            Target::SearchTab(true)
        ));
        assert_eq!(node(&frame, "grid:container").label, "Frequent, 12 apps");
        assert!(has(&frame, "item:app-0"));
        assert!(!has(&frame, "item:extra-0"));

        let first = node(&frame, "item:app-0").bounds;
        let grid = node(&frame, "grid:container").bounds;
        close(first[1], grid[1]);
        close(first[3], qlaunch_iced_row_height() - 40.0);
        assert!(actions(&frame, "grid:container").contains(&Action::ScrollBackward));
        assert!(actions(&frame, "grid:container").contains(&Action::ScrollForward));
    }

    #[test]
    fn search_results_with_a_query_show_their_offset() {
        let mut app = sample_app();
        app.searching = true;
        app.query = "app".into();
        app.scroll.insert(Region::Search, 240.0);
        let frame = super::frame(&app).unwrap();
        assert!(!has(&frame, "search:frequent"));
        assert!(!has(&frame, "search:recent"));
        assert_eq!(node(&frame, "search:field").value.as_deref(), Some("app"));
        assert_eq!(node(&frame, "grid:container").label, "39 results");
        assert!(!has(&frame, "item:app-0"));
        let clipped = node(&frame, "item:app-8").bounds;
        close(clipped[1], node(&frame, "grid:container").bounds[1]);
    }

    #[test]
    fn overlay_frame_omits_launcher_nodes_and_follows_card_order() {
        let mut app = sample_app();
        app.overlay = Some(app_menu("Calculator", AppStep::Actions));
        let frame = super::frame(&app).unwrap();

        assert_eq!(frame.screen, "overlay:app:actions");
        assert_eq!(announcement(&app), "Calculator");
        assert_eq!(node(&frame, "overlay:title").label, "Calculator");
        assert_eq!(node(&frame, "overlay:title").role, Role::Header);
        let labels: Vec<&str> = frame
            .nodes
            .iter()
            .filter(|node| node.key.starts_with("overlay:action:"))
            .map(|node| node.label.as_str())
            .collect();
        assert_eq!(labels, ["Rename", "Move", "Uninstall", "Remove", "Cancel"]);
        assert!(matches!(
            node(&frame, "overlay:action:rename").target,
            Target::Overlay(overlay::Message::AppRenameStart)
        ));
        assert!(!has(&frame, "title:current"));
        assert!(!has(&frame, "item:app-0"));
    }

    #[test]
    fn overlay_field_has_value_click_and_set_text() {
        let mut app = sample_app();
        app.overlay = Some(app_menu("Calculator", AppStep::Rename("Clock".into())));
        let frame = super::frame(&app).unwrap();
        let field = node(&frame, "overlay:field:app-name");
        assert_eq!(field.role, Role::TextField);
        assert_eq!(field.value.as_deref(), Some("Clock"));
        assert_eq!(field.actions, [Action::Click, Action::SetText]);
        assert!(matches!(
            field.target,
            Target::TextField(TextField::AppName)
        ));

        let mut app = sample_app();
        app.overlay = Some(launcher_menu(LauncherStep::FolderName("Fun".into())));
        let frame = super::frame(&app).unwrap();
        let field = node(&frame, "overlay:field:folder-name");
        assert_eq!(field.value.as_deref(), Some("Fun"));
        assert!(matches!(
            field.target,
            Target::TextField(TextField::FolderName)
        ));
    }

    #[test]
    fn overlay_list_is_one_scroll_node_plus_visible_rows() {
        let mut app = sample_app();
        app.overlay = Some(app_menu("Calculator", AppStep::Move));
        let frame = super::frame(&app).unwrap();
        let list = node(&frame, "overlay:list");
        assert_eq!(list.role, Role::Scroll);
        assert!(matches!(
            list.target,
            Target::Scroll {
                region: Region::OverlayList,
                ..
            }
        ));
        assert!(list.actions.contains(&Action::ScrollForward));
        assert!(list.actions.contains(&Action::Dismiss));
        assert!(!list.actions.contains(&Action::ScrollBackward));
        assert!(has(&frame, "overlay:item:home"));
        assert!(has(&frame, "overlay:item:spare-4"));
        assert!(!has(&frame, "overlay:item:apps"));
        assert!(!has(&frame, "overlay:item:spare-8"));

        let row = node(&frame, "overlay:item:home").bounds;
        close(row[0], list.bounds[0]);
        close(row[1], list.bounds[1]);
        close(row[2], list.bounds[2]);
        close(row[3], overlay::ACTION_HEIGHT);

        app.scroll.insert(Region::OverlayList, 200.0);
        let frame = super::frame(&app).unwrap();
        assert!(!has(&frame, "overlay:item:home"));
        assert!(actions(&frame, "overlay:list").contains(&Action::ScrollBackward));
        assert!(has(&frame, "overlay:item:spare-8"));
        let list_top = node(&frame, "overlay:list").bounds[1];
        for row in &frame.nodes {
            if row.key.starts_with("overlay:item:") {
                assert!(row.bounds[1] >= list_top);
            }
        }
    }

    #[test]
    fn node_ids_survive_republish_and_are_never_reused() {
        let mut app = sample_app();
        let mut ids = NodeIds::default();
        let mut first = super::frame(&app).unwrap();
        ids.assign(&mut first);
        let app_zero = node(&first, "item:app-0").id;
        let grid = node(&first, "grid:container").id;
        assert!(app_zero > 0);
        assert_eq!(
            first
                .nodes
                .iter()
                .map(|node| node.id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            first.nodes.len()
        );

        let mut again = super::frame(&app).unwrap();
        ids.assign(&mut again);
        assert_eq!(node(&again, "item:app-0").id, app_zero);
        assert_eq!(node(&again, "grid:container").id, grid);

        let new = item("new-app", "apps", "New App", Some(app_ref("new-app")));
        app.snapshot.as_mut().unwrap().items.insert(0, new);
        let mut third = super::frame(&app).unwrap();
        ids.assign(&mut third);
        let new_id = node(&third, "item:new-app").id;
        assert!(new_id > app_zero);
        assert!(!first.nodes.iter().any(|node| node.id == new_id));
    }

    #[test]
    fn bounds_never_escape_the_window() {
        let mut scenarios: Vec<App> = Vec::new();

        let mut launcher = sample_app();
        launcher.scroll.insert(Region::Group("apps".into()), 240.0);
        launcher.error = Some("boom".into());
        launcher.snapshot.as_mut().unwrap().warnings = vec!["careful".into()];
        scenarios.push(launcher);

        let mut folder = sample_app();
        folder.folder = Some("folder-group".into());
        scenarios.push(folder);

        let mut search = sample_app();
        search.searching = true;
        search.query = "app".into();
        search.scroll.insert(Region::Search, 200.0);
        scenarios.push(search);

        let mut menu_actions = sample_app();
        menu_actions.overlay = Some(app_menu("Calculator", AppStep::Actions));
        scenarios.push(menu_actions);

        let mut rename = sample_app();
        rename.overlay = Some(app_menu("Calculator", AppStep::Rename("Clock".into())));
        scenarios.push(rename);

        let mut moving = sample_app();
        moving.overlay = Some(app_menu("Calculator", AppStep::Move));
        moving.scroll.insert(Region::OverlayList, 200.0);
        scenarios.push(moving);

        let mut shortcuts = sample_app();
        let mut menu = LauncherMenu {
            step: LauncherStep::Shortcuts,
            providers: vec![],
            providers_loading: false,
            error: Some("no providers".into()),
        };
        for index in 0..12 {
            menu.providers
                .push(qlaunch_core::api::shortcuts::ShortcutProvider {
                    id: format!("provider-{index}"),
                    label: format!("Provider {index}"),
                });
        }
        shortcuts.overlay = Some(Overlay::Launcher(menu));
        scenarios.push(shortcuts);

        for app in scenarios {
            let mut ids = NodeIds::default();
            let mut frame = super::frame(&app).unwrap();
            ids.assign(&mut frame);
            for node in &frame.nodes {
                let [x, y, width, height] = node.bounds;
                assert!(width > 0.0 && height > 0.0, "{} has no area", node.key);
                assert!(x >= 0.0, "{} starts left of zero", node.key);
                assert!(y >= 0.0, "{} starts above zero", node.key);
                assert!(x + width <= frame.window[0] + 0.01, "{} too wide", node.key);
                assert!(
                    y + height <= frame.window[1] + 0.01,
                    "{} too tall",
                    node.key
                );
            }
        }
    }

    fn bounds_of(rect: iced::Rectangle) -> [f32; 4] {
        [rect.x, rect.y, rect.width, rect.height]
    }

    #[test]
    fn identical_frames_are_not_published_twice() {
        let app = sample_app();
        let mut ids = NodeIds::default();
        let mut publisher = Publisher::default();
        let mut frame = super::frame(&app).unwrap();
        ids.assign(&mut frame);
        let first = publisher.prepare(&frame, None).expect("first publish");
        publisher.commit(&first, &frame);
        let mut again = super::frame(&app).unwrap();
        ids.assign(&mut again);
        assert!(publisher.prepare(&again, None).is_none());
    }

    #[test]
    fn revision_increments_only_on_publish() {
        let mut state = State::default();
        let mut app = sample_app();
        let frame = super::frame(&app).unwrap();
        assert_eq!(state.publish(frame, None).unwrap(), 1);

        let unchanged = super::frame(&app).unwrap();
        assert_eq!(state.publish(unchanged, None).unwrap(), 1);

        app.scroll.insert(Region::Group("apps".into()), 100.0);
        let changed = super::frame(&app).unwrap();
        assert_eq!(state.publish(changed, None).unwrap(), 2);
    }

    #[test]
    fn changed_bounds_publish_a_new_revision() {
        let mut state = State::default();
        let app = sample_app();
        let first = super::frame(&app).unwrap();
        assert_eq!(state.publish(first, None).unwrap(), 1);

        let mut scrolled = sample_app();
        scrolled.scroll.insert(Region::Group("apps".into()), 100.0);
        let second = super::frame(&scrolled).unwrap();
        let payload = state.publisher.prepare(&second, None).unwrap();
        assert_eq!(payload.revision, 2);
        assert_ne!(
            payload.nodes[4].bounds,
            super::frame(&app).unwrap().nodes[4].bounds
        );
    }

    #[test]
    fn screen_change_sets_the_announcement_once() {
        let app = sample_app();
        let mut publisher = Publisher::default();
        let first = super::frame(&app).unwrap();
        let payload = publisher
            .prepare(&first, Some("Apps, page 1 of 3".into()))
            .unwrap();
        assert_eq!(payload.announcement.as_deref(), Some("Apps, page 1 of 3"));
        publisher.commit(&payload, &first);

        let same = super::frame(&app).unwrap();
        assert!(publisher.prepare(&same, Some("ignored".into())).is_none());

        let mut search = sample_app();
        search.searching = true;
        let search_frame = super::frame(&search).unwrap();
        let payload = publisher
            .prepare(&search_frame, Some("Search".into()))
            .unwrap();
        assert_eq!(payload.announcement.as_deref(), Some("Search"));
        assert_eq!(payload.revision, 2);
        publisher.commit(&payload, &search_frame);

        search.error = Some("boom".into());
        let changed = super::frame(&search).unwrap();
        let payload = publisher.prepare(&changed, Some("Search".into())).unwrap();
        assert!(payload.announcement.is_none());
        assert_eq!(payload.revision, 3);
    }

    #[test]
    fn launcher_page_change_announces_the_new_page() {
        let app = sample_app();
        let mut publisher = Publisher::default();
        let first = super::frame(&app).unwrap();
        let payload = publisher
            .prepare(&first, Some("Apps, page 1 of 3".into()))
            .unwrap();
        publisher.commit(&payload, &first);

        let mut second = sample_app();
        second.pager = Pager::new(3, 1);
        let frame = super::frame(&second).unwrap();
        assert_eq!(frame.announcement_key, "launcher:home");
        let payload = publisher
            .prepare(&frame, Some("Home, page 2 of 3".into()))
            .unwrap();
        assert_eq!(payload.announcement.as_deref(), Some("Home, page 2 of 3"));
        assert_eq!(payload.revision, 2);
    }

    #[test]
    fn stale_scroll_generation_does_not_settle() {
        let mut state = State::default();
        let first = state.scroll_started();
        let second = state.scroll_started();
        assert_ne!(first, second);
        assert!(state.pending_scroll);
        assert!(!state.scroll_settled(first));
        assert!(state.pending_scroll);
        assert!(state.scroll_settled(second));
        assert!(!state.pending_scroll);

        let third = state.scroll_started();
        state.cancel_scroll();
        assert!(!state.scroll_settled(third));
        assert!(!state.pending_scroll);
    }

    #[test]
    fn payload_serializes_logical_bounds_and_actions() {
        let app = sample_app();
        let mut frame = super::frame(&app).unwrap();
        NodeIds::default().assign(&mut frame);
        let payload = Publisher::default()
            .prepare(&frame, Some("Apps, page 1 of 3".into()))
            .unwrap();
        let json = serialize(&payload);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["revision"], 1);
        assert_eq!(value["screen"], "launcher");
        assert_eq!(value["window"]["width"], 420.0);
        assert_eq!(value["window"]["height"], 800.0);
        assert_eq!(value["announcement"], "Apps, page 1 of 3");
        let node = &value["nodes"][0];
        assert!(node.get("key").is_none());
        assert!(node.get("target").is_none());
        assert_eq!(node["bounds"].as_array().unwrap().len(), 4);
        assert_eq!(node["actions"][0], "click");
        assert_eq!(node["role"], "tab");
    }
}
