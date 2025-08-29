# ColdVox – AI workspace instructions

Use these notes to help AI agents work productively in this Rust repo. Main crate: `crates/app`. A vendored VAD library lives in `Forks/ColdVox-voice_activity_detector` (integrated via Silero V5).

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
- `stt/` (speech-to-text - behind the `vosk` feature)
  - `mod.rs`: TranscriptionEvent, WordInfo, TranscriptionConfig.
  - `processor.rs`: STT processor gated by VAD events; emits TranscriptionEvent.
  - `vosk.rs`: VoskTranscriber implementation (requires libvosk system library).
  - `persistence.rs`: Optional persistence of transcripts/audio.
- `telemetry/`: in-process counters/gauges (`PipelineMetrics`).
- Binaries: `src/main.rs` (app, STT when built with `--features vosk`), `bin/mic_probe.rs`, `bin/foundation_probe.rs`, `bin/tui_dashboard.rs`.

## Build, run, debug
- From `crates/app`:
  - App (basic): `cargo run`
  - App (with STT): `cargo run --features vosk` (requires libvosk system library)
  - TUI Dashboard:
    - Without STT: `cargo run --bin tui_dashboard`
    - With STT: `cargo run --features vosk --bin tui_dashboard`
    - Device selection: append `-- -D "<device name>"`
  - Probes:
    - `cargo run --bin mic_probe -- --duration 30 --device "<name>" --silence_threshold 120`
    - `cargo run --bin foundation_probe -- --duration 30 --simulate_errors --simulate_panics`
  - Release: `cargo build --release` or `cargo build --release --features vosk`
- Logging: `tracing` with `RUST_LOG` or `--log-level` in TUI; daily-rotated file at `logs/coldvox.log`.
  - App: logs to stderr and file.
  - TUI Dashboard: logs to file only (to avoid corrupting the TUI). Default level is `debug`; override with `--log-level <level>`.
- Tests: unit tests in source modules; VAD crate has extensive tests; run from its folder with optional `--features async`.

## Audio data flow and contracts
- Callback thread (CPAL) → i16 samples → rtrb ring buffer (SPSC) → FrameReader → AudioChunker → broadcast channel.
- AudioChunker output: 512-sample frames (32ms) distributed via broadcast to VAD and STT processors.
- VAD processing: Silero V5 model evaluates speech probability, generates VadEvent stream.
- STT processing: Gated by VAD events, transcribes speech segments when detected (requires vosk feature).
  - TUI: when STT is enabled and a model is present, partial/final transcripts are logged; the Status panel shows the last final transcript.
- Backpressure: if the consumer is slow, ring writes drop when full (warn logged); keep a reader draining via `FrameReader`.
- Preferred format: 16 kHz mono if supported; otherwise first supported config with automatic conversion.
- Watchdog: feed on each callback; after ~5s inactivity, `is_triggered()` becomes true; `AudioCapture::recover()` attempts up to 3 restarts.
- Silence: RMS-based; >3s continuous silence logs a warning (hinting device issues).

## Tuning knobs (where to tweak)

- Chunker (`audio/chunker.rs` → `ChunkerConfig`)
  - `frame_size_samples` (default 512): output frame size; matches VAD window.
  - `sample_rate_hz` (default 16000): target internal rate.
  - `resampler_quality`: `Fast` | `Balanced` (default) | `Quality`.

- VAD (`vad/config.rs`, `vad/types.rs`)
  - Mode: `UnifiedVadConfig.mode` → `Silero` (default) | `Level3`.
  - Silero (`SileroConfig`)
    - `threshold` (default 0.3): speech probability cutoff.
    - `min_speech_duration_ms` (default 250): min speech length before start.
    - `min_silence_duration_ms` (default 100): min silence before end.
    - `window_size_samples` (default 512): analysis window; aligns with chunker.
  - Level3 energy VAD (`Level3Config`) [disabled by default]
    - `enabled` (default false): toggle fallback engine.
    - `onset_threshold_db` (default 9.0 over floor).
    - `offset_threshold_db` (default 6.0 over floor).
    - `ema_alpha` (default 0.02): noise floor smoothing.
    - `speech_debounce_ms` (default 200): frames to confirm start.
    - `silence_debounce_ms` (default 400): frames to confirm end.
    - `initial_floor_db` (default -50.0): starting noise floor.
  - Frame basics
    - `UnifiedVadConfig.frame_size_samples` (default 512) and `sample_rate_hz` (default 16000) control window duration.

- STT (`stt/mod.rs`, `stt/processor.rs`, `stt/vosk.rs`) [feature `vosk`]
  - `TranscriptionConfig`
    - `enabled` (bool): gate STT.
    - `model_path` (string): defaults via `VOSK_MODEL_PATH` or `models/vosk-model-small-en-us-0.15`.
    - `partial_results` (bool, default true): emit interim text.
    - `max_alternatives` (u32, default 1): candidate count.
    - `include_words` (bool, default false): word timings/confidence.
    - `buffer_size_ms` (u32, default 512): STT chunk size fed to Vosk.

- Text Injection (`text_injection/session.rs`, `text_injection/processor.rs`)
  - `SessionConfig`
    - `silence_timeout_ms` (default 1500): finalize after silence.
    - `buffer_pause_timeout_ms` (default 500): pause boundary between chunks.
    - `max_buffer_size` (default 5000 chars): cap transcript buffer.
  - `InjectionProcessorConfig`
    - `poll_interval_ms` (in code via comments, default 100ms).

- Audio foundation (`foundation/error.rs`)
  - `AudioConfig.silence_threshold` (default 100): RMS-based silence detector threshold.

## Logging for tuning

- TUI Dashboard
  - Logs to `logs/coldvox.log` only (no stderr) with rotation.
  - Default level: `debug` for rich telemetry.
  - Override with `--log-level <trace|debug|info|warn|error>`.
  - Shows partial/final STT in Logs; last final transcript in Status (when STT enabled).

- App (main)
  - Uses `tracing` with `RUST_LOG` (e.g., `RUST_LOG=debug`).
  - Logs to stderr and daily-rotated `logs/coldvox.log`.

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

## STT system (feature-gated, available when enabled)
- Vosk-based transcription via `stt/vosk.rs` (requires libvosk system library).
- Gated by VAD: transcribes during detected speech segments.
- Event-driven: emits `TranscriptionEvent::{Partial,Final,Error}` via mpsc channel.
- Configuration: model path via `VOSK_MODEL_PATH` env var; defaults to `models/vosk-model-small-en-us-0.15` if unset.
- Build: enable with `--features vosk`. If the model path exists, STT runs; otherwise STT stays disabled.
