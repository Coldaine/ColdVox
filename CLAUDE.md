# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## AI Assistant Response Guidelines

**Prompt Response Format**: When asked to create a prompt for another agent, return ONLY the prompt content without any additional commentary, explanation, or wrapper text. Simply output the prompt as requested.

## ColdVox Overview

Rust-based voice AI pipeline implementing VAD-gated STT with text injection. The pipeline captures audio, detects speech activity, transcribes it to text, and injects the transcribed text into the active application.

## Workspace Structure

Multi-crate Cargo workspace:

- `crates/app/` - Main application crate (package: `coldvox-app`)
  - **Audio glue**: `src/audio/vad_adapter.rs`, `src/audio/vad_processor.rs`
  - **STT integration**: `src/stt/processor.rs`, `src/stt/vosk.rs`, `src/stt/persistence.rs`
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

- `crates/coldvox-vad/` - VAD core traits and legacy Level3 energy-based VAD
  - `config.rs`: `UnifiedVadConfig`, `VadMode` (Silero default, Level3 feature-gated)
  - `engine.rs`: `VadEngine` trait for VAD implementations
  - `level3.rs`: Legacy energy-based VAD (feature `level3`) - disabled by default and not recommended for use.
  - `types.rs`: `VadEvent`, `VadState`, `VadMetrics`

- `crates/coldvox-vad-silero/` - Silero V5 ONNX-based VAD (default)
  - `silero_wrapper.rs`: `SileroEngine` implementing `VadEngine`
  - Uses external `voice_activity_detector` crate for ONNX inference

- `crates/coldvox-stt/` - STT core abstractions
  - `types.rs`: Core STT types (`TranscriptionEvent`, `WordInfo`)
  - `processor.rs`: STT processing traits

- `crates/coldvox-stt-vosk/` - Vosk STT integration (feature `vosk`)
  - `vosk_transcriber.rs`: `VoskTranscriber` for offline speech recognition

- `crates/coldvox-text-injection/` - Text injection backends (feature-gated)
  - **Linux**: `atspi_injector.rs`, `ydotool_injector.rs`, `kdotool_injector.rs`
  - **Cross-platform**: `enigo_injector.rs`
  - **Composite strategies**:
    - `clipboard_paste_injector.rs` - Sets clipboard + triggers paste (AT-SPI first, ydotool fallback)
    - `clipboard_injector.rs` - Internal helper for clipboard operations only
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

# With Vosk STT
cargo build --features vosk

# Full feature set
cargo build --features vosk,text-injection

# Workspace build (all crates)
cargo build --workspace

# Release builds
cargo build --release --features vosk,text-injection
```

### Running

```bash
# Main application (default features)
cargo run

# With specific device
cargo run -- --device "USB Microphone"

# With Vosk STT (for actual voice dictation)
cargo run --features vosk,text-injection

# With specific device and STT
cargo run --features vosk,text-injection -- --device "HyperX QuadCast"

# TUI Dashboard (shared runtime)
cargo run --bin tui_dashboard  # S=Start, A=Toggle VAD/PTT, R=Reset, Q=Quit
# Optional explicit device or extra logging
cargo run --bin tui_dashboard -- --device "USB Microphone" --log-level "info,stt=debug,coldvox_audio=debug"

# Mic probe utility
cargo run --bin mic_probe -- --duration 30

# Examples (must include required features)
cargo run --example foundation_probe
cargo run --example record_10s
cargo run --example vosk_test --features vosk,examples
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

# End-to-end WAV test (requires Vosk model - auto-discovered from project root)
cargo test -p coldvox-app --features vosk test_end_to_end_wav --nocapture
```

### Linting & Formatting

```bash
cargo check --all-targets
cargo fmt -- --check
cargo clippy -- -D warnings
```

## Features

Default features: `silero`, `vosk`, `text-injection`

- `vosk` - Vosk STT support (requires libvosk system library)
- `text-injection` - Text injection backends (platform-specific)
- `silero` - Silero VAD (default)
- `level3` - Legacy Level3 energy-based VAD (not recommended for use)
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
- **Primary**: Silero V5 via ONNX (feature `silero`)
- **Legacy Fallback**: Level3 energy-based (feature `level3`, not recommended for use)
- **Events**: `VadEvent::{SpeechStart, SpeechEnd}` with debouncing

### STT Integration
- **Vosk**: Offline recognition (feature `vosk`)
- **Model**: `VOSK_MODEL_PATH` or `models/vosk-model-small-en-us-0.15/` (legacy root fallback supported temporarily)
- **Events**: `TranscriptionEvent::{Partial, Final, Error}`

### Text Injection
- **Direct insertion**: AT-SPI (accessibility API for text insertion)
- **Composite strategy**: ClipboardPaste (sets clipboard + triggers paste via AT-SPI/ydotool)
  - Note: There is no "clipboard-only" injector - setting clipboard without pasting is useless for automation
  - ClipboardPaste is ONE strategy that: saves clipboard → sets new text → pastes via AT-SPI or ydotool → restores clipboard
- **Optional backends**: ydotool (Wayland), kdotool (X11), enigo (cross-platform)
- **Strategy management**: Runtime selection with per-app success caching and fallback chains
- **Clipboard preservation**: Clipboard-based strategies automatically save/restore user clipboard (default 500ms delay)

## Configuration

### Audio Pipeline
- Target: 16 kHz, 16-bit i16, mono
- Frame size: 512 samples (32 ms)
- Resampler quality: Fast/Balanced/Quality

### VAD Config
- Silero threshold: 0.3
- Min speech duration: 250ms
- Min silence duration: 100ms

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
- **Graceful degradation**: Silero VAD as default with Level3 energy VAD as fallback
- **Automatic recovery**: Watchdog monitoring + automatic stream restart on errors
- **Platform awareness**: Build-time detection of OS and desktop environment
- **Lock-free communication**: rtrb ring buffer with atomic counters for audio data
- **Structured logging**: Rotation-based file logging; avoids stderr in TUI mode

## Important Files

### Core Implementation
- **Main app**: `crates/app/src/main.rs`
- **Audio pipeline**: `crates/coldvox-audio/src/capture.rs`, `frame_reader.rs`, `chunker.rs`
- **VAD engines**: `crates/coldvox-vad-silero/src/silero_wrapper.rs`, `crates/coldvox-vad/src/level3.rs`
- **STT integration**: `crates/app/src/stt/processor.rs`, `crates/app/src/stt/vosk.rs`
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

### Vosk STT
- Install libvosk system library
- Download model from https://alphacephei.com/vosk/models
- Set `VOSK_MODEL_PATH` or place in `models/vosk-model-small-en-us-0.15/`
- **Deprecation note**: Root-level model path (`vosk-model-small-en-us-0.15/`) will be removed after two minor releases

## Maintenance Notes

- Project status: Source of truth is `docs/PROJECT_STATUS.md`. The README badge is static and must be updated manually to match the current phase.
