# Remove Flutter and make `dev/iced` a pure-iced project — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete every Dart/Flutter file and reference from `dev/iced`, decouple `qlaunch_core` from `flutter_rust_bridge`, and rename the iced app and its artefacts so the branch is a pure-iced launcher.

**Architecture:** No behavioural change. The Rust core stays the source of truth; the iced crate (`qlaunch`) and the Kotlin/JNI OS adapter are untouched apart from names. Work is ordered so the tree builds at every commit: decouple the core, rename the app, scrub references, delete the Flutter layer, then rewrite the docs.

**Tech Stack:** Rust 2024 (`qlaunch_core` + iced app), iced/`iced_android` pinned to the Santuzius `iced_android-v0.1.1` forks, Kotlin/JNI Android adapter, Gradle/AGP 9.4.1, Android NDK 28.2.13676358, JDK 21 (`/opt/jbr-21`).

**Spec:** `docs/superpowers/specs/2026-11-01-remove-flutter-iced-only-design.md`

## Global Constraints

- Branch is `dev/iced`. Do not touch `main`, `java`, `origin/*`, or any other branch.
- Android `applicationId` stays `bitshift.qlaunch`; the saved configuration schema and the JNI/Kotlin adapter's behaviour are unchanged.
- Keep the pinned forks and tags exactly: `iced` and `iced_android` from `https://github.com/Santuzius/*`, tag `iced_android-v0.1.1`, and the `android-activity` `[patch.crates-io]`.
- `rust/Cargo.lock` is committed and is regenerated as part of this work.
- Zero-match rule (case-insensitive, all tracked files): no occurrences of `flutter`, `flutter_rust_bridge`, `frb`, `dart`, `iced-spike`, `iced_spike`, `build-iced`, `profile-iced`, `qlaunch_iced`, `qlaunch-iced`, `libqlaunch_iced`. No allowlist.
- Android builds run with `JAVA_HOME=/opt/jbr-21`; the device is Pixel 5a serial `18161JECB02915`.
- `--no-default-features` must not appear in any surviving command after Task 1 (the core has no features).
- Rename map: `qlaunch_iced`→`qlaunch` (crate, dir, bin); `qlaunch-iced`→`qlaunch` (log tag, bin); `libqlaunch_iced.so`→`libqlaunch.so`; `scripts/build-iced.sh`→`scripts/build.sh`; `scripts/profile-iced.sh`→`scripts/profile.sh`; `docs/iced-spike.md`→`docs/technical-notes.md`; `versionName` `1.0.0-iced-spike`→`1.0.0`.

## Review Focus

- **A surviving build path silently re-enables Flutter/frb** (a stale feature flag, a re-added default): `cargo tree -p qlaunch` must contain no `flutter`/`dart` crate, and the zero-match grep must be empty (Tasks 1, 4, 6).
- **A renamed artefact is split** — the native library, `loadLibrary`, or `lib_name` disagree, so the app crashes on launch: proven by building, installing and launching on the Pixel (Task 2).
- **A deleted Android file is still referenced** (a theme, a manifest, a jniLib a Gradle task expects): proven by an Android build+install after the deletions (Task 4).
- **A documented command no longer works** after the renames (`build-iced.sh`, `--no-default-features`, `run.sh`): the surviving commands are run as written (Task 2, Task 4, Task 6).
- **A doc links to a file this change deletes** (e.g. a link to `docs/iced-spike.md`): a link/reference sweep over the rewritten docs (Task 5).

---

### Task 1: Decouple `qlaunch_core` from Flutter

Removes the bridge entirely so the core is plain Rust. Nothing consumes the `flutter` feature on this branch.

