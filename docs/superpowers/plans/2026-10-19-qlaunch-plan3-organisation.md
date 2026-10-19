# QLaunch Plan 3 — Persistence, App Settings and the Group Editor

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the launcher's changes actually persist on Android and refresh when apps come and go, and add the two screens the user asked for: a long-press popup on an app that renames, moves and uninstalls it, and a Groups editor reachable from Settings that adds, renames, reorders, deletes and fills the tabs.

**Architecture:** Rust owns every rule and every byte of data; `QLaunchBridge.kt` only answers OS questions and performs OS actions; Dart draws what `snapshot()` returns and sends mutations back. This plan adds an Android configuration directory (via `Context.filesDir`) so the store writes a real file, a package-change broadcast receiver that asks the core to rescan, one new OS action (`uninstall`), and three Dart screens built entirely on the mutation functions the core already exposes.

**Tech Stack:** Rust (`qlaunch_core`, flutter_rust_bridge 2.13, `jni` crate), Kotlin/Android, Flutter/Dart, `flutter_test`.

**Spec:** `docs/superpowers/specs/2026-10-18-qlaunch-flutter-rust-design.md` (§8.1 screens, §8.2 Home, §8.4 app/group rules, roles/pages rules near lines 435/453/477).

## Global Constraints

- **Committing is deferred to the user.** Do NOT run `git commit` or `git push` in any task; leave every change in the working tree and record the task in the progress ledger (executing-plans creates it) with `commits: none`.
- Rust owns data and rules; Kotlin only answers questions and performs OS actions; Dart never recomputes an order or a role rule. Any list the core returns is already in display order — do not sort it again in Dart.
- Positional parameters belong to the `LauncherBackend` wrapper only; the generated bindings take named parameters.
- An app lives in **exactly one** group (§8.4). Moving an app never leaves a copy behind.
- Adding or changing any public Rust function in `crate::api` requires regenerating the Dart bindings with `flutter_rust_bridge_codegen generate` (run from the repo root) and committing the regenerated files into the change set. Functions marked `#[frb(ignore)]` and private functions do not.
- Every on-device claim must be preceded by `flutter build apk` and `adb install -r build/app/outputs/flutter-apk/app-debug.apk` — a stale install previously produced a false reading. Restore the default home app after device tests: `adb shell cmd package set-home-activity com.teslacoilsw.launcher/.NovaLauncher`.
- The device must be unlocked before `adb shell screencap` (it returns empty frames while locked).
- Build environment: `JAVA_HOME=/usr/lib/jvm/java-27` must be set for Gradle; `flutter build apk` runs from the repo root and writes `build/app/outputs/flutter-apk/app-debug.apk`.
- The Java reference in `git show origin/java:app/src/main/java/bitshift/qlaunch/…` is a guide, not a spec. Its custom-icon picker is dead code and is **out of scope**; its per-app "category" is not a model field and is **out of scope**.
- Do not add comments the surrounding file lacks; match each file's existing style.

## Review Focus

Each line is a behaviour a user would expect that the spec does not pin down; the named task's tests cover it.

1. **Android persistence really persists** — after renaming an app, killing QLaunch and relaunching, the new label is still there (Task 1).
2. **A change while running is noticed without a restart** — installing or disabling an app updates the grid (Task 2).
3. **A blank label is refused** — trimming to empty must not reach `rename_item`; Save is disabled (Task 5).
4. **Ticking an app already in the group, or double-moving, does not duplicate** — moving an item to its current group is a no-op (Tasks 5, 9).
5. **Unticking in the Apps group cannot orphan an app** — the checkbox is disabled there, because there is nowhere for the app to go (Task 9).

---

### Task 1: Write to a real configuration directory on Android

The store finds its root with `store::config_root()` (Linux XDG). On Android that resolves to nothing, so `init` runs with `config_root: None` and only a warning: nothing is ever saved. Provide the Android root from `Context.filesDir` and teach `init` to use it.

**Files:**
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt`
- Modify: `rust/qlaunch_core/src/platform/android/jni_host.rs`
- Modify: `rust/qlaunch_core/src/api/api.rs:69-83`
- Test: `rust/qlaunch_core/src/api/api.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: the existing `store::config_root() -> Result<PathBuf, StoreError>`; `JniHost::with_env` and `JniHost::class` (private, same module).
- Produces: `fn platform_config_root() -> Result<PathBuf, String>` and `fn choose_root(override_path: Option<PathBuf>, default: Result<PathBuf, String>) -> (Option<PathBuf>, Vec<String>)`, both private to `api.rs`. `JniHost::files_dir() -> Result<PathBuf, PlatformError>` and Kotlin `QLaunchBridge.filesDirPath() -> String`.

- [ ] **Step 1: Write the failing tests**

Add to `rust/qlaunch_core/src/api/api.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::choose_root;
    use std::path::PathBuf;

    #[test]
    fn choose_root_prefers_the_override() {
        let (root, warnings) =
            choose_root(Some(PathBuf::from("/tmp/x")), Err("no home".into()));
        assert_eq!(root, Some(PathBuf::from("/tmp/x")));
        assert!(warnings.is_empty());
    }

    #[test]
    fn choose_root_uses_the_default_when_there_is_no_override() {
        let (root, warnings) = choose_root(None, Ok(PathBuf::from("/tmp/y")));
        assert_eq!(root, Some(PathBuf::from("/tmp/y")));
        assert!(warnings.is_empty());
    }

    #[test]
    fn choose_root_warns_when_there_is_no_directory() {
        let (root, warnings) = choose_root(None, Err("no home".into()));
        assert_eq!(root, None);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("no home"));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml choose_root`
Expected: FAIL — `cannot find function choose_root in this scope`.

- [ ] **Step 3: Implement `choose_root` and `platform_config_root`, and use them in `init`**

In `api.rs`, add the two helpers and replace the body of `init` (lines 69-83) so it reads:

```rust
fn choose_root(
    override_path: Option<PathBuf>,
    default: Result<PathBuf, String>,
) -> (Option<PathBuf>, Vec<String>) {
    match override_path {
        Some(path) => (Some(path), Vec::new()),
        None => match default {
            Ok(path) => (Some(path), Vec::new()),
            Err(e) => (
                None,
                vec![format!(
                    "The configuration directory could not be found ({e})."
                )],
            ),
        },
    }
}

#[cfg(target_os = "android")]
fn platform_config_root() -> Result<PathBuf, String> {
    JniHost::files_dir().map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
fn platform_config_root() -> Result<PathBuf, String> {
    store::config_root().map_err(|e| e.to_string())
}
```

