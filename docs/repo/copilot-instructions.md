---
doc_type: reference
subsystem: general
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# ColdVox – AI workspace instructions

Use these notes to help AI agents work effectively in this Rust workspace. Main application crate: `crates/app` (package `coldvox-app`). Core subsystems live in split crates and are re-exported by the app where convenient.

## AI Assistant Response Guidelines

**Prompt Response Format**: When asked to create a prompt for another agent, return ONLY the prompt content without any additional commentary, explanation, or wrapper text. Simply output the prompt as requested.

Key defaults right now:
- Default audio/VAD windowing is 512 samples at 16 kHz (32 ms).

**Platform Detection**: Build system automatically detects platform/desktop at compile time (`crates/app/build.rs`) and enables appropriate text injection backends.

## Architecture (Multi-crate Workspace)

- `crates/coldvox-foundation/` — Core app scaffolding and foundation types
  - `state.rs`: `AppState` + `StateManager` with validated transitions
  - `shutdown.rs`: Graceful shutdown with Ctrl+C handler + panic hook (`ShutdownHandler`/`ShutdownGuard`)
  - `health.rs`: `HealthMonitor` for system health monitoring
  - `error.rs`: `AppError`/`AudioError`, `AudioConfig { silence_threshold }`

- `crates/coldvox-audio/` — Audio capture & processing pipeline
  - `device.rs`: CPAL host/device discovery with PipeWire-aware priorities
  - `capture.rs`: `AudioCaptureThread::spawn(...)` - dedicated capture thread with stream management
  - `ring_buffer.rs`: `AudioRingBuffer` - rtrb SPSC ring buffer for i16 samples (lock-free)
  - `frame_reader.rs`: `FrameReader` - normalizes device frames and handles format conversion
  - `chunker.rs`: `AudioChunker` - produces fixed 512-sample frames (32 ms at 16 kHz)
  - `resampler.rs`: `StreamResampler` - quality-configurable resampling (Fast/Balanced/Quality)
  - `watchdog.rs`: 5-second no-data watchdog with automatic recovery hooks
  - `detector.rs`: RMS-based `SilenceDetector` using `AudioConfig.silence_threshold`

- `crates/coldvox-vad/` — VAD core traits and configurations
  - `config.rs`: `UnifiedVadConfig`, `VadMode`
  - `engine.rs`: `VadEngine` trait for VAD implementations
  - `types.rs`: `VadEvent`, `VadState`, `VadMetrics`
  - `constants.rs`: `FRAME_SIZE_SAMPLES = 512`, `SAMPLE_RATE_HZ = 16_000`, `FRAME_DURATION_MS`

- `crates/coldvox-vad-silero/` — Silero V5 ONNX-based VAD (default, feature `silero`)
  - `silero_wrapper.rs`: `SileroEngine` implementing `VadEngine` with ML-based detection
  - Uses external `voice_activity_detector` crate for ONNX inference

- `crates/coldvox-stt/` — STT core abstractions and event system
  - `types.rs`: Core STT types and events (`TranscriptionEvent`, `WordInfo`)
  - `processor.rs`: STT processing building blocks (used by app-level processor)


- `crates/coldvox-telemetry/` — Pipeline metrics and performance tracking
  - `pipeline_metrics.rs`: `PipelineMetrics`, `metrics.rs`: `FpsTracker`

- `crates/coldvox-text-injection/` — Text injection backends (feature-gated, platform-aware)
  - **Linux**: `atspi_injector.rs`, `clipboard_injector.rs`, `ydotool_injector.rs`, `kdotool_injector.rs`
  - **Cross-platform**: `enigo_injector.rs`
  - **Management**: `manager.rs`, `session.rs`, `window_manager.rs`

- `crates/coldvox-gui/` — GUI components and interfaces (separate from CLI app)