**Files:**
- Modify: `rust/qlaunch_core/Cargo.toml`
- Modify: `rust/qlaunch_core/src/lib.rs`
- Modify: `rust/qlaunch_core/src/api/api.rs`
- Modify: `rust/qlaunch_core/src/api/events.rs`
- Modify: `rust/qlaunch_core/src/api/shortcuts.rs`
- Modify: `rust/qlaunch_core/src/api/state.rs`
- Modify: `rust/qlaunch_core/src/platform/android/mod.rs`
- Delete: `rust/qlaunch_core/src/frb_generated.rs`
- Modify (regenerated): `rust/Cargo.lock`

**Interfaces:**
- Consumes: nothing new.
- Produces: `qlaunch_core` with **no Cargo features**. The public functions and types are unchanged except that `api::events(sink: StreamSink<CoreEvent>) -> Result<(), String>` is deleted.

- [ ] **Step 1: Remove the feature and dependency**

In `rust/qlaunch_core/Cargo.toml`: delete the `flutter_rust_bridge = { ... optional = true }` line, delete the whole `[features]` block (`default = ["flutter"]` and `flutter = [...]`), and delete the `[lints.rust]` block (`unexpected_cfgs` / `cfg(frb_expand)`). Leave the other dependencies, target sections and `[dev-dependencies]` as they are.

- [ ] **Step 2: Remove the generated bridge module**

In `rust/qlaunch_core/src/lib.rs` delete these two lines:
```rust
#[cfg(feature = "flutter")]
mod frb_generated;
```
Then delete the file:
```bash
git rm rust/qlaunch_core/src/frb_generated.rs
```

- [ ] **Step 3: Remove the frb annotations and the Flutter stream**

Delete every line of the form `#[cfg_attr(feature = "flutter", flutter_rust_bridge::frb(ignore))]` in `api/api.rs` (4), `api/events.rs` (1), `api/shortcuts.rs` (6). In `api/api.rs` also delete the `#[cfg(feature = "flutter")] use crate::frb_generated::StreamSink;` import and the whole `events(sink: StreamSink<CoreEvent>)` function (its doc comment included). No other code changes.

- [ ] **Step 4: Reword the Dart/bridge prose**

Comments only. Reword the references to Dart and the "bridge" that describe the removed Flutter link, for example: `api/api.rs` "Everything Dart needs to draw the launcher." / "Dart never chooses." / "Streams the core's events to Dart."; `api/state.rs` "Everything the bridge keeps between calls." / "Dart compares snapshot revisions"; `platform/android/mod.rs` "the bridge sends to Dart." Describe the same behaviour in terms of the core and the iced UI. Do **not** rename `bitshift/qlaunch/QLaunchBridge` in `platform/android/jni_host.rs` — that is the Kotlin JNI class, not the Flutter bridge.

- [ ] **Step 5: Regenerate the lockfile and build**

```bash
cd rust && cargo build --workspace
```
Expected: succeeds, and `grep -c flutter_rust_bridge rust/Cargo.lock` prints `0`.

- [ ] **Step 6: Run the tests**

