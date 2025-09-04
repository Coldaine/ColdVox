# coldvox-audio

Audio capture, processing, and device management for ColdVox.

## Purpose

This crate handles all audio-related functionality in the ColdVox pipeline:

- **Audio Capture**: Real-time microphone input with device enumeration and selection
- **Audio Processing**: Format conversion, resampling, and channel mixing
- **Ring Buffers**: Lock-free audio buffering with backpressure handling
- **Device Management**: Audio device detection, configuration, and recovery
- **Frame Processing**: Chunking audio into fixed-size frames for downstream processing

## Key Components

### AudioCapture
- Cross-platform microphone capture using CPAL
- Automatic device recovery and error handling
- Configurable sample rates and formats

### AudioChunker
- Converts multi-channel audio to mono
- Resamples to target rate (typically 16kHz)
- Emits fixed-size frames (512 samples by default)
- Handles format conversions (f32 → i16)

### AudioRingBuffer
- Lock-free ring buffer for audio data
- Backpressure detection and metrics
- Thread-safe producer/consumer pattern

## API Overview

```rust
use coldvox_audio::{AudioCapture, AudioChunker, AudioRingBuffer};

// Set up audio capture pipeline
let capture = AudioCapture::new(device_config)?;
let ring_buffer = AudioRingBuffer::new(buffer_size);
let chunker = AudioChunker::new(chunker_config);
```

## Features

- `default`: Standard audio processing functionality

## Dependencies

- `cpal`: Cross-platform audio I/O
- `dasp`: Digital signal processing utilities
- `rubato`: High-quality resampling
- `rtrb`: Realtime-safe ring buffer
- `parking_lot`: Efficient synchronization primitives
