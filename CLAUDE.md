# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project Overview

ColdVox is a Rust-based voice AI project that implements a complete VAD-gated STT pipeline to capture audio, transcribe speech to text, and inject the transcribed text into the proper text field where the user is working. The text injection uses multiple backend methods (clipboard, AT-SPI, keyboard emulation) and is the critical final step that delivers the transcribed output to the user's active application.

**Platform Detection Update (2025-09-02):** The build system now automatically detects the platform and enables appropriate text injection backends:
- Linux: Automatically enables AT-SPI, Wayland clipboard, and ydotool backends
- Windows/macOS: Automatically enables Enigo backend
- Build.rs detects Wayland vs X11 at compile time for optimal backend selection
- No need to manually specify text-injection feature flags on Linux anymore

## Architecture (multi-crate)

- `crates/coldvox-foundation/` — App scaffolding and core types
  - `state.rs`: `AppState` + `StateManager`
  - `shutdown.rs`: Ctrl+C handler + panic hook (`ShutdownHandler`/`ShutdownGuard`)
  - `health.rs`: `HealthMonitor`
  - `error.rs`: `AppError`/`AudioError`, `AudioConfig { silence_threshold }`

- `crates/coldvox-audio/` — Capture & chunking pipeline
  - `device.rs`: CPAL host/device discovery; PipeWire-aware priorities
  - `capture.rs`: `AudioCaptureThread::spawn(...)` (input stream, watchdog, silence detection)
  - `ring_buffer.rs`: `AudioRingBuffer` (rtrb SPSC for i16 samples)
  - `frame_reader.rs`: `FrameReader` to normalize device frames
  - `chunker.rs`: `AudioChunker` → fixed 512-sample frames (32 ms at 16 kHz)
  - `watchdog.rs`: 5s no-data watchdog and auto-recovery hooks
  - `detector.rs`: RMS-based `SilenceDetector`

- `crates/coldvox-vad/` — VAD traits and configs; Level3 energy VAD (feature `level3`)
  - `config.rs`: `UnifiedVadConfig`, `VadMode`
  - `engine.rs`, `types.rs`, `constants.rs`, `VadProcessor` trait

- `crates/coldvox-vad-silero/` — Silero V5 ONNX VAD (feature `silero`)
  - `silero_wrapper.rs`: `SileroEngine` implementing `VadEngine`
  - Uses external `voice_activity_detector` crate

- `crates/coldvox-stt/` — STT core abstractions and events

- `crates/coldvox-stt-vosk/` — Vosk STT integration (feature `vosk`)

- `crates/coldvox-telemetry/` — Pipeline metrics (`PipelineMetrics`, `FpsTracker`)

- `crates/coldvox-text-injection/` — Text injection backends (feature-gated)

- `crates/app/` — App glue, UI, re-exports
  - `src/audio/`: `vad_adapter.rs`, `vad_processor.rs`, re-exports from `coldvox-audio`
  - `src/vad/mod.rs`: re-exports VAD types from VAD crates
  - `src/stt/`: processor/persistence wrappers and Vosk re-exports
  - Binaries: `src/main.rs` (app), `src/bin/tui_dashboard.rs`, probes under `src/probes/`

## Threading & Tasks

- Dedicated capture thread: owns CPAL stream; watchdog monitors no-data; restarts on errors
- Async tasks (Tokio): VAD processor, STT processor, UI/TUI tasks
- Channels: rtrb SPSC ring buffer, `broadcast` for audio frames, `mpsc` for events

## Audio Specifications

- Internal pipeline target: 16 kHz, 16-bit i16, mono
- Device capture: device-native format, converted to i16; channel/rate normalization downstream
- Chunker output: 512 samples (32 ms) at 16 kHz
- Conversions: stereo→mono and resampling via `FrameReader`/`AudioChunker` and VAD adapter when needed
- Backpressure: non-blocking writes; drop on full (metrics recorded)

### Resampler Quality

- Presets: `Fast`, `Balanced` (default), `Quality`
- Location: `crates/coldvox-audio/src/chunker.rs` (`ChunkerConfig { resampler_quality, .. }`)

## Development Commands

All commands below assume working from `crates/app` unless noted.

### Building

```bash
cd crates/app

# App (with STT by default - requires system libvosk)
cargo build
cargo build --release

# App without STT (for CI or environments without Vosk)
cargo build --no-default-features --features silero,text-injection

# TUI Dashboard
cargo build --bin tui_dashboard

# Examples (wired from root /examples via Cargo metadata)
cargo build --example foundation_probe
cargo build --example mic_probe
cargo build --example vad_demo
cargo build --example record_10s
```