- `crates/app/` — Main application crate with glue code, UI, and re-exports
  - **Audio glue**: `src/audio/vad_adapter.rs`, `src/audio/vad_processor.rs`
  - **Text injection**: `src/text_injection/` - integration with text injection backends
  - **Hotkey system**: `src/hotkey/` - global hotkey support with KDE KGlobalAccel integration
  - **Probes**: `src/probes/` - diagnostic and testing utilities
  - **Binaries**: `src/main.rs` (main app), `src/bin/tui_dashboard.rs` (TUI), `src/bin/mic_probe.rs`
  - **Re-exports**: VAD types, audio components, telemetry

## Build, run, debug

**Working Directory**: `crates/app` (package `coldvox-app`)

### Main Binaries
- App (default build, no STT): `cargo run`
- TUI Dashboard: `cargo run --bin tui_dashboard` (add `-- --device "<device name>"` and/or `--log-level <level>`)
- Mic Probe: `cargo run --bin mic_probe -- --duration 30 --device "<name>" --silence_threshold 120`
- Minimal (disable text injection too): `cargo run --no-default-features --features silero`

### Examples (at repo root `/examples/`, wired via Cargo metadata)
- Foundation: `cargo run --example foundation_probe -- --duration 30`
- Recording: `cargo run --example record_10s`
- Text Injection demo: `cargo run --features text-injection --example inject_demo`
- Hotkeys: `cargo run --example test_hotkey_backend`
- KDE KGlobalAccel: `cargo run --example test_kglobalaccel_hotkey`
- Silero VAD (wav): `cargo run --features examples --example test_silero_wav`

### Build Options
- **Release**: `cargo build --release`
- **Platform-specific**: Text injection backends auto-detected at build time
- Logging: `tracing` with `RUST_LOG` or `--log-level` in TUI; daily-rotated file at `logs/coldvox.log`.
  - App: logs to stderr and file.
  - TUI Dashboard: logs to file only (to avoid corrupting the TUI). Default level is `debug`; override with `--log-level <level>`.
### Testing Framework
- **Unit tests**: Within source modules across all crates
- **Integration tests**: `crates/app/tests/integration/`
- **End-to-end**: `crates/app/src/stt/tests/end_to_end_wav.rs`
- **Examples as tests**: Manual verification via example programs
- **Component tests**: Pipeline, VAD, text injection, timing validation

## Audio data flow and contracts
- CPAL callback → i16 samples → `AudioRingBuffer` (SPSC) → `FrameReader` → `AudioChunker` → broadcast channel
- Chunker output: 512-sample frames (32 ms) at 16 kHz to VAD/STT subscribers
- VAD: Silero V5 (default) generates `VadEvent`s
  - Activation gating: by default the app uses a hotkey workflow; enable `--activation-mode vad` to auto-activate on speech.
  - Transcribes segments during active speech (SpeechStart → SpeechEnd) and emits `TranscriptionEvent`s.
  - TUI: when STT is enabled and a model is present, partial/final transcripts are logged; Status shows last final transcript
- Backpressure: if the consumer is slow, writes drop when full (warn logged); keep a reader draining via `FrameReader`
- Preferred device format: choose 16 kHz mono when available; otherwise select best supported config and convert downstream
- Watchdog: 5s no-data triggers restart logic in capture thread
- Silence: RMS-based detector; >3s continuous silence logs a warning

## Configuration & Tuning

### Audio Pipeline (`crates/coldvox-audio/src/chunker.rs`)
```rust
ChunkerConfig {
    frame_size_samples: 512,        // Default frame size
    sample_rate_hz: 16_000,         // Target sample rate
    resampler_quality: Balanced,    // Fast/Balanced/Quality
}
```

### VAD Configuration (`crates/coldvox-vad/src/config.rs`)
```rust
UnifiedVadConfig {
    mode: VadMode::Silero,          // Silero (only available VAD implementation)
    silero: SileroConfig {
        threshold: 0.1,
        min_speech_duration_ms: 100,
        min_silence_duration_ms: 500,  // Increased to stitch natural pauses
        window_size_samples: 512,
    },
    frame_size_samples: 512,
    sample_rate_hz: 16_000,
}
```

```rust
TranscriptionConfig {
  enabled: true, // app sets this true only if the model path exists
  partial_results: true,
  max_alternatives: 1,
  include_words: false,
  buffer_size_ms: 512,
}
```
Notes:
- If the model path does not exist at runtime, the app disables STT and logs a warning.