```bash
cd rust && cargo test --workspace
```
Expected: PASS (the core's tests, including `subscribe_for_test`/`emit_for_test`, are unchanged; the iced suite is unchanged).

- [ ] **Step 7: Verify no Flutter left in the core**

```bash
cd rust && cargo tree -p qlaunch_core | grep -Ei 'flutter|dart|frb' ; echo "exit=$?"
rg -i 'flutter|frb|dart' qlaunch_core/src qlaunch_core/Cargo.toml ; echo "exit=$?"
```
Expected: both greps print nothing.

- [ ] **Step 8: Commit**

```bash
git add rust/qlaunch_core rust/Cargo.lock
git commit -m "refactor(core): drop the flutter_rust_bridge feature and bindings"
```

---

### Task 2: Rename the iced app and its artefacts

De-spikes every name. This is the highest-risk task because the native library name and `loadLibrary` must agree, so it is verified on the device.

**Files:**
- Rename: `rust/qlaunch_iced/` → `rust/qlaunch/`
- Modify: `rust/qlaunch/Cargo.toml`, `rust/qlaunch/src/main.rs`, `rust/qlaunch/src/lib.rs`, `rust/qlaunch/src/accessibility.rs`
- Modify: `rust/Cargo.toml`
- Rename: `scripts/build-iced.sh` → `scripts/build.sh`; `scripts/profile-iced.sh` → `scripts/profile.sh`
- Modify: `run.sh`, `run_adb.sh`
- Modify: `android/app/build.gradle.kts`
- Modify: `android/app/src/main/AndroidManifest.xml`
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt`
- Delete (untracked): `android/app/src/main/jniLibs/arm64-v8a/libqlaunch_iced.so`

**Interfaces:**
- Consumes: the feature-free `qlaunch_core` from Task 1.
- Produces: crate `qlaunch` with binary `qlaunch`, native library `libqlaunch.so`; `scripts/build.sh` and `scripts/profile.sh`.

- [ ] **Step 1: Move the crate and rename it**

```bash
git mv rust/qlaunch_iced rust/qlaunch
```
In `rust/qlaunch/Cargo.toml`: set `name = "qlaunch"` and `[[bin]] name = "qlaunch"` (path stays `src/main.rs`); change the core dependency to `qlaunch_core = { path = "../qlaunch_core" }` (drop `default-features = false`). In `rust/Cargo.toml`: `members = ["qlaunch_core", "qlaunch"]`.

- [ ] **Step 2: Update the Rust references to the old name**

- `rust/qlaunch/src/main.rs`: `qlaunch::run()`.
- `rust/qlaunch/src/lib.rs`: change `.with_tag("qlaunch-iced")` to `.with_tag("qlaunch")`.
- `rust/qlaunch/src/accessibility.rs`: rename the test helper `qlaunch_iced_row_height` to `row_height` (definition and its call sites; it is test-local).

- [ ] **Step 3: Rename the scripts and update their contents**

```bash
git mv scripts/build-iced.sh scripts/build.sh
git mv scripts/profile-iced.sh scripts/profile.sh
```
In `scripts/build.sh`: header comment becomes "arm64 Android build. Run with bash scripts/build.sh [debug|release]."; `cargo build ... -p qlaunch_iced` becomes `-p qlaunch`; the copied file becomes `libqlaunch.so` (both the source path and the destination under `android/app/src/main/jniLibs/arm64-v8a/`); drop `--no-default-features` from the `cargo metadata` call.
In `scripts/profile.sh`: header comment drops "qlaunch-iced"; every `profile-iced:` echo prefix becomes `profile:`; `logcat -d -s qlaunch-iced` becomes `-s qlaunch`.

- [ ] **Step 4: Update the Android and runner references**

- `android/app/build.gradle.kts`: `versionName = "1.0.0"`; the comment "Supplied by scripts/build-iced.sh…" becomes "scripts/build.sh".
- `android/app/src/main/AndroidManifest.xml`: `android.app.lib_name` value becomes `qlaunch`.
- `android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt`: `System.loadLibrary("qlaunch_iced")` → `System.loadLibrary("qlaunch")`; the log tag string `"qlaunch-iced"` (in `logLifecycle`) → `"qlaunch"`, and the comment that names it and says "Dart is told so separately" is reworded to name iced/the UI.
- `run_adb.sh`: `bash "$ROOT/scripts/build.sh" "$MODE"`.
- `run.sh`: drop the "port… for comparison with the Flutter build" comment; run `cargo run -p qlaunch --bin qlaunch`.

- [ ] **Step 5: Remove the stale native library and build**

```bash
rm -f android/app/src/main/jniLibs/arm64-v8a/libqlaunch_iced.so
cd rust && cargo test -p qlaunch
```
Expected: PASS.

- [ ] **Step 6: Verify the old names are gone from code and scripts**

```bash
git grep -nE 'qlaunch_iced|qlaunch-iced|libqlaunch_iced|build-iced|profile-iced' -- rust android scripts run.sh run_adb.sh ; echo "exit=$?"
```
Expected: prints nothing (historical docs are excluded here; they are handled in Task 5).

- [ ] **Step 7: Device verification (renamed library loads)**

```bash
JAVA_HOME=/opt/jbr-21 bash scripts/build.sh release
adb -s 18161JECB02915 install -r build/app/outputs/apk/release/app-release.apk
adb -s 18161JECB02915 shell am start -W -n bitshift.qlaunch/.MainActivity
```
Expected: build succeeds producing `build/app/outputs/apk/release/app-release.apk`; `install -r` succeeds; the launcher opens and shows the saved Home grid. If the library/`loadLibrary` names disagreed, the process would crash on start with `UnsatisfiedLinkError`. If `install -r` fails with a signature mismatch, stop and report it rather than clearing data.

- [ ] **Step 8: Commit**

```bash
git add -A rust scripts run.sh run_adb.sh android/app/build.gradle.kts android/app/src/main/AndroidManifest.xml android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt
git commit -m "refactor: rename qlaunch_iced to qlaunch and de-spike the artefacts"
```

---

### Task 3: Scrub Flutter/Dart references from surviving code

Comment-and-identifier sweep over the code that stays. No behaviour, signature or assertion changes.

**Files:**
- Modify: `rust/qlaunch/src/theme.rs`, `dialog.rs`, `scroll_fade.rs`, `view.rs`, `icons.rs`, `slots.rs`, `screens/advanced.rs`, `Cargo.toml`
- Modify: `android/app/src/main/kotlin/bitshift/qlaunch/MemoryTrimPolicy.kt`
- Modify: `android/app/src/test/kotlin/bitshift/qlaunch/MemoryTrimPolicyTest.kt`

**Interfaces:**
- Consumes: the renamed crate from Task 2.
- Produces: the same public items, with `Flutter` removed from comments and test function names.

- [ ] **Step 1: Reword the comments**

Describe the behaviour or the Android Material baseline without naming Flutter:
- `rust/qlaunch/src/theme.rs:4,11` (the dark Material 3 scheme origin, and remove the reference to the deleted `theme_data.dart` / `_colorSchemeDarkM3`).
- `rust/qlaunch/src/scroll_fade.rs:1` (scroll-edge opacity).
- `rust/qlaunch/src/view.rs:648` (sharp 1 dp shadow).
- `rust/qlaunch/src/dialog.rs:753,900` (chip wrap; rename-then-move flow).
- `rust/qlaunch/Cargo.toml` (the blend-space comment "to match Flutter and Android composition" → "to match Android composition").
- `android/app/src/main/kotlin/bitshift/qlaunch/MemoryTrimPolicy.kt:5` ("Flutter treats every level…").

- [ ] **Step 2: Rename the `_flutter` test functions**

- `rust/qlaunch/src/theme.rs:477` `colors_match_flutter_dark_scheme` → `colors_match_dark_scheme`
- `rust/qlaunch/src/icons.rs:42` `glyph_codepoints_match_flutter` → `glyph_codepoints_match`
- `rust/qlaunch/src/screens/advanced.rs:100` `rows_match_flutter_order` → `rows_match_order`
- `rust/qlaunch/src/slots.rs:800` `fade_and_single_column_travel_match_flutter` → `fade_and_single_column_travel_match`
- `android/app/src/test/kotlin/bitshift/qlaunch/MemoryTrimPolicyTest.kt:14` `realPressureAndOtherTrimLevelsStillReachFlutter` → `realPressureAndOtherTrimLevelsStillReachTheApp`; line 26 message "must still reach Flutter" → "must still reach the app".

Do not change what any test asserts.

- [ ] **Step 3: Run the Rust tests**

```bash
cd rust && cargo test -p qlaunch
```
Expected: PASS, with the renamed tests listed (e.g. `colors_match_dark_scheme`).

- [ ] **Step 4: Run the Kotlin unit tests**

```bash
JAVA_HOME=/opt/jbr-21 ./android/gradlew -p android testDebugUnitTest
```
Expected: PASS, including `realPressureAndOtherTrimLevelsStillReachTheApp`.

- [ ] **Step 5: Verify the surviving code is clean**

```bash
rg -i 'flutter|\bdart\b' rust/qlaunch android/app/src ; echo "exit=$?"
```
Expected: prints nothing.

- [ ] **Step 6: Commit**

```bash
git add rust/qlaunch android/app/src
git commit -m "docs: remove Flutter wording from the iced app and Android adapter"
```

---

### Task 4: Delete the Dart/Flutter layer and Android template leftovers

The bulk deletion. Doing it here (after the renames) keeps earlier commits small and green.

**Files:**
- Delete: `lib/`, `test/`, `integration_test/`, `test_driver/`, `rust_builder/`, `linux/`
- Delete: `pubspec.yaml`, `pubspec.lock`, `analysis_options.yaml`, `.metadata`, `flutter_rust_bridge.yaml`
- Delete: `android/app/src/main/res/values/styles.xml`, `android/app/src/main/res/values-night/styles.xml`, `android/app/src/main/res/drawable/launch_background.xml`, `android/app/src/main/res/drawable-v21/launch_background.xml`
- Delete: `android/app/src/debug/AndroidManifest.xml`, `android/app/src/profile/AndroidManifest.xml`
- Modify: `android/app/src/main/AndroidManifest.xml`, `android/gradle.properties`, `android/.gitignore`, `.gitignore`
- Delete (untracked): `.flutter-plugins-dependencies`, `android/app/src/main/java/io/flutter/plugins/GeneratedPluginRegistrant.java`, `.dart_tool/`

**Interfaces:**
- Consumes: the renamed, decoupled Rust from Tasks 1–3.
- Produces: a tree with no Flutter project files; the Android app still builds with only the Kotlin/JNI adapter and the iced `libqlaunch.so`.

- [ ] **Step 1: Delete the Dart/Flutter sources and project files**

```bash
git rm -r lib test integration_test test_driver rust_builder linux
git rm pubspec.yaml pubspec.lock analysis_options.yaml .metadata flutter_rust_bridge.yaml
rm -rf .flutter-plugins-dependencies .dart_tool
rm -f android/app/src/main/java/io/flutter/plugins/GeneratedPluginRegistrant.java
```

- [ ] **Step 2: Delete the unused Android Flutter template resources**

`LaunchTheme`/`NormalTheme` and `launch_background` are referenced only by each other; the manifest uses `@style/IcedTheme`. The debug/profile manifests only grant `INTERNET` for the Flutter tool.

```bash
git rm android/app/src/main/res/values/styles.xml \
       android/app/src/main/res/values-night/styles.xml \
       android/app/src/main/res/drawable/launch_background.xml \
       android/app/src/main/res/drawable-v21/launch_background.xml \
       android/app/src/debug/AndroidManifest.xml \
       android/app/src/profile/AndroidManifest.xml
```

- [ ] **Step 3: Remove the remaining Flutter config**

- `android/app/src/main/AndroidManifest.xml`: delete the trailing `PROCESS_TEXT` `<queries>` block and its comment.
- `android/gradle.properties`: keep the `android.newDsl` and `android.builtInKotlin` flags, delete the "added by the Flutter template" comments.
- `android/.gitignore`: remove the `GeneratedPluginRegistrant.java` line; reword the keystore comment that links to flutter.dev.
- `.gitignore` (root): delete the Flutter/Dart/Pub section (`.dart_tool/`, `.flutter-plugins-dependencies`, `.pub-cache/`, `.pub/`, the symbolication and obfuscation entries, `/android/app/debug|profile|release`, `.widget_preview/`). Keep the Rust and build entries, the IDE/OS entries, and `/android/app/src/main/jniLibs/`.

- [ ] **Step 4: Verify nothing references the deleted files, then test**

```bash
git grep -nE 'launch_background|LaunchTheme|NormalTheme|GeneratedPluginRegistrant|rust_builder' -- rust android scripts run.sh run_adb.sh ; echo "exit=$?"
cd rust && cargo test --workspace
```
Expected: grep prints nothing; tests PASS.

- [ ] **Step 5: Verify the Rust graph is Flutter-free**

```bash
cd rust && cargo tree -p qlaunch | grep -Ei 'flutter|dart' ; echo "exit=$?"
grep -c flutter_rust_bridge rust/Cargo.lock
```
Expected: grep prints nothing; count `0`.

- [ ] **Step 6: Device verification (deletions did not break the build)**

```bash
JAVA_HOME=/opt/jbr-21 bash scripts/build.sh release
adb -s 18161JECB02915 install -r build/app/outputs/apk/release/app-release.apk
adb -s 18161JECB02915 shell am start -W -n bitshift.qlaunch/.MainActivity
```
Expected: build succeeds (proves no surviving reference to the deleted themes/manifests), install succeeds, launcher opens with the Home grid.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: delete the Dart/Flutter sources, tooling and Android template leftovers"
```

---

### Task 5: Rewrite the documentation

Replace the spike framing and delete the Flutter-era records. The spec leaves the wording to the builder; the constraints below are what matter.

**Files:**
- Delete: `docs/scroll-performance.md`, `docs/cache-device-investigation-2026-10-23.md`, `docs/pixel-app-return-investigation-2026-10-23.md`
- Delete: the historical `docs/superpowers/plans/*` and `docs/superpowers/specs/*`, **except** the two files for this change
- Rename + rewrite: `docs/iced-spike.md` → `docs/technical-notes.md`
- Rewrite: `README.md`, `project_summary_architecture.md`, `implementation_roadmap.md`, `ai_agent_context_prompt.md`, `TODO.md`

**Interfaces:**
- Consumes: the final file names and commands from Tasks 1–4 (`qlaunch`, `scripts/build.sh`, `run.sh`, `run_adb.sh`, `docs/technical-notes.md`).
- Produces: docs that describe the iced launcher with no Flutter references and no links to deleted files.

- [ ] **Step 1: Carry forward still-true findings, then delete the Flutter-era docs**

Read `docs/scroll-performance.md`, `docs/cache-device-investigation-2026-10-23.md` and `docs/pixel-app-return-investigation-2026-10-23.md`; move any finding that is still true of the iced app (e.g. a defect or device behaviour) into `docs/technical-notes.md`, then delete them. Then delete the historical planning records, keeping the two files for this change:
```bash
git ls-files 'docs/superpowers/plans/*' 'docs/superpowers/specs/*' \
  | grep -v '2026-11-01-remove-flutter-iced-only' \
  | xargs git rm
git rm docs/scroll-performance.md docs/cache-device-investigation-2026-10-23.md docs/pixel-app-return-investigation-2026-10-23.md
```

- [ ] **Step 2: Turn the spike doc into the technical reference**

```bash
git mv docs/iced-spike.md docs/technical-notes.md
```
Rewrite it: keep the current architecture, gestures/transitions, build, performance methodology, tile-rendering and per-frame-work findings, device results, accessibility status and known gaps. Remove the "spike vs Flutter" framing and the Flutter baseline comparisons. Every command must use the new names (`cargo run -p qlaunch --bin qlaunch`, `scripts/build.sh`, `docs/technical-notes.md`) and must not contain `--no-default-features`.

- [ ] **Step 3: Rewrite the root documents**

- `README.md`: QLaunch is an iced + Rust launcher for Android and Linux; dev/iced status; build/run using `scripts/build.sh`, `run.sh`, `run_adb.sh`; the docs index (link `docs/technical-notes.md` and the other docs); a branch table describing `main` as the earlier UI retained for history **without the word "Flutter"**, and `java` as the original launcher.
- `project_summary_architecture.md`: iced architecture, Rust core, tech stack, platform scope, no Flutter.
- `implementation_roadmap.md`: the remaining iced work.
- `ai_agent_context_prompt.md`: conventions/constraints for iced-only work.
- `TODO.md`: keep the active blank/frozen-Home lifecycle item and its evidence; update the `qlaunch-iced` log tag to `qlaunch` and the `docs/iced-spike.md` reference to `docs/technical-notes.md`.

- [ ] **Step 4: Verify the docs are clean and the links resolve**

```bash
git grep -nEi 'flutter|\bdart\b|frb|iced-spike|build-iced|profile-iced|qlaunch_iced|qlaunch-iced' -- '*.md' ; echo "exit=$?"
git grep -nE 'docs/(iced-spike|scroll-performance|cache-device|pixel-app-return)' -- '*.md' ; echo "exit=$?"
```
Expected: both print nothing.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "docs: describe QLaunch as a pure-iced project and drop the Flutter records"
```

---

### Task 6: Whole-branch verification

No new behaviour; prove the spec's verification section end to end and report honestly.

**Files:**
- Modify: `docs/superpowers/specs/2026-11-01-remove-flutter-iced-only-design.md` (status line)

- [ ] **Step 1: Formatting, lints and the full suite**

```bash
cd rust && cargo fmt --all -- --check
cargo clippy --workspace --all-targets --no-deps -- -D warnings
cargo test --workspace
cargo test -p qlaunch -- --ignored
```
Expected: fmt clean; clippy passes (if the core's pre-existing `module_inception`/`collapsible_if` lints remain, record them explicitly rather than suppressing them); the full suite passes; the ignored GPU render test passes when a wgpu adapter is present, otherwise is reported as skipped.

- [ ] **Step 2: Prove the graph and the tree are Flutter-free**

```bash
cd rust && cargo tree -p qlaunch | grep -Ei 'flutter|dart|frb' ; echo "exit=$?"
cd .. && grep -c flutter_rust_bridge rust/Cargo.lock
git grep -nEi 'flutter|flutter_rust_bridge|frb|\bdart\b|iced-spike|iced_spike|build-iced|profile-iced|qlaunch_iced|qlaunch-iced|libqlaunch_iced' ; echo "exit=$?"
```
Expected: the tree grep and the repo-wide grep print nothing; the lockfile count is `0`.

- [ ] **Step 3: Run the documented commands exactly as written**

```bash
bash scripts/build.sh debug
# then, on the device:
adb -s 18161JECB02915 install -r build/app/outputs/apk/debug/app-debug.apk
adb -s 18161JECB02915 shell am start -W -n bitshift.qlaunch/.MainActivity
adb -s 18161JECB02915 logcat -d -s qlaunch | tail -20
```
Expected: debug build succeeds; install and launch succeed; `logcat` shows `qlaunch`-tagged lines. Also confirm `run.sh` still resolves the crate by starting the desktop binary under a timeout — `timeout 20s bash run.sh || test $? -eq 124` — and report that it started (timeout exit 124) rather than erroring immediately.

- [ ] **Step 4: Rebuild and install the release**

```bash
JAVA_HOME=/opt/jbr-21 bash scripts/build.sh release
adb -s 18161JECB02915 install -r build/app/outputs/apk/release/app-release.apk
adb -s 18161JECB02915 shell am start -W -n bitshift.qlaunch/.MainActivity
```
Expected: release builds, installs and launches with the saved layout intact.

- [ ] **Step 5: Sanity-check the working tree**

```bash
git status --short
git diff --stat
```
Expected: only the intended changes; no build output or `/tmp` artefacts added.

- [ ] **Step 6: Mark the design approved and commit any final fixups**

Set the design doc's status line to "approved". Commit any fixups the verification found; otherwise no commit is needed.

```bash
git add -A docs/superpowers/specs/2026-11-01-remove-flutter-iced-only-design.md
git commit -m "docs: mark the Flutter-removal design approved" || true
```
