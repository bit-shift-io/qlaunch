# Shared toast for brief messages

## Intent

A brief result message (backup saved, restore failed, permission needed) currently
paints the entire screen near-white for four seconds. It should be a small, dark,
floating message at the bottom. Shared UI components are a stated priority of this
codebase, so this message channel becomes a first-class shared component, and the
other bright surface — the start-up "Loading apps…" fallback — is fixed in the
same change.

## Root cause (confirmed on device)

`components::snackbar` builds its container with `.height(SNACKBAR_HEIGHT)` and
then `.center_y(Length::Fill)`. In iced, `Container::center_y(h)` *sets* the
height, so `Length::Fill` overrides the 48 px height. The container stretches to
the window and paints its `INVERSE_SURFACE` (`0xe6e0e9`, near-white) background
edge to edge. Captured on the tablet: tapping Advanced → Backup config turns the
whole screen near-white with the message, then the Advanced screen returns when
the 4 s generation expires.

## Goals

- One shared toast component used by every brief message.
- The toast is small, dark and bottom-anchored; the screen behind it stays visible.
- No full-window or bright background is ever painted for a message.
- The no-snapshot "Loading apps…" / error fallback draws on the dark surface.
- Record the shared-components rule in `ARCHITECTURE.md`.

## Non-goals

- Swipe-to-dismiss, action buttons, or a message queue (single latest message with
  a generation guard stays).
- Changing the 4 s duration or any message copy.
- Reworking `stack` ordering or the accessibility announcement content.

## Anti-goals

- Any message surface larger than its text needs.
- Any near-white background.
- A second, parallel message channel beside the existing one.

## Approaches

- **A (chosen): one shared toast.** Replace `components::snackbar` with
  `components::toast`; keep the existing `App` channel and rename it to match
  (`toast` / `Message::Toast` / `ToastExpired` / `toast_generation`). Every brief
  message already flows through it, so nothing new is wired up.
- **B (rejected): a separate `toast` module beside the snackbar.** Duplicates the
  generation/expiry logic and leaves two overlapping message concepts.
- **C (rejected): patch the height and colour in place.** Smallest diff, but keeps
  the one-off composition inline in `view.rs` and gives nothing to standardise on.

## Shape

`components.rs`

- Replace `snackbar` with `toast(&Text, &str)`: a content-hugging container
  (`width` shrink, `max_width = TOAST_MAX_WIDTH`) with the message in `BodyMedium`,
  `ON_SURFACE` text, padding `[14, 16]`, and the rounded dark card below.
- Extract `toast_style() -> container::Style` (pure, testable) returning
  `background = SURFACE_CONTAINER_HIGHEST`, `text_color = ON_SURFACE`,
  `border = rounded(TOAST_RADIUS)` and the same subtle shadow as `theme::menu`.

`theme.rs`

- Add `metrics::{TOAST_MAX_WIDTH = 480.0, TOAST_RADIUS = 8.0, TOAST_MARGIN = 16.0}`;
  drop the now-unused `SNACKBAR_HEIGHT`.
- `INVERSE_SURFACE` / `ON_INVERSE_SURFACE` keep their tokens but are no longer used
  by the message channel (left in place: they are part of the stated colour scheme).

`lib.rs`

- Rename `App.snackbar` → `toast`, `snackbar_generation` → `toast_generation`,
  `Message::Snackbar` → `Message::Toast`, `Message::SnackbarExpired` →
  `Message::ToastExpired`; update every arm and test that names them.

`view.rs`

- Toast stack slot: wrap `components::toast(..)` in a `Fill × Fill` container with
  `align_x(Center)`, `align_y(Bottom)`, and bottom/left/right padding
  `insets.* + TOAST_MARGIN`. Centring plus content width is what keeps it a
  message-sized card instead of a full-width banner.
- No-snapshot fallback (`view.rs:13`): draw the text on
  `theme::color::SURFACE` with `ON_SURFACE` colour, centred. No white background.

`accessibility.rs`

- Announcement key becomes `format!("toast:{generation}")`; announcement content
  is unchanged.

`ARCHITECTURE.md`

- In the iced-UI section, state the rule: new UI is composed from the shared
  `components.rs` widgets and `theme.rs` tokens, never one-off styling; brief
  result messages use the shared toast.

## Behaviour

| Message shown | Screen behind | Toast |
| :--- | :--- | :--- |
| single short line | unchanged | dark ~48 px card at the bottom |
| long line (backup path) | unchanged | dark card, wraps to two lines at most width |
| after 4 s | unchanged | gone |

## Verification

- Unit: `toast_style()` background is `SURFACE_CONTAINER_HIGHEST`, not
  `INVERSE_SURFACE`; text colour is `ON_SURFACE`.
- Headless layout (`#[ignore]`, run with `--ignored`): lay a `components::toast`
  out inside a `Fill × Fill` parent in a tall viewport and assert the toast's
  node height is far smaller than the viewport — this is the regression guard for
  the `center_y(Length::Fill)` stretch.
- Unit: existing backup/restore/expiry tests pass under the renamed messages.
- `cd rust && cargo test --workspace` and `cargo clippy`.
- Manual on device: Advanced → Backup config leaves the Advanced screen visible
  with a small dark toast at the bottom; force-stop and relaunch to check the
  start-up fallback is dark, not white.

## Open to the builder

- Exact shadow values reuse `theme::menu`; adjust only if it reads poorly on device.
- Whether to keep `TOAST_MAX_WIDTH` at 480 px or scale it with the content width;
  start at 480 and check on the phone and tablet.
