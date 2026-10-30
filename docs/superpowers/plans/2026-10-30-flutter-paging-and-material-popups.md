# Flutter Paging and Material Popups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore Flutter's overlapping, cross-fading page motion and compact Material folder/menu appearance in the ongoing Iced port.

**Architecture:** Keep the existing pager state machine and custom clipped `Slots` renderer. Change page travel and visual alpha, render folders as modal grids above the launcher, and share popup geometry between rendering, gestures and accessibility. Style stock Iced widgets ourselves; do not adopt `iced_material`.

**Tech Stack:** Rust, existing iced_android/Iced fork, qlaunch_core, existing Android accessibility bridge.

**Spec:** User-approved chat scope: one-column travel with cross-fading; compact content-sized folder popup with rounded corners, elevation and dimmed backdrop; Material-styled long-press menus preserving existing actions. References: `lib/src/ui/group_pager.dart`, `lib/src/ui/folder_popup.dart`, `lib/src/ui/app_settings_dialog.dart`, and the dark Material 3 theme in `lib/main.dart`.

## Execution Status (2026-10-30)

- Paging, compact folders and Material menu styling implemented on `dev/iced`, uncommitted at the user's request.
- Review fixes preserve underlying widget/scroll state, consume folder-dismissal pointer sequences, and use one menu scroll region so nested lists cannot trap swipes.
- Scrollbars are hidden while idle and appear only while a scroll offset is changing, matching Flutter.
- 157 Iced tests pass; core tests, strict Clippy and formatting pass.
- Desktop preview launched for user comparison. Android Rust release compilation passed; APK packaging encountered a local JDK/compiler problem. Physical-device visual, IME and TalkBack checks remain outstanding.

## Global Constraints

- No Material dependency; use Flutter as the design reference.
- Preserve existing menu actions and core model rules; this is not an app-menu workflow rewrite.
- Retain the Android/Iced setup, Flutter-free builds and saved user data.
- Preserve settled-page input gating, wrapping, existing settle timing and idle subscription behavior.
- Update accessibility bounds and gesture regions with visual geometry, not in a later pass.
- Format only `qlaunch_iced`; do not format unrelated generated core files.
- Treat this as port work, not a throwaway spike. Do not rewrite historical spike reports.

## Review Focus

- One-column/list layouts: use Flutter's half-width travel fallback rather than a full-width slide.
- Empty, large and landscape folders: stay within usable screen bounds and scroll when needed.
- Folder dismissal/nested app menus: never launch or page the background; Back closes the topmost layer first.
- Hidden icons/labels, custom alpha and contrasting label shadows: all visible page primitives fade consistently.
- Small windows, large text and IME: popup actions remain reachable and semantic bounds track the rendered card.

## File Responsibilities

- `rust/qlaunch_iced/src/slots.rs`: column-dependent page translations; retain title-strip geometry and event routing.
- `rust/qlaunch_iced/src/view.rs`: page alpha propagation, base launcher behind folders, reusable grid drawing at an explicit column count.
- Create `rust/qlaunch_iced/src/material.rs`: local dark Material surface/text/accent tokens, rounded card/elevation and control style functions.
- Create `rust/qlaunch_iced/src/folder.rs`: compact folder geometry and popup view, using the existing cell renderer and core items.
- `rust/qlaunch_iced/src/lib.rs`: module wiring, folder-specific gesture bounds, modal dismissal and scroll operations.
- `rust/qlaunch_iced/src/accessibility.rs`: folder popup nodes and compact-grid columns; preserve topmost-modal tree isolation.
- `rust/qlaunch_iced/src/overlay.rs`: Material-styled menu controls and responsive shared card geometry.

---

### Task 1: Flutter Page Travel and Cross-fade

**Files:** Modify `rust/qlaunch_iced/src/slots.rs` and `rust/qlaunch_iced/src/view.rs`; tests in their existing test modules (or `lib.rs` for view integration).

**Interfaces:** Add column count to `Slots::pages`; encode it in the page arrangement, leaving `Strip` unchanged. Expose `page_opacity(relative: f32) -> f32` and `page_travel(columns: usize) -> f32` as crate-local pure helpers. Internal grid/cell drawing consumes an explicit page alpha; folder/search callers use `1.0`.

- [ ] Write failing tests for four-column width 600: center slot at offset 0.5 translates -75; incoming slot translates +75. Single-column travel is 0.5. Mirror these assertions for negative offsets. Strip translation remains unchanged.
- [ ] Write failing alpha tests: relative 0 gives 1; ±0.5 gives 0.25; ±1 gives 0; values outside the fade range clamp to 0. Assert configured icon alpha multiplies page alpha rather than being replaced.
- [ ] Run `cargo test -p qlaunch_iced --no-default-features --lib` from `rust/`; confirm the new expectations fail against full-width opaque paging.
- [ ] Implement translation `(index - 1 - offset) * width * travel`, where travel is `1 / columns` for columns > 1 and `0.5` otherwise. Implement opacity `clamp(1 - abs(relative) * 1.5, 0, 1)` in page content.
- [ ] Propagate alpha to images, placeholder images, label foreground/shadow and pressed highlights. The current renderer has no group-opacity API: do not introduce page textures or a renderer fork. Skip fully transparent slots in draw, but preserve tree reconciliation and event gating.
- [ ] Update existing slot geometry tests for the new explicit columns. Add coverage for hidden labels/icons and shadow alpha; retain redraw forwarding and moving-page input tests.
- [ ] Run the crate tests and formatting check. Review that no opaque per-page background hides the overlapping page.
- [ ] Commit only this task's changes when commits are authorized.

### Task 2: Compact Material Folder Popup

