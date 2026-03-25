# ColdVox STT

Core speech-to-text abstraction layer for ColdVox.

## Overview

This crate provides the foundational types and traits for speech-to-text functionality in ColdVox:

- `TranscriptionEvent`: Enum representing different types of transcription results
- `TranscriptionConfig`: Configuration for STT behavior
- `WordInfo`: Word-level timing and confidence information
- `Transcriber`: Legacy trait for backward compatibility
- `EventBasedTranscriber`: Modern event-based interface for STT implementations
- `SttProcessor`: Generic VAD-gated audio processor that works with any STT implementation

## Supported Backends

| Backend | Status | Feature Flag | Description |
|---------|--------|--------------|-------------|
| **Moonshine** | ✅ Working | `moonshine` | Python-based, CPU/GPU via PyO3 |
| **Parakeet** | 🚧 Planned | `parakeet` | Pure-Rust/ONNX (CUDA/DirectML) |
| Mock | ✅ Test | - | Mock plugin for testing |
| Noop | ✅ Test | - | No-op passthrough plugin |

> **Note:** Whisper, Coqui, Leopard, and Silero-STT backends have been removed as part of the nuclear pruning initiative. Only Moonshine and Parakeet are supported.

## Usage

```rust
use coldvox_stt::{EventBasedTranscriber, TranscriptionConfig, TranscriptionEvent};

// Configure transcription
let config = TranscriptionConfig {
    enabled: true,
    model_path: "path/to/model".to_string(),
    partial_results: true,
    include_words: true,
    ..Default::default()
};

// Use with an implementation (e.g., MoonshineTranscriber)
let mut transcriber = MoonshineTranscriber::new(config, 16000.0)?;

// Process audio
match transcriber.accept_frame(&audio_samples)? {
    Some(TranscriptionEvent::Final { text, .. }) => println!("Final: {}", text),
    Some(TranscriptionEvent::Partial { text, .. }) => println!("Partial: {}", text),
    Some(TranscriptionEvent::Error { message, .. }) => eprintln!("Error: {}", message),
    None => {} // No result yet
}
```

## Default Model Path

The default model path can be configured via:
1. `MOONSHINE_MODEL_PATH` environment variable
2. Falls back to `models/moonshine/base`

## Related Crates

- `coldvox-app`: Main application using STT functionality
