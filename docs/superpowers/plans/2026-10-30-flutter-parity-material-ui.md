# Flutter-parity Material UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring every Iced dialog and the Settings, Advanced, Groups and Choose Apps screens to Flutter visual and behavioural parity through one shared Material 3 theme, without changing core model rules or adding dependencies.

**Architecture:** New `theme.rs` (tokens, typography, widget styles), `icons.rs` (bundled Material Icons font and glyphs) and `components.rs` (composed controls) feed a new `dialog.rs` that replaces the list-based `overlay.rs`, plus a `screens/` module for the four pages. `App` gains a `Vec<Screen>` stack, one `Option<Dialog>`, one open `Select` dropdown and one snackbar; `view.rs` renders them as stable top-level layers and `accessibility.rs` publishes a frame for the top-most layer only.

**Tech Stack:** Rust, Iced `3d8e9cb` (widget stack/pin/mouse_area, text_input, slider, toggler, checkbox, scrollable), `qlaunch_core` API, bundled `MaterialIcons-Regular.otf`.

**Spec:** `docs/superpowers/specs/2026-10-30-flutter-parity-material-ui-design.md`

## Global Constraints

- Work is on `dev/iced`. The user performs all Git operations; do not run `git commit`, `git push`, or create worktrees. Each task ends with a checkpoint report instead of a commit.
- No new crate dependencies. No `iced_material`. No Flutter/Dart dependency in the Rust build.
- `qlaunch_iced` builds with `--no-default-features`.
- Core calls run off Iced's UI thread through the existing `work` helper / `Task::perform`.
- Format only `qlaunch_iced` (`cargo fmt -p qlaunch_iced`); never `cargo fmt --all`.
- Strict Clippy: `cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings`.
- Every model or settings change goes through `qlaunch_core::api::api`; never reimplement a rule.
- Screen and dialog text uses `theme::Text::from_settings`, scaling by `settings.label_size / 16`.
- Icon glyphs come from Flutter's `lib/src/material/icons.dart`; the font is `Material Icons`.
- The icon asset and its licence travel with the crate: `rust/qlaunch_iced/assets/MaterialIcons-Regular.otf` and `MaterialIcons_LICENSE.txt`.

## Review Focus

The inputs and failure modes the spec implies but no single task's happy-path test fully covers; each is pinned by the named task's tests.

1. **Extreme text scale.** `label_size` 0 or 100 must not overlap rows or clip labels; row heights derive from the scaled line height. (Task 10.)
2. **Repeated activation while busy.** Double-tapping Save / a role chip / a checklist row while a request is in flight must not send duplicate mutations or close the dialog twice. (Tasks 7, 8, 12, 13.)
3. **Back/Home with layered state.** Back and Home while a dropdown, dialog and screen are stacked must close exactly one correct layer and leave no stale selection or busy flag. (Tasks 6, 9.)
4. **Many or long options.** A group with many folders, long labels, or a select near a window edge must scroll, ellipsize and clamp inside the window. (Tasks 5, 6, 7.)
5. **Core errors and platform gaps.** Android-only wallpaper/uninstall on Linux, an empty backup list, and a failed persist must surface in the dialog or a snackbar, never leave the UI stuck or silent. (Tasks 7, 11.)

---

### Task 1: Theme tokens and typography

**Files:**
- Create: `rust/qlaunch_iced/src/theme.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod theme;`)

**Interfaces:**
- Consumes: `qlaunch_core::settings::{Settings, Typeface}`.
- Produces:
  - `pub(crate) mod color` consts: `SURFACE`, `ON_SURFACE`, `SURFACE_CONTAINER_LOW`, `SURFACE_CONTAINER`, `SURFACE_CONTAINER_HIGH`, `SURFACE_CONTAINER_HIGHEST`, `ON_SURFACE_VARIANT`, `OUTLINE`, `OUTLINE_VARIANT`, `PRIMARY`, `ON_PRIMARY`, `PRIMARY_CONTAINER`, `ON_PRIMARY_CONTAINER`, `SECONDARY_CONTAINER`, `ON_SECONDARY_CONTAINER`, `ERROR`, `ON_ERROR`, `INVERSE_SURFACE`, `ON_INVERSE_SURFACE`, `SCRIM`.
  - `pub(crate) mod metrics` consts: `APP_BAR_HEIGHT: f32 = 64.0`, `LIST_ONE_LINE: f32 = 56.0`, `LIST_TWO_LINE: f32 = 72.0`, `FIELD_HEIGHT: f32 = 56.0`, `ACTION_HEIGHT: f32 = 40.0`, `DIALOG_RADIUS: f32 = 28.0`, `DIALOG_SIDE_MARGIN: f32 = 40.0`, `DIALOG_VERTICAL_MARGIN: f32 = 24.0`, `DIALOG_MIN_WIDTH: f32 = 280.0`, `DIALOG_MAX_WIDTH: f32 = 560.0`, `DIALOG_PADDING: f32 = 24.0`, `CONTENT_TOP_GAP: f32 = 16.0`, `ACTION_GAP: f32 = 8.0`, `SNACKBAR_HEIGHT: f32 = 48.0`, `ICON_SIZE: f32 = 24.0`, `TOUCH_TARGET: f32 = 48.0`, `SLIDER_HEIGHT: f32 = 48.0`, `CHIP_HEIGHT: f32 = 32.0`, `SWITCH_WIDTH: f32 = 52.0`, `SWITCH_HEIGHT: f32 = 32.0`, `CHECKBOX_SIZE: f32 = 18.0`, `SELECT_OPTION_HEIGHT: f32 = 48.0`, `SELECT_MENU_MAX_HEIGHT: f32 = 280.0`.
  - `pub(crate) enum TextRole { HeadlineSmall, TitleLarge, TitleMedium, TitleSmall, BodyLarge, BodyMedium, BodySmall, LabelLarge, LabelMedium }`.
  - `pub(crate) struct Typography { pub size: f32, pub line_height: f32, pub font: Font }`.
  - `pub(crate) struct Text { pub scale: f32, pub font: Font }` with `impl Text { pub(crate) fn from_settings(settings: &Settings) -> Self; pub(crate) fn role(&self, role: TextRole) -> Typography }`.
  - `pub(crate) fn base_font(settings: &Settings) -> Font`.

- [ ] **Step 1: Write the failing tests** in `theme.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use qlaunch_core::settings::{Settings, Typeface};

    #[test]
    fn colors_match_flutter_dark_scheme() {
        assert_eq!(color::SURFACE, Color::from_rgb8(0x14, 0x12, 0x18));
        assert_eq!(color::SURFACE_CONTAINER_HIGH, Color::from_rgb8(0x2b, 0x29, 0x30));
        assert_eq!(color::PRIMARY, Color::from_rgb8(0xd0, 0xbc, 0xff));
        assert_eq!(color::SECONDARY_CONTAINER, Color::from_rgb8(0x4a, 0x44, 0x58));
        assert_eq!(color::ERROR, Color::from_rgb8(0xf2, 0xb8, 0xb5));
    }

    #[test]
    fn scale_factor_follows_label_size() {
        let mut settings = Settings::default();
        settings.label_size = 8;
        assert_eq!(Text::from_settings(&settings).scale, 0.5);
        settings.label_size = 32;
        assert_eq!(Text::from_settings(&settings).scale, 2.0);
    }

    #[test]
    fn typeface_maps_to_family() {
        let mut settings = Settings::default();
        settings.label_typeface = Typeface::Serif;
        assert_eq!(base_font(&settings).family, Font::Family::Serif);
        settings.label_typeface = Typeface::Monospace;
        assert_eq!(base_font(&settings).family, Font::Family::Monospace);
        settings.label_typeface = Typeface::Normal;
        assert_eq!(base_font(&settings).family, Font::Family::SansSerif);
    }

    #[test]
    fn roles_scale_and_weight_like_material() {
        let text = Text::from_settings(&Settings::default());
        let title = text.role(TextRole::TitleMedium);
        assert_eq!(title.size, 16.0);
        assert_eq!(title.line_height, 24.0);
        assert_eq!(title.font.weight, Weight::Medium);
        let headline = text.role(TextRole::HeadlineSmall);
        assert_eq!(headline.size, 24.0);
        assert_eq!(headline.font.weight, Weight::Normal);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib theme::`
