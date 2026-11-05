# Release and publish scripts — design

## Intent

Give qlaunch two small, separate scripts: one that builds versioned desktop and
Android release artifacts into a single folder, and one that publishes those
artifacts as a GitHub release. The Cargo crate version is the single source of
truth for the version, and the version scheme is calendar-based (year.month.day).

## Goals

- `scripts/release.sh [all|android|desktop]` (default `all`) builds the Linux
  desktop binary and the arm64 Android APK into `dist/` at the repo root, named
  with the version.
- `scripts/publish.sh` creates a GitHub release from the contents of `dist/`.
- The version comes only from the `qlaunch` crate in `rust/`.
- Keep the existing `scripts/` bash style; reuse `scripts/build.sh` for Android.

## Non-goals

- Desktop packaging beyond the bare binary (no AppImage, `.deb`, tarball) — for
  now.
- Code signing beyond the committed test-phone keystore already in use.
- CI / GitHub Actions; both scripts run locally.
- Building for ABIs or desktop architectures other than what the machine and
  Gradle already target (Android arm64-v8a; desktop `uname -m`).
- More than one release per day.

## Anti-goals

- A second source of truth for the version. `android/app/build.gradle.kts`
  must stop hardcoding `1.0.0`, and the release scripts must not allow a version
  that disagrees with Cargo.
- A script that publishes implicitly, or that pushes tags without the user
  asking.

## Constraints

- Android release builds already go through `scripts/build.sh release`, which
  sets up the NDK toolchain and calls Gradle.
- GNU bash, `python3`, and `cargo` are already project dependencies.
- `gh` is not installed on this machine; `publish.sh` will check for it and
  fail clearly rather than assume.

## Version scheme

Calendar versioning, `YYYY.MM.DD`, e.g. `2026.11.5`.

- **Cargo requires valid semver, which forbids leading zeros**, so the day is
  not zero-padded: `2026.11.5`, never `2026.11.05`. `2026.11.05` is rejected by
  Cargo outright.
- Defined once at the workspace level: `[workspace.package] version` in
  `rust/Cargo.toml`, with `qlaunch` and `qlaunch_core` both using
  `version.workspace = true`. That keeps both crates in lockstep with no second
  literal.
- Bumped by hand in `rust/Cargo.toml` and committed before `release.sh` runs;
  the script reads the version, it does not stamp the date. (Auto-stamping is a
  possible later change.)
- Tags and filenames use the crate version verbatim (`v2026.11.5`,
  `qlaunch-2026.11.5-...`) so nothing diverges from the truth.
- Android `versionCode` is derived as `major*10000 + minor*100 + patch`, which
  for a `YYYY.MM.DD` version is exactly `YYYYMMDD` (`2026.11.5` → `20261105`):
  monotonic for free.

## Approaches

1. **Single orchestrator + separate publish script (chosen).** `release.sh`
   reads the version, builds both targets, and copies into `dist/`;
   `publish.sh` handles the GitHub release. Small, matches existing scripts.
2. Separate per-target build scripts plus a wrapper — more files for no gain.
3. Rust `cargo xtask` — type-safe but adds a workspace member and diverges from
   the bash scripts already in use.

## Design

### Version helper — `scripts/version.sh`

Prints the `qlaunch` package version by parsing `cargo metadata` (the project
already calls `python3` + `cargo metadata` from `build.sh`). `release.sh`,
`build.sh` and `publish.sh` all use it, so there is exactly one parser.

### `scripts/release.sh [all|android|desktop]`

- Deletes files in `dist/` that are not for the current version, so the folder
  never mixes versions while the two targets can still be built separately.
- `VERSION=$(bash scripts/version.sh)`.
- Desktop: `cargo build --locked --release -p qlaunch --bin qlaunch`, then copy
  `rust/target/release/qlaunch` to
  `dist/qlaunch-$VERSION-linux-$(uname -m)`.
- Android: `bash scripts/build.sh release`, then copy
  `build/app/outputs/apk/release/app-release.apk` to
  `dist/qlaunch-$VERSION-android-arm64.apk`.
- Echo the resulting file list.

### `scripts/build.sh` + `android/app/build.gradle.kts`

So a direct `build.sh release` is also correctly versioned, `build.sh` derives
the version and `versionCode` with `scripts/version.sh` and passes them to
Gradle as `-PqlaunchVersionName` and `-PqlaunchVersionCode`. Gradle reads those
properties (with `0.0.0` / `1` fallbacks) instead of the hardcoded
`1.0.0` / `1`.

*Builder's call:* the Gradle fallbacks become `0.0.0` / `1` so a bare Gradle
invocation is visibly unversioned rather than showing a stale `1.0.0`.

### `scripts/publish.sh [--draft]`

- Fails clearly if `gh` is missing or not authenticated, or if `dist/` is
  empty.
- `VERSION=$(bash scripts/version.sh)`; fails if no
  `dist/qlaunch-$VERSION-*` file is present, so the artifacts can't disagree
  with the tag.
- Ensures tag `v$VERSION` exists: creates it locally at HEAD if missing,
  pushes it if not on the remote.
- `gh release create "v$VERSION" dist/* --title "qlaunch $VERSION"
  --generate-notes`, adding `--draft` when requested.

### Housekeeping

- Change the workspace version and the two crate versions to the CalVer scheme
  (start at the current date, committed as part of this change).
- Add `/dist/` to `.gitignore`.
- Add a short "Releasing" section to `README.md`.

### Left to the builder

- Exact argument validation and error text.
- Whether `publish.sh` should also allow uploading to an existing release
  (`--clobber`); default is a plain create.
- The exact commit that introduces the initial CalVer version.