and `init`:

```rust
pub async fn init(config_root_override: Option<String>) -> Result<ModelSnapshot, String> {
    let (root, warnings) = choose_root(
        config_root_override.map(PathBuf::from),
        platform_config_root(),
    );
    state::install(root, platform_source(), warnings)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml choose_root`
Expected: PASS (3 tests). Then run the whole suite: `cargo test --manifest-path rust/qlaunch_core` — the existing `init_creates_the_config_root_and_the_first_run_model` must still pass.

- [ ] **Step 5: Add `JniHost::files_dir` and the Kotlin accessor**

In `rust/qlaunch_core/src/platform/android/jni_host.rs`, add `use std::path::PathBuf;` and, inside the existing `impl JniHost` block:

```rust
pub fn files_dir() -> Result<PathBuf, PlatformError> {
    let path = JniHost::with_env(|env| {
        let class = JniHost::class()?;
        match env.call_static_method(class, "filesDirPath", "()Ljava/lang/String;", &[]) {
            Ok(value) => {
                let object = value.l().map_err(|e| {
                    clear_pending(env);
                    jni_error(e)
                })?;
                let java_str: jni::objects::JString = object.into();
                env.get_string(&java_str).map_err(jni_error).map(Into::into)
            }
            Err(e) => {
                clear_pending(env);
                Err(jni_error(e))
            }
        }
    })?;
    Ok(path)
}
```

In `QLaunchBridge.kt` add, next to `getAppContext`:

```kotlin
@JvmStatic
fun filesDirPath(): String = getAppContext().filesDir.absolutePath
```

- [ ] **Step 6: Verify on the device that something is now saved**

