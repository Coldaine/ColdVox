# ColdVox – AI workspace instructions

Use these notes to help AI agents work productively in this Rust repo. Main crate: `crates/app`. A vendored VAD library lives in `Forks/ColdVox-voice_activity_detector` (not yet wired into the app).

## Architecture
- `foundation/` (app scaffolding)
  - `state.rs`: `AppState` + `StateManager` with validated transitions.
  - `shutdown.rs`: Ctrl+C handler + panic hook via `ShutdownHandler`/`ShutdownGuard`.
  - `health.rs`: `HealthMonitor` with periodic checks (none registered yet).
  - `error.rs`: `AppError`/`AudioError`, `AudioConfig { silence_threshold }`, `recovery_strategy()` hints.
- `audio/` (capture pipeline)
  - `device.rs`: CPAL host/device discovery; prefers 16 kHz mono when available.
  - `capture.rs`: builds CPAL input stream; pushes `AudioFrame` to bounded channel (100 frames) with overflow tracking.
  - `watchdog.rs`: 5s no-data watchdog; `is_triggered()` used to drive recovery.
  - `detector.rs`: RMS-based silence detection using `AudioConfig.silence_threshold`.
- `telemetry/`: in-process counters/gauges (`BasicMetrics`).
- Binaries: `src/main.rs` (app), `bin/mic_probe.rs`, `bin/foundation_probe.rs`.

## Build, run, debug
- From `crates/app`:
  - App: `cargo run`
  - Probes:
    - `cargo run --bin mic_probe -- --duration 30 --device "<name>" --silence_threshold 120`
    - `cargo run --bin foundation_probe -- --duration 30 --simulate_errors --simulate_panics`
  - Release: `cargo build --release`
- Logging: `tracing` with fixed `with_env_filter("info"|"debug")` in code; adjust in source to change verbosity.
- Tests: none in `crates/app/tests`. The VAD crate has tests; run from its folder with optional `--features async`.

## Audio data flow and contracts
- Callback thread (CPAL) → `AudioFrame { samples: Vec<i16>, timestamp, sample_rate, channels }` → crossbeam bounded channel (size 100).
- Backpressure: if the consumer is slow, frames are dropped and `frames_dropped` increments; keep a reader draining `AudioCapture::get_receiver()`.
- Preferred format: 16 kHz mono if supported; otherwise first supported config (see `DeviceManager::get_supported_configs`).
- Watchdog: feed on each callback; after ~5s inactivity, `is_triggered()` becomes true; `AudioCapture::recover()` attempts up to 3 restarts.
- Silence: RMS-based; >3s continuous silence logs a warning (hinting device issues).

## Usage patterns
- Start capture (optionally choosing device): `AudioCapture::start(Some("Device Name")).await?`.
- Consume frames off-thread to avoid drops:
  - `let rx = capture.get_receiver(); tokio::spawn(async move { while let Ok(f) = rx.recv() { /* process f.samples */ } });`
- Stats: call `get_stats()`; check `last_frame_age` to detect stalls; use watchdog + `recover()`.
- Enumerate devices: `DeviceManager::new()?.enumerate_devices()`; marks default device.

## VAD crate (vendored)
- `Forks/ColdVox-voice_activity_detector`: Silero V5 via ONNX Runtime. 16 kHz expects 512-sample windows per prediction.
- Runtime binaries provided under `runtimes/` for major platforms; see its `README.md` for usage and feature flags (`async`, `load-dynamic`).
