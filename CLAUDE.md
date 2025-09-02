# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project Overview

ColdVox is a Rust-based voice AI project that implements a complete VAD-gated STT pipeline to capture audio, transcribe speech to text, and inject the transcribed text into the proper text field where the user is working. The text injection uses multiple backend methods (clipboard, AT-SPI, keyboard emulation) and is the critical final step that delivers the transcribed output to the user's active application.

**Platform Detection (Build System)**: The build system automatically detects the platform and desktop environment at compile time:
- **Linux**: Detects Wayland vs X11 and enables appropriate text injection backends (AT-SPI, Wayland clipboard, ydotool, kdotool)
- **KDE Detection**: Automatically enables KGlobalAccel backend for KDE Plasma environments
- **Windows/macOS**: Automatically enables Enigo backend
- **Build-time Configuration**: All platform detection occurs in `crates/app/build.rs`

## Architecture (Multi-crate Workspace)

- `crates/coldvox-foundation/` — Core app scaffolding and foundation types
  - `state.rs`: `AppState` + `StateManager` for application state management
  - `shutdown.rs`: Graceful shutdown with Ctrl+C handler + panic hook (`ShutdownHandler`/`ShutdownGuard`)
  - `health.rs`: `HealthMonitor` for system health monitoring
  - `error.rs`: `AppError`/`AudioError`, `AudioConfig { silence_threshold }`

- `crates/coldvox-audio/` — Audio capture & processing pipeline
  - `device.rs`: CPAL host/device discovery with PipeWire-aware priorities
  - `capture.rs`: `AudioCaptureThread::spawn(...)` - dedicated capture thread with stream management and watchdog
  - `ring_buffer.rs`: `AudioRingBuffer` - rtrb SPSC ring buffer for i16 samples (lock-free audio data flow)
  - `frame_reader.rs`: `FrameReader` - normalizes device frames and handles format conversion
  - `chunker.rs`: `AudioChunker` - produces fixed 512-sample frames (32 ms at 16 kHz)
  - `resampler.rs`: `StreamResampler` - quality-configurable audio resampling (Fast/Balanced/Quality)
  - `watchdog.rs`: 5-second no-data watchdog with automatic recovery hooks
  - `detector.rs`: RMS-based `SilenceDetector` for silence detection

- `crates/coldvox-vad/` — VAD core traits, configurations, and Level3 energy-based VAD
  - `config.rs`: `UnifiedVadConfig`, `VadMode` (Silero default, Level3 feature-gated)
  - `engine.rs`: `VadEngine` trait for VAD implementations
  - `level3.rs`: Energy-based VAD (feature `level3`) - disabled by default
  - `types.rs`: `VadEvent`, `VadState`, `VadMetrics`

- `crates/coldvox-vad-silero/` — Silero V5 ONNX-based VAD (default, feature `silero`)
  - `silero_wrapper.rs`: `SileroEngine` implementing `VadEngine` with ML-based voice activity detection
  - Uses external `voice_activity_detector` crate for ONNX inference

- `crates/coldvox-stt/` — STT core abstractions and event system
  - `types.rs`: Core STT types and events (`TranscriptionEvent`, `WordInfo`)
  - `processor.rs`: STT processing traits and abstractions

- `crates/coldvox-stt-vosk/` — Vosk STT integration (feature `vosk`, default enabled)
  - `vosk_transcriber.rs`: `VoskTranscriber` implementing offline speech recognition
  - Default model path: `models/vosk-model-small-en-us-0.15/` (configurable via `VOSK_MODEL_PATH`)

- `crates/coldvox-text-injection/` — Text injection backends (feature-gated, platform-aware)
  - **Linux backends**: `atspi_injector.rs` (AT-SPI), `clipboard_injector.rs` (wl-clipboard-rs), `ydotool_injector.rs` (uinput), `kdotool_injector.rs` (X11)
  - **Cross-platform**: `enigo_injector.rs`, `mki_injector.rs` 
  - **Combined**: `combo_clip_atspi.rs` (clipboard + AT-SPI fallback)
  - **Management**: `manager.rs` (StrategyManager), `session.rs` (InjectionSession), `window_manager.rs`
  - **Features**: `atspi`, `wl_clipboard`, `enigo`, `xdg_kdotool`, `ydotool`, `mki`, `all-backends`, `linux-desktop`

- `crates/coldvox-telemetry/` — Pipeline metrics and performance tracking
  - `pipeline_metrics.rs`: `PipelineMetrics` for performance monitoring
  - `metrics.rs`: `FpsTracker` and telemetry collection

- `crates/coldvox-gui/` — GUI components and interfaces
  - GUI implementation for ColdVox (separate from main CLI app)

