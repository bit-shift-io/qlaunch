# Iced: configurable actions, search shortcuts, unified scrollbar

Working plan (2026-11-01). Target: `dev/iced` (Rust/Iced + Kotlin adapter).

## 1. Configurable gesture actions (core-owned settings)

- [x] Core `GestureAction { None, Lock, Search, Notifications }`; Settings fields
      `home_action = Notifications`, `swipe_up_action = None`,
      `pull_down_action = None`, `double_tap_action = Lock`; SettingsPatch +
      apply + serde.
- [x] Iced Settings: new "Actions" card with four choice rows
      (Home button, Swipe up, Swipe down, Double tap).
- [x] Iced wiring:
  - Home-when-home performs `home_action` (was always search).
  - Double-tap performs `double_tap_action` (was always lock).
  - New swipe-up gesture performs `swipe_up_action` (default off).
  - Pull-down performs `pull_down_action` (was hard-coded notifications; still
    gated to the top of the page).
- [x] "Search" means the in-app search screen (`open_search`).
- [x] Search icon: `SEARCH` was `0xe8b6` = `delete_forever_sharp` in the bundled
      font (a trash can); corrected to `0xe567` = `search_baseline`.

## 2. New ways to reach search

- [x] Blank-space long-press menu gains a **Search** row.
- [x] **Add Shortcut** dialog gains a **Search** tile; new
      `ShortcutPayload::Search` + `api::add_search_shortcut`; tapping the item
      opens the search screen.

## 3. Unified scrollbar

- [x] Screen scaffold and dialog scrollers use `theme::vertical_scrollbar(...)`
      (the slim 6 dp bar) instead of Iced's default chunky one.
      Visual wiring; not unit-testable at this seam.

## Verification

- [x] `cargo test --workspace` in `rust/` green.

## Notes

- `frb_generated.rs` was hand-updated so the default `flutter` feature still
  compiles; the Flutter `main` Dart bindings would need regenerating to use the
  new fields/variant there.
- The scrollbar change has no unit test: Iced exposes no scrollbar direction to
  assert on. Verified by build plus the existing `theme` scrollbar test.
- Interpretation: "a shortcut to the app for a search screen" = a placeable
  **Search** item in the launcher's Add Shortcut dialog, alongside Lock Screen
  and Notifications — not an Android static app shortcut.

