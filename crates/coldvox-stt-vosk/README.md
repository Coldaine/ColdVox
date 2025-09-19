# ColdVox STT Vosk

Vosk speech recognition implementation for ColdVox STT.

## Overview

This crate provides a Vosk-based implementation of the ColdVox STT traits. Vosk is an offline speech recognition toolkit that supports many languages and runs entirely locally.

## Features

- **Offline Recognition**: No internet connection required
- **Multiple Languages**: Support for many language models
- **Feature Gated**: Only compiled when `vosk` feature is enabled
- **Event-Based Interface**: Implements modern `EventBasedTranscriber` trait
- **Backward Compatibility**: Also implements legacy `Transcriber` trait
- **Word-Level Timing**: Optional word-by-word timing information
- **Partial Results**: Real-time intermediate transcription results

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
coldvox-stt-vosk = { path = "../coldvox-stt-vosk", features = ["vosk"] }
```

Basic usage:

```rust
use coldvox_stt_vosk::{VoskTranscriber, TranscriptionConfig};

// Configure Vosk transcriber
let config = TranscriptionConfig {
    enabled: true,
    model_path: "models/vosk-model-small-en-us-0.15".to_string(),
    partial_results: true,
    max_alternatives: 1,
    include_words: true,
    ..Default::default()
};

// Create transcriber
let mut transcriber = VoskTranscriber::new(config, 16000.0)?;

// Process audio samples
match transcriber.accept_frame(&pcm_samples)? {
    Some(event) => println!("Transcription: {:?}", event),
    None => {} // No result yet
}
```

## Model Setup

- **Automatic**: Place a `vosk-model-*.zip` file in the project root or `models/` directory. The model will be automatically extracted on first run.
- **Manual**: Extract a Vosk model to `models/vosk-model-*`.

The model path is resolved as follows:
1.  `VOSK_MODEL_PATH` environment variable.
2.  Path from configuration.
3.  Autodetection of `vosk-model-*` directories in `models/` and up to 3 parent directories.
4.  Auto-extraction of `vosk-model-*.zip` files in the same locations.


## Configuration Options

- `enabled`: Enable/disable transcription
- `model_path`: Path to Vosk model directory
- `partial_results`: Enable real-time partial results
- `max_alternatives`: Number of alternative transcriptions (1-10)
- `include_words`: Include word-level timing information
- `buffer_size_ms`: Audio buffer size in milliseconds
- `auto_extract_model`: Enable/disable automatic model extraction.

## Performance Notes

- Vosk works best with 16kHz mono audio
- Larger models provide better accuracy but use more memory
- Small models (~40MB) are suitable for real-time transcription
- Large models (~1.8GB) provide highest accuracy

## Feature Gating

This crate is now enabled by default in the `coldvox-app` crate (added to `default` features), as Vosk is the primary working STT implementation in the alpha stage. This ensures real speech recognition is used by default, preventing fallback to the mock plugin that skips transcription in tests and production.

- To disable: Build without "vosk" (e.g., `cargo build --no-default-features`).
- For other backends: Enable their features (e.g., "whisper") and set `--stt-preferred whisper`.
- Rationale: Defaulting Vosk promotes robust feature testing and avoids mock as an "easy out" for skipping real work.

## Related Crates

- `coldvox-stt`: Core STT abstractions and traits
- `coldvox-app`: Main application that uses this implementation
