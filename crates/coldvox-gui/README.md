# ColdVox GUI

This crate is currently a placeholder while ColdVox transitions to a Windows-first GUI path.

## Current Direction

- The old Qt/QML prototype direction is superseded.
- The active GUI plan is documented in [`../../docs/plans/windows-multi-agent-recovery.md`](../../docs/plans/windows-multi-agent-recovery.md).
- The intended shell is a Windows-first **Tauri v2 + React** overlay that carries forward the relevant UX and workflow ideas from **ColdVox_Mini**.

## Scope

The GUI direction is centered on:

- live provisional text in the UI while the user speaks
- final-text injection at utterance end
- Windows-first overlay and hotkey behavior
- porting the relevant ColdVox_Mini UX, voice-command, and configuration patterns into the new shell

## Current Runtime

Until the replacement GUI shell is scaffolded, use the main application and TUI binaries for runtime validation:

```bash
cargo run -p coldvox-app --bin coldvox
cargo run -p coldvox-app --bin tui_dashboard
```
