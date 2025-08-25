# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ColdVox is a Rust-based voice AI project focused on real-time audio processing with emphasis on reliability and automatic recovery. The project implements a multi-phase STT (Speech-to-Text) system with voice activity detection (VAD) and resilient audio capture.

## Architecture

### Core Components

- **Foundation Layer** (`crates/app/src/foundation/`): Error handling, health monitoring, state management, and graceful shutdown
- **Audio System** (`crates/app/src/audio/`): Microphone capture, device management, watchdog monitoring, and automatic recovery
  - `AudioCapture`: Multi-format device capture with automatic conversion
  - `AudioChunker`: Converts variable-sized frames to fixed 512-sample chunks
  - `VadAdapter`: Trait for pluggable VAD implementations
- **VAD System** (`crates/app/src/vad/`): Progressive energy-based VAD with multiple levels (Level1-4)
- **Telemetry** (`crates/app/src/telemetry/`): Metrics collection and monitoring
- **VAD Fork** (`Forks/ColdVox-voice_activity_detector/`): Voice activity detection using Silero model with ONNX runtime

### Threading Model

- **Mic Thread**: Owns audio device, handles capture
- **Processing Thread**: Runs VAD and chunking  
- **Main Thread**: Orchestrates and monitors components
- Communication via lock-free ring buffers (rtrb) and mpsc channels

### Audio Specifications

- Internal format: 16kHz, 16-bit signed (i16), mono
- Capture frames: Variable-sized (CPAL BufferSize::Default)
- Chunker output: 512 samples (32ms) for VAD processing
- Conversion: Stereo→mono averaging, rate conversion via fractional-phase resampling
- Overflow handling: Configurable policy (DropOldest/DropNewest/Panic)

## Development Commands

### Building
```bash
# Main application
cd crates/app
cargo build
cargo build --release

# Specific binary
cargo build --bin foundation_probe
cargo build --bin mic_probe
cargo build --bin vad_demo
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
```

### Running Test Binaries
```bash
# Main application
cargo run

# Development/debugging probes
cargo run --bin foundation_probe -- --duration 60
cargo run --bin mic_probe -- --duration 120 --expect-disconnect
cargo run --bin vad_demo  # Test VAD with microphone
cargo run --bin record_10s  # Record 10 seconds to WAV
cargo run --bin test_silero_minimal  # Test Silero VAD minimal implementation
cargo run --bin test_silero_wav  # Test Silero VAD with WAV files
```

### Type Checking & Linting
```bash
cargo check
cargo clippy
```

## Key Design Principles

1. **Monotonic Timing**: Use `std::time::Instant` for all durations/intervals
2. **Graceful Degradation**: Primary VAD with energy-based fallback
3. **Automatic Recovery**: Exponential backoff with jitter for reconnection
4. **Lock-free Communication**: Ring buffers (rtrb) with atomic operations
5. **Structured Logging**: Rate-limited, JSON-formatted logs with daily rotation
6. **Power-of-two Buffers**: For efficient index masking in ring buffers

## Phase Implementation Status

- **Phase 0**: Foundation & Safety Net ✅ **COMPLETE**
- **Phase 1**: Microphone Capture with Recovery ✅ **COMPLETE** (all critical bugs fixed)
- **Phase 2**: Lock-free Ring Buffer ✅ **COMPLETE** (using rtrb library)
- **Phase 3**: VAD with Fallback 📋 **IN PROGRESS** (Progressive energy VAD implemented, Silero integration pending)
- **Phase 4**: Smart Chunking 📋 **PLANNED**
- **Phase 5+**: Stress Testing & Polish 📋 **PLANNED**

## Configuration

Config structure is defined in `docs/1_foundation/EnhancedPhasePlanV2.md`. Key parameters:
- Window/overlap for audio processing (default: 500ms window, 0.5 overlap)
- VAD thresholds and debouncing (speech_threshold: 0.6, min_speech_ms: 200)
- Retry policies and timeouts (exponential backoff with jitter)
- Buffer overflow handling (DropOldest/DropNewest/Panic)
- Logging and metrics settings (JSON structured, rate-limited)

## Important Files

- `docs/PROJECT_STATUS.md`: Current project status and next steps
- `docs/1_foundation/EnhancedPhasePlanV2.md`: Complete technical specification
- `docs/4_vad/EnergyBasedVAD.md`: Energy VAD implementation details
- `crates/app/src/main.rs`: Main application entry point
- `crates/app/src/audio/capture.rs`: Core audio capture with format negotiation
- `crates/app/src/audio/chunker.rs`: Audio chunking for VAD processing
- `crates/app/src/vad/level3.rs`: Level3 energy-based VAD implementation
- `crates/app/src/foundation/state.rs`: Application state machine

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
- Test binaries for manual testing (`foundation_probe`, `mic_probe`, `vad_demo`)
- Mock traits using `mockall` for isolation
- WAV file testing for VAD validation