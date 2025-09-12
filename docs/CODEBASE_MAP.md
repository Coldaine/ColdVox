# Codebase Map (Initial Draft: Audio Components)

This document provides a high-level overview of the ColdVox codebase, with an initial focus on the audio processing pipeline.

## Directory Structure

- `crates/` - The main workspace for all Rust crates.
  - `app/` - The main application crate that integrates all other components.
    - `src/main.rs` - Main application entry point.
  - `coldvox-audio/` - Crate responsible for all audio capture and processing.
    - `src/capture.rs` - Manages the dedicated audio capture thread.
    - `src/device.rs` - Handles audio device discovery and management.
    - `src/chunker.rs` - Segments the audio stream into fixed-size frames.
    - `src/resampler.rs` - Resamples audio to the target frequency (16kHz).
    - `src/ring_buffer.rs` - Implements the lock-free buffer for audio data.
    - `tests/` - Integration tests for the audio crate.
  - `coldvox-vad/` - Voice Activity Detection (VAD) core traits.
  - `coldvox-vad-silero/` - Default Silero VAD implementation.
  - `coldvox-stt/` - Speech-To-Text (STT) core abstractions.
  - `coldvox-stt-vosk/` - Vosk STT implementation.

## Core Components

- **Main Entry Point**: `crates/app/src/main.rs`
- **Primary Business Domain (Pilot)**: Voice Processing
- **Technical Systems (Audio)**:
  - **Audio Service**: Manages the entire audio pipeline from capture to framing. Implemented in `coldvox-audio`.
  - **VAD Service**: Detects speech in the audio stream. Implemented in `coldvox-vad` and `coldvox-vad-silero`.
  - **STT Service**: Transcribes speech to text. Implemented in `coldvox-stt` and `coldvox-stt-vosk`.

## Data Flows (Audio Pipeline)

1.  **`main.rs`** initializes the `AudioCapture` system from `coldvox-audio`.
2.  **`capture.rs`** spawns a dedicated thread that reads raw audio samples from the selected microphone device via `cpal`.
3.  The raw audio is passed to the **`chunker.rs`** and **`resampler.rs`** to be converted into standardized 16kHz, mono, 16-bit PCM frames.
4.  These frames are pushed into a lock-free **`ring_buffer.rs`**.
5.  Downstream consumers (like the VAD engine in `coldvox-vad`) read the frames from the ring buffer for processing.

## Architectural Patterns

- **Monorepo**: The codebase is structured as a Cargo workspace with multiple interdependent crates.
- **Dedicated Thread for I/O**: Audio capture runs on a dedicated thread to avoid blocking the main application logic, communicating via a lock-free ring buffer. This is a common pattern for real-time applications.
- **Plugin-based Architecture (for STT/VAD)**: The system uses traits (`VadEngine`) to allow for different backends to be used for key functionalities like VAD.