Expected: FAIL / compile error: `theme` module not found.

- [ ] **Step 3: Implement `theme.rs`**

Define the colour consts with `Color::from_rgb8`, the metric consts, `TextRole`, `Typography`, `Text`, and `base_font` mapping `Typeface::Normal → Font::Family::SansSerif`, `Sans → Font::Family::SansSerif`, `Serif → Font::Family::Serif`, `Monospace → Font::Family::Monospace`. `Text::role` returns M3 sizes/line heights scaled by `self.scale`, with weight `Medium` for `TitleMedium`, `TitleSmall`, `LabelLarge`, `LabelMedium` and `Normal` for the rest, and `font` = `Font { weight, ..self.font }`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib theme::`
Expected: PASS (4 tests).

- [ ] **Step 5: Checkpoint** — report the task complete; do not commit.

---

### Task 2: Theme widget styles and metrics

**Files:**
- Modify: `rust/qlaunch_iced/src/theme.rs`

**Interfaces:**
- Consumes: `theme::color`, `theme::metrics`.
- Produces:
  - `pub(crate) fn card(_: &iced::Theme) -> container::Style` (dialog and folder popup: `SURFACE_CONTAINER_HIGH`, radius 28, shadow `(0, 6) blur 12 black 30%`).
  - `pub(crate) fn menu(_: &iced::Theme) -> container::Style` (`SURFACE_CONTAINER`, radius 4, same shadow).
  - `pub(crate) fn scrim(_: &iced::Theme) -> container::Style` (`SCRIM`).
  - `pub(crate) fn text_button(_: &Theme, Status) -> button::Style`, `filled_button`, `destructive_button`, `icon_button` (all 20 radius for the first three; icon button circular).
  - `pub(crate) fn input(_: &Theme, text_input::Status) -> text_input::Style`.
  - `pub(crate) fn vertical_scrollbar(visible: bool) -> scrollable::Direction`, `pub(crate) fn scrollable(...) -> scrollable::Style`, `pub(crate) const SCROLLBAR_WIDTH: f32 = 6.0`.
  - `pub(crate) fn toggler(_: &Theme, toggler::Status) -> toggler::Style`.
  - `pub(crate) fn slider(_: &Theme, slider::Status) -> slider::Style`.
  - `pub(crate) fn checkbox(_: &Theme, checkbox::Status) -> checkbox::Style`.

- [ ] **Step 1: Write the failing tests** appended to `theme.rs::tests`:

```rust
#[test]
fn text_button_has_primary_text_and_press_layer() {
    let active = text_button(&iced::Theme::Dark, button::Status::Active);
    let pressed = text_button(&iced::Theme::Dark, button::Status::Pressed);
    assert_eq!(active.text_color, color::PRIMARY);
    assert_ne!(active.background, pressed.background);
    assert_eq!(active.border.radius.top_left, 20.0);
}

#[test]
fn filled_and_destructive_colors() {
    let filled = filled_button(&iced::Theme::Dark, button::Status::Active);
    assert_eq!(filled.background, Some(color::PRIMARY.into()));
    assert_eq!(filled.text_color, color::ON_PRIMARY);
    let destructive = destructive_button(&iced::Theme::Dark, button::Status::Active);
    assert_eq!(destructive.text_color, color::ERROR);
}

#[test]
fn toggler_on_and_off_tracks() {
    use iced::widget::toggler::Status;
    let on = toggler(&iced::Theme::Dark, Status::Active { is_toggled: true });
    let off = toggler(&iced::Theme::Dark, Status::Active { is_toggled: false });
    assert_eq!(on.background, color::PRIMARY.into());
    assert_eq!(off.background, color::SURFACE_CONTAINER_HIGHEST.into());
}

#[test]
fn slider_uses_primary_active_rail() {
    let style = slider(&iced::Theme::Dark, iced::widget::slider::Status::Active);
    assert_eq!(style.rail.backgrounds.0, color::SURFACE_CONTAINER_HIGHEST.into());
    assert_eq!(style.rail.backgrounds.1, color::PRIMARY.into());
    assert_eq!(style.rail.width, 4.0);
}

#[test]
fn scrollbar_is_hidden_until_scrolling() {
    use iced::widget::scrollable::{Direction, Scrollbar};
    assert_eq!(vertical_scrollbar(false), Direction::Vertical(Scrollbar::hidden()));
    assert_eq!(
        vertical_scrollbar(true),
        Direction::Vertical(
            Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH).margin(2.0)
        )
    );
}

