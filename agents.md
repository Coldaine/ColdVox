# Agents Guide

This branch (`anchor/oct-06-2025`) introduces a large documentation/configuration refactor but leaves many tracked issues unresolved. Use the notes below when collaborating with additional agents.

## Branch State Overview

- **Configuration:** Runtime settings now come from `config/default.toml`, layered with `COLDVOX_*` environment overrides. CLI flags have been reduced to `--list-devices`, `--tui`, and `--injection-fail-fast`.
- **Text Injection:** `StrategyManager` gained clipboard restoration logic, but AT-SPI focus detection currently returns `FocusStatus::Unknown` (`crates/coldvox-text-injection/src/focus.rs`). Clipboard restoration tests only assert behaviour when `wl_clipboard` is enabled.
- **Audio/STT:** No functional changes landed for callback allocations or STT pipeline improvements—most code matches `main`.
- **Documentation:** Several docs authored in this branch contained optimistic claims; updated copies in `docs/` now highlight the real status.

## Critical Caveats

1. **AT-SPI regression:** Restore accurate focus detection before shipping; current logic short-circuits to `Unknown`.
2. **Testing gaps:** Only `cargo test -p coldvox-text-injection -- --list` has been re-run. Workspace builds/tests will still fail locally without system ALSA headers.
3. **GUI features:** GuiBridge remains a stub (state toggles only); GUI integration issues (#58-#60, #62) stay open.

## Recommended Next Steps

- Revert or fix the AT-SPI focus tracker regression and add coverage that runs without Wayland-specific features.
- Add CI jobs that exercise clipboard restoration with `wl_clipboard` enabled, or provide mocks so the tests assert on all platforms.
- Reconcile CLI documentation with the new configuration approach (see `docs/user/runflags.md`) and ensure future docs avoid aspirational language.
- Re-evaluate outstanding issues (#100, #63, #36, #40, #38, #58-#62, STT backlog) before attempting to close anything in PR #121.
