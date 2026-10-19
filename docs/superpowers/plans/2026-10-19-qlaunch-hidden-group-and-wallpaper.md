# Hidden Group + Wallpaper Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface the existing, undeletable `Hidden` group in the Groups editor, and add a "Choose wallpaper" button to Settings that opens Android's wallpaper chooser and shows the chosen wallpaper behind the launcher.

**Architecture:** The `Hidden` group and its undeletability already exist in the core (`GroupRole::Hidden`; `delete_group`/`set_group_role` refuse it; `pages()` excludes it). Feature A is UI-only: list it as a locked row. Feature B follows the existing `uninstall_app` precedent — Kotlin `QLaunchBridge` gains `chooseWallpaper()`/`wallpaperPng()`; Rust exposes them via `JniHost` under `cfg(target_os = "android")`; Dart fetches the bytes into `LauncherState` and `BackgroundLayer` draws them; an `ACTION_WALLPAPER_CHANGED` receiver in `MainActivity` emits `CoreEvent::WallpaperChanged` so Dart re-fetches.

**Tech Stack:** Flutter 3.47.7 / Dart 3.13.5, Rust 1.98.1, flutter_rust_bridge 2.13.0, Kotlin Android host.

**Spec:** `docs/superpowers/specs/2026-10-18-qlaunch-flutter-rust-design.md` (§8.4 groups/roles + Hidden; §7.7 wallpaper bytes; §8.2 Settings; §8.6)

## Global Constraints

- Keep `SCHEMA_VERSION = 1`. No model/schema change in this plan.
- Generated bridge code: after any `api.rs` change, run `flutter_rust_bridge_codegen generate` (repo root) then `cargo build --release --manifest-path rust/qlaunch_core/Cargo.toml`, or `test/core_smoke_test.dart` fails its hash check.
- Rust stays platform-independent except the existing `cfg(target_os = "android")` `JniHost` paths; no new platform calls outside that.
- ~~Wallpaper is drawn in Flutter from PNG bytes (§7.7); do not make the window translucent.~~ **Superseded:** on device, `WallpaperManager.getDrawable()` throws `SecurityException` on Android 14+, so the wallpaper is shown through a transparent window (`windowShowWallpaper` + `BackgroundMode.transparent`) instead. See the ledger Task 7b and §7.7 of the design.
- `git commit` is unavailable (harness policy): record `commits: none`.
- Android build: `JAVA_HOME=/usr/lib/jvm/java-27 flutter build apk --debug`.
- After any install, re-assert home: `adb shell cmd package set-home-activity --user 0 bitshift.qlaunch/.MainActivity`.

## Review Focus

- **Hidden row is not a page.** It must not get up/down arrows (moving it is `NotAPage`), and it must never enter `pageIds` or the pager.
- **Deleting Hidden is impossible from the UI.** The edit dialog's Delete must be disabled for it, and the core must still refuse it.
- **A wallpaper that cannot be decoded.** `wallpaperPng()` returning null, or bytes that fail to decode, must fall back to the base colour — never take the screen down.
- **Wallpaper bytes are large.** They are fetched once into `LauncherState`, not on every rebuild, and not put in the model snapshot.
- **Chooser absent.** A device with no `ACTION_SET_WALLPAPER` handler must not crash; the failure is surfaced as `error`, never a throw.

---

### Task 1: List the Hidden group in the Groups editor

**Files:**
- Modify: `lib/src/state/launcher_state.dart`
- Modify: `lib/src/ui/group_editor_screen.dart`
- Test: `test/group_editor_screen_test.dart`

**Interfaces:**
- Produces: `LauncherState.hiddenGroup` — `Group?`, the group holding `GroupRole.hidden`, or null.
- Consumes: existing `GroupEditorScreen.rowKey(id)`, `_GroupEditDialog` (`_locked` already disables Delete and role chips for any non-Home role).

- [ ] **Step 1: Write the failing tests** in `test/group_editor_screen_test.dart`. Add a `hidden` group to the `setUp` snapshot's `groups` (role `GroupRole.hidden`, order 999) — do **not** add it to `pageIds`. Add:

