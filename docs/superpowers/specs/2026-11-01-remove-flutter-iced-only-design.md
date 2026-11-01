# Remove Flutter and make `dev/iced` a pure-iced project — Design

Date: 2026-11-01
Branch: `dev/iced`
Status: proposed — approach B chosen by the maintainer; awaiting approval of this document

## Intent and why

The iced UI has progressed far enough to replace Flutter. The maintainer wants
`dev/iced` to stop being a comparison spike and become a pure-iced project:
all Dart/Flutter sources, tooling, configuration and prose are removed, the
shared Rust core is decoupled from the Flutter bridge, and the remaining repo
is renamed so nothing is left carrying an "iced spike" qualifier.

`main` still holds the Flutter implementation, so nothing here is destructive to
the project's history: this branch becomes iced-only, and its commits are
recoverable.

## Goals

- The repository builds and runs with **no Flutter/Dart anywhere**: no Dart
  sources, no Flutter project files, no `flutter_rust_bridge`, no Flutter Gradle
  or CMake integration.
- `qlaunch_core` no longer knows about Flutter at all (no `flutter` feature, no
  `frb_generated`, no `frb(...)` annotations, no `StreamSink`).
- Names convey "the launcher", not "the iced spike": the app crate, native
  library, build/profile scripts, log tag and version are renamed.
- Documentation describes iced as *the* launcher, not a spike measured against
  Flutter. Flutter-era research and planning docs are removed.
- Behaviour is unchanged: this is a removal and rename, not a port or redesign.

## Non-goals

- Do not port, complete or change any launcher feature.
- Do not restructure the repository layout beyond the renames below (no moving
  `rust/` to the root, no renaming `qlaunch_core`).
- Do not touch `main`, `java`, or any other branch.
- Do not change the Android application id (`bitshift.qlaunch`), the saved
  configuration schema, or the JNI/Kotlin OS adapter's behaviour.

## Anti-goals (would make this a failure even if it "compiles")

- Breaking the on-device Android build or desktop run.
- Losing behaviour that currently works, or leaving a `flutter`/`dart`/`frb`
  reference in a build path that would silently re-enable a Flutter dependency.
- Leaving stale names in docs that point at files that no longer exist.

## Constraints

- The Kotlin/JNI OS adapter and iced_android are **kept**. Android is already
  built without the Flutter Gradle plugin; it starts `IcedActivity` and the
  native library directly.
- The iced stack stays pinned: `https://github.com/Santuzius/iced` and
  `https://github.com/Santuzius/iced_android`, tag `iced_android-v0.1.1`,
  including the `android-activity` `[patch.crates-io]`.
- `Cargo.lock` is committed and must be regenerated cleanly after the
  dependency changes.
- The Rust core's public API (function names, types, event model) stays as is,
  apart from deleting the Flutter-only `events()` stream function.

## Approaches considered

All three delete the same Flutter code; they differ in how much repo identity
changes.

- **A — remove in place, keep all names.** Lowest churn, but leaves
  `qlaunch_iced`, `build-iced.sh`, `libqlaunch_iced.so`, `-iced-spike` version
  and `docs/iced-spike.md` behind, still implying a comparison build.
- **B — remove + de-spike the names (chosen).** Clean identity for a pure-iced
  branch. Touches the working native glue and path references, which is the main
  risk, so each rename is verified by an actual build (or, where the Android
  toolchain is unavailable, by compilation of the renamed lib and a grep proof).
- **C — remove + a fresh minimal doc set.** Rejected: throws away device
  findings and technical notes that are still accurate and useful.

## The how (decided)

### 1. Delete the Flutter layer

Remove these tracked paths/files entirely:

- `lib/` (Dart UI, 48 files) — including `lib/src/rust/frb_generated.dart` and
  the generated API/model bindings.
- `test/` (Dart widget/unit tests, 40 files), `integration_test/` (7),
  `test_driver/` (2), and the fixtures under `test/` they use.
- `pubspec.yaml`, `pubspec.lock`, `analysis_options.yaml`, `.metadata`,
  `flutter_rust_bridge.yaml`, `.flutter-plugins-dependencies`.
