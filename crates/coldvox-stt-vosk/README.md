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

1. Download a Vosk model from https://alphacephei.com/vosk/models
2. Extract to `models/vosk-model-small-en-us-0.15` (or set `VOSK_MODEL_PATH`)
3. The model path can be configured via:
   - `TranscriptionConfig::model_path` field
   - `VOSK_MODEL_PATH` environment variable
   - Default: `models/vosk-model-small-en-us-0.15`

Example setup:
```bash
wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip
unzip vosk-model-small-en-us-0.15.zip
mv vosk-model-small-en-us-0.15 models/
```

## Configuration Options

- `enabled`: Enable/disable transcription
- `model_path`: Path to Vosk model directory
- `partial_results`: Enable real-time partial results
- `max_alternatives`: Number of alternative transcriptions (1-10)
- `include_words`: Include word-level timing information
- `buffer_size_ms`: Audio buffer size in milliseconds

## Performance Notes

- Vosk works best with 16kHz mono audio
- Larger models provide better accuracy but use more memory
- Small models (~40MB) are suitable for real-time transcription
- Large models (~1.8GB) provide highest accuracy

## Feature Gating

This crate uses feature gating to make Vosk optional:

- Enable with: `--features vosk`
- Without the feature, only stub functions are available
- This allows building ColdVox without speech recognition dependencies

## Related Crates

- `coldvox-stt`: Core STT abstractions and traits
- `coldvox-app`: Main application that uses this implementation
