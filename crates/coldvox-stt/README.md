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

## Features

- **Event-Based Architecture**: Clean separation between transcription events and implementation details
- **VAD Integration**: Built-in support for Voice Activity Detection gating
- **Flexible Configuration**: Support for partial results, word timing, alternatives, etc.
- **Backward Compatibility**: Legacy `Transcriber` trait still supported
- **Engine Agnostic**: Works with any STT implementation (Vosk, Whisper, etc.)

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

// Use with any implementation (e.g., VoskTranscriber from coldvox-stt-vosk)
let mut transcriber = SomeTranscriber::new(config, 16000.0)?;

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
1. `VOSK_MODEL_PATH` environment variable
2. Falls back to `models/vosk-model-small-en-us-0.15`

## Related Crates

- `coldvox-stt-vosk`: Vosk STT implementation (feature-gated)
- `coldvox-app`: Main application using STT functionality
