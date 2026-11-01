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
- **Config**: JSON schema shared with the original Flutter build

## Verification

```sh
cd rust && cargo test --workspace
# On-device:
adb shell am start -W -n bitshift.qlaunch/.MainActivity
```

## Documents

| File | Purpose |
| :--- | :--- |
| `docs/technical-notes.md` | Build notes, measurements, integration findings |
| `project_summary_architecture.md` | Architecture, tech stack and platform scope |
| `implementation_roadmap.md` | Phased implementation plan |
| `ai_agent_context_prompt.md` | Coding conventions and constraints for contributors/agents |

## Licence

GPL-3.0. See `LICENSE`.