- `crates/app/` — Main application crate with glue code, UI, and re-exports
  - **Audio glue**: `src/audio/vad_adapter.rs`, `src/audio/vad_processor.rs`
  - **STT integration**: `src/stt/processor.rs`, `src/stt/vosk.rs`, `src/stt/persistence.rs`
  - **Text injection**: `src/text_injection/` - integration with text injection backends
  - **Hotkey system**: `src/hotkey/` - global hotkey support with KDE KGlobalAccel integration
  - **Probes**: `src/probes/` - diagnostic and testing utilities
  - **Binaries**: `src/main.rs` (main app), `src/bin/tui_dashboard.rs` (TUI), `src/bin/mic_probe.rs`

## Threading & Task Model

- **Dedicated capture thread**: Owns CPAL input stream; watchdog monitors for no-data conditions; automatic restart on stream errors
- **Async tasks (Tokio)**: VAD processor, STT processor, text injection, hotkey handling, UI/TUI tasks
- **Communication**: 
  - rtrb SPSC ring buffer for audio data (lock-free)
  - `broadcast` channels for audio frames and configuration updates
  - `mpsc` channels for events and control messages

## Audio Specifications & Pipeline

- **Target format**: 16 kHz, 16-bit i16, mono (internal pipeline standard)
- **Device capture**: Device-native format converted to i16; handles stereo→mono and resampling
- **Frame processing**: 512 samples (32 ms) chunks at 16 kHz
- **Quality settings**: Resampler quality presets in `ChunkerConfig`: `Fast`, `Balanced` (default), `Quality`
- **Backpressure handling**: Non-blocking writes with metrics on dropped frames

## Development Commands

**Working Directory**: All commands assume working from `crates/app/` unless noted.

### Building

```bash
cd crates/app

# Main app (includes STT by default via Vosk)
cargo build
cargo build --release

# App without STT (for environments without Vosk)
cargo build --no-default-features --features silero,text-injection

# TUI Dashboard
cargo build --bin tui_dashboard

# Mic probe utility
cargo build --bin mic_probe

# Examples (linked from /examples via Cargo metadata)
cargo build --example foundation_probe
cargo build --example vad_demo
cargo build --example record_10s
cargo build --example vosk_test --features vosk,examples
cargo build --example inject_demo --features text-injection
cargo build --example test_hotkey_backend
cargo build --example test_silero_wav --features examples
```

### Running

```bash
# Main application (with STT enabled by default)
cargo run

# App without STT 
cargo run --no-default-features --features silero,text-injection

# TUI Dashboard (optionally specify device)
cargo run --bin tui_dashboard
cargo run --bin tui_dashboard -- --device "USB Microphone"

# Mic probe utility
cargo run --bin mic_probe -- --duration 120 --device "pipewire" --silence_threshold 120

# Examples
cargo run --example foundation_probe -- --duration 60
cargo run --example vad_demo
cargo run --example record_10s
cargo run --example inject_demo --features text-injection
cargo run --example vosk_test --features vosk,examples
```

### Testing

```bash
# Workspace tests
cargo test

# Verbose test output
cargo test -- --nocapture

# Specific crate/module
cargo test -p coldvox-app vad_pipeline_tests

# Integration tests
cargo test integration
cargo test pipeline_integration

# End-to-end WAV pipeline test (requires Vosk model)
VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 \
  cargo test -p coldvox-app --features vosk test_end_to_end_wav -- --ignored --nocapture
```

### Type Checking & Linting

```bash
cargo check --all-targets
cargo fmt -- --check  
cargo clippy -- -D warnings
```

## Key Design Principles

