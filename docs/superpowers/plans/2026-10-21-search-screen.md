# Search Screen Implementation Plan

> **For agentic workers:** Use executing-plans to implement inline. User requested immediate implementation, without mockups or another approval round.

**Goal:** Make search a standalone launcher view with matching wallpaper styling, shared history paging, and correct system Home behavior.

**Architecture:** Keep the main pager mounted but offstage while search is visible, preserving its group and scroll state without painting or accepting input. Reuse GroupPager and TabStrip for Frequent/Recent; query results keep the existing search path. Android distinguishes returning to a stopped launcher from Home presses while already in it.

**Tech Stack:** Flutter/Dart, existing Rust usage/search APIs, Kotlin Android lifecycle.

**Spec:** Approved conversation: search hides the main foreground, uses the same wallpaper/dimming and settings; Home from an app returns to the last main tab, then goes Home, then opens/closes search; search launches return to Home with search closed; Frequent/Recent share main paging/animation, each capped at three rows; reopen on Frequent; typing replaces history with results.

## Global Constraints
- Preserve current workspace changes and schema; no commits or branches.
- Keep wallpaper transparent when configured; do not paint a fixed dark search scrim.
- Reuse the existing 150 ms, one-column crossing fade and previous/current/next strip.
- History capacity is three times current orientation's column count; query results are not capped.

## Review Focus
- External app Home versus a second Home already on the launcher: device-check both.
- Background/foreground through Back must not consume the next deliberate Home press.
- Rotation and settings changes must recalculate history capacity.
- Query clearing restores the chosen history page; reopening resets to Frequent.
- Offstage launcher must preserve group/scroll state and never paint or receive search taps.

### Task 1: Main/search visibility and Home lifecycle
Files: `lib/src/ui/home_screen.dart`, `android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt`, `test/home_screen_test.dart`.
- [x] Add failing tests for hidden main view, preserved tab on close, and backgrounding search.
- [x] Run focused tests and observe failures.
- [x] Keep main content Offstage/TickerMode while searching; close search on app pause; filter real MAIN/HOME intents and suppress navigation on external return using stopped/resumed activity state.
- [x] Run widget regressions; validate native behavior on device after rebuilding.

### Task 2: Shared history pager
Files: `lib/src/ui/app_search.dart`, `test/app_search_test.dart`.
- [x] Add failing tests for swipe/tab navigation, three-row limits in both orientations, uncapped query results, query clearing, and reopening on Frequent.
- [x] Run focused tests and observe failures.
- [x] Replace bespoke history buttons with GroupPager/TabStrip; retain query and launch behavior; derive history limit from settings and orientation; remove fixed scrim and hardcoded search text color.
- [x] Run search and shared pager tests.

### Task 3: Verification and delivery
- [x] Run `flutter test` and `flutter analyze lib test`; build profile APK.
- [x] Install and inspect search visibility, wallpaper, history motion and Home sequence; restore original Home selection.
- [x] Record actual verification and limitations in this plan.

## Execution evidence
- Existing workspace retained per the ongoing inline session; no commits or branch changes.
- Root causes: AppSearch painted a fixed translucent black scrim over an active main grid; MainActivity forwarded every new intent as Home, even when returning from another app.
- Task 1: main-view visibility and paused-search tests failed before implementation; all 38 HomeScreen tests passed afterward.
- Task 2: history/pager/cap regressions failed before implementation; all 50 focused search/Home tests passed afterward.
- Full verification: `flutter test` passed 259 tests; `flutter analyze lib test` reported no issues; Android profile build succeeded and was installed.
- Pixel 5a, Android 14: launched Settings from Apps, Home returned to Apps, next Home navigated Home, next opened search, next closed search. Swiped to Recent, typed a query (history disappeared), launched its result and returned to Home with search closed. Reopening selected Frequent. Returning from Calculator with Back did not consume the next Home navigation.
- Visual inspection: search shows configured wallpaper, matching icon/text settings, and no main grid/title strip behind it. Device history contained fewer than three rows; the full cap and rotation were checked with 50-entry widget fixtures.
- Original Pixel Launcher restored and confirmed via `cmd package resolve-activity`.
- Independent scoped code review returned no findings.
- A whole-worktree whitespace check reports pre-existing trailing whitespace in generated `lib/src/rust/model.freezed.dart:186`; search-scope diff check is clean.
- System-bar follow-up: day/night launch and normal themes request transparent status/navigation/divider colors with contrast scrims disabled. Flutter's startup status-bar shade required matching runtime `SystemUiOverlayStyle` plus edge-to-edge mode. Profile APK rebuilt/installed; Pixel 5a screenshots confirm wallpaper behind status icons and three-button navigation, including after returning from settings. Existing theme tests (2) pass; analysis clean. Pixel Launcher default restored; QLaunch left foreground for visual inspection.
