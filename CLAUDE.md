# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ColdVox is a Rust-based voice AI project focused on real-time audio processing with emphasis on reliability and automatic recovery. The project implements a multi-phase STT (Speech-to-Text) system with voice activity detection (VAD) and resilient audio capture using lock-free ring buffers for real-time communication.

## Architecture

### Core Components

- **Foundation Layer** (`crates/app/src/foundation/`): Error handling, health monitoring, state management, graceful shutdown
- **Audio System** (`crates/app/src/audio/`): Microphone capture, device management, watchdog monitoring, automatic recovery
  - `AudioCapture`: Device-native capture (no resampling, converts device sample format → i16)
  - `AudioChunker`: Downmixes to mono, resamples to 16 kHz, converts variable-sized frames to fixed 512-sample chunks
  - `VadAdapter`: Trait for pluggable VAD implementations
  - `VADProcessor`: VAD processing pipeline integration
- **VAD System** (`crates/app/src/vad/`): Dual VAD implementation with power-based VAD and ML models
  - `Level3VAD`: Progressive energy-based VAD implementation **[DISABLED BY Default]**
  - `SileroEngine`: Silero model wrapper for ML-based VAD **[Default ACTIVE VAD]**
  - `VADStateMachine`: State management for VAD transitions
  - `UnifiedVADConfig`: Configuration supporting both VAD modes (defaults to Silero)
- **STT System** (`crates/app/src/stt/`): Speech-to-text transcription
  - `VoskTranscriber`: Vosk-based STT implementation
  - `STTProcessor`: STT processing gated by VAD events
  - `Transcriber` trait for pluggable STT backers
- **Text Injection System** (`crates/app/src/text_injection/`): Session-based text injection
  - `TextInjector`: Production text injection using ydotool/clipboard
  - `InjectionProcessor`: Session management with silence timeout and buffering
  - `AsyncInjectionProcessor`: Async wrapper for pipeline integration
- **Telemetry** (`crates/app/src/telemetry/`): Metrics collection and monitoring
  - `PipelineMetrics`: Real-time pipeline performance metrics
  - Cross-thread monitoring of audio levels, latency, and throughput

### Threading Model

- **Mic Thread**: Owns audio device, handles capture
- **Processing Thread**: Runs VAD and chunking
- **STT Thread**: Processes speech segments when VAD detects speech
- **Main Thread**: Orchestrates and monitors components
- Communication via lock-free ring buffers (rtrb), broadcast channels, and mpsc channels

### Audio Specifications

- Internal processing format: 16 kHz, 16-bit signed (i16), mono
- Capture: Device‑native format and rate; converted to i16 only
- Chunker output: 512 samples (32 ms) at 16 kHz
- Conversion: Stereo→mono averaging and resampling happen in the chunker task
- Overflow handling: Lock‑free ring buffer backpressure with stats

### Resampler Quality

- Presets: `Fast`, `Balanced` (default), `Quality`
- Trade‑offs:
  - Fast: lowest CPU, slightly more aliasing
  - Balanced: good default balance
  - Quality: higher CPU, best stopband attenuation
- Where: set via `ChunkerConfig { resampler_quality, .. }`

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

# Run end-to-end WAV pipeline test (requires Vosk model)
VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15 \
    cargo test --features vosk test_end_to_end_wav_pipeline -- --ignored --nocapture
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

## Configuration

Configuration parameters:
- Window/overlap for audio processing (default: 500ms window, 0.5 overlap)
- VAD thresholds and debouncing (speech_threshold: 0.6, min_speech_ms: 200)
- Retry policies and timeouts (exponential backoff with jitter)
- Buffer overflow handling (DropOldest/DropNewest/Panic)
- Logging and metrics settings (JSON structured, rate-limited)

## Important Files

- `crates/app/src/main.rs`: Main application entry point with Vosk STT
- `crates/app/src/bin/tui_dashboard.rs`: Real-time monitoring dashboard
- `crates/app/src/audio/capture.rs`: Core audio capture with format negotiation
- `crates/app/src/audio/chunker.rs`: Audio chunking for VAD processing
- `crates/app/src/vad/processor.rs`: VAD processing pipeline integration
- `crates/app/src/stt/processor.rs`: STT processor gated by VAD
- `crates/app/src/text_injection/processor.rs`: Session-based text injection processor
- `crates/app/src/telemetry/pipeline_metrics.rs`: Real-time metrics tracking
- `crates/app/src/stt/tests/end_to_end_wav.rs`: End-to-end pipeline test with WAV files

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
- TUI dashboard (`tui_dashboard`) for real-time monitoring
- Mock traits using `mockall` for isolation
- WAV file testing for VAD validation
- **End-to-end pipeline testing** (`crates/app/src/stt/tests/end_to_end_wav.rs`): Complete pipeline validation using real WAV files

## Vosk Model Setup

ColdVox uses Vosk for speech-to-text transcription. A small English model is already installed:

- **Location**: `models/vosk-model-small-en-us-0.15/`
- **Environment Variable**: Set `VOSK_MODEL_PATH=models/vosk-model-small-en-us-0.15` if not using default
- **Alternative Models**: Download larger models from https://alphacephei.com/vosk/models for better accuracy

## Known Issues

- **Example paths**: Cargo.toml references `crates/app/examples/` but actual files are in root `/examples/` directory
- **Device selection**: TUI dashboard device selection (-D flag) requires exact device name match
- **Dynamic device reconfiguration**: Capture→Chunker config update is driven by frame metadata; ensure `FrameReader` is updated on device changes