Run: `JAVA_HOME=/usr/lib/jvm/java-27 flutter build apk && adb install -r build/app/outputs/flutter-apk/app-debug.apk`, open the launcher, then:
- `adb shell run-as bitshift.qlaunch ls files/` — expect `qlaunch.json` (alongside Flutter's `profileInstalled`).
- Read it back: `adb shell run-as bitshift.qlaunch cat files/qlaunch.json` — expect the model JSON.
- Long-press an app and rename it (Step 5 of Task 5 makes this possible; until then, use any other path that mutates, or note that this check is done at Task 5's verification). Force-stop (`adb shell am force-stop bitshift.qlaunch`) and reopen: the change is still there.
Then restore the home app: `adb shell cmd package set-home-activity com.teslacoilsw.launcher/.NovaLauncher`.

- [ ] **Step 7: Record the task in the ledger (no commit)**

Mark Task 1 done in the progress ledger; `commits: none`.

---

### Task 2: Refresh when apps are installed, removed or changed

Nothing today notices an app appearing or vanishing until the next `init`. Add a manifest broadcast receiver that asks the core to rescan and emit `AppsChanged`; Dart already refetches `snapshot()` on that event, so no Dart change is needed.

**Files:**
- Create: `android/app/src/main/kotlin/bitshift/qlaunch/PackageChangeReceiver.kt`
- Modify: `android/app/src/main/AndroidManifest.xml`
- Modify: `rust/qlaunch_core/src/platform/android/jni_host.rs`

**Interfaces:**
- Consumes: `crate::api::state::refresh_apps_now() -> Result<ModelSnapshot, String>` (already `pub(crate)`, rescans, persists and emits `CoreEvent::AppsChanged`); `QLaunchBridge.install(Context)`.
- Produces: JNI symbol `Java_bitshift_qlaunch_PackageChangeReceiver_onPackagesChanged`, following the same pattern as `Java_bitshift_qlaunch_MainActivity_onHomePressed`.

- [ ] **Step 1: Add the Rust handler**

Append to `rust/qlaunch_core/src/platform/android/jni_host.rs`, mirroring `Java_bitshift_qlaunch_MainActivity_onHomePressed`:

```rust
/// The OS telling us an app was installed, removed or changed. Refresh now;
/// Dart refetches its snapshot when the `AppsChanged` event arrives.
#[unsafe(no_mangle)]
pub extern "system" fn Java_bitshift_qlaunch_PackageChangeReceiver_onPackagesChanged(
    _env: JNIEnv,
    _this: jni::objects::JObject,
) {
    let _ = crate::api::state::refresh_apps_now();
}
```

There is no host test for this symbol: it is JNI glue, verified on the device in Step 4. The logic it calls (`refresh_apps_now`) is already covered by the Rust suite.

- [ ] **Step 2: Add the Kotlin receiver**

Create `android/app/src/main/kotlin/bitshift/qlaunch/PackageChangeReceiver.kt`:

```kotlin
package bitshift.qlaunch

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

class PackageChangeReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        // A package change can start this process on its own, before
        // MainActivity has ever run, so load the library and install the
        // bridge here too. Both are no-ops when the launcher is already up.
        runCatching {
            System.loadLibrary("qlaunch_core")
            QLaunchBridge.install(context.applicationContext)
            onPackagesChanged()
        }
    }

    private external fun onPackagesChanged()
}
```

- [ ] **Step 3: Register the receiver**

In `android/app/src/main/AndroidManifest.xml`, inside `<application>` (after the `</activity>` element, before the `flutterEmbedding` meta-data), add:

```xml
<receiver
    android:name=".PackageChangeReceiver"
    android:exported="false">
    <intent-filter>
        <action android:name="android.intent.action.PACKAGE_ADDED"/>
        <action android:name="android.intent.action.PACKAGE_REMOVED"/>
        <action android:name="android.intent.action.PACKAGE_REPLACED"/>
        <action android:name="android.intent.action.PACKAGE_CHANGED"/>
        <data android:scheme="package"/>
    </intent-filter>
</receiver>
```

- [ ] **Step 4: Verify on the device**

Run: `JAVA_HOME=/usr/lib/jvm/java-27 flutter build apk && adb install -r build/app/outputs/flutter-apk/app-debug.apk`.
With the launcher open, force a package change without installing anything — `adb shell pm disable-user --user 0 <some.third.party.package>` (or `adb install` a spare APK), then watch the grid change **without** restarting the launcher. Re-enable/reinstall to restore. Confirm in logcat that no exception was thrown. Restore the home app.

- [ ] **Step 5: Record the task in the ledger (no commit)**

`commits: none`.

---

### Task 3: Uninstall an app (Rust action, Kotlin action, Dart backend)

The App Settings popup offers Uninstall, as the Java reference did (`ACTION_DELETE`, which shows the system confirmation). This is the only new public bridge function, so it needs a codegen run.

**Files:**
- Modify: `rust/qlaunch_core/src/api/api.rs`
- Modify: `rust/qlaunch_core/src/platform/android/jni_host.rs`
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/QLaunchBridge.kt`
- Test: `rust/qlaunch_core/tests/api.rs`
- Regenerate: `lib/src/rust/**` (via `flutter_rust_bridge_codegen generate`)
- Modify: `lib/src/state/launcher_backend.dart`
- Modify: `test/fake_backend.dart`

**Interfaces:**
- Produces: `pub async fn uninstall_app(package_name: String) -> Result<(), String>` (bridge surface, becomes Dart `rust.uninstallApp({required String packageName})`); `LauncherBackend.uninstallApp(String package) -> Future<void>`; `JniHost::uninstall(package: &str) -> Result<(), PlatformError>`; Kotlin `QLaunchBridge.uninstall(packageName: String)`.
- Consumers: Task 5 (`AppSettingsDialog`), Task 4 (state wrapper).

- [ ] **Step 1: Write the failing test**

Append to `rust/qlaunch_core/tests/api.rs` (it already has `block_on`):

```rust
#[cfg(not(target_os = "android"))]
#[test]
fn uninstall_app_is_unsupported_off_android() {
    let result = block_on(api::uninstall_app("com.example".into()));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Android"));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml uninstall_app`
Expected: FAIL — `cannot find function uninstall_app`.

- [ ] **Step 3: Implement the Rust function**

In `api.rs`:

```rust
/// Asks the system to uninstall `package_name`. On Android this opens the
/// system confirmation; the app list is refreshed by the package-change
/// receiver, not here.
pub async fn uninstall_app(package_name: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        JniHost::uninstall(&package_name).map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = package_name;
        Err("uninstalling an app is only available on Android".into())
    }
}
```

In `jni_host.rs`, inside `impl JniHost`:

```rust
pub fn uninstall(package: &str) -> Result<(), PlatformError> {
    JniHost::with_env(|env| {
        let class = JniHost::class()?;
        let pkg = env.new_string(package).map_err(jni_error)?;
        match env.call_static_method(
            class,
            "uninstall",
            "(Ljava/lang/String;)V",
            &[(&pkg).into()],
        ) {
            Ok(_) => Ok(()),
            Err(e) => {
                clear_pending(env);
                Err(jni_error(e))
            }
        }
    })
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path rust/qlaunch_core/Cargo.toml uninstall_app`
Expected: PASS. Then `cargo test --manifest-path rust/qlaunch_core` — all green.

- [ ] **Step 5: Add the Kotlin action**

In `QLaunchBridge.kt`, add `import android.net.Uri` and:

```kotlin
/// Opens the system's uninstall confirmation for `packageName`.
@JvmStatic
fun uninstall(packageName: String) {
    val context = getAppContext()
    val intent = Intent(Intent.ACTION_DELETE, Uri.parse("package:$packageName"))
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    context.startActivity(intent)
}
```

- [ ] **Step 6: Regenerate the Dart bindings and update the wrapper and fake**

Run: `flutter_rust_bridge_codegen generate` (repo root). `lib/src/rust/api/api.dart` now has `uninstallApp`.

In `lib/src/state/launcher_backend.dart`, add to `LauncherBackend`:

```dart
Future<void> uninstallApp(String package);
```

and to `RustBackend`:

```dart
@override
Future<void> uninstallApp(String package) =>
    rust.uninstallApp(packageName: package);
```

In `test/fake_backend.dart` add a `_plain` call (uninstall has no snapshot):

```dart
@override
Future<void> uninstallApp(String package) async {
  await _plain('uninstallApp($package)', null);
}
```

- [ ] **Step 7: Verify no Dart regressions**

Run: `flutter analyze lib test` and `flutter test` — all existing tests still pass (nothing new uses the method yet).

- [ ] **Step 8: Device check and ledger (no commit)**

Build, install, and (once Task 5 exists) confirm the system uninstall dialog appears. Restore the home app. Record the task, `commits: none`.

---

### Task 4: Mutation wrappers and page/Apps lookups on `LauncherState`

The screens never touch the backend directly; they call `LauncherState`. Add the wrappers the App Settings popup and the Group Editor need, plus a `pages` getter and an `appsGroup` lookup.

**Files:**
- Modify: `lib/src/state/launcher_state.dart`
- Test: `test/launcher_state_test.dart`

**Interfaces:**
- Consumes: `LauncherBackend` methods `renameItem`, `moveItem`, `uninstallApp`, `addGroup`, `renameGroup`, `moveGroup`, `deleteGroup`, `setGroupRole`, and the existing private `_fetch`/`_fail`.
- Produces (relied on by Tasks 5-10):
  - `Future<void> renameItem(String id, String label)`
  - `Future<void> moveItem(String id, String toGroup, int newOrder)`
  - `Future<void> uninstallApp(String package)`
  - `Future<void> addGroup(String label)`
  - `Future<void> renameGroup(String id, String label)`
  - `Future<void> moveGroup(String id, int newOrder)`
  - `Future<void> deleteGroup(String id)`
  - `Future<void> setGroupRole(String id, GroupRole? role)`
  - `Group? get appsGroup`
  - `List<Group> get pages`

- [ ] **Step 1: Write the failing tests**

Append to `test/launcher_state_test.dart` (it already imports `roles.dart`, `fake_backend.dart`, `snapshot_builders.dart`):

```dart
group('mutations', () {
  test('renameItem forwards the label and adopts the snapshot', () async {
    fake.current = snapshotOf(groups: [makeGroup('apps')]);
    await state.load();

    await state.renameItem('i1', 'Firefox');

    expect(fake.calls, contains('renameItem(i1,Firefox)'));
  });

  test('moveItem forwards the destination and order', () async {
    fake.current = snapshotOf(groups: [makeGroup('apps')]);
    await state.load();

    await state.moveItem('i1', 'games', 3);

    expect(fake.calls, contains('moveItem(i1,games,3)'));
  });

  test('appsGroup is the group holding the Apps role', () async {
    fake.current = snapshotOf(
      groups: [
        makeGroup('apps', role: GroupRole.apps),
        makeGroup('games', role: GroupRole.games),
      ],
    );
    await state.load();

    expect(state.appsGroup?.id, 'apps');
  });

  test('pages are the page groups in page order', () async {
    fake.current = snapshotOf(
      pageIds: ['home', 'apps'],
      groups: [makeGroup('apps'), makeGroup('home')],
    );
    await state.load();

    expect(state.pages.map((g) => g.id), ['home', 'apps']);
  });

  test('uninstallApp reports a failure rather than throwing', () async {
    await state.load();
    fake.failure = StateError('gone');

    await state.uninstallApp('com.example');

    expect(state.error, contains('gone'));
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `flutter test test/launcher_state_test.dart`
Expected: FAIL — `The method 'renameItem' isn't defined`.

- [ ] **Step 3: Implement the wrappers**

In `launcher_state.dart` add, next to `updateSettings` (snapshot-returning calls all go through `_fetch`, which is why the mutation results are adopted automatically):

```dart
Future<void> renameItem(String id, String label) =>
    _fetch(() => _backend.renameItem(id, label));

Future<void> moveItem(String id, String toGroup, int newOrder) =>
    _fetch(() => _backend.moveItem(id, toGroup, newOrder));

Future<void> addGroup(String label) => _fetch(() => _backend.addGroup(label));

Future<void> renameGroup(String id, String label) =>
    _fetch(() => _backend.renameGroup(id, label));

Future<void> moveGroup(String id, int newOrder) =>
    _fetch(() => _backend.moveGroup(id, newOrder));

Future<void> deleteGroup(String id) => _fetch(() => _backend.deleteGroup(id));

Future<void> setGroupRole(String id, GroupRole? role) =>
    _fetch(() => _backend.setGroupRole(id, role));
```

`uninstallApp` returns nothing, so it follows `launchApp`:

```dart
/// Asks the system to uninstall an app. The core's own list updates when the
/// package-change receiver fires; a failure is reported through [error].
Future<void> uninstallApp(String package) async {
  try {
    await _backend.uninstallApp(package);
  } catch (e, st) {
    _fail(e, st);
  }
}
```

And the two lookups (place beside `homeGroup`):

```dart
/// The group holding the Apps role, or `null` if none does. Every app has a
/// home here when it is unticked from another group.
Group? get appsGroup {
  for (final g in groups) {
    if (g.role == GroupRole.apps) return g;
  }
  return null;
}

/// The page groups in page order. `pageIds` is the order the core computed.
List<Group> get pages {
  final s = _snapshot;
  if (s == null) return const [];
  final byId = {for (final g in s.groups) g.id: g};
  return [
    for (final id in s.pageIds)
      if (byId[id] != null) byId[id]!,
  ];
}
```

Add `import 'package:qlaunch/src/rust/model/roles.dart';` if `GroupRole` is not already in scope (it comes in via `model.dart`'s re-export check — add the import only if the analyzer asks).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `flutter test test/launcher_state_test.dart`
Expected: PASS. Then `flutter analyze lib test`.

- [ ] **Step 5: Record the task in the ledger (no commit)**

`commits: none`.

---

### Task 5: The App Settings popup

Long-pressing an app opens a dialog to rename it, move it to another group, or uninstall it, with a little read-only info. The Save button is disabled on a blank label (Review Focus 3); a move to the item's current group is skipped (Review Focus 4).

**Files:**
- Create: `lib/src/ui/app_settings_dialog.dart`
- Test: `test/app_settings_dialog_test.dart`

**Interfaces:**
- Consumes: `LauncherState.renameItem`, `moveItem`, `uninstallApp`, `itemsIn`, `groups`; `Item`, `Group`, `GroupRole`.
- Produces: `AppSettingsDialog({required LauncherState state, required Item item})` with static keys `labelFieldKey`, `saveKey`, `cancelKey`, `uninstallKey`, `groupKey`, and `static Key groupOptionKey(String groupId)`.

- [ ] **Step 1: Write the failing tests**

Create `test/app_settings_dialog_test.dart`, modelled on `test/app_search_test.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qlaunch/src/rust/model.dart';
import 'package:qlaunch/src/rust/model/roles.dart';
import 'package:qlaunch/src/state/launcher_state.dart';
import 'package:qlaunch/src/ui/app_settings_dialog.dart';

import 'fake_backend.dart';
import 'snapshot_builders.dart';

void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() {
    fake = FakeBackend(
      snapshot: snapshotOf(
        rootGroupId: 'root',
        pageIds: ['apps', 'games'],
        groups: [
          Group(id: 'root', label: 'root', order: 0),
          makeGroup('apps', role: GroupRole.apps),
          makeGroup('games', role: GroupRole.games),
        ],
        items: [
          makeItem('i1', 'apps', app: AppRef(package: 'com.x', activity: 'Main')),
        ],
      ),
    );
    state = LauncherState(fake);
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pumpDialog(WidgetTester tester) async {
    await state.load();
    final item = state.itemsIn('apps').single;
    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: AppSettingsDialog(state: state, item: item))),
    );
    await tester.pump();
  }

  testWidgets('saving a new label renames the item', (tester) async {
    await pumpDialog(tester);

    await tester.enterText(find.byKey(AppSettingsDialog.labelFieldKey), 'Firefox');
    await tester.tap(find.byKey(AppSettingsDialog.saveKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('renameItem(i1,Firefox)'));
  });

  testWidgets('save is disabled when the label is blank', (tester) async {
    await pumpDialog(tester);

    await tester.enterText(find.byKey(AppSettingsDialog.labelFieldKey), '   ');
    await tester.pump();

    final save = tester.widget<FilledButton>(find.byKey(AppSettingsDialog.saveKey));
    expect(save.onPressed, isNull);
  });

  testWidgets('choosing another group moves the item to the end', (tester) async {
    await pumpDialog(tester);

    await tester.tap(find.byKey(AppSettingsDialog.groupKey));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(AppSettingsDialog.groupOptionKey('games')).last);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(AppSettingsDialog.saveKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('moveItem(i1,games,0)'));
  });

  testWidgets('uninstall asks the backend for the package', (tester) async {
    await pumpDialog(tester);

    await tester.tap(find.byKey(AppSettingsDialog.uninstallKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('uninstallApp(com.x)'));
  });

  testWidgets('an item with no app hides the uninstall button', (tester) async {
    fake.current = snapshotOf(
      pageIds: ['apps'],
      groups: [makeGroup('apps', role: GroupRole.apps)],
      items: [makeItem('i1', 'apps')],
    );
    await state.load();
    final item = state.itemsIn('apps').single;
    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: AppSettingsDialog(state: state, item: item))),
    );
    await tester.pump();

    expect(find.byKey(AppSettingsDialog.uninstallKey), findsNothing);
  });
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `flutter test test/app_settings_dialog_test.dart`
Expected: FAIL — `app_settings_dialog.dart` does not exist.

- [ ] **Step 3: Implement `AppSettingsDialog`**

Create `lib/src/ui/app_settings_dialog.dart`. A `StatefulWidget` that returns an `AlertDialog` directly (so tests pump it as a body):

- State: `late final TextEditingController _label` seeded with `widget.item.label`; `late String _groupId = widget.item.groupId`.
- Title: `Text(widget.item.label)`.
- Content, a `Column(mainAxisSize: MainAxisSize.min)`:
  - `TextField(key: labelFieldKey, controller: _label, onChanged: (_) => setState(() {}))` labelled "Name".
  - Read-only info lines: the app's package (`widget.item.app?.package`), and "Uninstalled" when `widget.item.uninstalled` — both only when `app != null`.
  - A `DropdownButtonFormField<String>(key: groupKey, value: _groupId, items: …)` over every group in `widget.state.groups` whose `id != rootGroupId`, each `DropdownMenuItem(value: g.id, key: groupOptionKey(g.id), child: Text(g.label))`.
- Actions:
  - If `widget.item.app != null && !widget.item.uninstalled`: a `TextButton(key: uninstallKey, child: Text('Uninstall'))` that calls `widget.state.uninstallApp(widget.item.app!.package)` then pops.
  - `TextButton(key: cancelKey, child: Text('Cancel'))` → pop.
  - `FilledButton(key: saveKey, onPressed: _label.text.trim().isEmpty ? null : _save, child: Text('Save'))`.
- `_save()`: read `final label = _label.text.trim();`. If `label != widget.item.label`, `await widget.state.renameItem(widget.item.id, label)`. If `_groupId != widget.item.groupId`, `await widget.state.moveItem(widget.item.id, _groupId, widget.state.itemsIn(_groupId).length)` (the length is the end index, and `move_item` clamps). Then pop. All calls awaited before popping.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `flutter test test/app_settings_dialog_test.dart`
Expected: PASS (5 tests). Then `flutter analyze lib test`.

- [ ] **Step 5: Record the task in the ledger (no commit)**

`commits: none`.

---

### Task 6: Open the popup from a long press on an app

Wire the existing `AppGrid.onItemLongPress` (currently unset) to the new dialog.

**Files:**
- Modify: `lib/src/ui/home_screen.dart`
- Test: `test/home_screen_test.dart`

**Interfaces:**
- Consumes: `AppGrid.onItemLongPress` (already `ValueChanged<Item>?`); `AppSettingsDialog` from Task 5.
- Produces: `void _openAppSettings(Item item)` on `_HomeScreenState`.

- [ ] **Step 1: Write the failing test**

Append to the `a long press on blank space` group in `test/home_screen_test.dart`, or as a new group:

```dart
group('a long press on an item', () {
  testWidgets('opens the app settings popup', (tester) async {
    setSurface(tester, const Size(400, 800));
    await load();
    await pumpHome(tester);

    await tester.longPress(find.text('item-home'));
    await tester.pumpAndSettle();

    expect(find.byType(AppSettingsDialog), findsOneWidget);
  });
});
```

Add `import 'package:qlaunch/src/ui/app_settings_dialog.dart';` to the test file.

- [ ] **Step 2: Run the test to verify it fails**

Run: `flutter test test/home_screen_test.dart --plain-name "opens the app settings popup"`
Expected: FAIL — `find.byType(AppSettingsDialog)` finds nothing.

- [ ] **Step 3: Wire the callback**

In `home_screen.dart`, add the import and the method beside `_openSettings`:

```dart
/// Opens the per-app popup, the destination §8.2 gives a long press on an
/// item (rename, move, uninstall).
void _openAppSettings(Item item) {
  unawaited(
    showDialog<void>(
      context: context,
      builder: (context) => AppSettingsDialog(state: widget.state, item: item),
    ),
  );
}
```

and in `build`, pass it to the grid:

```dart
gridFor: (context, groupId) => AppGrid(
  state: widget.state,
  groupId: groupId,
  onItemLongPress: _openAppSettings,
),
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `flutter test test/home_screen_test.dart`
Expected: PASS. The existing `on an item does not open the settings screen` still passes: it asserts only that `SettingsScreen` is absent.

- [ ] **Step 5: Device check**

Build, install. Long-press an app: the popup opens, rename and move work, Uninstall opens the system dialog. Also confirm Task 1's persistence: force-stop, reopen, the rename is still there. Restore the home app.

- [ ] **Step 6: Record the task in the ledger (no commit)**

`commits: none`.

---

### Task 7: The Group Editor screen — list, add and reorder

A screen reached from Settings that lists the tabs in order, lets the user add one, and moves one up or down. Role and rename and delete live in Task 8's dialog; the per-group app checklist in Task 9.

**Files:**
- Create: `lib/src/ui/group_editor_screen.dart`
- Test: `test/group_editor_screen_test.dart`

**Interfaces:**
- Consumes: `LauncherState.pages`, `pageIds`, `addGroup`, `moveGroup` (Task 4).
- Produces: `GroupEditorScreen({required LauncherState state})` with static keys `addKey`, `rowKey(String groupId)`, `upKey(String groupId)`, `downKey(String groupId)`, and `static const title = 'Groups'`. It also hosts the private `_AddGroupDialog` and (Task 8) `_GroupEditDialog`.

- [ ] **Step 1: Write the failing tests**

Create `test/group_editor_screen_test.dart`, modelled on `test/settings_screen_test.dart`:

```dart
void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() {
    fake = FakeBackend(
      snapshot: snapshotOf(
        rootGroupId: 'root',
        pageIds: ['apps', 'games', 'tools'],
        groups: [
          Group(id: 'root', label: 'root', order: 0),
          makeGroup('apps', role: GroupRole.apps, order: 0),
          makeGroup('games', role: GroupRole.games, order: 1),
          makeGroup('tools', order: 2),
        ],
      ),
    );
    state = LauncherState(fake);
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pumpEditor(WidgetTester tester) async {
    await state.load();
    await tester.pumpWidget(MaterialApp(home: GroupEditorScreen(state: state)));
    await tester.pump();
  }

  testWidgets('lists each page in order', (tester) async {
    await pumpEditor(tester);

    expect(find.byKey(GroupEditorScreen.rowKey('apps')), findsOneWidget);
    expect(find.byKey(GroupEditorScreen.rowKey('games')), findsOneWidget);
    expect(find.byKey(GroupEditorScreen.rowKey('tools')), findsOneWidget);
  });

  testWidgets('the first page cannot move up and the last cannot move down',
      (tester) async {
    await pumpEditor(tester);

    expect(
      tester.widget<IconButton>(find.byKey(GroupEditorScreen.upKey('apps'))).onPressed,
      isNull,
    );
    expect(
      tester.widget<IconButton>(find.byKey(GroupEditorScreen.downKey('tools'))).onPressed,
      isNull,
    );
  });

  testWidgets('moving the first page down sends the next index', (tester) async {
    await pumpEditor(tester);

    await tester.tap(find.byKey(GroupEditorScreen.downKey('apps')));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('moveGroup(apps,1)'));
  });

  testWidgets('adding a group sends the typed label', (tester) async {
    await pumpEditor(tester);

    await tester.tap(find.byKey(GroupEditorScreen.addKey));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(GroupEditorScreen.addFieldKey), 'Tools');
    await tester.tap(find.byKey(GroupEditorScreen.addConfirmKey));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('addGroup(Tools)'));
  });
}
```

Imports as in much of the suite: `flutter/material.dart`, `flutter_test`, `qlaunch/src/rust/model.dart`, `model/roles.dart`, `state/launcher_state.dart`, `ui/group_editor_screen.dart`, plus `fake_backend.dart` and `snapshot_builders.dart`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `flutter test test/group_editor_screen_test.dart`
Expected: FAIL — the screen does not exist.

- [ ] **Step 3: Implement the screen and the add dialog**

Create `lib/src/ui/group_editor_screen.dart`. `GroupEditorScreen` is a `StatelessWidget` wrapping its body in `ListenableBuilder(listenable: state, …)`; `Scaffold(appBar: AppBar(title: Text(title), actions: [IconButton(key: addKey, icon: Icon(Icons.add), onPressed: …)]), body: ListView(...))`.

- Body: for each `final group in state.pages`, a `ListTile(key: rowKey(group.id), title: Text(group.label), subtitle: group.role == null ? null : Text(roleName(group.role!)), trailing: Row(mainAxisSize: MainAxisSize.min, children: [up, down]), onTap: …)`. `roleName` maps `GroupRole.apps → 'Apps'`, `games → 'Games'`, `home → 'Home'` (a page never holds `root` or `hidden`).
- Up: `IconButton(key: upKey(group.id), icon: Icon(Icons.arrow_upward), onPressed: index == 0 ? null : () => state.moveGroup(group.id, index - 1))`; down mirrors it with `index == pages.length - 1 ? null : moveGroup(group.id, index + 1)`.
- `onTap` is Task 8's `_edit`; in this task leave it a no-op that opens nothing yet (or wire it in Task 8 — do not merge the two).
- `_AddGroupDialog`: an `AlertDialog` with `TextField(key: addFieldKey)`, a `TextButton(key: addCancelKey, 'Cancel')` and a `FilledButton(key: addConfirmKey, 'Add')` whose `onPressed` is null while the trimmed text is empty; confirming awaits `state.addGroup(label)` then pops. Keys: `addKey`, `addFieldKey`, `addConfirmKey`, `addCancelKey` are `static const Key`s on `GroupEditorScreen`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `flutter test test/group_editor_screen_test.dart`
Expected: PASS. Then `flutter analyze lib test`.

- [ ] **Step 5: Record the task in the ledger (no commit)**

`commits: none`.

---

### Task 8: Rename a tab, change its role, delete it

Tapping a row opens an edit dialog. This is where §8.4's one-holder-per-role rule is surfaced: choosing a role that another group holds moves it (the core does this), and a group that holds `Apps`, `Games` or `Hidden` cannot give it up or be deleted, so its role control and Delete are disabled — as the Java reference did.

**Files:**
- Modify: `lib/src/ui/group_editor_screen.dart`
- Test: `test/group_editor_screen_test.dart`

**Interfaces:**
- Consumes: `LauncherState.renameGroup`, `setGroupRole`, `deleteGroup` (Task 4); `_GroupEditDialog`.
- Produces: `GroupEditorScreen.editDialogKey`, `editLabelFieldKey`, `editSaveKey`, `editDeleteKey`, and `static Key roleKey(GroupRole? role)`.

- [ ] **Step 1: Write the failing tests**

Append to `test/group_editor_screen_test.dart`:

```dart
Future<void> openEdit(WidgetTester tester, String id) async {
  await tester.tap(find.byKey(GroupEditorScreen.rowKey(id)));
  await tester.pumpAndSettle();
}

testWidgets('renaming a tab sends the new label', (tester) async {
  await pumpEditor(tester);
  await openEdit(tester, 'tools');

  await tester.enterText(find.byKey(GroupEditorScreen.editLabelFieldKey), 'Stuff');
  await tester.tap(find.byKey(GroupEditorScreen.editSaveKey));
  await tester.pumpAndSettle();

  expect(fake.calls, contains('renameGroup(tools,Stuff)'));
});

testWidgets('assigning the Home role sends setGroupRole', (tester) async {
  await pumpEditor(tester);
  await openEdit(tester, 'tools');

  await tester.tap(find.byKey(GroupEditorScreen.roleKey(GroupRole.home)));
  await tester.pumpAndSettle();

  expect(fake.calls, contains('setGroupRole(tools,home)'));
});

testWidgets('the role control is disabled for the Apps group', (tester) async {
  await pumpEditor(tester);
  await openEdit(tester, 'apps');

  expect(
    tester.widget<ChoiceChip>(find.byKey(GroupEditorScreen.roleKey(GroupRole.home))).onSelected,
    isNull,
  );
});

testWidgets('deleting an ordinary group asks first', (tester) async {
  await pumpEditor(tester);
  await openEdit(tester, 'tools');

  await tester.tap(find.byKey(GroupEditorScreen.editDeleteKey));
  await tester.pumpAndSettle();
  await tester.tap(find.text('Delete'));
  await tester.pumpAndSettle();

  expect(fake.calls, contains('deleteGroup(tools)'));
});
```

Note the second `find.text('Delete')` targets the confirmation button, which differs from the dialog's own Delete label — give the confirmation button its own key, `GroupEditorScreen.confirmDeleteKey`, and tap that instead to keep the test unambiguous.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `flutter test test/group_editor_screen_test.dart --plain-name "renaming a tab"`
Expected: FAIL — tapping the row opens nothing.

- [ ] **Step 3: Implement `_edit` and `_GroupEditDialog`**

In `group_editor_screen.dart`:

- `onTap: () => showDialog<void>(context: context, builder: (_) => _GroupEditDialog(state: state, group: group))`.
- `_GroupEditDialog` is a `StatefulWidget` returning an `AlertDialog(key: GroupEditorScreen.editDialogKey)`:
  - `TextField(key: editLabelFieldKey, controller: seeded with group.label)`.
  - Role chips: for `[GroupRole.home, GroupRole.apps, GroupRole.games]`, a `ChoiceChip(key: roleKey(role), label: Text(roleName(role)), selected: group.role == role, onSelected: locked ? null : (_) => state.setGroupRole(group.id, role))`, where `locked = group.role != null && group.role != GroupRole.home` (the core refuses to change `Apps`/`Games`; `Hidden` is never a page). Also allow clearing a Home role with a `roleKey(null)` "None" chip enabled only when the group holds Home.
  - Delete: `TextButton(key: editDeleteKey, child: Text('Delete'), onPressed: group.role == null ? _confirmDelete : null)`. `_confirmDelete` shows an `AlertDialog` (title "Delete <label>?", body explaining the apps move to the Apps group) whose confirm button has key `confirmDeleteKey`; confirming awaits `state.deleteGroup(group.id)` and pops both dialogs.
  - `TextButton(key: editCancelKey)`, `FilledButton(key: editSaveKey, onPressed: label trimmed empty ? null : save)`; save awaits `renameGroup` when the label changed and pops.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `flutter test test/group_editor_screen_test.dart`
Expected: PASS. Then `flutter analyze lib test`.

- [ ] **Step 5: Record the task in the ledger (no commit)**

`commits: none`.

---

### Task 9: The per-group app checklist

The user's own design: a list of every app, scrolled and ticked, deciding which apps belong to the group being edited. Ticking an app moves it in; unticking moves it to the Apps group. The Apps group's own list cannot untick (there is nowhere to move the app to), so those checkboxes are disabled (Review Focus 5). Moving to the group an app is already in is never sent (Review Focus 4).

**Files:**
- Create: `lib/src/ui/group_apps_screen.dart`
- Modify: `lib/src/ui/group_editor_screen.dart` (a button in `_GroupEditDialog` opens it)
- Test: `test/group_apps_screen_test.dart`

**Interfaces:**
- Consumes: `LauncherState.itemsIn`, `groups`, `appsGroup`, `moveItem` (Task 4).
- Produces: `GroupAppsScreen({required LauncherState state, required String groupId})` with `static Key tileKey(String itemId)` and `GroupEditorScreen.selectAppsKey`.

- [ ] **Step 1: Write the failing tests**

Create `test/group_apps_screen_test.dart`:

```dart
void main() {
  late FakeBackend fake;
  late LauncherState state;

  setUp(() {
    fake = FakeBackend(
      snapshot: snapshotOf(
        rootGroupId: 'root',
        pageIds: ['apps', 'games'],
        groups: [
          Group(id: 'root', label: 'root', order: 0),
          makeGroup('apps', role: GroupRole.apps, order: 0),
          makeGroup('games', role: GroupRole.games, order: 1),
        ],
        items: [
          makeItem('a1', 'apps', app: AppRef(package: 'p.a1', activity: 'Main')),
          makeItem('g1', 'games', app: AppRef(package: 'p.g1', activity: 'Main')),
          makeItem('dead', 'apps', app: AppRef(package: 'p.dead', activity: 'Main'), uninstalled: true),
          makeItem('cut', 'apps'),
        ],
      ),
    );
    state = LauncherState(fake);
  });

  tearDown(() async {
    state.dispose();
    await fake.close();
  });

  Future<void> pump(WidgetTester tester, String groupId) async {
    await state.load();
    await tester.pumpWidget(
      MaterialApp(home: GroupAppsScreen(state: state, groupId: groupId)),
    );
    await tester.pump();
  }

  testWidgets('apps already in the group are checked', (tester) async {
    await pump(tester, 'games');

    expect(
      tester.widget<CheckboxListTile>(find.byKey(GroupAppsScreen.tileKey('g1'))).value,
      isTrue,
    );
    expect(
      tester.widget<CheckboxListTile>(find.byKey(GroupAppsScreen.tileKey('a1'))).value,
      isFalse,
    );
  });

  testWidgets('ticking an app moves it into the group', (tester) async {
    await pump(tester, 'games');

    await tester.tap(find.byKey(GroupAppsScreen.tileKey('a1')));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('moveItem(a1,games,1)'));
  });

  testWidgets('unticking an app moves it to the Apps group', (tester) async {
    await pump(tester, 'games');

    await tester.tap(find.byKey(GroupAppsScreen.tileKey('g1')));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('moveItem(g1,apps,1)'));
  });

  testWidgets('unticking is disabled in the Apps group', (tester) async {
    await pump(tester, 'apps');

    expect(
      tester.widget<CheckboxListTile>(find.byKey(GroupAppsScreen.tileKey('a1'))).onChanged,
      isNull,
    );
  });

  testWidgets('uninstalled and app-less items are not listed', (tester) async {
    await pump(tester, 'apps');

    expect(find.byKey(GroupAppsScreen.tileKey('dead')), findsNothing);
    expect(find.byKey(GroupAppsScreen.tileKey('cut')), findsNothing);
  });
}
```

The move orders in the assertions are the "end of the destination group" index: `games` holds `g1` (length 1) so a tick from elsewhere sends `1`; `apps` holds `a1`, `dead`, `cut` (length 3) but `dead`/`cut` move too — assert against what the implementation computes and adjust the expected number to the destination's length at the moment of the tap.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `flutter test test/group_apps_screen_test.dart`
Expected: FAIL — the screen does not exist.

- [ ] **Step 3: Implement the screen**

Create `lib/src/ui/group_apps_screen.dart`. `GroupAppsScreen` is a `StatelessWidget` with a `ListenableBuilder` body:

- Title: the label of the group with `id == groupId` (from `state.groups`).
- Rows: every item in `state.snapshot!.items` where `item.app != null && !item.uninstalled`, sorted in Dart by `item.label.toLowerCase()` **for display only** (finding an app by name is the point of the list; the group's own order is untouched until a move).
- Each row: `CheckboxListTile(key: tileKey(item.id), title: Text(item.label), value: item.groupId == groupId, onChanged: handler)`.
- `handler`: skip when the value already equals `item.groupId == groupId`. Ticking → `state.moveItem(item.id, groupId, state.itemsIn(groupId).length)`. Unticking → `final apps = state.appsGroup; if (apps == null) return; await state.moveItem(item.id, apps.id, state.itemsIn(apps.id).length);`. When `groupId == state.appsGroup?.id`, pass `onChanged: null`.

- [ ] **Step 4: Add the entry point in the edit dialog**

In `group_editor_screen.dart`'s `_GroupEditDialog`, add a `ListTile(key: GroupEditorScreen.selectAppsKey, leading: Icon(Icons.checklist), title: Text('Choose apps'), onTap: () => Navigator.of(context).push(MaterialPageRoute(builder: (_) => GroupAppsScreen(state: state, groupId: group.id))))`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `flutter test test/group_apps_screen_test.dart`
Expected: PASS. Then `flutter analyze lib test`.

- [ ] **Step 6: Record the task in the ledger (no commit)**

`commits: none`.

---

### Task 10: Reach the editor from Settings

The user asked for a button in Settings that opens the Group Editor. Add one row.

**Files:**
- Modify: `lib/src/ui/settings_screen.dart`
- Test: `test/settings_screen_test.dart`

**Interfaces:**
- Consumes: `GroupEditorScreen` (Task 7).
- Produces: `SettingsScreen.groupsKey` (`ValueKey('setting.groups')`).

- [ ] **Step 1: Write the failing test**

Append to `test/settings_screen_test.dart` (it has `pumpSettings` and `tapControl` helpers):

```dart
testWidgets('the Groups row opens the group editor', (tester) async {
  await pumpSettings(tester);

  await tapControl(tester, find.byKey(SettingsScreen.groupsKey));

  expect(find.byType(GroupEditorScreen), findsOneWidget);
});
```

Add `import 'package:qlaunch/src/ui/group_editor_screen.dart';` to the test file.

- [ ] **Step 2: Run the test to verify it fails**

Run: `flutter test test/settings_screen_test.dart --plain-name "Groups row"`
Expected: FAIL — no such key.

- [ ] **Step 3: Add the row**

In `settings_screen.dart`, add `static const groupsKey = ValueKey('setting.groups');` beside `restartKey`, and in `build` insert, between the Text section and `_header('Advanced')`:

```dart
_header('Groups'),
ListTile(
  key: SettingsScreen.groupsKey,
  title: const Text('Groups'),
  subtitle: const Text('Add, rename, reorder and fill the tabs.'),
  onTap: () => unawaited(
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => GroupEditorScreen(state: widget.state),
      ),
    ),
  ),
),
```

This row is not a `SettingField`, so the existing "every setting in the table has one" test is unaffected.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `flutter test test/settings_screen_test.dart`
Expected: PASS.

- [ ] **Step 5: Full verification**

Run: `flutter analyze lib test` and `flutter test` — expect zero analyzer issues and the whole suite green (was 172 at the end of Plan 2; this plan adds roughly 20 tests). Then `cargo test --manifest-path rust/qlaunch_core`.

- [ ] **Step 6: Device check**

Build, install. Pick QLaunch as the home app; from the launcher long-press blank space → Settings → Groups: add a tab, rename it, reorder it, open Choose apps and tick two apps in; back out and confirm the new tab shows those apps and the Apps tab no longer does. Force-stop and reopen: everything is still there (Task 1). Restore the home app.

- [ ] **Step 7: Record the task in the ledger (no commit)**

`commits: none`.

---

## Self-Review

**Spec coverage.** §8.1's App Settings and Group Editor screens are Tasks 5, 7-9; §8.2's long-press destinations are Tasks 6 (item) and the existing blank-space settings; §8.4's one-group-per-app rule is enforced server-side by `move_item` and surfaced by Tasks 5 and 9; the role holder rules are surfaced by Task 8; §8.8's "changes are saved" is Task 1. The two Plan 2 leftovers (`getFilesDir`, no package receiver) are Tasks 1 and 2. The custom-icon picker and per-app category from the Java reference are deliberately absent (out of scope, stated in Global Constraints); `Item.iconOverride` still cannot be read (the `AppGrid` TODO(plan-3) remains, but this plan does not add custom icons, so it is not a gap here).

**Type consistency.** `uninstallApp` is the only new bridge name and is spelled the same in Rust, the regenerated Dart, `LauncherBackend`, `RustBackend`, `FakeBackend` and `LauncherState`. `appsGroup`/`pages` are `Group?`/`List<Group>` everywhere. `moveItem`'s third argument is an `int` order in the wrapper, the backend and the fake.

**Review Focus.** Task 1 covers persistence; Task 2 covers on-line refresh; Task 5 covers the blank label and the same-group no-op; Task 9 covers the Apps-group untick and the same-group no-op. Each has a named test.

**Proportion.** The plan carries signatures, keys and tests, not widget bodies; the two dialogs are described by their fields and callbacks rather than transcribed.
