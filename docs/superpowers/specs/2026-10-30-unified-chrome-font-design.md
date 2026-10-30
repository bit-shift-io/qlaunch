# Unified chrome font, launcher-only custom font

Date: 2026-10-30 · Branch: `dev/iced` · Small change (short design).

## Intent and why

Give the app one chrome font and one theme. The Material chrome — dialogs,
long-press menus, the Settings/Groups/Advanced/Group-apps screens, buttons,
chips, sliders, the snackbar and the app bar — must render in the platform
font, sized as Material specifies, regardless of the launcher's Text settings.
Android and Linux already give every app its own GUI font size; the launcher
should look like the rest of the OS there.

The launcher's own special interface — the home-screen grid app labels, the
tab/title strip, folder item labels and shortcut labels — is the only place
the Font-size and Typeface settings apply.

## Root cause this also fixes

`theme::Text::from_settings` set the whole UI's type from
`settings.label_size` and `settings.label_typeface`, and used iced's generic
families (`Family::SansSerif`/`Serif`/`Monospace`). cosmic-text maps those to
`Open Sans` / `DejaVu Serif` / `Noto Sans Mono`, which Android never loads
(no fontconfig), so all chrome text drew blank while app labels — which used
the platform default `Roboto` — rendered. The chrome and the labels were
two different font identities.

## Decisions

- **`theme::Text` is settings-free.** `Text::new()` returns scale `1.0` and
  `platform::DEFAULT_FONT`. `Text::from_settings`, `base_font` and
  `typeface_font` are removed from `theme`. Every chrome call site passes
  `Text::new()`.
- **The OS font scale still applies.** The UI is scaled once by
  `scale_factor(app.scale)`, fed from Android's system font scale
  (`platform::system_scale()`), so chrome tracks the device like any app.
- **`view::label()` keeps reading settings** (`label_size`, `label_typeface`,
  colour, shadow). Grid, tab strip, folders and shortcuts all route through it,
  so all four respond to the settings.
- **Label typefaces resolve to families that exist per platform**, so no
  choice draws blank:
  - Android: `Normal`/`Sans` → `Roboto`; `Monospace` → `Droid Sans Mono`;
    `Serif` → `Noto Serif`, loaded from `/system/fonts` at start-up.
  - Linux: `Normal`/`Sans` → the Iced default; `Serif`/`Monospace` → the
    generic families (fontconfig provides them).

## Non-goals

- No change to how settings are stored or to the settings UI.
- No separate font controls per screen.
- Launcher chrome text sizes are fixed Material sizes, not user-scaled.

## Approaches

- **Chosen:** make the theme type scale fixed and settings-free; keep the
  settings reading isolated to the label path. One source per concern.
- **Rejected:** keep `Text::from_settings` and ignore its arguments — a
  signature that lies about what it uses.

## Verification

- Unit tests: the chrome type roles have fixed Material sizes and the platform
  font; the label typeface maps to the expected family per platform.
- Device: menus, dialogs and the Settings screen render text on Android; the
  grid, tab strip, folder and shortcut labels follow the Font-size and
  Typeface settings, and all four typefaces render.