- `rust_builder/` (flutter_rust_bridge `cargokit`, 40 files).
- `linux/` (Flutter Linux runner and CMake, 11 files).
- `rust/qlaunch_core/src/frb_generated.rs` (3862 generated lines).
- Android Flutter template leftovers that are provably unused:
  - `android/app/src/main/res/values/styles.xml` and
    `android/app/src/main/res/values-night/styles.xml` (`LaunchTheme` /
    `NormalTheme`; only the manifest's `@style/IcedTheme` is used, and these
    two styles are referenced nowhere else).
  - `android/app/src/main/res/drawable/launch_background.xml` and
    `drawable-v21/launch_background.xml` (only referenced by `LaunchTheme`).
  - `android/app/src/debug/AndroidManifest.xml` and
    `android/app/src/profile/AndroidManifest.xml` (both only grant `INTERNET`
    for the Flutter tool; a launcher does not need it).
- Delete, but not via git (they are gitignored): the on-disk
  `android/app/src/main/java/io/flutter/plugins/GeneratedPluginRegistrant.java`
  and the local `.dart_tool/` directory; and (locally, not committed) the
  `flutter.*` lines in `android/local.properties`. The `build/` directory holds
  stale Flutter+Flutter-Gradle output (about 9.6 GB) and does not need to be
  deleted for the branch to be Flutter-free, but clearing it gives a genuinely
  clean working tree; it is regenerable, so this is an optional local cleanup,
  not a tracked change.

### 2. Decouple `qlaunch_core` from Flutter

- `rust/qlaunch_core/Cargo.toml`:
  - Delete the `flutter_rust_bridge` dependency line.
  - Delete the whole `[features]` block (`default = ["flutter"]` and
    `flutter = [...]`), so the crate has no features.
  - Delete the `[lints.rust] unexpected_cfgs`/`cfg(frb_expand)` check, which
    existed only for `frb`.
- `rust/qlaunch_core/src/lib.rs`: delete the `#[cfg(feature = "flutter")]` /
  `mod frb_generated;` lines.
- `rust/qlaunch_core/src/api/api.rs`:
  - Delete the `#[cfg(feature = "flutter")] use crate::frb_generated::StreamSink;`
    import.
  - Delete the `events(sink: StreamSink<CoreEvent>)` function (Flutter stream
    only).
  - Delete every `#[cfg_attr(feature = "flutter", flutter_rust_bridge::frb(ignore))]`
    line (4 in `api.rs`, plus the one on `EventBus` in `events.rs` and 6 in
    `shortcuts.rs`). The attributes only suppressed bridge generation.