**Files:** Create `rust/qlaunch_iced/src/material.rs` and `rust/qlaunch_iced/src/folder.rs`; modify `view.rs`, `lib.rs`, and `accessibility.rs`.

**Interfaces:** `folder::geometry(size: iced::Size, insets: iced_android::Insets, settings: &Settings, item_count: usize) -> FolderGeometry`, with `bounds: Rectangle`, `viewport: Rectangle`, `columns: usize`, `row_height: f32`. `folder::view(app: &App) -> Element<'_, Message>` renders the modal. Reuse a crate-visible grid renderer accepting columns and alpha. Geometry is the only source of folder layout for the view, raw-touch classifier and semantics.

- [ ] Write failing geometry tests: 0, 1, 2, 9 and many items; portrait/landscape column settings; list layout; nonzero system insets; narrow windows. Assert all bounds lie within the usable window and the scroll viewport is height-capped.
- [ ] Match Flutter's sizing rules: 24 logical px screen margin, 16 popup padding, columns `min(configured, max(1, ceil(sqrt(count))))`; natural grid cell width `max(96, icon_size + 60)`, list cell width `max(180, icon_size + 100)`. Cap cell widths to available space. Cap content height at 70% of usable height minus popup padding; clamp dimensions for tiny windows. Empty content is 140 × 64 where space permits.
- [ ] Write failing integration tests for scrim dismissal, item launch/long-press, scrolling and nested app-menu Back. Assert background title/item actions cannot fire while a folder is open and horizontal gestures do not page behind it.
- [ ] Run the new tests and confirm failures before implementing the popup.
- [ ] Add local dark Material tokens matching Flutter's `ThemeData.dark(useMaterial3: true)`. Use an opaque surface-container-high card, 28 radius, elevation-like shadow and black54 scrim. Resolve the reference colors from the checked-in Flutter SDK/theme rather than inventing a new palette. Do not apply launcher label shadows to menu UI text.
- [ ] Render the launcher page/title strip behind the scrim and place the compact folder grid in the center. Remove the full-screen folder header/back row. Preserve icon/label settings inside the folder and show `Empty folder` for an empty group. Add an explicit folder-dismiss message if needed so a delayed scrim event cannot close an app submenu accidentally.
- [ ] Use folder viewport geometry for blank-space recognition and scroll limits. Suppress background raw-touch inference outside the card and page/pull-down gestures while the modal is open. Keep scrollable identifiers and per-folder scroll tracking coherent with existing reset-on-open behavior.
- [ ] Update `folder_frame` to expose only modal content, a dismiss action and folder items, not the removed header or background. Extend `push_grid` to consume the explicit folder column count rather than assuming `app.columns()`.
- [ ] Run crate tests, including new assertions for semantics clipped to the popup, many-item scroll offsets, empty folders and menu-over-folder modal isolation.
- [ ] Commit only this task's changes when commits are authorized.

### Task 3: Material Long-press Menu Styling

**Files:** Modify `material.rs`, `overlay.rs`, and `accessibility.rs` tests as necessary.

**Interfaces:** Keep `Overlay`, `entries`, existing action messages and model calls unchanged. `overlay::card(app, overlay) -> CardLayout` remains the shared geometry source, extended with a bounded scroll viewport if the card exceeds usable height.

- [ ] Write failing layout tests for phone-width, very narrow and landscape windows, insets, oversized text and long action lists. Assert the card remains within usable bounds and visible action rectangles are clipped to the actual card viewport.
- [ ] Run targeted overlay tests and confirm the new bounds fail with the current fixed 320-wide card.
- [ ] Use shared dark Material surface, on-surface text, accent controls, 28-radius dialog shape, elevation and black54 backdrop. Replace launcher-shadowed menu headings with Material typography. Use 24 content padding, 48-high action rows, 56-high text fields, 8 spacing and a 320 preferred width clamped to available width with 24 screen margins (relaxed only when a tiny viewport requires it).
- [ ] Style action rows as Material text controls with rounded hover/pressed state layers; distinguish Save as a filled primary action and destructive actions with an appropriate error color. Preserve current action order, rename/move steps, focus IDs, error feedback and Back/Cancel behavior.
- [ ] Bound tall card content with a scrollable viewport. Keep existing move/provider list scroll and semantics synchronized; avoid multiple active scroll regions sharing an ID. Update card geometry and accessibility clipping together.
- [ ] Run overlay/accessibility regressions for every menu step, including menu-over-folder, large labels and clipped action lists.
- [ ] Commit only this task's changes when commits are authorized.

### Task 4: Verification and Visual Parity

**Files:** Update this plan's checkboxes; record any remaining visual/device limitations in the handoff rather than claiming unobserved parity.

- [ ] Run `cargo test -p qlaunch_core --no-default-features` from `rust/`.
- [ ] Run `cargo test -p qlaunch_iced --no-default-features --lib` from `rust/`.
- [ ] Run `cargo fmt -p qlaunch_iced --check` from `rust/`.
- [ ] Run `cargo clippy -p qlaunch_iced --no-default-features --all-targets --no-deps -- -D warnings` from `rust/`.
- [ ] Run `bash scripts/build-iced.sh release` from the repository root if the configured Android toolchain is available. Do not install, clear data or replace a device build without permission.
- [ ] Compare Flutter and Iced at quarter/half/three-quarter drags in both directions; verify icon overlap, label/shadow fading and unchanged title-strip behavior. Check single-column mode and wraparound.
- [ ] Compare empty, small and overflowing folder popups and every long-press menu step. Verify scrim/Back behavior, no background launches, folder-member long press, text entry/IME and accessibility focus bounds.
- [ ] Obtain a whole-change review and fix regressions. Clearly distinguish automated verification from visual checks actually performed on a device.