```dart
testWidgets('lists the Hidden group', (tester) async {
  await pumpEditor(tester);
  expect(find.byKey(GroupEditorScreen.rowKey('hidden')), findsOneWidget);
});

testWidgets('the Hidden group cannot be reordered', (tester) async {
  await pumpEditor(tester);
  expect(find.byKey(GroupEditorScreen.upKey('hidden')), findsNothing);
  expect(find.byKey(GroupEditorScreen.downKey('hidden')), findsNothing);
});

testWidgets('the Hidden group cannot be deleted', (tester) async {
  await pumpEditor(tester);
  await tester.tap(find.byKey(GroupEditorScreen.rowKey('hidden')));
  await tester.pumpAndSettle();
  expect(
    tester
        .widget<TextButton>(find.byKey(GroupEditorScreen.editDeleteKey))
        .onPressed,
    isNull,
  );
});
```

- [ ] **Step 2: Run to verify they fail**

Run: `flutter test test/group_editor_screen_test.dart`
Expected: FAIL — no `hidden` row.

- [ ] **Step 3: Implement.**

In `launcher_state.dart`, beside `appsGroup`:

```dart
/// The group holding the Hidden role, or `null` if none does. Hidden apps
/// live here and it is never a page.
Group? get hiddenGroup {
  for (final g in groups) {
    if (g.role == GroupRole.hidden) return g;
  }
  return null;
}
```

In `group_editor_screen.dart`, replace the `ListView.builder` body with a `ListView` whose children are the page rows (built exactly as today, keeping the arrows and keys), followed — when `state.hiddenGroup != null` — by that group's row **without** arrows. Extract the current page row into a `_pageRow(context, group, index, pages.length)` helper so the same `ListTile`/keys are reused; the hidden row is its own `ListTile` with `key: rowKey(hidden.id)`, `subtitle: Text(_roleName(GroupRole.hidden))`, and `onTap: () => _edit(context, hidden)`.

- [ ] **Step 4: Run to verify they pass**

Run: `flutter test test/group_editor_screen_test.dart`
Expected: PASS.

- [ ] **Step 5: Full checks**

Run: `flutter analyze lib test` then `flutter test`
Expected: clean (ignoring vendored `rust_builder`); all pass.

- [ ] **Step 6: Record in the ledger**, `commits: none`.

---

### Task 2: Kotlin bridge — choose and read the wallpaper