### Running

```bash
# App (with STT by default)
cargo run

# App without STT (for CI or environments without Vosk)
cargo run --no-default-features --features silero,text-injection

# TUI Dashboard (optionally select device)
cargo run --bin tui_dashboard
cargo run --bin tui_dashboard -- -D "USB Microphone"

# Examples
cargo run --example foundation_probe -- --duration 60
cargo run --example mic_probe -- --duration 120 --device "pipewire" --silence_threshold 120
cargo run --example vad_demo
cargo run --example record_10s
```

### Testing

```bash
# Workspace tests
cargo test

# Verbose
cargo test -- --nocapture

# Specific crate/module
cargo test -p coldvox-app vad_pipeline_tests

# End-to-end WAV pipeline test (requires Vosk model)
VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 \
  cargo test -p coldvox-app --features vosk test_end_to_end_wav_pipeline -- --ignored --nocapture
```

### Type Checking & Linting

```bash
cargo check --all-targets
cargo fmt -- --check
cargo clippy -- -D warnings
```

## Key Design Principles

1. Monotonic time (`std::time::Instant`) for durations and timestamps
2. Graceful degradation: Silero VAD default, Level3 fallback via feature
3. Automatic recovery: watchdog + restart on stream error
4. Lock-free communication: rtrb ring buffer with atomic counters
5. Structured logging with rotation; avoid TUI stderr logging
6. Power-of-two buffers for efficient masking

## Tuning Knobs

- Chunker (`crates/coldvox-audio/src/chunker.rs` → `ChunkerConfig`)
  - `frame_size_samples` (default 512), `sample_rate_hz` (16000), `resampler_quality`

- VAD (`crates/coldvox-vad/src/config.rs`)
  - `UnifiedVadConfig.mode`: `Silero` (default) | `Level3`
  - Silero (`crates/coldvox-vad-silero/src/config.rs`): `threshold`, `min_speech_duration_ms`, `min_silence_duration_ms`, `window_size_samples`
  - Level3 (feature `level3`): `onset_threshold_db`, `offset_threshold_db`, `ema_alpha`, `speech_debounce_ms`, `silence_debounce_ms`, `initial_floor_db`

- STT (`crates/app/src/stt/` wrappers; core in `crates/coldvox-stt/`) [enabled by default, disable with `--no-default-features`]
  - `TranscriptionConfig`: `model_path`, `partial_results`, `max_alternatives`, `include_words`, `buffer_size_ms`

- Text Injection (`crates/coldvox-text-injection/`; app glue in `crates/app/src/text_injection/`)
  - Backends via features: `text-injection-*`

- Foundation (`crates/coldvox-foundation/src/error.rs`)
  - `AudioConfig.silence_threshold` (default 100)

## Important Files

- App entry points: `crates/app/src/main.rs`, `crates/app/src/bin/tui_dashboard.rs`
- Audio glue: `crates/app/src/audio/vad_adapter.rs`, `crates/app/src/audio/vad_processor.rs`
- Audio core: `crates/coldvox-audio/src/capture.rs`, `frame_reader.rs`, `chunker.rs`, `ring_buffer.rs`, `device.rs`
- VAD: `crates/coldvox-vad/src/*`, `crates/coldvox-vad-silero/src/silero_wrapper.rs`
- STT: `crates/app/src/stt/processor.rs`, `crates/app/src/stt/vosk.rs`, `crates/app/src/stt/tests/end_to_end_wav.rs`
- Telemetry: `crates/coldvox-telemetry/src/*`

## Error Handling & Recovery

- `AppError` and `AudioError` types in foundation
- Watchdog monitors no-data; stream errors trigger restarts
- Clean shutdown via `AudioCaptureThread::stop()` and task aborts

## Testing Approach

- Unit tests within crates; integration tests under `crates/app/tests/`
- Example programs under `/examples` for manual verification
- End-to-end WAV pipeline test: `crates/app/src/stt/tests/end_to_end_wav.rs`

## Vosk Model Setup

- Default model path: `models/vosk-model-small-en-us-0.15/`
- Override with `VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15`
- Larger models at https://alphacephei.com/vosk/models

## Notes / Known Behaviors

- Linux/PipeWire: `DeviceManager` prioritizes `pipewire` → default device → others
- TUI `-D` expects a device name; use exact device string shown by system when possible
- On format changes (rate/channels), `FrameReader` should receive updated device config via broadcast