- Reword the remaining `Dart`/`bridge` wording in core comments so the prose
  matches iced (examples: `api/api.rs` "Everything Dart needs to draw the
  launcher.", "Dart never chooses.", the `events`/stream docs; `api/state.rs`
  "Everything the bridge keeps between calls." and "Dart compares snapshot
  revisions"; `platform/android/mod.rs` "the bridge sends to Dart"). This is
  comments only; no signatures change.
- Keep the `bitshift/qlaunch/QLaunchBridge` class name in
  `platform/android/jni_host.rs`: it names the Kotlin JNI bridge class, not the
  Flutter bridge, and renaming it would require touching Kotlin for no benefit.

### 2b. Scrub Flutter/Dart references from surviving source

"Pure iced only" applies to the code that stays, not just the files that go.
Several surviving comments and test identifiers still name Flutter, and some
point at files this change deletes:

- `rust/qlaunch/src/theme.rs` — "Flutter's dark Material 3 colour scheme",
  naming deleted `theme_data.dart` / `_colorSchemeDarkM3`.
- `rust/qlaunch/src/dialog.rs` — "matching Flutter's `Wrap`", "exactly as
  Flutter's `AppSettingsDialog._save`".
- `rust/qlaunch/src/scroll_fade.rs`, `view.rs`, and `Cargo.toml` (the
  blend-space comment "to match Flutter and Android composition").
- Test identifiers: `colors_match_flutter_dark_scheme`,
  `glyph_codepoints_match_flutter`, `rows_match_flutter_order`,
  `fade_and_single_column_travel_match_flutter`.
- `android/app/src/main/kotlin/bitshift/qlaunch/MemoryTrimPolicy.kt`
  ("Flutter treats every level…") and
  `MemoryTrimPolicyTest.kt` (`realPressureAndOtherTrimLevelsStillReachFlutter`,
  "must still reach Flutter").

Reword the comments to describe the behaviour or the Android Material baseline
without naming Flutter, and rename the test functions to drop the `_flutter`
suffix (`colors_match_dark_scheme`, `glyph_codepoints_match`,
`rows_match_order`, `fade_and_single_column_travel_match`, and the Kotlin test
to e.g. `realPressureAndOtherTrimLevelsStillReachTheApp`). This is a
comment-and-identifier sweep; no behaviour, signature or assertion changes.

### 3. Rename the iced app and its artefacts

- `git mv rust/qlaunch_iced rust/qlaunch`.
- `rust/qlaunch/Cargo.toml`: package `name = "qlaunch"`; `[lib]` unchanged;
  `[[bin]] name = "qlaunch"`. The native library therefore becomes
  `libqlaunch.so` (Cargo derives `lib<name>.so`).
- `rust/Cargo.toml`: workspace `members = ["qlaunch_core", "qlaunch"]`.
- `rust/qlaunch/src/main.rs`: `qlaunch::run()`.
- `rust/qlaunch/src/lib.rs`: log tag `with_tag("qlaunch")`; rename the test
  helper `qlaunch_iced_row_height` to `row_height` (test-local).
- `rust/qlaunch/Cargo.toml`: dependency on core can drop
  `default-features = false` (core now has no features), e.g.
  `qlaunch_core = { path = "../qlaunch_core" }`.
- Android native loading:
  - `android/app/src/main/AndroidManifest.xml`: `android.app.lib_name` value
    `qlaunch`.
  - `android/app/src/main/kotlin/bitshift/qlaunch/MainActivity.kt`:
    `System.loadLibrary("qlaunch")`, and the log tag string `qlaunch` (used by
    `logLifecycle`). Reword the "Dart is told so separately" comment.
- `android/app/build.gradle.kts`: `versionName = "1.0.0"`; update the comment
  that names `scripts/build-iced.sh`.
- `git mv scripts/build-iced.sh scripts/build.sh`; header comment becomes
  "arm64 Android build…"; copy `libqlaunch.so`.
- `git mv scripts/profile-iced.sh scripts/profile.sh`; internal `profile-iced:`
  strings become `profile:`; log tag `qlaunch`; header comment de-spiked.
- `run_adb.sh`: call `scripts/build.sh`.
- `run.sh`: comment no longer says "port… for comparison with the Flutter
  build"; run `cargo run -p qlaunch --bin qlaunch`. Note `--no-default-features`
  is no longer needed anywhere once core has no features.
- Update all tracked references to the old names
  (`qlaunch_iced`, `qlaunch-iced`, `build-iced.sh`, `profile-iced.sh`,
  `libqlaunch_iced`, `iced-spike`), excluding the historical docs being deleted
  in step 5.

### 4. Android and repo-config cleanup

- `android/app/src/main/AndroidManifest.xml`: delete the trailing Flutter
  `PROCESS_TEXT` `<queries>` block and its comment.
- `android/gradle.properties`: keep the `android.newDsl` and
  `android.builtInKotlin` flags (AGP 9.4.1 may need them) but remove the
  "added by the Flutter template" comments.
- `android/.gitignore`: remove the `GeneratedPluginRegistrant.java` line and
  reword the Flutter keystore URL comment.
- Root `.gitignore`: delete the Flutter/Dart/Pub section (`.dart_tool/`,
  `.flutter-plugins-dependencies`, `.pub-cache/`, `.pub/`, the symbolication
  and obfuscation entries, `/android/app/debug|profile|release`,
  `.widget_preview/`). Keep the Rust, build-output, IDE and OS entries, and keep
  `/android/app/src/main/jniLibs/`.
- `rust/Cargo.lock`: regenerate after the dependency changes (drop
  `flutter_rust_bridge` and its transitive crates).

### 5. Documentation

Remove:

- `docs/scroll-performance.md`, `docs/cache-device-investigation-2026-10-23.md`,
  `docs/pixel-app-return-investigation-2026-10-23.md` — Flutter-era research.
  Before deleting, carry any still-true findings into `docs/technical-notes.md`.
- The whole historical `docs/superpowers/plans/` and `docs/superpowers/specs/`
  trees (Flutter- and iced-era planning records). Their in-progress value is
  superseded, and they reference file names this change removes. This design
  doc and its plan are the only files kept/created under `docs/superpowers/`.

Rewrite:

- `README.md` — QLaunch is an iced + Rust launcher for Android and Linux; the
  dev/iced status, build/run commands (`scripts/build.sh`, `run.sh`,
  `run_adb.sh`), the docs index, and the branch table (main = Flutter history,
  java = original launcher).
- `project_summary_architecture.md` — the iced architecture and Rust core, tech
  stack, platform scope, no Flutter.
- `implementation_roadmap.md` — the remaining iced work.
- `ai_agent_context_prompt.md` — conventions and constraints for iced-only work.
- `TODO.md` — keep the active lifecycle/blank-screen item; update the
  `qlaunch-iced` log tag and `docs/iced-spike.md` references.
- `docs/iced-spike.md` → **`docs/technical-notes.md`**, rewritten to state the
  current architecture, gestures, build, performance methodology, device
  findings, accessibility status and known gaps, with the "spike vs Flutter"
  framing and Flutter baseline comparisons removed. It remains the deep
  reference; the root docs stay short.

### 6. Where this design lives

Per the project's superpowers convention this design is committed at
`docs/superpowers/specs/2026-11-01-remove-flutter-iced-only-design.md`, and the
plan that follows it at `docs/superpowers/plans/`. Keeping that directory is
deliberate even though its historical contents are deleted.

## Verification

Run from `rust/` unless stated:

- `cargo test --workspace` passes (core without the old `flutter` feature, plus
  iced). The `--no-default-features` flags documented in the old spike doc are
  obsolete and must not appear in any surviving command.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings` passes, or
  any remaining core lints are recorded rather than silently added.
- `cargo tree -p qlaunch | grep -Ei 'flutter|dart'` prints nothing, and the
  lockfile no longer contains `flutter_rust_bridge`.
- A case-insensitive search of all tracked files returns **zero** matches for
  each of: `flutter`, `flutter_rust_bridge`, `frb`, `dart`, `iced-spike`,
  `iced_spike`, `build-iced`, `profile-iced`, `qlaunch_iced`, `qlaunch-iced`,
  `libqlaunch_iced`. There is no allowlist. In particular the dependency
  `iced_android` does not contain any of these tokens and needs no exemption;
  the `/tmp` baseline directory is outside the repo. **My call:** the README's
  branch table describes `main` without using the word "Flutter" (for example,
  "the earlier UI, retained for history"), so the zero-match rule can hold
  literally. Say so if you would rather keep the historical wording and
  allowlist that one mention.
- Android (where the NDK/JDK are available): `bash scripts/build.sh debug`
  builds, and `adb install -r` + launch still works. If the toolchain is not
  available in the working environment, this is reported as **unverified**, not
  assumed.
- `git status` shows deletions and renames as expected, with no stray build
  output added.

## What's left to the builder

- Exact ordering of edits and the commit split (see the plan).
- The precise wording of the rewritten docs, within the goals above.
- Whether any Flutter-era finding (for example an icon or scroll defect) is
  still relevant enough to restate in `docs/technical-notes.md`.
- The device/toolchain verification the environment allows, stated honestly as
  run or not run.
Status: approved (2026-11-01) — all tasks 1–6 completed, branch verified pure-iced.
