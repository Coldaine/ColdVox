# ColdVox – AI workspace instructions

Use these notes to help AI agents work effectively in this Rust workspace. Main application crate: `crates/app` (package `coldvox-app`). Core subsystems live in split crates and are re-exported by the app where convenient.

## Architecture (multi-crate)

- `crates/coldvox-foundation/` — App scaffolding
  - `state.rs`: `AppState` + `StateManager` with validated transitions
  - `shutdown.rs`: Ctrl+C handler + panic hook (`ShutdownHandler`/`ShutdownGuard`)
  - `health.rs`: `HealthMonitor`
  - `error.rs`: `AppError`/`AudioError`, `AudioConfig { silence_threshold }`

- `crates/coldvox-audio/` — Capture & chunking pipeline
  - `device.rs`: CPAL host/device discovery; PipeWire-aware candidates
  - `capture.rs`: `AudioCaptureThread::spawn(...)` input stream, watchdog, silence detection
  - `ring_buffer.rs`: `AudioRingBuffer` (rtrb SPSC for i16 samples)
  - `frame_reader.rs`: `FrameReader` to normalize device frames
  - `chunker.rs`: `AudioChunker` → fixed 512-sample frames (32 ms at 16 kHz)
  - `watchdog.rs`: 5s no-data watchdog used for auto-recovery
  - `detector.rs`: RMS-based `SilenceDetector` using `AudioConfig.silence_threshold`

- `crates/coldvox-vad/` — VAD traits, config, Level3 energy VAD (feature `level3`)
  - `config.rs`: `UnifiedVadConfig`, `VadMode`
  - `engine.rs`, `types.rs`, `constants.rs`, `VadProcessor` trait

- `crates/coldvox-vad-silero/` — Silero V5 ONNX VAD (feature `silero`)
  - `silero_wrapper.rs`: `SileroEngine` implementing `VadEngine`
  - Uses the external `voice_activity_detector` crate (Silero V5 backend)

- `crates/coldvox-stt/` — STT core abstractions

- `crates/coldvox-stt-vosk/` — Vosk integration (feature `vosk`)

- `crates/coldvox-telemetry/` — In-process metrics (`PipelineMetrics`, `FpsTracker`)

- `crates/coldvox-text-injection/` — Text injection backends (feature-gated)

- `crates/app/` — App glue, UI, re-exports
  - `src/audio/`:
    - `vad_adapter.rs`: Bridges `UnifiedVadConfig` to a concrete `VadEngine` (Silero or Level3)
    - `vad_processor.rs`: Async VAD pipeline task publishing `VadEvent`s
    - `mod.rs`: Re-exports from `coldvox-audio`
  - `src/vad/mod.rs`: Re-exports VAD types from `coldvox-vad` and `coldvox-vad-silero`
  - `src/stt/`: Processor/persistence wrappers and re-exports for Vosk
  - Binaries: `src/main.rs` (app), `src/bin/tui_dashboard.rs`, probes under `src/probes/`

## Build, run, debug

- From `crates/app` (package `coldvox-app`):
  - App: `cargo run`
  - App + STT (Vosk): `cargo run --features vosk` (requires system libvosk and a model)
  - TUI Dashboard:
    - No STT: `cargo run --bin tui_dashboard`
    - With STT: `cargo run --features vosk --bin tui_dashboard`
    - Device selection: append `-- -D "<device name>"`
  - Probes (examples live at repo root under `examples/`, wired via Cargo metadata):
    - `cargo run --bin mic_probe -- --duration 30 --device "<name>" --silence_threshold 120`
    - `cargo run --bin foundation_probe -- --duration 30 --simulate_errors --simulate_panics`
  - Release: `cargo build --release` or `cargo build --release --features vosk`
- Logging: `tracing` with `RUST_LOG` or `--log-level` in TUI; daily-rotated file at `logs/coldvox.log`.
  - App: logs to stderr and file.
  - TUI Dashboard: logs to file only (to avoid corrupting the TUI). Default level is `debug`; override with `--log-level <level>`.
- Tests: unit tests in source modules; integration tests under `crates/app/tests/`; VAD crates include unit tests.

