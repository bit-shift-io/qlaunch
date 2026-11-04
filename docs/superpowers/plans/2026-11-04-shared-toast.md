# Shared Toast Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the full-screen near-white "snackbar" with a small, dark, shared toast for brief result messages, and make the start-up fallback dark.

**Architecture:** A brief message already flows through one channel (`App` field + `Message` + a generation-guarded 4 s expiry). We rename that channel to `toast`, replace the widget in `components.rs` with a content-hugging dark card built from `theme.rs` tokens, and fix `view.rs` to centre it at the bottom above the nav inset. The no-snapshot fallback gets the dark surface too.

**Tech Stack:** Rust edition 2024, iced (`iced_android-v0.1.1` fork), `qlaunch` UI crate.

**Spec:** `docs/superpowers/specs/2026-11-04-shared-toast-design.md`

## Global Constraints

- Every colour, size and text style comes from `theme.rs`; `components.rs` and screens invent no values.
- Keep the existing message copy and the 4 s expiry unchanged.
- `cd rust && cargo test --workspace` is the gate. Pre-existing clippy warnings are intentional; do not fix unrelated ones.
- The `toast` component is shared: it lives in `components.rs` and is driven by the shared message channel, not by a screen-local surface.

## Review Focus

Inputs and failure modes the spec implies that no automated test fully covers; verify manually on the tablet (1200x1920):

1. A long backup-path message on a narrow phone wraps inside the toast instead of clipping or stretching the surface.
2. The toast must not swallow taps meant for the screen behind it.
3. With the launcher base (no screen) visible, the toast still sits bottom-centre above the nav inset.
4. A toast shown while search or a folder is open renders above it (stack order unchanged) and does not cover a dialog.
5. Rotating/resizing while a toast is visible re-anchors it using the new insets.

---

### Task 1: Rename the message channel `snackbar` → `toast`

Pure rename, no behaviour change, so the later tasks speak one name. Every occurrence is mechanical.

**Files:**
- Modify: `rust/qlaunch/src/lib.rs`
- Modify: `rust/qlaunch/src/view.rs`
- Modify: `rust/qlaunch/src/components.rs`
- Modify: `rust/qlaunch/src/theme.rs`
- Modify: `rust/qlaunch/src/accessibility.rs`
- Modify: `rust/qlaunch/src/screens/advanced.rs`
- Modify: `rust/qlaunch/src/screens/settings.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `App.toast: Option<(u64, String)>`, `App.toast_generation: u64`, `Message::Toast(String)`, `Message::ToastExpired(u64)`, `components::toast(&Text, &str) -> Element<Message>`, `theme::metrics::TOAST_HEIGHT`.

- [ ] **Step 1: Rename identifiers across the crate**

Apply this mapping everywhere (including tests):

| From | To |
| :--- | :--- |
| `App.snackbar` / `self.snackbar` / `app.snackbar` | `toast` |
| `snackbar_generation` | `toast_generation` |
| `Message::Snackbar` | `Message::Toast` |
| `Message::SnackbarExpired` | `Message::ToastExpired` |
| `components::snackbar` | `components::toast` |
| `theme::metrics::SNACKBAR_HEIGHT` | `theme::metrics::TOAST_HEIGHT` |
| `format!("snackbar:{generation}")` | `format!("toast:{generation}")` |
| the local `let snackbar` in `view.rs` | `let toast` |

- [ ] **Step 2: Build and test**

Run: `cd rust && cargo test -p qlaunch`
Expected: PASS, same tests as before the rename.

- [ ] **Step 3: Check for leftovers**

Run: `grep -rin snackbar rust/qlaunch/src`
Expected: no matches.

- [ ] **Step 4: Commit**

```bash
git add rust/qlaunch/src
git commit -m "refactor: rename the snackbar message channel to toast"
```

---

### Task 2: Fix the toast — dark, content-hugging, bottom-centre

Replaces the widget body. The old body's `.center_y(Length::Fill)` set the container's height to `Fill`, overriding its 48 px height and painting `INVERSE_SURFACE` over the whole window.

**Files:**
- Modify: `rust/qlaunch/src/theme.rs`
- Modify: `rust/qlaunch/src/components.rs`
- Modify: `rust/qlaunch/src/view.rs`
- Test: `rust/qlaunch/src/components.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `components::toast`, `Message`, `theme::Text`/`TextRole`, `theme::color`, `theme::metrics` from Task 1.
- Produces: `components::toast_style() -> container::Style`; `theme::metrics::{TOAST_MAX_WIDTH, TOAST_RADIUS, TOAST_MARGIN}`.

- [ ] **Step 1: Write the failing style test**

Add to `components.rs`'s test module:

```rust
#[test]
fn toast_style_is_a_dark_surface() {
    let style = toast_style();
    assert_eq!(
        style.background,
        Some(theme::color::SURFACE_CONTAINER_HIGHEST.into())
    );
    assert_eq!(style.text_color, Some(theme::color::ON_SURFACE));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd rust && cargo test -p qlaunch components::tests::toast_style_is_a_dark_surface`
Expected: FAIL — `toast_style` not found.

- [ ] **Step 3: Add the metrics**

In `theme.rs` under `mod metrics`, add:

```rust
pub(crate) const TOAST_MAX_WIDTH: f32 = 480.0;
pub(crate) const TOAST_RADIUS: f32 = 8.0;
pub(crate) const TOAST_MARGIN: f32 = 16.0;
```

