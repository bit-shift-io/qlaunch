# qlaunch

A pure **iced + Rust** Android launcher. No Flutter, no Dart, no bridge.

## Build / Install

```sh
ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/28.2.13676358" bash scripts/build.sh release
adb install -r build/app/outputs/apk/release/app-release.apk
adb shell am start -W -n bitshift.qlaunch/.MainActivity
```

Device log tag: `qlaunch`

```sh
adb logcat -s qlaunch
```

## Architecture

- **UI**: iced (Rust) via `iced_android` + pinned `iced` fork
- **Core**: `qlaunch_core` (Rust) — app discovery, search, launch, shortcuts, config
- **OS adapter**: Kotlin/JNI (`bitshift.qlaunch`) — package monitor, intent dispatch
- **Config**: JSON schema compatible with the original Flutter-era build

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the crate layout, data flow, snapshot
invariants and where to change what.

## Verification

```sh
cd rust && cargo test --workspace
# On-device:
adb shell am start -W -n bitshift.qlaunch/.MainActivity
```

## Documents

| File | Purpose |
| :--- | :--- |
| `ARCHITECTURE.md` | Architecture, crate layout, data flow and invariants for agents |
| `docs/technical-notes.md` | Build notes, measurements, integration findings |
| `TASKS.md` | Feature implementation tasks (dashboard widget) |
| `TODO.md` | Outstanding work and incident notes |

## Licence

GPL-3.0. See `LICENSE`.