## Audio data flow and contracts
- CPAL callback → i16 samples → `AudioRingBuffer` (SPSC) → `FrameReader` → `AudioChunker` → broadcast channel
- Chunker output: 512-sample frames (32 ms) at 16 kHz to VAD/STT subscribers
- VAD: Silero V5 (default) or Level3 energy engine generates `VadEvent`s
- STT: Gated by VAD events; transcribes segments when speech is active (feature `vosk`)
  - TUI: when STT is enabled and a model is present, partial/final transcripts are logged; Status shows last final transcript
- Backpressure: if the consumer is slow, writes drop when full (warn logged); keep a reader draining via `FrameReader`
- Preferred device format: choose 16 kHz mono when available; otherwise select best supported config and convert downstream
- Watchdog: 5s no-data triggers restart logic in capture thread
- Silence: RMS-based detector; >3s continuous silence logs a warning

## Tuning knobs (where to tweak)

- Chunker (`crates/coldvox-audio/src/chunker.rs` → `ChunkerConfig`)
  - `frame_size_samples` (default 512), `sample_rate_hz` (default 16000)
  - `resampler_quality`: `Fast` | `Balanced` (default) | `Quality`

- VAD (`crates/coldvox-vad/src/config.rs`)
  - `UnifiedVadConfig.mode` → `Silero` (default) | `Level3`
  - Silero (`crates/coldvox-vad-silero/src/config.rs`)
    - `threshold` (default 0.3), `min_speech_duration_ms` (250), `min_silence_duration_ms` (100), `window_size_samples` (512)
  - Level3 (`feature = "level3"`, disabled by default)
    - `onset_threshold_db` (9.0), `offset_threshold_db` (6.0), `ema_alpha` (0.02)
    - `speech_debounce_ms` (200), `silence_debounce_ms` (400), `initial_floor_db` (-50.0)

- STT (`crates/app/src/stt/` wrappers; core types in `crates/coldvox-stt/`) [feature `vosk`]
  - `TranscriptionConfig`: `model_path`, `partial_results`, `max_alternatives`, `include_words`, `buffer_size_ms`

- Text Injection (`crates/coldvox-text-injection/`; app glue in `crates/app/src/text_injection/`)
  - `SessionConfig`, injector backends via features: `text-injection-*`

- Foundation (`crates/coldvox-foundation/src/error.rs`)
  - `AudioConfig.silence_threshold` (default 100)

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
- Start capture (coldvox-audio):
  - `(capture, device_cfg, cfg_rx) = AudioCaptureThread::spawn(audio_cfg, ring_producer, device_name_opt)?`
  - Stop: `capture.stop()`
- Create pipeline:
  - `FrameReader` (from consumer) → `AudioChunker` → `broadcast::Sender<AudioFrame>`
- VAD (app glue): `VadProcessor::spawn(vad_cfg, audio_rx, event_tx, Some(metrics))?`
- STT (feature `vosk`): construct processor under `crates/app/src/stt/processor.rs`
- Metrics: use `Arc<PipelineMetrics>` across components
- Devices: `DeviceManager::new()?.enumerate_devices()`; `candidate_device_names()` prefers PipeWire → default → others

## VAD system
- Silero V5 via `crates/coldvox-vad-silero/` (feature `silero`, default enabled in app)
  - Depends on external `voice_activity_detector` crate for ONNX runtime integration
- 16 kHz, 512-sample windows per prediction
- Events: `VadEvent::{SpeechStart, SpeechEnd}` with debouncing and thresholds
- Fallback: Level3 energy VAD available (feature `level3`, disabled by default)

## STT system (feature-gated)
- Vosk-based transcription via `crates/coldvox-stt-vosk/` (re-exported in `crates/app/src/stt/vosk.rs`)
- Gated by VAD: transcribes during detected speech segments
- Events: `TranscriptionEvent::{Partial, Final, Error}` via mpsc
- Model path via `VOSK_MODEL_PATH` or default `models/vosk-model-small-en-us-0.15`
- Enable with `--features vosk`; if model path is missing, STT stays disabled
