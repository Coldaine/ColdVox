# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Documentation
- **Major documentation restructure** (#180): Implemented Master Documentation Playbook v1.0.0
  - Added comprehensive documentation structure under `/docs` with canonical layout
  - Created Master Documentation Playbook defining standards, metadata schema, and governance
  - Organized documentation into domains (audio, stt, text-injection, vad, gui, foundation)
  - Added revision tracking system with automated CSV logger
  - Established PR workflow requirements including metadata validation
  - Migrated legacy documentation to new structure with proper categorization
  - Added Python virtual environment management using uv with Python 3.12
  - Fixed docs validation script to handle deleted files correctly
  - Updated CLAUDE.md with detailed workspace structure and development guidelines

### Dependencies
- Bump `toml` from 0.8.23 to 0.9.8 (#182)
- Bump `clap` from 4.5.49 to 4.5.50 (#181)
- Keep `atspi` at 0.28.0 (defer 0.29.0 upgrade due to breaking API changes)

## v2.0.2 — 2025-09-12

Highlights
- STT Plugin Manager: Full runtime integration, failover/GC, metrics/TUI, Vosk finalization
- Tests: Added failover, GC, hot-reload coverage
- Docs: Plugin README section, migration notes

Details
- Complete STT plugin manager with telemetry integration, TUI exposure, and configuration persistence
- Plugin operations instrumented with lifecycle events, transcription statistics, error tracking, and performance timing
- TUI dashboard with Plugins tab, plugin status display, interactive controls ([P] toggle, [L] load, [U] unload)
- Configuration persistence via serde_json to ./plugins.json with load on init and save on changes
- End-to-end STT pipeline test and concurrent process_audio/GC safety test
- Updated README.md with STT plugins section and migration notes

Upgrade Notes
- STT configuration now uses --stt-* flags instead of VOSK_MODEL_PATH
- Plugin settings are automatically persisted to ./plugins.json
- TUI now available with --tui flag (requires tui feature)

PRs
- STT Plugin Completion: Telemetry, TUI, and Configuration Persistence

## v2.0.1 — 2025-09-05

Highlights
- Text Injection: FocusProvider dependency injection for reliable focus handling
- Mocked fallback tests and utilities for deterministic behavior and coverage
- Headless CI: Xvfb + fluxbox readiness checks; workflow validation via `gh`
- Quality: clippy/doc warning cleanup; async `ydotool` availability check
- Documentation: testing guide, architecture diagram updates, coverage analysis

Details
- Add `MockFocusProvider`, `TestInjectorFactory`, and comprehensive tests under `crates/coldvox-text-injection/src/tests/`
- Introduce `combo_clip_ydotool` injector with async `ydotool` check
- Improve `.github/workflows/ci.yml` with readiness loops and clearer dependency setup
- Fix TUI mutability for gated fields; adjust tests to satisfy clippy best practices
- Validate workspace with `fmt`, `clippy`, `check`, `build`, `doc`, and tests

Upgrade Notes
- No breaking API changes in this release
- Optional: install `xdpyinfo` and `wmctrl` if running GUI-dependent tests locally under Xvfb

PRs
- #33 Text Injection: Focus DI, Mocked Fallback Tests, and Headless CI (Xvfb)
