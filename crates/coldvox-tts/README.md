# ColdVox TTS

Core text-to-speech abstraction layer for ColdVox.

## Overview

This crate provides the foundational types and traits for text-to-speech functionality in ColdVox:

- `SynthesisEvent`: Enum representing different types of synthesis results
- `TtsConfig`: Configuration for TTS behavior  
- `VoiceInfo`: Voice metadata including language, gender, and properties
- `TtsEngine`: Core trait for TTS implementations
- `TextToSpeechSynthesizer`: High-level synthesizer interface

## Features

- **Event-Based Architecture**: Clean separation between synthesis events and implementation details
- **Multiple Engines**: Support for different TTS backends (eSpeak, Festival, etc.)
- **Voice Management**: Comprehensive voice discovery and selection
- **Flexible Configuration**: Support for speech rate, pitch, volume, and format options
- **Async Interface**: Non-blocking synthesis operations
- **Engine Agnostic**: Works with any TTS implementation

## Usage

```rust
use coldvox_tts::{TtsEngine, TtsConfig, SynthesisEvent};

// Configure TTS
let config = TtsConfig {
    enabled: true,
    default_voice: Some("en-us".to_string()),
    speech_rate: Some(180),
    pitch: Some(1.0),
    volume: Some(0.8),
    ..Default::default()
};

// Use with any implementation (e.g., EspeakEngine from coldvox-tts-espeak)
let mut engine = SomeEngine::new();
engine.initialize(config).await?;

// Synthesize text
match engine.synthesize("Hello from ColdVox TTS!", None).await? {
    SynthesisEvent::AudioData { data, sample_rate, channels, .. } => {
        // Process or save audio data
        println!("Generated {} bytes at {} Hz", data.len(), sample_rate);
    }
    SynthesisEvent::Failed { error, .. } => {
        eprintln!("Synthesis failed: {}", error);
    }
    _ => {}
}
```

## Audio Formats

TTS engines output audio data in various formats:

- `AudioFormat::Wav16bit`: 16-bit WAV format (most common)
- `AudioFormat::Wav8bit`: 8-bit WAV format
- `AudioFormat::Raw16bit`: Raw 16-bit PCM data
- `AudioFormat::Raw8bit`: Raw 8-bit PCM data

## Voice Selection

```rust
// List available voices
let voices = engine.list_voices().await?;
for voice in voices {
    println!("{}: {} ({})", voice.id, voice.name, voice.language);
}

// Set voice
engine.set_voice("en-gb").await?;

// Or override for specific synthesis
let options = SynthesisOptions {
    voice: Some("fr-fr".to_string()),
    speech_rate: Some(120),
    ..Default::default()
};
```

## Synthesis Options

Customize synthesis on a per-request basis:

```rust
let options = SynthesisOptions {
    voice: Some("en-us".to_string()),      // Voice override
    speech_rate: Some(150),                 // Words per minute
    pitch: Some(1.2),                      // Pitch multiplier (0.5-2.0)
    volume: Some(0.9),                     // Volume (0.0-1.0)
    high_priority: true,                   // Interrupt current synthesis
};

let event = engine.synthesize("Priority message", Some(options)).await?;
```

## Error Handling

Comprehensive error types cover common TTS scenarios:

```rust
use coldvox_tts::TtsError;

match result {
    Err(TtsError::EngineNotAvailable(msg)) => {
        eprintln!("TTS engine not found: {}", msg);
    }
    Err(TtsError::VoiceNotFound(voice)) => {
        eprintln!("Voice '{}' not available", voice);
    }
    Err(TtsError::SynthesisError(msg)) => {
        eprintln!("Synthesis failed: {}", msg);
    }
    _ => {}
}
```

## Features

- `default`: Include core functionality
- `async`: Enable async/await support (requires tokio)

## Related Crates

- `coldvox-tts-espeak`: eSpeak TTS implementation
- `coldvox-app`: Main application using TTS functionality
- `coldvox-text-injection`: Text injection system that can integrate with TTS