#[test]
fn input_underline_focus_colors() {
    use iced::widget::text_input::Status;
    let idle = input(&iced::Theme::Dark, Status::Active);
    let focused = input(&iced::Theme::Dark, Status::Focused { is_hovered: false });
    assert_eq!(idle.border.width, 1.0);
    assert_eq!(focused.border.width, 2.0);
    assert_eq!(focused.border.color, color::PRIMARY);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib theme::`
Expected: FAIL (missing style functions).

- [ ] **Step 3: Implement the style functions**

Move the existing working styles out of `material.rs` (`card`, buttons, `input`, `scrollable`, `vertical_scrollbar`, `SCROLLBAR_WIDTH`) and add the missing ones. Keep `material.rs` untouched for now; later tasks switch call sites. `slider` uses `with_circular_handle(10.0)`; `checkbox` uses an 18 dp box, 2 dp outline, primary fill and `ON_PRIMARY` icon when selected; `toggler` uses `SWITCH_WIDTH`/`SWITCH_HEIGHT` with radius = `SWITCH_HEIGHT / 2`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib theme::`
Expected: PASS (10 tests).

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 3: Material Icons font and glyphs

**Files:**
- Create: `rust/qlaunch_iced/src/icons.rs`
- Create: `rust/qlaunch_iced/assets/MaterialIcons-Regular.otf` (copy)
- Create: `rust/qlaunch_iced/assets/MaterialIcons_LICENSE.txt` (copy)
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod icons;` and load the font in the application builder)

**Interfaces:**
- Produces:
  - `pub(crate) const FONT: Font = Font::with_name("Material Icons");`
  - `pub(crate) const SETTINGS: char = '\u{e57f}'; ADD_LINK = '\u{e053}'; CREATE_NEW_FOLDER = '\u{ef8c}'; DASHBOARD_CUSTOMIZE = '\u{e1b2}'; WALLPAPER = '\u{e6ca}'; LOCK_OUTLINE = '\u{e3b1}'; NOTIFICATIONS_NONE = '\u{e451}'; LINK = '\u{e380}'; CHECKLIST = '\u{e15b}'; ARROW_UPWARD = '\u{e0a0}'; ARROW_DOWNWARD = '\u{e097}'; ARROW_BACK = '\u{e092}'; FOLDER = '\u{e2a3}'; KEYBOARD_ARROW_DOWN = '\u{e353}'; ADD = '\u{e047}'; CHECK = '\u{e156}'; CLOSE = '\u{e16a}'; ERROR_OUTLINE = '\u{e238}';`
  - `pub(crate) fn glyph(character: char) -> String` returning `character.to_string()`.

- [ ] **Step 1: Copy the assets**

Run:
```bash
mkdir -p rust/qlaunch_iced/assets
cp /home/bronson/Projects/flutter/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf rust/qlaunch_iced/assets/
cp /home/bronson/Projects/flutter/bin/cache/artifacts/material_fonts/MaterialIcons_LICENSE.txt rust/qlaunch_iced/assets/
```

- [ ] **Step 2: Write the failing tests** in `icons.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_codepoints_match_flutter() {
        assert_eq!(SETTINGS, '\u{e57f}');
        assert_eq!(ADD_LINK, '\u{e053}');
        assert_eq!(CREATE_NEW_FOLDER, '\u{ef8c}');
        assert_eq!(DASHBOARD_CUSTOMIZE, '\u{e1b2}');
        assert_eq!(WALLPAPER, '\u{e6ca}');
        assert_eq!(CHECKLIST, '\u{e15b}');
        assert_eq!(ARROW_BACK, '\u{e092}');
        assert_eq!(KEYBOARD_ARROW_DOWN, '\u{e353}');
    }

    #[test]
    fn font_family_is_material_icons() {
        assert_eq!(FONT.family, iced::font::Family::Name("Material Icons"));
    }
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib icons::`
Expected: FAIL (module not found).

- [ ] **Step 4: Implement `icons.rs` and load the font**

Add the constants and `glyph`. In `lib.rs`'s application construction (the block that calls `.default_font(platform::DEFAULT_FONT)` and `.font(font)`), add:

```rust
.font(include_bytes!("../assets/MaterialIcons-Regular.otf").as_slice())
```

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib icons::`
Expected: PASS (2 tests).

- [ ] **Step 6: Checkpoint** — report complete.

---

### Task 4: Core components (bar, tiles, buttons, checkbox)

**Files:**
- Create: `rust/qlaunch_iced/src/components.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod components;`)

**Interfaces:**
- Consumes: `theme`, `icons`, `crate::Message`.
- Produces:
  - `pub(crate) fn list_tile_height(has_subtitle: bool) -> f32` (56 or 72).
  - `pub(crate) fn app_bar_height() -> f32` (64).
  - `pub(crate) fn app_bar<'a>(text: &theme::Text, title: &str, on_back: Option<Message>, actions: Vec<Element<'a, Message>>) -> Element<'a, Message>`.
  - `pub(crate) fn section_header<'a>(text: &theme::Text, label: &str) -> Element<'a, Message>`.
  - `pub(crate) fn list_tile<'a>(text: &theme::Text, title: &str, subtitle: Option<&str>, leading: Option<char>, trailing: Option<Element<'a, Message>>, message: Option<Message>, enabled: bool) -> Element<'a, Message>`.
  - `pub(crate) fn action_tile<'a>(text: &theme::Text, title: &str, subtitle: &str, message: Message) -> Element<'a, Message>`.
  - `pub(crate) fn text_button<'a>(text: &theme::Text, label: &str, message: Message, enabled: bool) -> Element<'a, Message>`; `filled_button`, `destructive_button` with the same shape.
  - `pub(crate) fn icon_button<'a>(glyph: char, label: &str, message: Message, enabled: bool) -> Element<'a, Message>`.
  - `pub(crate) fn checkbox_row<'a>(text: &theme::Text, label: &str, checked: bool, message: Message, enabled: bool) -> Element<'a, Message>`.

- [ ] **Step 1: Write the failing tests** in `components.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_tile_height_by_subtitle() {
        assert_eq!(list_tile_height(false), 56.0);
        assert_eq!(list_tile_height(true), 72.0);
    }

    #[test]
    fn app_bar_height_is_64() {
        assert_eq!(app_bar_height(), 64.0);
    }

    #[test]
    fn buttons_are_40_high() {
        assert_eq!(button_height(), 40.0);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib components::`
Expected: FAIL (module not found).

- [ ] **Step 3: Implement the components**

`list_tile` is a `button` with `.style` transparent plus an 8% state layer, containing a row: optional 24 dp icon, a `Fill` column (title = body-large, subtitle = body-medium in `ON_SURFACE_VARIANT`), optional trailing. Height from `list_tile_height`. Disabled rows use `.on_press_maybe(None)` and 38% text colour. `app_bar` is a 64 dp row with an optional 48 dp back icon button, title (title-large) and actions. Buttons use `theme::text_button`/`filled_button`/`destructive_button`, height `metrics::ACTION_HEIGHT`. Add `pub(crate) fn button_height() -> f32` returning `metrics::ACTION_HEIGHT`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib components::`
Expected: PASS (3 tests).

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 5: Form components (switch, slider, chips, field, select)

**Files:**
- Modify: `rust/qlaunch_iced/src/components.rs`

**Interfaces:**
- Consumes: `theme`, `icons`, `crate::Message`.
- Produces:
  - `pub(crate) fn switch_row<'a>(text: &theme::Text, label: &str, value: bool, message: Message, enabled: bool) -> Element<'a, Message>`.
  - `pub(crate) fn slider_row<'a>(text: &theme::Text, label: &str, value: f32, unit: &str, range: RangeInclusive<f32>, on_change: Message, on_release: Message) -> Element<'a, Message>`.
  - `pub(crate) fn choice_row<'a>(text: &theme::Text, label: &str, options: &[(String, bool)], choose: fn(usize) -> Message) -> Element<'a, Message>` (each tuple is `(label, selected)`).
  - `pub(crate) fn text_field<'a>(text: &theme::Text, label: &str, value: &str, placeholder: &str, widget_id: &'static str, on_input: fn(String) -> Message, on_submit: Message) -> Element<'a, Message>`.
  - `pub(crate) fn select_field<'a>(text: &theme::Text, label: &str, value: &str, widget_id: &'static str, on_open: Message) -> Element<'a, Message>`.
  - `pub(crate) fn select_menu_bounds(anchor: Rectangle, option_count: usize, window: Size) -> Rectangle`.
  - `pub(crate) fn select_menu<'a>(text: &theme::Text, options: &[(String, String, bool)], choose: fn(usize) -> Message) -> Element<'a, Message>` (tuple is `(id, label, is_folder)`).
  - `pub(crate) fn snackbar<'a>(text: &theme::Text, message: &str) -> Element<'a, Message>`.

- [ ] **Step 1: Write the failing tests** in `components.rs::tests`:

