# ColdVox – AI workspace instructions

Use these notes to help AI agents work productively in this Rust repo. Main crate: `crates/app`. A vendored VAD library lives in `Forks/ColdVox-voice_activity_detector` (fully integrated via Silero V5).

## Architecture
- `foundation/` (app scaffolding)
  - `state.rs`: `AppState` + `StateManager` with validated transitions.
  - `shutdown.rs`: Ctrl+C handler + panic hook via `ShutdownHandler`/`ShutdownGuard`.
  - `health.rs`: `HealthMonitor` with periodic checks (none registered yet).
  - `error.rs`: `AppError`/`AudioError`, `AudioConfig { silence_threshold }`, `recovery_strategy()` hints.
- `audio/` (capture pipeline)
  - `device.rs`: CPAL host/device discovery; prefers 16 kHz mono when available.
  - `ring_buffer.rs`: rtrb SPSC ring buffer for i16 samples (producer/consumer split).
  - `capture.rs`: builds CPAL input stream; writes samples into the rtrb ring buffer (non-blocking, drop-on-full).
  - `watchdog.rs`: 5s no-data watchdog; `is_triggered()` used to drive recovery.
  - `detector.rs`: RMS-based silence detection using `AudioConfig.silence_threshold`.
  - `chunker.rs`: Converts variable-sized frames to fixed 512-sample chunks for VAD.
  - `vad_processor.rs`: VAD processing pipeline with broadcast channel distribution.
- `vad/` (voice activity detection)
  - `silero_wrapper.rs`: Silero V5 model integration via ONNX runtime.
  - `processor.rs`: VAD state machine and event generation.
  - `config.rs`: Unified VAD configuration (Silero mode is default).
- `stt/` (speech-to-text - requires vosk feature)
  - `vosk.rs`: VoskTranscriber implementation (requires libvosk system library).
  - `processor.rs`: STT processor gated by VAD events.
  - `types.rs`: Transcription event types and configuration.
- `telemetry/`: in-process counters/gauges (`PipelineMetrics`).
- Binaries: `src/main.rs` (app with STT), `bin/mic_probe.rs`, `bin/foundation_probe.rs`, `bin/tui_dashboard.rs`.

## Build, run, debug
- From `crates/app`:
  - App (basic): `cargo run`
  - App (with STT): `cargo run --features vosk` (requires libvosk system library)
  - Probes:
    - `cargo run --bin mic_probe -- --duration 30 --device "<name>" --silence_threshold 120`
    - `cargo run --bin foundation_probe -- --duration 30 --simulate_errors --simulate_panics`
    - `cargo run --bin tui_dashboard` (real-time monitoring dashboard)
  - Release: `cargo build --release` or `cargo build --release --features vosk`
- Logging: `tracing` with `RUST_LOG` env var control ("info"|"debug"); logs to both stderr and daily-rotated file at `logs/coldvox.log`.
- Tests: unit tests in source modules; VAD crate has extensive tests; run from its folder with optional `--features async`.

## Audio data flow and contracts
- Callback thread (CPAL) → i16 samples → rtrb ring buffer (SPSC) → FrameReader → AudioChunker → broadcast channel.
- AudioChunker output: 512-sample frames (32ms) distributed via broadcast to VAD and STT processors.
- VAD processing: Silero V5 model evaluates speech probability, generates VadEvent stream.
- STT processing: Gated by VAD events, transcribes speech segments when detected (requires vosk feature).
- Backpressure: if the consumer is slow, ring writes drop when full (warn logged); keep a reader draining via `FrameReader`.
- Preferred format: 16 kHz mono if supported; otherwise first supported config with automatic conversion.
- Watchdog: feed on each callback; after ~5s inactivity, `is_triggered()` becomes true; `AudioCapture::recover()` attempts up to 3 restarts.
- Silence: RMS-based; >3s continuous silence logs a warning (hinting device issues).

## Usage patterns
- Start capture: `AudioCaptureThread::spawn(config, ring_producer, device)`.
- Create pipeline: `FrameReader` → `AudioChunker` → broadcast channel → VAD/STT processors.
- VAD integration: `VadProcessor::spawn(config, audio_rx, event_tx, metrics)`.
- STT integration: `SttProcessor::new(audio_rx, vad_rx, transcription_tx, config)` (requires vosk feature).
- Metrics: pass `Arc<PipelineMetrics>` to all components for unified telemetry.
- Enumerate devices: `DeviceManager::new()?.enumerate_devices()`; marks default device.

## VAD system (fully integrated)
- `Forks/ColdVox-voice_activity_detector`: Silero V5 via ONNX Runtime. 16 kHz expects 512-sample windows per prediction.
- Runtime binaries provided under `runtimes/` for major platforms; see its `README.md` for usage and feature flags (`async`, `load-dynamic`).
- Integration: `vad/silero_wrapper.rs` provides `SileroEngine` implementation.
- State machine: VAD events (SpeechStart, SpeechEnd) generated with configurable thresholds and debouncing.
- Fallback: Energy-based VAD available as alternative (currently disabled by default).

## STT system (framework complete, blocked by dependencies)
- Vosk-based transcription via `stt/vosk.rs` (requires libvosk system library).
- VAD-gated processing: Only transcribes during detected speech segments.
- Event-driven: Generates Partial/Final transcription events via mpsc channel.
- Configuration: Model path via `VOSK_MODEL_PATH` env var or default `models/vosk-model-small-en-us-0.15`.
- Status: Framework implemented and integrated but blocked by missing libvosk dependency.
