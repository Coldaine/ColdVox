---
id: IMP-001
title: Audio Capture Module Implementation
level: implementation
status: drafting
owners:
  - CDIS
criticality: 3
parent: SYS-001
pillar_trace:
  - PIL-001
  - DOM-001
  - SUB-001
  - SYS-001
implements:
  - "SPEC-001"
---

# Audio Capture Module Implementation [IMP-001]

## 1. Overview

This document describes the implementation of the Audio Service System, as defined in [SPEC-001](SPEC-001-audio-capture-spec.md). The core logic is located in the `coldvox-audio` crate.

## 2. Code-level Traceability

This implementation directly maps to the following source code files and symbols:

-   **Primary Capture Logic**: `CODE:repo://crates/coldvox-audio/src/capture.rs#symbol=AudioCaptureThread`
-   **Device Management**: `CODE:repo://crates/coldvox-audio/src/device.rs`
-   **Audio Chunking**: `CODE:repo://crates/coldvox-audio/src/chunker.rs#symbol=AudioChunker`
-   **Ring Buffer**: `CODE:repo://crates/coldvox-audio/src/ring_buffer.rs#symbol=AudioRingBuffer`

## 3. Key Components

### `AudioCaptureThread`

This is the main struct that manages the dedicated audio capture thread.

**Key functions:**
- `spawn()`: The entry point that creates and starts the thread. It takes ownership of the CPAL stream and the ring buffer producer.

```rust
// From: crates/coldvox-audio/src/capture.rs
pub struct AudioCaptureThread;

impl AudioCaptureThread {
    pub fn spawn(
        stream: cpal::Stream,
        mut producer: rtrb::Producer<i16>,
        watchdog: Arc<Watchdog>,
    ) -> Self {
        // ... implementation ...
    }
}
```

### `AudioChunker`

This component is responsible for all DSP tasks required to prepare the audio for the VAD and STT engines.

**Key functions:**
- `process_input()`: Takes raw f32 samples, resamples them to 16kHz, converts them to mono i16 samples, and groups them into 512-sample frames.

## 4. Dependencies

-   `cpal`: For cross-platform audio I/O.
-   `rubato`: For high-quality audio resampling.
-   `rtrb`: For the lock-free ring buffer implementation.