Remove `TOAST_HEIGHT` (the content-sized toast no longer needs a fixed height).

- [ ] **Step 4: Rewrite `components::toast` and add `toast_style`**

Replace the whole `toast` function body with:

```rust
/// The rounded dark card behind the toast, shared so a test can pin its colour.
pub(crate) fn toast_style() -> container::Style {
    container::Style {
        background: Some(theme::color::SURFACE_CONTAINER_HIGHEST.into()),
        text_color: Some(theme::color::ON_SURFACE),
        border: iced::border::rounded(theme::metrics::TOAST_RADIUS),
        shadow: iced::Shadow {
            color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.30),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

pub(crate) fn toast<'a>(text_settings: &Text, message: &str) -> Element<'a, Message> {
    let style = text_settings.role(TextRole::BodyMedium);
    container(
        text(message.to_string())
            .size(style.size)
            .line_height(style.line_height())
            .font(style.font)
            .color(theme::color::ON_SURFACE),
    )
    .max_width(theme::metrics::TOAST_MAX_WIDTH)
    .padding([14, 16])
    .style(|_| toast_style())
    .into()
}
```

- [ ] **Step 5: Run the style test**

Run: `cd rust && cargo test -p qlaunch components::tests::toast_style_is_a_dark_surface`
Expected: PASS.

- [ ] **Step 6: Write the ignored layout regression test**

The old bug made the toast's layout node fill a tall parent. Add to the same test module:

```rust
#[test]
#[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
fn toast_hugs_its_text_height() {
    let node = control_layout(components::toast(&Text::new(), "Backup saved."));
    // 1000 px is the layout limit in `control_layout`; a Fill height would hit it.
    assert!(node.bounds().height < 100.0, "toast stretched: {node:?}");
    assert!(node.bounds().width <= 360.0, "toast exceeded the width limit");
}
```

- [ ] **Step 7: Run the ignored test**

Run: `cd rust && cargo test -p qlaunch -- --ignored toast_hugs_its_text_height`
Expected: PASS.

- [ ] **Step 8: Wire the toast into `view.rs` and darken the fallback**

In `view.rs`, replace the `toast` stack-slot body with:

```rust
let toast: Element<'_, Message> = if let Some((_, message)) = &app.toast {
    let typography = crate::theme::Text::new();
    let margin = crate::theme::metrics::TOAST_MARGIN;
    container(crate::components::toast(&typography, message))
        .width(Fill)
        .height(Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(iced::Padding {
            top: 0.0,
            right: app.insets.right + margin,
            bottom: app.insets.bottom + margin,
            left: app.insets.left + margin,
        })
        .into()
} else {
    space().into()
};
```

In the same file, replace the no-snapshot early return with:

```rust
let Some(snapshot) = &app.snapshot else {
    return container(
        text(app.error.as_deref().unwrap_or("Loading apps…"))
            .color(crate::theme::color::ON_SURFACE),
    )
    .center(Fill)
    .style(|_| container::Style {
        background: Some(crate::theme::color::SURFACE.into()),
        ..Default::default()
    })
    .into();
};
```

- [ ] **Step 9: Run the whole suite**

Run: `cd rust && cargo test --workspace`
Expected: PASS.

- [ ] **Step 10: Manual device check**

Build/install per `ARCHITECTURE.md`, then on the tablet: Advanced → Backup config. Expected: the Advanced screen stays visible with a small dark toast at the bottom for ~4 s, then it disappears; nothing near-white paints the window. Force-stop and relaunch: the start-up "Loading apps…" screen is dark, not white.

- [ ] **Step 11: Commit**

```bash
git add rust/qlaunch/src
git commit -m "fix: shared dark toast instead of a full-screen message"
```

---

### Task 3: Record the shared-components rule in `ARCHITECTURE.md`

**Files:**
- Modify: `ARCHITECTURE.md`

**Interfaces:**
- Consumes: nothing.
- Produces: nothing (documentation).

- [ ] **Step 1: Add the rule**

In `ARCHITECTURE.md`, in the iced-UI section (near the module table under "The iced UI (`rust/qlaunch/src`)"), add:

```markdown
### Shared components

Reusable UI lives in `components.rs`, with every colour, size and text style coming
from `theme.rs`. New screens and dialogs compose these; they do not invent one-off
styling. Brief result messages use the shared toast (`App.toast`, `Message::Toast`),
never a bespoke surface.
```

- [ ] **Step 2: Verify**

Run: `grep -n "Shared components" ARCHITECTURE.md`
Expected: one match.

- [ ] **Step 3: Commit**

```bash
git add ARCHITECTURE.md
git commit -m "docs: shared components are a priority"
```

---

## Self-Review

- **Spec coverage:** root-cause fix (Task 2), one shared component (Tasks 1–2), dark fallback (Task 2), `ARCHITECTURE.md` rule (Task 3). All spec sections covered.
- **Step scan:** each step has one checkable action; signatures and exact tokens are given, bodies are left to the implementer.
- **Type consistency:** `toast_style`, `toast`, `TOAST_MAX_WIDTH`/`TOAST_RADIUS`/`TOAST_MARGIN`, `App.toast`, `Message::Toast`/`ToastExpired` are used consistently across tasks.
- **Review Focus:** long-message wrapping (Task 2 Step 6 layout test + manual), tap-through/stack/insets are manual; noted above.
- **Proportion:** plan stays near the spec's length; only the two non-obvious bodies are shown.
