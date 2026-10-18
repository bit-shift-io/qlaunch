# qlaunch

A quick, easy and slim application launcher for **Android** and **Linux (Wayland)**.

This is a ground-up rewrite of the original Android/Java launcher (see the `java`
branch) using **Flutter** for the UI and a **Rust** core for all system
interfacing, bridged with `flutter_rust_bridge`.

## Status

Project setup. No application code yet — the design documents below are the
current spec.

## Documents

| File | Purpose |
| :--- | :--- |
| `project_summary_architecture.md` | Architecture, tech stack and platform scope. |
| `implementation_roadmap.md` | Phased implementation plan. |
| `ai_agent_context_prompt.md` | Coding conventions and constraints for contributors/agents. |

## Branches

| Branch | Contents |
| :--- | :--- |
| `main` | The Flutter + Rust launcher. |
| `java` | The original Android/Java launcher, frozen for reference. |

## Licence

GPL-3.0. See `LICENSE`.
