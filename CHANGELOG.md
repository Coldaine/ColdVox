# Changelog

All notable changes to this project are documented here.

## Unreleased

Highlights
- Refactor: Stabilized audio→VAD→STT→injection pipeline with real WAV-driven tests
- Text Injection: Prefer AT‑SPI first on Wayland with fast fallback to Clipboard; ydotool path gated
- Audio Capture: Fixed device monitor lifecycle (restores FPS under PipeWire)
- VAD/STT: Unified VAD spawn, deterministic end-to-end with Vosk + Silero; trailing silence flush in WAV loader
- Tests/Docs: Reworked E2E WAV test, added `wav_file_loader`, updated integration plan docs

Details
- crates/app
	- Add `audio/wav_file_loader.rs` to stream real WAV frames and pad trailing silence to flush VAD
	- Refactor `runtime.rs` to wire unified VAD/STT pipeline and test hooks
	- Rewrite E2E WAV test to use real WAV, deterministic timing, and validated injection
- crates/coldvox-audio
	- `capture.rs`: Start device monitor with running=true; improves stability and FPS
	- Watchdog/format logs tuned; minor cleanups
- crates/coldvox-vad(-silero)
	- Debounce/timing cleanup; consistent 512-sample windows; improved logging
- crates/coldvox-stt(-vosk)
	- Helpers/constants for partial/final events; finalize handling emits last result reliably
- crates/coldvox-text-injection
	- StrategyManager prefers AT‑SPI first on Wayland, then Clipboard; Combo Clipboard+ydotool gated via config
	- Cooldown and per-method success cache to avoid retries on flaky backends; environment-first ordering
- crates/coldvox-telemetry
	- Minor additions for STT metrics integration
- docs
	- `docs/refactoring_and_integration_plan.md` expanded with architecture and test guidance

Upgrade Notes
- Wayland: AT‑SPI is attempted first and fails fast if unavailable; Clipboard remains reliable fallback
- ydotool integration is off by default unless explicitly allowed
- Ensure VOSK model present (VOSK_MODEL_PATH or default models folder) for STT path

PRs
- Refactor: Unified pipeline, injection ordering on Wayland, WAV-based E2E tests

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