1. **Monotonic timing**: Uses `std::time::Instant` for all durations and timestamps
2. **Graceful degradation**: Silero VAD as default with Level3 energy VAD as fallback
3. **Automatic recovery**: Watchdog monitoring + automatic stream restart on errors
4. **Platform awareness**: Build-time detection of OS and desktop environment
5. **Lock-free communication**: rtrb ring buffer with atomic counters for audio data
6. **Structured logging**: Rotation-based file logging; avoids stderr in TUI mode
7. **Power-of-two buffers**: Efficient buffer masking for audio processing

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
    mode: VadMode::Silero,          // Silero (default) | Level3 (feature-gated)
    // Silero config (crates/coldvox-vad-silero/src/config.rs)
    silero: SileroConfig {
        threshold: 0.3,
        min_speech_duration_ms: 250,
        min_silence_duration_ms: 100,
        window_size_samples: 512,
    },
    // Level3 config (feature level3)
    level3: Level3Config {
        enabled: false,             // Disabled by default
        onset_threshold_db: 9.0,
        offset_threshold_db: 6.0,
        ema_alpha: 0.02,
        speech_debounce_ms: 200,
        silence_debounce_ms: 400,
    }
}
```

### STT Configuration
```rust
TranscriptionConfig {
    model_path: "models/vosk-model-small-en-us-0.15/",  // Default Vosk model
    partial_results: true,
    max_alternatives: 1,
    include_words: true,
    buffer_size_ms: 1000,
}
```

### Text Injection (Platform-specific feature activation)
- **Linux**: Automatically enables `atspi`, `wl_clipboard`, `ydotool` (Wayland) or `kdotool` (X11)  
- **Windows/macOS**: Automatically enables `enigo`
- **Backend selection**: Runtime fallback strategy with availability detection

## Binaries & Entry Points

### Main Binaries
- **`coldvox`** (`src/main.rs`): Main voice-to-text application with full pipeline
- **`tui_dashboard`** (`src/bin/tui_dashboard.rs`): Real-time TUI dashboard for monitoring
- **`mic_probe`** (`src/bin/mic_probe.rs`): Audio device testing and analysis utility

### Example Programs (`/examples/`)
- **`foundation_probe`**: Tests foundation components (state, health, shutdown)
- **`vad_demo`**: Interactive VAD testing with real-time audio
- **`record_10s`**: Simple 10-second audio recording utility
- **`vosk_test`**: STT testing with Vosk integration
- **`inject_demo`**: Text injection backend testing
- **`test_hotkey_backend`**: Hotkey system testing
- **`test_kglobalaccel_hotkey`**: KDE KGlobalAccel integration testing
- **`test_silero_wav`**: Silero VAD testing with WAV files

## Testing Framework

### Test Organization
- **Unit tests**: Within individual crates (`src/` directories)
- **Integration tests**: `crates/app/tests/integration/`
- **Common utilities**: `crates/app/tests/common/`
- **End-to-end tests**: `crates/app/src/stt/tests/`

### Key Test Areas
- Audio pipeline integration (`pipeline_integration.rs`)
- VAD processing (`vad_pipeline_tests.rs`) 
- Text injection (`text_injection_integration_test.rs`)
- Timing and chunker behavior (`chunker_timing_tests.rs`)
- Component isolation (watchdog, silence detector)

## Vosk Model Setup

- **Default path**: `models/vosk-model-small-en-us-0.15/`
- **Environment override**: `VOSK_MODEL_PATH=path/to/model`  
- **Model downloads**: https://alphacephei.com/vosk/models
- **Requirements**: libvosk system library for compilation

## Platform-Specific Behavior

### Linux
- **PipeWire priority**: `DeviceManager` prioritizes `pipewire` → default device → others
- **Desktop detection**: KDE/Plasma environments get KGlobalAccel hotkey support
- **Session detection**: Wayland vs X11 determines text injection backend selection

### Display Environments
- **Wayland**: Enables AT-SPI, wl-clipboard, ydotool backends
- **X11**: Enables AT-SPI, wl-clipboard, kdotool backends  
- **Build environment**: If no display vars detected, enables all backends

### Text Injection Strategy
- **Adaptive selection**: Runtime backend availability testing
- **Fallback chains**: Primary → secondary → no-op injector
- **Permission checking**: AT-SPI accessibility service detection
- **Focus tracking**: Window manager integration for target application detection

## Error Handling & Recovery

- **Structured errors**: `AppError` and `AudioError` types in foundation crate
- **Stream recovery**: Watchdog monitors no-data conditions; automatic restart on stream errors  
- **Graceful shutdown**: Clean shutdown via `AudioCaptureThread::stop()` and task aborts
- **State management**: `StateManager` tracks application state transitions
- **Health monitoring**: `HealthMonitor` provides system health checks

## Hotkey System

- **Global hotkeys**: System-wide hotkey capture and processing
- **KDE integration**: KGlobalAccel backend for Plasma desktop environments
- **Backend detection**: Build-time detection of KDE environment variables
- **Configurable bindings**: Customizable hotkey combinations for voice activation

## Logging & Observability

- **Dual output**: Both stdout and daily-rotated files in `logs/coldvox.log`
- **Environment control**: `RUST_LOG` variable controls log levels
- **Non-blocking**: File logging uses non-blocking writers for performance
- **Structured**: Tracing-based logging with component-specific context
- **Metrics**: Telemetry collection via `PipelineMetrics` and `FpsTracker`

## Important Files Reference

### Core Implementation
- **Main app**: `crates/app/src/main.rs`
- **Audio pipeline**: `crates/coldvox-audio/src/capture.rs`, `frame_reader.rs`, `chunker.rs`
- **VAD engines**: `crates/coldvox-vad-silero/src/silero_wrapper.rs`, `crates/coldvox-vad/src/level3.rs`
- **STT integration**: `crates/app/src/stt/processor.rs`, `crates/app/src/stt/vosk.rs`  
- **Text injection**: `crates/coldvox-text-injection/src/manager.rs`, `crates/app/src/text_injection/`

### Configuration & Build
- **Build-time platform detection**: `crates/app/build.rs`
- **VAD configuration**: `crates/coldvox-vad/src/config.rs`
- **Feature definitions**: `crates/app/Cargo.toml`
- **Workspace setup**: `Cargo.toml` (workspace root)

### Testing & Examples
- **End-to-end tests**: `crates/app/src/stt/tests/end_to_end_wav.rs`
- **Integration tests**: `crates/app/tests/integration/`
- **Example programs**: `/examples/` directory
- **Diagnostic utilities**: `crates/app/src/probes/`