**Files:**
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt`

**Interfaces:**
- Produces: `QLaunchBridge.chooseWallpaper()` (`()` → void) and `QLaunchBridge.wallpaperPng()` (`()` → `byte[]?`), found by Rust as static methods.

- [ ] **Step 1: Implement `chooseWallpaper()`** — start the system chooser:

```kotlin
@JvmStatic
fun chooseWallpaper() {
    val context = getAppContext()
    val intent = Intent(Intent.ACTION_SET_WALLPAPER)
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    context.startActivity(Intent.createChooser(intent, "Choose wallpaper")
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
}
```

- [ ] **Step 2: Implement `wallpaperPng()`** — `WallpaperManager.getInstance(context).drawable`, drawn to a PNG via the existing `drawableToBitmap`; return null on any failure. Add `import android.app.WallpaperManager`.

- [ ] **Step 3: Build to check it compiles**

Run: `JAVA_HOME=/usr/lib/jvm/java-27 flutter build apk --debug`
Expected: builds (Rust still has no caller, which is fine at this step).

- [ ] **Step 4: Record in the ledger**, `commits: none`.

---

### Task 3: Rust exposes wallpaper, and the change event

**Files:**
- Modify: `rust/qlaunch_core/src/platform/android/jni_host.rs`
- Modify: `rust/qlaunch_core/src/api/api.rs`

**Interfaces:**
- Produces: `JniHost::choose_wallpaper() -> Result<(), PlatformError>`, `JniHost::wallpaper_png() -> Result<Option<Vec<u8>>, PlatformError>`; bridge fns `choose_wallpaper() -> Result<(), String>` and `wallpaper_png() -> Option<Vec<u8>>`; JNI export `Java_bitshift_qlaunch_MainActivity_onWallpaperChanged` emitting `CoreEvent::WallpaperChanged`.

- [ ] **Step 1: `jni_host.rs`.** Add a `JniHost::choose_wallpaper` calling `QLaunchBridge.chooseWallpaper()` (`()V`) and `JniHost::wallpaper_png` calling `QLaunchBridge.wallpaperPng()` (`()[B`, null→None), mirroring `uninstall`/`icon_png`. Add the export:

```rust
#[unsafe(no_mangle)]
pub extern "system" fn Java_bitshift_qlaunch_MainActivity_onWallpaperChanged(
    _env: JNIEnv,
    _this: jni::objects::JObject,
) {
    crate::api::state::emit(CoreEvent::WallpaperChanged);
}
```

- [ ] **Step 2: `api.rs`.** Add, mirroring `uninstall_app`'s `cfg` split:

```rust
/// Opens the system wallpaper chooser. Only Android can; elsewhere this is an
/// error the settings screen reports like any other.
pub async fn choose_wallpaper() -> Result<(), String> { /* android: JniHost::choose_wallpaper; else Err */ }

/// The current wallpaper as PNG bytes, or `None` if there is none, it cannot be
/// read, or the platform has no wallpaper.
pub async fn wallpaper_png() -> Option<Vec<u8>> { /* android: JniHost::wallpaper_png().ok().flatten(); else None */ }
```

- [ ] **Step 3: Check the core still builds for the host**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml`
Expected: PASS (the new fns are unused by host tests but must compile; wrap JNI-only code in `#[cfg(target_os = "android")]` so the host build is unaffected).

- [ ] **Step 4: Record in the ledger**, `commits: none`.

---

### Task 4: Register the wallpaper-change receiver

**Files:**
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt`

**Interfaces:**
- Consumes: `onWallpaperChanged()` external (Task 3).

- [ ] **Step 1: Implement.** In `onCreate`, register a runtime receiver for `Intent.ACTION_WALLPAPER_CHANGED` (an anonymous `BroadcastReceiver` calling `onWallpaperChanged()`), with `Context.RECEIVER_NOT_EXPORTED`; unregister it in `onDestroy` alongside `packageReceiver`. Add `private external fun onWallpaperChanged()`.

- [ ] **Step 2: Build**

Run: `JAVA_HOME=/usr/lib/jvm/java-27 flutter build apk --debug`
Expected: builds.

- [ ] **Step 3: Record in the ledger**, `commits: none`.

---

### Task 5: Regenerate the bridge

**Files:**
- Generated: `lib/src/rust/api/api.dart`, `lib/src/rust/frb_generated*.dart` (+ any `.freezed`).

- [ ] **Step 1:** Run `flutter_rust_bridge_codegen generate` at the repo root.
- [ ] **Step 2:** Run `cargo build --release --manifest-path rust/qlaunch_core/Cargo.toml`.
- [ ] **Step 3:** Run `flutter test test/core_smoke_test.dart` → PASS (hash matches).

- [ ] **Step 4: Record in the ledger**, `commits: none`.

---

### Task 6: Dart backend and state for the wallpaper

**Files:**
- Modify: `lib/src/state/launcher_backend.dart`
- Modify: `lib/src/state/launcher_state.dart`
- Modify: `test/fake_backend.dart`
- Test: `test/launcher_state_test.dart`

**Interfaces:**
- Produces: `LauncherBackend.wallpaperPng() -> Future<Uint8List?>`, `LauncherBackend.chooseWallpaper() -> Future<void>`; `LauncherState.wallpaper` (`Uint8List?`), `LauncherState.chooseWallpaper() -> Future<void>`.
- Consumes: generated `rust.wallpaperPng()`, `rust.chooseWallpaper()`.

- [ ] **Step 1: Write the failing tests** in `test/launcher_state_test.dart`:

```dart
test('load fetches the wallpaper bytes', () async { /* fake.wallpaperPngResult = [..]; await state.load(); expect(state.wallpaper, [..]); */ });
test('chooseWallpaper reaches the backend', () async { /* await state.chooseWallpaper(); expect(fake.calls, contains('chooseWallpaper')); */ });
test('WallpaperChanged re-fetches and notifies', () async { /* load with null; set fake.wallpaperPngResult; emit wallpaperChanged; pump; expect(state.wallpaper, [..]) */ });
```

- [ ] **Step 2: Run to verify they fail.**

Run: `flutter test test/launcher_state_test.dart`
Expected: FAIL — no `wallpaper`/`chooseWallpaper`.

- [ ] **Step 3: Implement.** Add the two methods to `LauncherBackend`/`RustBackend`. In `FakeBackend`, add `Uint8List? wallpaperPngResult;` and record `chooseWallpaper` in `calls`. In `LauncherState`: a `_wallpaper` field + getter; fetch it in `load()` after `init` succeeds and in a `_refreshWallpaper()`; handle `CoreEvent_WallpaperChanged()` by calling `_refreshWallpaper()` (replacing the `break`); add `chooseWallpaper()` that awaits the backend and reports a failure through `_fail` (never throws).

- [ ] **Step 4: Run to verify they pass**

Run: `flutter test test/launcher_state_test.dart`
Expected: PASS.

- [ ] **Step 5: Full checks** — `flutter analyze lib test` then `flutter test`.

- [ ] **Step 6: Record in the ledger**, `commits: none`.

---

### Task 7: Settings button and showing the wallpaper

**Files:**
- Modify: `lib/src/ui/settings_screen.dart`
- Modify: `lib/src/ui/home_screen.dart`
- Test: `test/settings_screen_test.dart`, `test/home_screen_test.dart`

**Interfaces:**
- Consumes: `LauncherState.chooseWallpaper()`, `LauncherState.wallpaper`, `BackgroundLayer(wallpaper:)`.

- [ ] **Step 1: Write the failing tests.**

In `settings_screen_test.dart`: a "Choose wallpaper" row exists (`SettingsScreen.chooseWallpaperKey`) and tapping it calls the backend (`fake.calls` contains `chooseWallpaper`).
In `home_screen_test.dart`: with `fake.wallpaperPngResult` set, the home screen draws `find.byKey(BackgroundLayer.wallpaperKey)`; with it null, `BackgroundLayer.baseKey`.

- [ ] **Step 2: Run to verify they fail.**

Run: `flutter test test/settings_screen_test.dart test/home_screen_test.dart`
Expected: FAIL.

- [ ] **Step 3: Implement.** Add `static const chooseWallpaperKey = ValueKey('setting.chooseWallpaper');` to `SettingsScreen` and a `ListTile` under the Window header (near wallpaper dimming) calling `unawaited(widget.state.chooseWallpaper())`. In `home_screen.dart`, pass `wallpaper: widget.state.wallpaper` to `BackgroundLayer` (update the stale comment about F2).

- [ ] **Step 4: Run to verify they pass.**

- [ ] **Step 5: Full checks** — `flutter analyze lib test` then `flutter test`.

- [ ] **Step 6: Record in the ledger**, `commits: none`.

---

### Task 8: Device verification

**Files:** none (record evidence in the ledger).

- [ ] **Step 1:** Build and install; re-assert home.
- [ ] **Step 2:** Verify the Hidden group: Settings → Groups shows a Hidden row; tapping it shows Delete disabled; its apps are not on any page.
- [ ] **Step 3:** Verify the wallpaper: Settings → Choose wallpaper opens the system chooser; after picking, the launcher shows it; a failed/absent chooser surfaces an error, not a crash.
- [ ] **Step 4:** Record commands and observations in the ledger, `commits: none`. If `wm size` is used, reset it in the same command chain.

### Task 9 (added): Hide any group with a switch

**Files:** `rust/qlaunch_core/src/model/{mod,mutate}.rs`, `api.rs`, `lib/src/state/{launcher_backend,launcher_state}.dart`, `test/fake_backend.dart`, `lib/src/ui/group_editor_screen.dart`; tests in `rust/qlaunch_core/tests/{model_mutations,store}.rs`, `test/group_editor_screen_test.dart`.

- `Group` gains `hidden: bool` (`#[serde(default)]`, schema stays 1). `is_page` excludes hidden groups. `Model::set_group_hidden(id, hidden)` refuses the root and the `Hidden`-role group (`NotAPage`). API + bridge regenerated + Dart plumbing.
- The Groups editor lists every group (all but the root), each with a `Switch` that calls `setGroupHidden`; the `Hidden`-role group's switch is on and disabled; arrows only for a page.
- TDD: Rust mutation tests, the on-disk key set, a default-on-load test, and the editor switch tests.

## Self-Review

**Spec coverage:** §8.4 Hidden (already in core) is surfaced by Task 1; §8.2/§8.6 Settings button by Task 7; §7.7 wallpaper bytes + change listener by Tasks 2–4, 6, 7; codegen by Task 5; device by Task 8.
**Type consistency:** `chooseWallpaper`/`wallpaperPng` names are identical across Kotlin, `JniHost`, `api.rs`, the backend, and the state; `wallpaper` is `Uint8List?` end to end.
**Proportion:** short relative to the code; only bodies the signatures do not determine are shown.
