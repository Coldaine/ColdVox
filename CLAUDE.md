# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ColdVox is a Rust-based voice AI project focused on real-time audio processing with emphasis on reliability and automatic recovery. The project implements a multi-phase STT (Speech-to-Text) system with voice activity detection (VAD) and resilient audio capture.

## Architecture

### Core Components

- **Foundation Layer** (`crates/app/src/foundation/`): Error handling, health monitoring, state management, and graceful shutdown
- **Audio System** (`crates/app/src/audio/`): Microphone capture, device management, watchdog monitoring, and automatic recovery
- **Telemetry** (`crates/app/src/telemetry/`): Metrics collection and monitoring
- **VAD Fork** (`Forks/ColdVox-voice_activity_detector/`): Voice activity detection using Silero model with ONNX runtime

### Threading Model

- **Mic Thread**: Owns audio device, handles capture
- **Processing Thread**: Runs VAD and chunking  
- **Main Thread**: Orchestrates and monitors components
- Communication via lock-free ring buffers and mpsc channels

### Audio Specifications

- Internal format: 16kHz, 16-bit signed (i16), mono
- Frame size: 320 samples (20ms)
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
```

### Running
```bash
# Main application
cargo run

# Test probes (development/debugging)
cargo run --bin foundation_probe -- --duration 60
cargo run --bin mic_probe -- --duration 120 --expect-disconnect
```

### Type Checking
```bash
cargo check
cargo clippy  # For linting
```

## Key Design Principles

1. **Monotonic Timing**: Use `std::time::Instant` for all durations/intervals
2. **Graceful Degradation**: Primary VAD with energy-based fallback
3. **Automatic Recovery**: Exponential backoff with jitter for reconnection
4. **Lock-free Communication**: Ring buffers with atomic operations
5. **Structured Logging**: Rate-limited, JSON-formatted logs
6. **Power-of-two Buffers**: For efficient index masking

## Phase Implementation Status

- **Phase 0**: Foundation & Safety Net ✓
- **Phase 1**: Microphone Capture with Recovery ✓ (with known critical bugs)
- **Phase 2**: Lock-free Ring Buffer ✓ (using rtrb library)
- **Phase 3**: VAD with Fallback (Planned)
- **Phase 4**: Smart Chunking (Planned)

### Critical Issues Requiring Immediate Attention

1. **Watchdog Timer Logic Error** - Timer cannot detect timeouts due to epoch mismatch
2. **CPAL Sample Format Hardcoding** - Fails on devices not supporting i16 format
3. **Channel Negotiation Failure** - Forces mono, fails on stereo-only devices
4. **Missing Stop/Cleanup Methods** - Violates clean shutdown requirements

## Configuration

Config structure is defined in `EnhancedPhasePlanV2.md`. Key parameters:
- Window/overlap for audio processing
- VAD thresholds and debouncing
- Retry policies and timeouts
- Buffer overflow handling
- Logging and metrics settings

## Important Files

- `docs/EnhancedPhasePlanV2.md`: Complete technical specification
- `docs/Phase0_Phase1_Design.md`: Foundation and audio capture design
- `crates/app/src/main.rs`: Main application entry point
- `crates/app/src/audio/capture.rs`: Core audio capture implementation
- `crates/app/src/foundation/state.rs`: Application state machine

## Error Handling

Hierarchical error types with recovery strategies:
- `AppError`: Top-level application errors
- `AudioError`: Audio subsystem specific errors
- Recovery via exponential backoff with jitter
- Watchdog monitoring for device disconnection

## Testing Approach

- Unit tests for individual components
- Integration tests for subsystems
- Test probes (`foundation_probe`, `mic_probe`) for manual testing
- Mock traits using `mockall` for isolation