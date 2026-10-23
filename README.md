# qlaunch

A quick, easy and slim application launcher for **Android** and **Linux (Wayland)**.

This is a ground-up rewrite of the original Android/Java launcher (see the `java`
branch) using **Flutter** for the UI and a **Rust** core for all system
interfacing, bridged with `flutter_rust_bridge`.

## Status

Android and Linux both discover and launch apps from a Flutter UI over the Rust
core. The Linux build is verified to compile in release mode and targets
Wayland. On Android (verified on Android 14, device `18161JECB02915`) the grid
shows each app's real icon, tapping launches it, and QLaunch can be chosen as the
system home app (*Settings → Home app*): the Home button returns to the Home-role
group and Back does nothing. Home-app selection is manual, not automatic.

Implemented: paged groups with a Java-style title strip, content-sized tiles/list/
centred layouts with two-line labels, Home-button search with frequent and recent
apps, Android and Linux shortcuts, system wallpaper with dimming, display settings
applied at runtime, scroll fading (none, edge gradient, or whole-cell), accessibility
screen lock and notification shade opening, and a confirmed factory reset. The blank-space long-press menu replaces
the old side drawers.

Double-tap on blank space and the Lock Screen shortcut use Android's accessibility
lock action so fingerprint/face unlock remains available under normal Android
security rules. The first lock or notification attempt opens Settings: enable
**QLaunch system actions**, return to the launcher, and try again. This service
does not read screen content. The old device-admin permission is revoked on the
next lock attempt after upgrading.

An **Advanced** page (a single row in Settings) holds the destructive actions —
Restart Launcher, Backup config, Restore config and Factory Reset. Backup writes the
whole document to the Downloads folder as a dated `qlaunch-backup-….json`, including
shortcut icons that cannot be looked up again; Restore lists those backups
newest-first, confirms the chosen file, validates it, keeps a pre-restore copy, then
replaces and reconciles the layout. Shortcut icons supplied as Android resource
references are re-read from the providing app instead of being stored in the backup.

Search shares the launcher's wallpaper and styling while hiding its main grid.
Frequent and Recent are swipeable history pages, each limited to three rows.
Home from another app restores the last tab; subsequent presses go to Home,
open search, then close search. A Home press that dismisses the notification
shade returns to Home without opening search.

The notification shade opens from a downward pull at the top of a page or from a
Notifications shortcut; both use the same accessibility service as screen lock.
Deferred: custom app-icon picker, notification expansion, quick-settings tile and
widget scaffolding.

## Documents

| File | Purpose |
| :--- | :--- |
| `project_summary_architecture.md` | Architecture, tech stack and platform scope. |
| `implementation_roadmap.md` | Phased implementation plan. |
| `ai_agent_context_prompt.md` | Coding conventions and constraints for contributors/agents. |
| `docs/superpowers/specs/2026-10-21-java-parity-audit.md` | Java-to-rewrite feature audit. |
| `docs/superpowers/specs/2026-10-21-java-parity-design.md` | Approved parity design and behaviour. |
| `docs/superpowers/plans/2026-10-21-java-parity-progress.md` | Execution and device-verification ledger. |

## Branches

| Branch | Contents |
| :--- | :--- |
| `main` | The Flutter + Rust launcher. |
| `java` | The original Android/Java launcher, frozen for reference. |

## Licence

GPL-3.0. See `LICENSE`.