### Text Injection (Platform-aware)
- **Linux**: Auto-enables `atspi`, `wl_clipboard`, and Wayland/X11-specific backends (`ydotool` or `kdotool`) based on session; also enables `enigo`.
- **Windows/macOS**: Auto-enables `enigo`.
- **Backend selection**: Runtime availability testing with fallback chains

## Logging & Observability

### TUI Dashboard
- **File-only logging**: `logs/coldvox.log` with daily rotation (no stderr to avoid TUI corruption)
- **Default level**: `debug` for rich telemetry
- **Override**: `--log-level <trace|debug|info|warn|error>`
- **STT display**: Partial/final transcripts in Logs pane; last final in Status

### Main App
- **Dual output**: stderr + daily-rotated `logs/coldvox.log`
- **Environment control**: `RUST_LOG` (e.g., `RUST_LOG=debug`)
- **Structured logging**: Tracing-based with component context

### CLI Highlights (main app)
- `--list-devices`: List available input devices and exit
- `--device <name>`: Select preferred input device (exact or substring)
- `--resampler-quality <fast|balanced|quality>`: Controls chunker resampler
- `--activation-mode <hotkey|vad>`: Choose activation workflow (default: `hotkey`)

### Metrics & Telemetry
- **Pipeline metrics**: `Arc<PipelineMetrics>` shared across components
- **Performance tracking**: `FpsTracker` for frame rate monitoring
- **Error tracking**: Structured error types with context

## Common Usage Patterns

### Audio Capture Setup
```rust
// Start capture thread
let (capture, device_cfg, cfg_rx) = AudioCaptureThread::spawn(
    audio_cfg, ring_producer, device_name_opt
)?;
// Stop: capture.stop()
```

### Pipeline Creation
```rust
// Audio pipeline
FrameReader (from consumer) → AudioChunker → broadcast::Sender<AudioFrame>

// VAD processing
VadProcessor::spawn(vad_cfg, audio_rx, event_tx, Some(metrics))?

// See crates/app/src/stt/processor.rs
```

### Device Management
```rust
let device_manager = DeviceManager::new()?;
let devices = device_manager.enumerate_devices();
// Prioritizes: PipeWire → default device → others
```

### Platform Detection (Build-time)
- **Linux**: Detects Wayland (`WAYLAND_DISPLAY`) vs X11 (`DISPLAY`)
- **KDE**: Detects KDE environment for KGlobalAccel hotkey backend
- **Fallback**: If no display vars, enables all backends for build environments

## VAD System Details
- **Primary**: Silero V5 via `crates/coldvox-vad-silero/` (feature `silero`, default)
  - ONNX-based ML VAD using external `voice_activity_detector` crate
  - 16 kHz, 512-sample windows per prediction
- **Legacy Fallback**: Level3 energy VAD (feature `level3`, disabled by default)
  - RMS-based energy detection with configurable thresholds. Not recommended for use.
- **Events**: `VadEvent::{SpeechStart, SpeechEnd}` with debouncing
- **Configuration**: Thresholds, durations, and windowing via `UnifiedVadConfig`

## STT System (Default Enabled)
- Gating: Transcribes only during detected speech segments
- Events: `TranscriptionEvent::{Partial, Final, Error}` via mpsc channels

## Hotkey System
- **Global hotkeys**: System-wide hotkey capture and processing (`src/hotkey/`)
- **KDE integration**: KGlobalAccel backend for Plasma desktop environments
- **Detection**: Build-time KDE environment variable detection
- **Examples**: `test_hotkey_backend`, `test_kglobalaccel_hotkey`

## Text Injection Backends
- **Linux backends**: AT-SPI (accessibility), wl-clipboard (Wayland), ydotool (uinput), kdotool (X11)
- **Cross-platform**: Enigo (input simulation)
- **Combined strategies**: Combo clipboard + AT-SPI fallback
- **Session management**: Focus tracking, window manager integration
- **Adaptive selection**: Runtime availability testing with fallback chains