```rust
#[test]
fn select_menu_clamps_below_the_field() {
    let window = Size::new(400.0, 300.0);
    let anchor = Rectangle { x: 40.0, y: 240.0, width: 300.0, height: 56.0 };
    let bounds = select_menu_bounds(anchor, 5, window);
    assert!(bounds.y + bounds.height <= window.height);
    assert_eq!(bounds.width, anchor.width);
}

#[test]
fn select_menu_never_exceeds_max_height() {
    let window = Size::new(400.0, 900.0);
    let anchor = Rectangle { x: 40.0, y: 100.0, width: 300.0, height: 56.0 };
    let bounds = select_menu_bounds(anchor, 50, window);
    assert_eq!(bounds.height, theme::metrics::SELECT_MENU_MAX_HEIGHT);
}

#[test]
fn slider_track_is_48_high() {
    assert_eq!(slider_height(), 48.0);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib components::`
Expected: FAIL (missing functions).

- [ ] **Step 3: Implement the components**

`switch_row` uses `toggler(value, |_| message)` with `theme::toggler`. `slider_row` is a label row (body-large + body-small value with the unit) over a `slider(range, value, move |_| on_change).step(1.0).on_release(on_release).style(theme::slider)`. `choice_row` is a wrapping row of chip buttons styled from `theme::text_button` (selected uses `SECONDARY_CONTAINER`/`ON_SECONDARY_CONTAINER`). `select_field` is the 56 dp field with a chevron. `select_menu_bounds` places the menu under the anchor, clamps x/width to the window, flips above when it would not fit below, and caps height at `SELECT_MENU_MAX_HEIGHT`. `select_menu` is a scrollable column of 48 dp rows (selected `SECONDARY_CONTAINER`, folder rows indented with the folder glyph). `snackbar` is the inverse-surface bar. Add `pub(crate) fn slider_height() -> f32`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib components::`
Expected: PASS (6 tests).

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 6: Dialog model and layout

**Files:**
- Create: `rust/qlaunch_iced/src/dialog.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod dialog;`, add `dialog: Option<dialog::Dialog>`, `select: Option<dialog::Select>`)

**Interfaces:**
- Consumes: `theme`, `crate::App`, `qlaunch_core::model::{Group, Id, Item}`, `qlaunch_core::model::roles::GroupRole`.
- Produces:
  - `pub(crate) enum Dialog { App(AppEditor), Folder(FolderEditor), Launcher, AddShortcut(AddShortcutState), CommandShortcut(CommandShortcutState), AddGroup(AddGroupState), GroupEditor(GroupEditorState), Confirm(ConfirmState), RestorePicker(RestorePickerState) }` with the state structs below.
  - `pub(crate) struct AppEditor { item: Item, name: String, group_id: Id, busy: bool, error: Option<String> }`.
  - `pub(crate) enum FolderEditor { Create { containing: Id, name: String, group_id: Id, busy: bool, error: Option<String> }, Edit { folder: Id, label: String, name: String, group_id: Id, busy: bool, error: Option<String> } }`.
  - `pub(crate) struct AddShortcutState { target_group: Id, providers: Option<Result<Vec<ShortcutProvider>, String>>, busy: bool }`.
  - `pub(crate) struct CommandShortcutState { target_group: Id, name: String, command: String, busy: bool }`.
  - `pub(crate) struct AddGroupState { name: String, busy: bool }`.
  - `pub(crate) struct GroupEditorState { group: Id, name: String, role: Option<GroupRole>, busy: bool, error: Option<String> }`.
  - `pub(crate) struct ConfirmState { title: String, body: String, confirm_label: String, destructive: bool, action: ConfirmAction }` and `pub(crate) enum ConfirmAction { DeleteGroup(Id), Restore(String), FactoryReset }`.
  - `pub(crate) struct RestorePickerState { backups: Option<Result<Vec<String>, String>> }`.
  - `pub(crate) struct Select { pub field: SelectField, pub options: Vec<SelectOption>, pub selected: Id }`, `pub(crate) enum SelectField { AppGroup, FolderGroup }`, `pub(crate) struct SelectOption { pub id: Id, pub label: String, pub folder: bool }`.
  - `pub(crate) enum Row { Title(String), Field { label: String, value: String, widget_id: &'static str }, Text { label: String, error: bool }, Select { label: String, value: String }, Chip { label: String, selected: bool, enabled: bool }, Tile { label: String, subtitle: Option<String> }, Note { label: String } }`.
  - `pub(crate) struct DialogLayout { pub card: Rectangle, pub title: Option<Rectangle>, pub content: Rectangle, pub content_height: f32, pub actions: Vec<(Rectangle, String)>, pub rows: Vec<(Rectangle, usize)> }`.
  - `pub(crate) fn layout(app: &App, dialog: &Dialog, window: Size) -> DialogLayout`.
  - `pub(crate) fn rows(app: &App, dialog: &Dialog) -> Vec<Row>`.
  - `pub(crate) fn heading(dialog: &Dialog) -> Option<String>`.
  - `pub(crate) fn app_save_plan(item: &Item, name: &str, group: &Id, snapshot: &ModelSnapshot) -> Vec<Change>` and `folder_save_plan(...)`, with `pub(crate) enum Change { RenameItem(Id, String), MoveItem(Id, Id, i32), RenameGroup(Id, String), AddFolder(Id, String), DeleteFolder(Id) }`.

- [ ] **Step 1: Write the failing tests** in `dialog.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::app; // reuse the existing lib.rs test fixture

    #[test]
    fn card_is_centered_and_width_capped() {
        let app = app();
        let dialog = Dialog::Launcher;
        let layout = layout(&app, &dialog, Size::new(1280.0, 800.0));
        assert_eq!(layout.card.width, theme::metrics::DIALOG_MAX_WIDTH);
        assert!((layout.card.x - (1280.0 - layout.card.width) / 2.0).abs() < 0.5);
    }

    #[test]
    fn narrow_window_uses_side_margins() {
        let app = app();
        let layout = layout(&app, &Dialog::Launcher, Size::new(360.0, 640.0));
        assert_eq!(layout.card.width, 360.0 - 2.0 * theme::metrics::DIALOG_SIDE_MARGIN);
    }

    #[test]
    fn app_save_plan_renames_then_moves() {
        let app = app();
        let snapshot = app.snapshot.as_ref().unwrap();
        let item = &snapshot.items[0];
        let plan = app_save_plan(item, "New name", "other", snapshot);
        assert!(matches!(plan[0], Change::RenameItem(_, ref n) if n == "New name"));
        assert!(matches!(plan[1], Change::MoveItem(_, ref g, _) if g == "other"));
    }

    #[test]
    fn app_save_plan_is_empty_when_nothing_changed() {
        let app = app();
        let snapshot = app.snapshot.as_ref().unwrap();
        let item = &snapshot.items[0];
        assert!(app_save_plan(item, &item.label, &item.group_id, snapshot).is_empty());
    }

    #[test]
    fn content_scrolls_when_taller_than_the_window() {
        let app = app();
        let layout = layout(&app, &Dialog::Launcher, Size::new(360.0, 200.0));
        assert!(layout.content_height > layout.content.height);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib dialog::`
Expected: FAIL (module not found).

- [ ] **Step 3: Implement the model and layout**

`layout` computes the card rect (centred, width `(window.width - 80).clamp(280, 560)`, max height `window.height - 48`), then stacks title (24 top, 24 sides, 32 high + 16 gap), content (24 sides, 24 bottom, scrollable when its natural height exceeds the card) and the action row (40 high, 24 sides/bottom, 8 gap). `rows` returns each variant's rows in draw order with fixed heights: fields 56, text 20, selects 56, chips 32, tiles 56/72, 16 between forms and 8 between list rows. `heading` returns the Flutter title or `None` (app editor). `app_save_plan`/`folder_save_plan` return the ordered `Change`s and use `snapshot.items` plus `folder_group_id` for append order and folder membership; empty when nothing differs. Add test helpers `folder_of(snapshot, group)` and `append_order(snapshot, group)` as needed (they may already exist in `overlay.rs`; move them).

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib dialog::`
Expected: PASS (5 tests).

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 7: Dialog messages and view (app, folder, launcher)

**Files:**
- Modify: `rust/qlaunch_iced/src/dialog.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (replace `overlay` handling with `dialog`, add `Message::Dialog`, `Message::ModelDone`, update long-press paths)
- Delete: `rust/qlaunch_iced/src/overlay.rs` (after call sites move)

**Interfaces:**
- Consumes: Task 6 model, `components`, `theme`.
- Produces:
  - `pub(crate) enum Message { Close, Noop, Back, OpenGroupSelect, GroupChosen(Id), CloseSelect, AppName(String), AppSave, AppUninstall, AppRemove, FolderName(String), FolderSave, FolderDelete, LauncherSettings, LauncherAddShortcut, LauncherCreateFolder, LauncherEditGroups, LauncherWallpaper, Providers(Result<Vec<ShortcutProvider>, String>), ShortcutLock, ShortcutNotifications, ShortcutProvider(String), ShortcutCommand, CommandName(String), CommandText(String), CommandAdd, AddGroupName(String), AddGroupConfirm, GroupName(String), GroupRole(Option<GroupRole>), GroupSave, GroupDelete, GroupChooseApps, RestoreChosen(String), ConfirmYes }`.
  - `pub(crate) fn view<'a>(app: &'a App, dialog: &'a Dialog) -> Element<'a, Message>`.
  - `pub(crate) enum SelectMessage { Chosen(Id), Close }`.
  - `pub(crate) fn select_view<'a>(app: &'a App, select: &'a Select) -> Element<'a, Message>`.
  - `Message::ModelDone { result: Result<ModelSnapshot, String>, close: bool }` (defined here and reused by later tasks; on `Ok` the snapshot is applied and the dialog closes when `close` is true, on `Err` the message goes to the active dialog's `error` or a snackbar).

- [ ] **Step 1: Write the failing tests** in `lib.rs::tests` (replace the overlay tests for these paths):

```rust
#[test]
fn long_press_opens_the_app_editor() {
    let mut app = app();
    let item = app_item(&app);
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
    assert!(matches!(app.dialog, Some(dialog::Dialog::Folder(dialog::FolderEditor::Edit { .. }))));
}

#[test]
fn app_editor_hides_remove_for_plain_apps() {
    let mut app = app();
    let item = app_item(&app);
    app.snapshot.as_mut().unwrap().items.push(item.clone());
    app.dialog = Some(dialog::Dialog::App(dialog::AppEditor::new(item)));
    let rows = dialog::rows(&app, app.dialog.as_ref().unwrap());
    assert!(!rows.iter().any(|row| matches!(row, dialog::Row::Tile { label, .. } if label == "Remove")));
}

#[test]
fn launcher_menu_has_five_rows() {
    let app = app();
    let rows = dialog::rows(&app, &dialog::Dialog::Launcher);
    assert_eq!(rows.iter().filter(|row| matches!(row, dialog::Row::Tile { .. })).count(), 5);
}

#[test]
fn back_steps_the_app_editor_closed() {
    let mut app = app();
    app.dialog = Some(dialog::Dialog::Launcher);
    let _ = app.update(Message::Back);
    assert!(app.dialog.is_none());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib`
Expected: FAIL (constructors and messages missing).

- [ ] **Step 3: Implement the dialog view and update wiring**

Add constructors (`AppEditor::new`, etc.) and `Message` handling in `App::update`:

- `Close`/`Back` clear `dialog` and `select`.
- `OpenGroupSelect`/`GroupChosen`/`CloseSelect` manage `app.select` for the app/folder group field.
- App Save builds `app_save_plan` and runs the `Change`s sequentially in one `Task`, then `Message::ModelDone { result, close: true }`.
- Uninstall/Remove call the core and use `ModelDone`.
- Folder Save/Delete build `folder_save_plan` (or `DeleteFolder`) and use `ModelDone`.
- Launcher rows push screens or open the child dialogs; Wallpaper calls `choose_wallpaper` and shows an error snackbar.
- `view` renders scrim + `components::dialog` card: title (if any), the rows from `dialog::rows`, the error as a `Row::Text`, and the right-aligned action row. Reuse `components` for every element.
- Replace every `overlay::` use in `lib.rs`/`view.rs`/`accessibility.rs` with `dialog::` and delete `overlay.rs`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib`
Expected: PASS, with the old overlay tests updated or removed.

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 8: Dialogs for shortcuts, groups, confirmations and restore

**Files:**
- Modify: `rust/qlaunch_iced/src/dialog.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs`

**Interfaces:**
- Consumes: Task 6/7.
- Produces: full handling for `AddShortcut`, `CommandShortcut`, `AddGroup`, `GroupEditor`, `Confirm`, `RestorePicker`, and the `Message::ModelDone` cases those dialogs emit (the message itself is defined in Task 7).

- [ ] **Step 1: Write the failing tests** in `dialog.rs::tests` and `lib.rs::tests`:

```rust
#[test]
fn group_role_locked_for_one_holder_roles() {
    let mut state = GroupEditorState { group: "g".into(), name: "G".into(), role: Some(GroupRole::Apps), busy: false, error: None };
    assert!(state.role_locked());
    state.role = Some(GroupRole::Home);
    assert!(!state.role_locked());
    state.role = None;
    assert!(!state.role_locked());
}

#[test]
fn add_group_confirm_disabled_until_named() {
    let state = AddGroupState { name: "  ".into(), busy: false };
    assert!(!state.can_confirm());
    let state = AddGroupState { name: " Fun ".into(), busy: false };
    assert!(state.can_confirm());
}

#[test]
fn command_shortcut_ready_needs_both_fields() {
    let mut state = CommandShortcutState { target_group: "apps".into(), name: "X".into(), command: String::new(), busy: false };
    assert!(!state.ready());
    state.command = "firefox".into();
    assert!(state.ready());
}

#[test]
fn group_delete_opens_a_confirmation() {
    let mut app = app();
    app.dialog = Some(dialog::Dialog::GroupEditor(GroupEditorState { group: "apps".into(), name: "Apps".into(), role: Some(GroupRole::Apps), busy: false, error: None }));
    // role is locked, so Delete is present but disabled; a None-role group confirms.
    app.dialog = Some(dialog::Dialog::GroupEditor(GroupEditorState { group: "fun".into(), name: "Fun".into(), role: None, busy: false, error: None }));
    let _ = app.update(Message::Dialog(dialog::Message::GroupDelete));
    assert!(matches!(app.dialog, Some(dialog::Dialog::Confirm(_))));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib`
Expected: FAIL (methods and handlers missing).

- [ ] **Step 3: Implement**

Add the helper methods (`role_locked`, `can_confirm`, `ready`), the rows for each variant, and the update handlers. Role chips and checklist moves send core calls and keep the dialog/screen open (`close: false`); Save closes the dialog; Delete opens `Confirm`; `ConfirmYes` runs the associated `ConfirmAction` and shows the matching snackbar (restore/reset) or closes both dialogs (group delete). `AddShortcut` loads providers through the existing core call; the command provider opens `CommandShortcut`, whose Add runs `add_command_shortcut` and closes both dialogs.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib`
Expected: PASS.

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 9: Screen stack, back order and input gating

**Files:**
- Modify: `rust/qlaunch_iced/src/lib.rs`
- Modify: `rust/qlaunch_iced/src/view.rs`

**Interfaces:**
- Produces:
  - `pub(crate) enum Screen { Settings, Advanced, Groups, GroupApps(Id) }`.
  - `App::screens: Vec<Screen>`.
  - `Message::Push(Screen)`, `Message::PopScreen`, `Message::Snackbar(String)`, `Message::SnackbarExpired(u64)`.
  - `App::snackbar: Option<(u64, String)>`.

- [ ] **Step 1: Write the failing tests** in `lib.rs::tests`:

```rust
#[test]
fn back_order_closes_select_then_dialog_then_screen() {
    let mut app = app();
    app.screens.push(Screen::Settings);
    app.dialog = Some(dialog::Dialog::Launcher);
    app.select = Some(dialog::Select { field: dialog::SelectField::AppGroup, options: vec![], selected: "apps".into() });
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
fn snackbar_expires_only_for_its_own_generation() {
    let mut app = app();
    let _ = app.update(Message::Snackbar("hello".into()));
    let (generation, _) = app.snackbar.clone().unwrap();
    let _ = app.update(Message::SnackbarExpired(generation + 1));
    assert!(app.snackbar.is_some());
    let _ = app.update(Message::SnackbarExpired(generation));
    assert!(app.snackbar.is_none());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib`
Expected: FAIL.

- [ ] **Step 3: Implement**

Add the `Screen` enum, `screens`, `snackbar`, and the back order (select → dialog → screen → overlay → search → folder). Add early returns for `ItemPressed`, `BlankPressed`, `Search`, `FolderDismiss` and pull-down when a screen/dialog is open. Add `Message::Snackbar` (stores a new generation and returns a 4 s `Task::perform(sleep, SnackbarExpired)`) and `SnackbarExpired`. Extend `view.rs` to the stable layer list `[base, folder, overlay-or-empty, screen, snackbar, dialog, select]`, each absent layer `space()`, screens and dialogs wrapped in a full-window `mouse_area` barrier `opaque` layer.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib`
Expected: PASS.

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 10: Settings screen

**Files:**
- Create: `rust/qlaunch_iced/src/screens/mod.rs`
- Create: `rust/qlaunch_iced/src/screens/settings.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (add `mod screens;`, `App::settings_drag: Option<(SettingField, f32)>`, and the settings messages)

**Interfaces:**
- Consumes: `components`, `theme`, `qlaunch_core::settings::{Settings, SettingsPatch}`.
- Produces:
  - `pub(crate) enum SettingField { GridColumnsPortrait, GridColumnsLandscape, Layout, TabVisibility, ScrollFade, ContinuousScroll, PageTransitionMs, WallpaperAlpha, StatusBarVisible, IconsVisible, IconSize, IconAlpha, LabelsVisible, LabelSize, LabelTypeface, LabelColour, LabelShadow, LabelShadowBlur, Orientation }` with `pub(crate) fn all() -> [SettingField; 19]`.
  - `pub(crate) enum Row { Header(&'static str), Switch { field: SettingField, label: &'static str, value: bool }, Slider { field: SettingField, label: &'static str, value: i64, unit: &'static str, min: i64, max: i64 }, Choice { field: SettingField, label: &'static str, options: Vec<(String, bool)> }, Action { label: &'static str, subtitle: &'static str, target: ActionTarget } }` and `pub(crate) enum ActionTarget { Groups, Advanced, Wallpaper }`.
  - `pub(crate) fn rows(settings: &Settings, drag: Option<(SettingField, f32)>) -> Vec<Row>` — exactly one control per setting, in Flutter order.
  - `pub(crate) fn message(field: SettingField, value: i64) -> SettingsPatch` and `pub(crate) fn choice_patch(field: SettingField, index: usize, settings: &Settings) -> SettingsPatch`.
  - `pub(crate) fn view<'a>(app: &'a App) -> Element<'a, Message>`.

- [ ] **Step 1: Write the failing tests** in `screens/settings.rs`:

```rust
#[test]
fn rows_cover_every_setting_once_in_order() {
    let rows = rows(&Settings::default(), None);
    let controls: Vec<SettingField> = rows.iter().filter_map(|row| match row {
        Row::Switch { field, .. } | Row::Slider { field, .. } | Row::Choice { field, .. } => Some(*field),
        _ => None,
    }).collect();
    assert_eq!(controls.len(), 19);
    let mut sorted = controls.clone();
    sorted.sort_by_key(|field| SettingField::all().iter().position(|f| f == field));
    assert_eq!(controls, sorted);
}

#[test]
fn rows_are_grouped_display_window_icon_text_groups_advanced() {
    let rows = rows(&Settings::default(), None);
    let headers: Vec<&str> = rows.iter().filter_map(|row| match row { Row::Header(h) => Some(*h), _ => None }).collect();
    assert_eq!(headers, ["Display", "Window", "Icon", "Text", "Groups", "Advanced"]);
}

#[test]
fn slider_range_matches_core_clamp() {
    let rows = rows(&Settings::default(), None);
    let transition = rows.iter().find_map(|row| match row {
        Row::Slider { field: SettingField::PageTransitionMs, min, max, .. } => Some((*min, *max)),
        _ => None,
    });
    assert_eq!(transition, Some((100, 800)));
}

#[test]
fn drag_value_overrides_the_saved_value() {
    let mut settings = Settings::default();
    settings.page_transition_ms = 300;
    let rows = rows(&settings, Some((SettingField::PageTransitionMs, 480.0)));
    let shown = rows.iter().find_map(|row| match row {
        Row::Slider { field: SettingField::PageTransitionMs, value, .. } => Some(*value),
        _ => None,
    });
    assert_eq!(shown, Some(480));
}

#[test]
fn switch_and_choice_patches_target_the_field() {
    assert_eq!(message(SettingField::IconsVisible, 0), SettingsPatch { icons_visible: Some(false), ..Default::default() });
    assert_eq!(message(SettingField::LabelShadowBlur, 3), SettingsPatch { label_shadow_blur: Some(3), ..Default::default() });
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib screens::settings::`
Expected: FAIL.

- [ ] **Step 3: Implement**

`rows` mirrors `settings_screen.dart`: the Display/Window/Icon/Text controls in order, the wallpaper/ Groups / Advanced action tiles, and the choice labels from the same match arms. `view` renders the app bar, a scrollable column of `components` rows (applying the drag override), and dispatches: switches and chips send `update_settings` immediately (`Message::Dialog`/`Message::Settings`), sliders set `app.settings_drag` on change and send one patch on release. `Message::ModelDone { close: false }` clears the drag on success; an error shows a snackbar. Wire `Message::Push(Screen::Settings)` from the launcher menu and the search gear.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib screens::settings::`
Expected: PASS (5 tests).

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 11: Advanced screen and sidebar snackbars

**Files:**
- Create: `rust/qlaunch_iced/src/screens/advanced.rs`
- Modify: `rust/qlaunch_iced/src/screens/mod.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub(crate) fn rows() -> Vec<Row>` with `pub(crate) enum Row { Restart, Backup, Restore, FactoryReset }`.
  - `pub(crate) fn view<'a>(app: &'a App) -> Element<'a, Message>`.
  - `pub(crate) fn backup_message(result: &Result<String, String>) -> String` and `pub(crate) fn restore_message(result: &Result<ModelSnapshot, String>, name: &str) -> String`.

- [ ] **Step 1: Write the failing tests** in `screens/advanced.rs`:

```rust
#[test]
fn rows_match_flutter_order() {
    assert_eq!(rows().len(), 4);
}

#[test]
fn backup_message_names_the_downloads_file() {
    assert_eq!(backup_message(&Ok("qlaunch-2026.json".into())), "Backup saved to Downloads/qlaunch-2026.json.");
    assert_eq!(backup_message(&Err("disk full".into())), "Backup failed: disk full");
}

#[test]
fn restore_message_reports_success_and_failure() {
    assert_eq!(restore_message(&Err("bad file".into()), "a.json"), "Could not restore: bad file");
}

#[test]
fn restore_success_names_the_file() {
    let snapshot = crate::tests::app().snapshot.unwrap();
    assert_eq!(restore_message(&Ok(snapshot), "a.json"), "Config restored from a.json.");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib screens::advanced::`
Expected: FAIL.

- [ ] **Step 3: Implement**

`view` renders the app bar and four action tiles. Restart runs `core::init(None)` through `Message::Loaded` and keeps the screen. Backup/restore/reset use dedicated messages that set the snackbar text from the helpers. Restore with a non-empty list opens `Dialog::RestorePicker`, then `Dialog::Confirm { action: ConfirmAction::Restore(name) }`. Factory reset opens `Dialog::Confirm { action: ConfirmAction::FactoryReset }`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib screens::advanced::`
Expected: PASS.

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 12: Groups screen

**Files:**
- Create: `rust/qlaunch_iced/src/screens/groups.rs`
- Modify: `rust/qlaunch_iced/src/screens/mod.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub(crate) struct GroupRow { pub id: Id, pub label: String, pub role: Option<GroupRole>, pub hidden: bool, pub always_hidden: bool, pub page_index: Option<usize>, pub page_count: usize }`.
  - `pub(crate) fn rows(snapshot: &ModelSnapshot) -> Vec<GroupRow>`.
  - `pub(crate) fn view<'a>(app: &'a App) -> Element<'a, Message>`.

Note: `fixture_snapshot()` in the tests below is a small local helper that builds a `ModelSnapshot` with a root group, an Apps group, two page groups, a hidden group and a folder, in the style of the existing `lib.rs` test fixture. Add it in the test module.

- [ ] **Step 1: Write the failing tests** in `screens/groups.rs`:

```rust
#[test]
fn rows_exclude_root_and_folders_and_follow_order() {
    let snapshot = fixture_snapshot();
    let rows = rows(&snapshot);
    assert!(rows.iter().all(|row| row.id != snapshot.root_group_id));
    let mut orders: Vec<usize> = rows.iter().map(|row| snapshot.groups.iter().position(|g| g.id == row.id).unwrap()).collect();
    let before = orders.clone();
    orders.sort_by_key(|&i| snapshot.groups[i].order);
    assert_eq!(before, orders);
}

#[test]
fn hidden_role_is_always_hidden() {
    let rows = rows(&fixture_snapshot());
    let hidden = rows.iter().find(|row| row.role == Some(GroupRole::Hidden)).unwrap();
    assert!(hidden.always_hidden && hidden.hidden);
}

#[test]
fn page_groups_get_an_index_and_others_do_not() {
    let snapshot = fixture_snapshot();
    let rows = rows(&snapshot);
    for row in &rows {
        assert_eq!(row.page_index.is_some(), snapshot.page_ids.contains(&row.id));
        if row.page_index.is_some() {
            assert_eq!(row.page_count, snapshot.page_ids.len());
        }
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib screens::groups::`
Expected: FAIL.

- [ ] **Step 3: Implement**

`rows` filters and sorts exactly as `group_editor_screen.dart`. `view` renders the app bar (with the `+` action opening `Dialog::AddGroup`) and one row per group: `components::list_tile` with the hide switch and up/down `icon_button`s (disabled at the ends) as trailing; tapping the row opens `Dialog::GroupEditor`. Handlers call `set_group_hidden` and `move_group` with `close: false`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib screens::groups::`
Expected: PASS.

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 13: Choose Apps screen

**Files:**
- Create: `rust/qlaunch_iced/src/screens/group_apps.rs`
- Modify: `rust/qlaunch_iced/src/screens/mod.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub(crate) struct AppRow { pub id: Id, pub label: String, pub checked: bool, pub enabled: bool }`.
  - `pub(crate) fn rows(snapshot: &ModelSnapshot, group_id: &Id) -> Vec<AppRow>`.
  - `pub(crate) fn view<'a>(app: &'a App, group_id: &Id) -> Element<'a, Message>`.
  - `pub(crate) fn move_plan(snapshot: &ModelSnapshot, item: &Item, group_id: &Id, checked: bool) -> Option<Change>`.

Reuse the `fixture_snapshot()` helper described in Task 12 (make it shared in `screens/mod.rs`'s test module if convenient).

- [ ] **Step 1: Write the failing tests** in `screens/group_apps.rs`:

```rust
#[test]
fn rows_list_installed_apps_sorted_and_checked() {
    let snapshot = fixture_snapshot();
    let group = snapshot.page_ids[1].clone();
    let rows = rows(&snapshot, &group);
    let mut labels: Vec<String> = rows.iter().map(|row| row.label.to_lowercase()).collect();
    let sorted = { let mut l = labels.clone(); l.sort(); l };
    assert_eq!(labels, sorted);
    labels.clear();
    assert!(rows.iter().all(|row| row.checked == snapshot.items.iter().any(|i| i.id == row.id && i.group_id == group)));
}

#[test]
fn apps_group_checklist_is_disabled() {
    let snapshot = fixture_snapshot();
    let apps = snapshot.groups.iter().find(|g| g.role == Some(GroupRole::Apps)).unwrap();
    assert!(rows(&snapshot, &apps.id).iter().all(|row| !row.enabled));
}

#[test]
fn check_moves_to_group_and_uncheck_to_apps() {
    let snapshot = fixture_snapshot();
    let item = snapshot.items.iter().find(|i| i.app.is_some()).unwrap();
    let apps = snapshot.groups.iter().find(|g| g.role == Some(GroupRole::Apps)).unwrap();
    let target = snapshot.page_ids.iter().find(|id| **id != apps.id).unwrap();
    assert!(matches!(move_plan(&snapshot, item, target, true), Some(Change::MoveItem(_, ref g, _)) if g == target));
    assert!(matches!(move_plan(&snapshot, item, target, false), Some(Change::MoveItem(_, ref g, _)) if g == &apps.id));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib screens::group_apps::`
Expected: FAIL.

- [ ] **Step 3: Implement**

`rows` lists installed, non-uninstalled apps sorted case-insensitively; `enabled` is false in the Apps group. `view` renders the group label as the app bar and `components::checkbox_row` per app. Toggling runs `move_plan` through a `Task` with `close: false`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib screens::group_apps::`
Expected: PASS.

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 14: Accessibility frames for screens, dialogs and the select menu

**Files:**
- Modify: `rust/qlaunch_iced/src/accessibility.rs`

**Interfaces:**
- Consumes: `screens`, `dialog`, `components::select_menu_bounds`.
- Produces:
  - `Region::Screen` with widget id `screen-scroll`.
  - `pub(crate) enum Target { ..., Message(Message), Slider { step_up: Message, step_down: Message } }`.
  - `fn dialog_frame(app: &App, dialog: &Dialog) -> Frame`, `fn screen_frame(app: &App) -> Frame`, `fn select_frame(app: &App, select: &Select) -> Frame`.
  - `frame(app)` precedence: `select` → `dialog` → `screen` → existing overlay/search/folder/launcher.

- [ ] **Step 1: Write the failing tests** in `accessibility.rs::tests`:

```rust
#[test]
fn settings_frame_exposes_back_title_and_controls() {
    let mut app = app();
    app.screens.push(Screen::Settings);
    let frame = frame(&app).unwrap();
    assert_eq!(frame.screen, "settings");
    assert!(frame.nodes.iter().any(|n| n.label == "Back"));
    assert!(frame.nodes.iter().any(|n| n.role == Role::Header));
    assert!(frame.nodes.iter().any(|n| n.label.starts_with("Continuous scrolling")));
}

#[test]
fn dialog_frame_hides_the_screen_below() {
    let mut app = app();
    app.screens.push(Screen::Groups);
    app.dialog = Some(dialog::Dialog::Launcher);
    let frame = frame(&app).unwrap();
    assert!(frame.screen.starts_with("dialog"));
    assert!(!frame.nodes.iter().any(|n| n.key.starts_with("group:")));
}

#[test]
fn select_frame_exposes_every_option() {
    let mut app = app();
    app.select = Some(dialog::Select { field: dialog::SelectField::AppGroup, options: vec![
        dialog::SelectOption { id: "apps".into(), label: "Apps".into(), folder: false },
        dialog::SelectOption { id: "games".into(), label: "Games".into(), folder: false },
    ], selected: "apps".into() });
    let frame = frame(&app).unwrap();
    assert!(frame.screen.starts_with("select"));
    assert!(frame.nodes.iter().any(|n| n.label.contains("Games")));
}

#[test]
fn slider_advertises_scroll_actions() {
    let mut app = app();
    app.screens.push(Screen::Settings);
    let frame = frame(&app).unwrap();
    let slider = frame.nodes.iter().find(|n| n.label.starts_with("Page transition duration")).unwrap();
    assert!(slider.actions.contains(&Action::ScrollForward));
    assert!(slider.actions.contains(&Action::ScrollBackward));
    assert!(!slider.actions.contains(&Action::ScrollLeft));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib accessibility::`
Expected: FAIL.

- [ ] **Step 3: Implement**

Build the frames from the same `dialog::layout`, `screens::*::rows` and `select_menu_bounds` used by the view. Add `Target::Message`/`Target::Slider` dispatch in `App::accessibility_action` (steps sliders through their messages; buttons through the carried `Message`). Annotate slider nodes with `scroll_forward`/`scroll_backward` and a label of `<label>, <value><unit>`. Keep clipping to the tracked scroll region and the window.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p qlaunch_iced --no-default-features --lib accessibility::`
Expected: PASS.

- [ ] **Step 5: Checkpoint** — report complete.

---

### Task 15: Search gear, retire `material.rs`, full verification

**Files:**
- Modify: `rust/qlaunch_iced/src/view.rs` (search gear + close icon, theme styles)
- Modify: `rust/qlaunch_iced/src/folder.rs`, `rust/qlaunch_iced/src/lib.rs` (theme styles)
- Delete: `rust/qlaunch_iced/src/material.rs`
- Modify: `rust/qlaunch_iced/src/lib.rs` (remove `mod material;`)

**Interfaces:**
- Consumes: everything above.
- Produces: no new public items; the search header gains a settings gear (`icons::SETTINGS`) and a close icon (`icons::CLOSE`), and every `material::` call becomes `theme::`.

- [ ] **Step 1: Write the failing test** in `lib.rs::tests`:

```rust
#[test]
fn search_gear_opens_settings() {
    let mut app = app();
    app.searching = true;
    let _ = app.update(Message::Push(Screen::Settings));
    assert_eq!(app.screens.last(), Some(&Screen::Settings));
    assert!(app.searching);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p qlaunch_iced --no-default-features --lib search_gear_opens_settings`
Expected: FAIL if `Push` is not handled from search; PASS if Task 9 already wired it (then fold this test in and skip to Step 3).

- [ ] **Step 3: Implement**

Add the gear `icon_button` to the search row and wire it to `Message::Push(Screen::Settings)`; switch the close control to `icons::CLOSE`. Replace every `material::` reference with the matching `theme::` item and delete `material.rs`. Keep the existing scrollbar and folder popup behaviour identical.

- [ ] **Step 4: Run the full verification**

Run:
```bash
cargo test -p qlaunch_iced --no-default-features --lib
cargo test -p qlaunch_core --no-default-features
cargo fmt -p qlaunch_iced
cargo fmt -p qlaunch_iced --check
cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings
cargo check -p qlaunch_iced --no-default-features
```
Expected: all pass.

- [ ] **Step 5: Run the desktop preview**

Run (from `rust/`):
```bash
XDG_RUNTIME_DIR=/run/user/1000 WAYLAND_DISPLAY=wayland-1 cargo run -p qlaunch_iced --no-default-features --bin qlaunch-iced
```
Expected: launches; open each dialog and screen and compare against the Flutter build; no runtime errors.

- [ ] **Step 6: Checkpoint** — report the task complete with the test counts, clippy result, and the desktop findings; the user owns the commit.

---

## Self-Review

**Spec coverage**
- Theme tokens and typography → Tasks 1–2. Icons → Task 3. Components → Tasks 4–5.
- Dialog model and layout → Task 6; app/folder/launcher → Task 7; shortcuts/groups/confirm/restore → Task 8.
- Navigation, back order, input gating → Task 9.
- Settings → Task 10; Advanced + snackbar → Task 11; Groups → Task 12; Choose Apps → Task 13.
- Accessibility → Task 14. Search gear and existing chrome → Task 15.
- Core integration (`ModelDone`, plans, error paths) → Tasks 6–8, 10–13.
- Verification and desktop comparison → Task 15.

**Step scan**
Each task's steps are: write named tests with assertions, run to fail, implement against named signatures, run to pass, checkpoint. No step says "handle edge cases" or "write tests for the above".

**Type consistency**
`Dialog`/`Row`/`Change`/`Select`/`Screen`/`Target::Message`/`ModelDone` are defined in Tasks 6 and 9 and reused verbatim. `screens::*::rows` return the shapes tests assert. `components` names match the calls in Tasks 7–13.

**Review Focus coverage**
1. Text scale → Task 10 slider/row tests plus Task 14 node labels.
2. Busy re-activation → busy flags in Tasks 6–8 and `close: false` paths in 12–13.
3. Layered Back/Home → Task 9 order test and Task 14 precedence test.
4. Many/long options → Task 5 `select_menu_bounds` tests and Task 6 scroll test.
5. Core errors → Task 7 `ModelDone` error path, Task 11 message helpers, Task 8 confirmations.

**Proportion**
The plan names interfaces and tests rather than bodies; the only code blocks are test assertions and the asset copy, matching the spec's size.
