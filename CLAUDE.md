# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

**@import AGENTS.md** - Read `AGENTS.md` for canonical project instructions (structure, commands, do/don't rules, worktrees).

## Claude-Specific Guidelines

### Response Format

**Prompt Response Format**: When asked to create a prompt for another agent, return ONLY the prompt content without any additional commentary, explanation, or wrapper text.

### Subagent Usage (Opus 4.5)

- Use subagents for verification tasks before claiming completion
- Use TDD skills for new features (`@./.claude/skills/test.md` if available)
- Prefer crate-scoped commands to reduce latency

## ColdVox Deep Dive

This section provides detailed context beyond `AGENTS.md` for complex tasks.

### Workspace Structure (Detailed)

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
  - `plugins/whisper_plugin.rs`: `WhisperPlugin` for offline speech recognition via faster-whisper
  - `plugins/parakeet.rs`: `ParakeetPlugin` for GPU-accelerated recognition via NVIDIA Parakeet (pure Rust)

- `crates/coldvox-text-injection/` - Text injection backends (feature-gated)
  - **Linux**: `atspi_injector.rs`, `clipboard_injector.rs`, `ydotool_injector.rs`, `kdotool_injector.rs`
  - **Cross-platform**: `enigo_injector.rs`
  - **Combined**: `combo_clip_ydotool.rs` (clipboard + AT-SPI paste, fallback to ydotool)
  - **Management**: `manager.rs` (StrategyManager), `session.rs`, `window_manager.rs`

- `crates/coldvox-telemetry/` - Pipeline metrics
  - `pipeline_metrics.rs`: `PipelineMetrics`
  - `metrics.rs`: `FpsTracker`

- `crates/coldvox-gui/` - GUI components (separate from CLI)

### Key Components (Technical Details)

#### Audio Pipeline
- **Capture**: `AudioCaptureThread` - dedicated thread with CPAL stream
- **Ring Buffer**: `AudioRingBuffer` - lock-free SPSC for i16 samples
- **Chunking**: `AudioChunker` - 512-sample frames at 16 kHz
- **Resampling**: `StreamResampler` - Fast/Balanced/Quality modes
- **Watchdog**: 5-second no-data detection with auto-recovery

#### VAD System
- **Engine**: Silero V5 ONNX-based VAD (feature `silero`, enabled by default)
- **Configuration**: threshold=0.1, min_speech=100ms, min_silence=500ms
- **Events**: `VadEvent::{SpeechStart, SpeechEnd}` with debouncing

#### STT Integration
- **Parakeet** (feature `parakeet`): GPU-accelerated via NVIDIA Parakeet (pure Rust, GPU-only)
  - Model: nvidia/parakeet-tdt-1.1b (1.1B params, multilingual) or nvidia/parakeet-ctc-1.1b (English-only)
  - Environment: `PARAKEET_MODEL_PATH`, `PARAKEET_VARIANT` (tdt/ctc), `PARAKEET_DEVICE` (cuda/tensorrt)
  - Requires: CUDA-capable GPU, no CPU fallback
- **Whisper** (feature `whisper`): Offline recognition via faster-whisper (Python-based)
  - Model: `WHISPER_MODEL_PATH` or standard identifiers (e.g., "base.en", "small.en")
- **Events**: `TranscriptionEvent::{Partial, Final, Error}`

#### Text Injection
- **Linux backends**: AT-SPI, wl-clipboard, ydotool (Wayland), kdotool (X11)
- **Cross-platform**: Enigo
- **Strategy**: Runtime backend selection with fallback chains

### Configuration Details

#### Audio Pipeline
- Target: 16 kHz, 16-bit i16, mono
- Frame size: 512 samples (32 ms)
- Resampler quality: Fast/Balanced/Quality

#### VAD Config
- Silero threshold: 0.1
- Min speech duration: 100ms
- Min silence duration: 500ms (increased to stitch natural pauses)

#### Logging
- Main app: stderr + `logs/coldvox.log` (daily rotation)
- TUI: file-only to `logs/coldvox.log` (avoids display corruption)
- Control: `RUST_LOG` environment variable or `--log-level` flag

### Platform Detection

Build-time detection in `crates/app/build.rs`:
- Checks `WAYLAND_DISPLAY`, `DISPLAY`, `XDG_SESSION_TYPE`
- Detects KDE via `KDE_FULL_SESSION`, `PLASMA_SESSION`
- Enables appropriate text injection and hotkey backends

### Threading & Communication

- **Dedicated capture thread**: Owns CPAL input stream; watchdog monitors for no-data conditions
- **Async tasks (Tokio)**: VAD processor, STT processor, text injection, hotkey handling, UI/TUI
- **Communication**:
  - rtrb SPSC ring buffer for audio data (lock-free)
  - `broadcast` channels for audio frames and configuration updates
  - `mpsc` channels for events and control messages

### Key Design Principles

- **Monotonic timing**: Uses `std::time::Instant` for all durations and timestamps
- **Single VAD implementation**: Silero V5 ONNX-based VAD with no fallback
- **Automatic recovery**: Watchdog monitoring + automatic stream restart on errors
- **Platform awareness**: Build-time detection of OS and desktop environment
- **Lock-free communication**: rtrb ring buffer with atomic counters for audio data
- **Structured logging**: Rotation-based file logging; avoids stderr in TUI mode

### Setup Requirements

#### Linux Text Injection
Run `scripts/setup_text_injection.sh` to install:
- wl-clipboard (required)
- ydotool (recommended)
- kdotool (optional)
- Configures uinput permissions and user groups

#### Parakeet STT (GPU-only)
- **Requirements**: CUDA-capable NVIDIA GPU
- **Verify GPU**: `nvidia-smi` must succeed
- **Models**: Auto-downloaded to `~/.cache/parakeet/` on first use
- **Configuration**:
  - `PARAKEET_VARIANT`: "tdt" (multilingual, default) or "ctc" (English-only)
  - `PARAKEET_DEVICE`: "cuda" (default) or "tensorrt" (optimized)
  - `PARAKEET_MODEL_PATH`: Override model location

#### Whisper STT (CPU/GPU hybrid)
- Install faster-whisper Python package: `pip install faster-whisper`
- Models are automatically downloaded on first use
- Set `WHISPER_MODEL_PATH` to specify a model identifier or custom model directory

### Changelog Maintenance

**REQUIRED**: All user-visible changes MUST be documented in `CHANGELOG.md` following the rubric in `docs/standards.md`.

See `docs/standards.md` for the detailed rubric on when to update and format guidelines.

### Future Vision

See [`docs/architecture.md`](docs/architecture.md#coldvox-future-vision) for the always-on intelligent listening plan, decoupled threading model, and tiered STT memory strategy under active research.
