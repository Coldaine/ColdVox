# CLAUDE.md

## Project Overview

ColdVox is a Rust-based voice AI project focused on real-time audio processing with emphasis on reliability and automatic recovery. The project implements a multi-phase STT (Speech-to-Text) system with voice activity detection (VAD) and resilient audio capture using lock-free ring buffers for real-time communication.

## Architecture

### Core Components

- **Foundation Layer** (`crates/app/src/foundation/`): Error handling, health monitoring, state management, graceful shutdown
- **Audio System** (`crates/app/src/audio/`): Microphone capture, device management, watchdog monitoring, automatic recovery
  - `AudioCapture`: Multi-format device capture with automatic conversion
  - `AudioChunker`: Converts variable-sized frames to fixed 512-sample chunks
  - `VadAdapter`: Trait for pluggable VAD implementations
  - `VADProcessor`: VAD processing pipeline integration
- **VAD System** (`crates/app/src/vad/`): Dual VAD implementation with power-based VAD and ML models
  - `Level3VAD`: Progressive energy-based VAD implementation **[DISABLED BY Default]**
  - `SileroEngine`: Silero model wrapper for ML-based VAD **[Default ACTIVE VAD]**
  - `VADStateMachine`: State management for VAD transitions
  - `UnifiedVADConfig`: Configuration supporting both VAD modes (defaults to Silero)
- **STT System** (`crates/app/src/STT/`): Speech-to-text transcription
  - `VoskTranscriber`: Vosk-based STT implementation
  - `STTProcessor`: STT processing gated by VAD events
  - `Transcriber` trait for pluggable STT backers
- **Telemetry** (`crates/app/src/telemetry/`): Metrics collection and monitoring
  - `PipelineMetrics`: Real-time pipeline performance metrics
  - Cross-thread monitoring of audio levels, latency, and throught
- **Probes** (`crates/app/src/probes/`): Test utilities and live hardware checks
- **VAD Fork** (`Forks/ColdVox-voice_activity_detector/`): Voice activity detection using Silero model with ONX runtime

### Threading Model

- **Mic Thread**: Owns audio device, handles capture
- **Processing Thread**: Runs VAD and chunkinging
- **STT Thread**: Processes speech segments when VAD detects speech
- **Main Thread**: Orchestrates and monitors components
- Communication via lock-free ring buffers (rtrb), broadcast channels, and mpsc channels

### Audio Specifications

- Internal format: 16kHz, 16-bit signed (i16), mono
- Capture frames: Variable-sized (CPAL BufferSize::Default)
- Chunker output: 512 samples (32ms) for VAD processing
- Conversion: Stereo→mono averaging, rate conversion via fractional-phase resampling
- Overflow handling: Configurable policy (DropOldest/DropNewest/Panic)

## Development Commands

### Building

```bash
# Main application (requires --features vosk)
cd crates/app
cargo build --features vosk
cargo build --release --features vosk

# TUI Dashboard binary
cargo build --bin tui_dashboard

# Build specific examples (from crates/app directory)
cargo build --example foundation_probe
cargo build --example mic_probe
cargo build --example vad_demo
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with verbose output  
cargo test -- --nocapture

# Run tests for specific module
cargo test audio::
cargo test vad::

# Run test suite script (from project root)
./scripts/run_phase1_tests.sh all  # All automated tests
./scripts/run_phase1_tests.sh unit  # Unit tests only
./scripts/run_phase1_tests.sh live  # Live hardware tests
```

### Running Test Binaries

```bash
# Main application (requires --features vosk)
cargo run --features vosk

# TUI Dashboard for real-time monitoring
cargo run --bin tui_dashboard
cargo run --bin tui_dashboard -- -D "USB Microphone"  # Specific device

# Examples (from crates/app directory):
cargo run --example foundation_probe -- --duration 60
cargo run --example mic_probe -- --duration 120 --expect-disconnect
cargo run --example vad_demo  # Test VAD with microphone
cargo run --example record_10s  # Record 10 seconds to WAV
cargo run --example test_silero_minimal  # Test Silero VAD minimal implementation
cargo run --example test_silero_wav  # Test Silero VAD with WAV files
```

### Type Checking & Linting
```bash
cargo check --all-targets
cargo fmt -- --check  # Check formatting
cargo clippy -- -D warnings  # Strict linting
```

## Key Design Principles

1. **Monotonic Time**: Use `std::time::Instant` for all durations/intervals
2. **Graceful Degradation**: Primary VAD with energy-based fallback
3. **Automatic Recovery**: Exponential backoff with jitter for reconnection
4. **Lock-free Communication**: Ring buffers (rtrb) with atomic operations
5. **Structured Logging**: Rate-limited, JSON-formatted logs with daily rotation
6. **Power-of-two Buffers**: For efficient index masking in ring buffers

## Phase Implementation Status

- **Phase 0**: Foundation & Safety Net ✅ **COMPLETE**
- **Phase 1**: Microphone Capture with Recovery ✅ **COMPLETE** (all critical bugs fixed)
- **Phase 2**: Lock-free Ring Buffer ✅ **COMPLETE** (using rtrb library)
- **Phase 3**: VAD with Fallback ✅ **COMPLETE** (Silero VAD integrated, energy-based VAD available)
- **Phase 4**: STT Integration ✅ **COMPLETE** (Vosk transcriber integrated with VAD gating)
- **Phase 5+**: Stress Testing & Polish 📋 **IN PROGRESS**

## Configuration

Configuration parameters:
- Window/overlap for audio processing (default: 500ms window, 0.5 overlap)
- VAD thresholds and debouncing (speech_threshold: 0.6, min_speech_ms: 200)
- Retry policies and timeouts (exponential backoff with jitter)
- Buffer overflow handling (DropOldest/DropNewest/Panic)
- Logging and metrics settings (JSON structured, rate-limited)

## Important Files

- `docs/PROJECT_STATUS.md`: Current project status and next steps
- `crates/app/src/main.rs`: Main application entry point with Vosk STT
- `crates/app/src/bin/tui_dashboard.rs`: Real-time monitoring dashboard
- `crates/app/src/audio/capture.rs`: Core audio capture with format negotiation
- `crates/app/src/audio/chunker.r`: Audio chunking for VAD processing
- `crates/app/src/vad/processor.rs`: VAD processing pipeline integration
- `crates/app/src/stt/processor.r`: STT processor gated by VAD
- `crates/app/src/telemetry/pipeline_metrics.rs`: Real-time metrics tracking

## Error Handling

Hierarchical error types with recovery strategies:
- `AppError`: Top-level application errors
- `AudioError`: Audio subsystem specific errors (supports all CPAL formats)
- Recovery via exponential backoff with jitter
- Watchdog monitoring for device disconnection (with proper epoch handling)
- Clean shutdown with `stop()` methods on all components

## Testing Approach

- Unit tests for individual components
- Integration tests for subsystems  
- Examples for manual testing (in `/examples/` directory)
- Probe modules in `src/probes/` for live hardware testing
- TUI dashboard (`tui_dashboard`) for real-time monitoring
- Mock traits using `mockall` for isolation
- WAV file testing for VAD validation

## Known Issues

- **Example paths**: Cargo.toml references `crates/app/examples/` but actual files are in root `/examples/` directory
- **Vosk models**: Requires Vosk model files to be downloaded separately for STT functionality
- **Device selection**: TUI dashboard device selection (-D flag) requires exact device name match