# qlaunch

A quick, easy and slim application launcher for **Android** and **Linux (Wayland)**.

This is a ground-up rewrite of the original Android/Java launcher (see the `java`
branch) using **Flutter** for the UI and a **Rust** core for all system
interfacing, bridged with `flutter_rust_bridge`.

## Status

Android and Linux both discover and launch apps from a Flutter UI over the Rust
core. The Linux build runs under Wayland. On Android (verified on Android 14,
device `6bfcc911`) the grid shows each app's real icon, tapping launches it, and
QLaunch can be chosen as the system home app (*Settings → Home app*): the Home
button returns to the Home-role group and Back does nothing. Home-app selection
is manual, not automatic.

Not yet implemented: shortcuts, wallpaper, the side drawers, the package-change
receiver, and the notification-shade toggle (deferred in §8.2).

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
