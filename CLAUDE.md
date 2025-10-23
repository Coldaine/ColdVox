# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## AI Assistant Response Guidelines

**Prompt Response Format**: When asked to create a prompt for another agent, return ONLY the prompt content without any additional commentary, explanation, or wrapper text. Simply output the prompt as requested.

## ColdVox Overview

Rust-based voice AI pipeline implementing VAD-gated STT with text injection. The pipeline captures audio, detects speech activity, transcribes it to text, and injects the transcribed text into the active application.

**Future Vision (Experimental)**: See [`docs/architecture.md`](docs/architecture.md#coldvox-future-vision) for the always-on intelligent listening plan, decoupled threading model, and tiered STT memory strategy under active research.

## Workspace Structure

Multi-crate Cargo workspace:

- `crates/app/` - Main application crate (package: `coldvox-app`)
  - **Audio glue**: `src/audio/vad_adapter.rs`, `src/audio/vad_processor.rs`
  - **STT integration**: `src/stt/processor.rs`, `src/stt/whisper.rs`, `src/stt/persistence.rs`
  - **Text injection**: `src/text_injection/` - integration layer
  - **Hotkey system**: `src/hotkey/` - global hotkey support with KDE KGlobalAccel
  - **Binaries**: `src/main.rs` (main), `src/bin/tui_dashboard.rs`, `src/bin/mic_probe.rs`

- `crates/coldvox-foundation/` - Core app scaffolding and foundation types
  - `state.rs`: `AppState` + `StateManager` with validated transitions
  - `shutdown.rs`: Graceful shutdown with Ctrl+C handler + panic hook (`ShutdownHandler`/`ShutdownGuard`)
  - `health.rs`: `HealthMonitor` for system health monitoring
  - `error.rs`: `AppError`/`AudioError`, `AudioConfig { silence_threshold }`

- `crates/coldvox-audio/` - Audio capture & processing pipeline
  - `device.rs`: CPAL host/device discovery with PipeWire-aware priorities
  - `capture.rs`: `AudioCaptureThread::spawn(...)` - dedicated capture thread
  - `ring_buffer.rs`: `AudioRingBuffer` - rtrb SPSC ring buffer for i16 samples (lock-free)
  - `frame_reader.rs`: `FrameReader` - normalizes device frames
  - `chunker.rs`: `AudioChunker` - produces fixed 512-sample frames (32 ms at 16 kHz)
  - `resampler.rs`: `StreamResampler` - quality-configurable (Fast/Balanced/Quality)
  - `watchdog.rs`: 5-second no-data watchdog with automatic recovery
  - `detector.rs`: RMS-based `SilenceDetector`

- `crates/coldvox-vad/` - VAD core traits and configurations
  - `config.rs`: `UnifiedVadConfig`, `VadMode`
  - `engine.rs`: `VadEngine` trait for VAD implementations
  - `energy.rs`: Energy calculation utilities for audio analysis
  - `types.rs`: `VadEvent`, `VadState`, `VadMetrics`

- `crates/coldvox-vad-silero/` - Silero V5 ONNX-based VAD (default)
  - `silero_wrapper.rs`: `SileroEngine` implementing `VadEngine`
  - Uses external `voice_activity_detector` crate for ONNX inference

- `crates/coldvox-stt/` - STT core abstractions
  - `types.rs`: Core STT types (`TranscriptionEvent`, `WordInfo`)
  - `processor.rs`: STT processing traits

- `crates/coldvox-stt/` - STT core abstractions
  - `plugins/whisper_plugin.rs`: `WhisperPlugin` for offline speech recognition via faster-whisper

- `crates/coldvox-text-injection/` - Text injection backends (feature-gated)
  - **Linux**: `atspi_injector.rs`, `clipboard_injector.rs`, `ydotool_injector.rs`, `kdotool_injector.rs`
  - **Cross-platform**: `enigo_injector.rs`
  - **Combined**: `combo_clip_ydotool.rs` (clipboard + AT-SPI paste, fallback to ydotool)
  - **Management**: `manager.rs` (StrategyManager), `session.rs`, `window_manager.rs`

- `crates/coldvox-telemetry/` - Pipeline metrics
  - `pipeline_metrics.rs`: `PipelineMetrics`
  - `metrics.rs`: `FpsTracker`

- `crates/coldvox-gui/` - GUI components (separate from CLI)

## Development Commands

**Working Directory**: Project root for all commands (standard Rust workspace practice).

### Building

```bash
# Main app with default features (Silero VAD + text injection, no STT by default)
cargo build

# With Whisper STT
cargo build --features whisper

# Full feature set
cargo build --features whisper,text-injection

# Workspace build (all crates)
cargo build --workspace

# Release builds
cargo build --release --features whisper,text-injection
```

### Running

```bash
# Main application (default features)
cargo run

# With specific device
cargo run -- --device "USB Microphone"

# With Whisper STT (for actual voice dictation)
cargo run --features whisper,text-injection

# With specific device and STT
cargo run --features whisper,text-injection -- --device "HyperX QuadCast"

# TUI Dashboard (shared runtime)
cargo run --bin tui_dashboard  # S=Start, A=Toggle VAD/PTT, R=Reset, Q=Quit
# Optional explicit device or extra logging
cargo run --bin tui_dashboard -- --device "USB Microphone" --log-level "info,stt=debug,coldvox_audio=debug"

# Mic probe utility
cargo run --bin mic_probe -- --duration 30

# Examples (must include required features)
cargo run --example foundation_probe
cargo run --example record_10s
cargo run --example whisper_test --features whisper,examples
cargo run --example inject_demo --features text-injection
cargo run --example test_silero_wav --features examples
```

### Testing

```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific package
cargo test -p coldvox-app

# Integration tests
cargo test integration

# End-to-end WAV test (requires Whisper model - auto-discovered from project root)
cargo test -p coldvox-app --features whisper test_end_to_end_wav --nocapture
```

### Linting & Formatting

```bash
cargo check --all-targets
cargo fmt -- --check
cargo clippy -- -D warnings
```

## Features

Default features: `silero`, `whisper`, `text-injection`

- `whisper` - Faster-Whisper STT support (requires faster-whisper Python package)
- `text-injection` - Text injection backends (platform-specific)
- `silero` - Silero V5 ONNX-based VAD (default and only VAD implementation)
- `examples` - Enable example-specific dependencies
- `live-hardware-tests` - Hardware-specific test suites

Platform-specific text injection backends are automatically enabled at build time via `crates/app/build.rs`:
- Linux: Detects Wayland/X11 and enables appropriate backends (AT-SPI, clipboard, ydotool/kdotool)
- Windows/macOS: Enables Enigo backend
- KDE: Enables KGlobalAccel hotkey support

## Key Components

### Audio Pipeline
- **Capture**: `AudioCaptureThread` - dedicated thread with CPAL stream
- **Ring Buffer**: `AudioRingBuffer` - lock-free SPSC for i16 samples
- **Chunking**: `AudioChunker` - 512-sample frames at 16 kHz
- **Resampling**: `StreamResampler` - Fast/Balanced/Quality modes
- **Watchdog**: 5-second no-data detection with auto-recovery

### VAD System
- **Engine**: Silero V5 ONNX-based VAD (feature `silero`, enabled by default)
- **Configuration**: threshold=0.1, min_speech=100ms, min_silence=500ms
- **Events**: `VadEvent::{SpeechStart, SpeechEnd}` with debouncing

### STT Integration
- **Whisper**: Offline recognition via faster-whisper (feature `whisper`)
- **Model**: `WHISPER_MODEL_PATH` or standard Whisper model identifiers (e.g., "base.en", "small.en")
- **Events**: `TranscriptionEvent::{Partial, Final, Error}`

### Text Injection
- **Linux backends**: AT-SPI, wl-clipboard, ydotool (Wayland), kdotool (X11)
- **Cross-platform**: Enigo
- **Strategy**: Runtime backend selection with fallback chains

## Configuration

### Audio Pipeline
- Target: 16 kHz, 16-bit i16, mono
- Frame size: 512 samples (32 ms)
- Resampler quality: Fast/Balanced/Quality

### VAD Config
- Silero threshold: 0.1
- Min speech duration: 100ms
- Min silence duration: 500ms (increased to stitch natural pauses)

### Logging
- Main app: stderr + `logs/coldvox.log` (daily rotation)
- TUI: file-only to `logs/coldvox.log` (avoids display corruption)
- Control: `RUST_LOG` environment variable or `--log-level` flag

## Platform Detection

Build-time detection in `crates/app/build.rs`:
- Checks `WAYLAND_DISPLAY`, `DISPLAY`, `XDG_SESSION_TYPE`
- Detects KDE via `KDE_FULL_SESSION`, `PLASMA_SESSION`
- Enables appropriate text injection and hotkey backends

## Threading & Communication

- **Dedicated capture thread**: Owns CPAL input stream; watchdog monitors for no-data conditions
- **Async tasks (Tokio)**: VAD processor, STT processor, text injection, hotkey handling, UI/TUI
- **Communication**:
  - rtrb SPSC ring buffer for audio data (lock-free)
  - `broadcast` channels for audio frames and configuration updates
  - `mpsc` channels for events and control messages

## Key Design Principles

- **Monotonic timing**: Uses `std::time::Instant` for all durations and timestamps
- **Single VAD implementation**: Silero V5 ONNX-based VAD with no fallback
- **Automatic recovery**: Watchdog monitoring + automatic stream restart on errors
- **Platform awareness**: Build-time detection of OS and desktop environment
- **Lock-free communication**: rtrb ring buffer with atomic counters for audio data
- **Structured logging**: Rotation-based file logging; avoids stderr in TUI mode

## Important Files

### Core Implementation
- **Main app**: `crates/app/src/main.rs`
- **Audio pipeline**: `crates/coldvox-audio/src/capture.rs`, `frame_reader.rs`, `chunker.rs`
- **VAD engine**: `crates/coldvox-vad-silero/src/silero_wrapper.rs`
- **STT integration**: `crates/app/src/stt/processor.rs`, `crates/coldvox-stt/src/plugins/whisper_plugin.rs`
- **Text injection**: `crates/coldvox-text-injection/src/manager.rs`

### Configuration & Build
- **Build-time platform detection**: `crates/app/build.rs`
- **VAD configuration**: `crates/coldvox-vad/src/config.rs`
- **Feature definitions**: `crates/app/Cargo.toml`

### Testing & Examples
- **End-to-end tests**: `crates/app/src/stt/tests/end_to_end_wav.rs`
- **Integration tests**: `crates/app/tests/integration/`
- **Example programs**: `/examples/` directory

## Setup Requirements

### Linux Text Injection
Run `scripts/setup_text_injection.sh` to install:
- wl-clipboard (required)
- ydotool (recommended)
- kdotool (optional)
- Configures uinput permissions and user groups

### Whisper STT
- Install faster-whisper Python package: `pip install faster-whisper`
- Models are automatically downloaded on first use
- Set `WHISPER_MODEL_PATH` to specify a model identifier or custom model directory
- Common model identifiers: "tiny.en", "base.en", "small.en", "medium.en"

## Maintenance Notes

- Project status: Source of truth is `docs/PROJECT_STATUS.md`. The README badge is static and must be updated manually to match the current phase.
