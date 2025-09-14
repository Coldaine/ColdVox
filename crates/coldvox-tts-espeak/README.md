# ColdVox TTS eSpeak

eSpeak text-to-speech implementation for ColdVox.

## Overview

This crate provides an eSpeak-based implementation of the ColdVox TTS traits. eSpeak is a compact open-source software speech synthesizer that supports many languages and runs entirely locally.

## Features

- **Offline Synthesis**: No internet connection required
- **Multiple Languages**: Support for many language models through eSpeak
- **Feature Gated**: Only compiled when `espeak` feature is enabled
- **Cross-Platform**: Works on Linux, macOS, and Windows
- **Voice Customization**: Speech rate, pitch, and volume control
- **WAV Output**: Generates standard WAV audio files

## Installation

### System Requirements

**Linux (Ubuntu/Debian):**
```bash
sudo apt install espeak espeak-data
# or for newer eSpeak-NG
sudo apt install espeak-ng espeak-ng-data
```

**Linux (Arch):**
```bash
sudo pacman -S espeak-ng
```

**Linux (Fedora):**
```bash
sudo dnf install espeak espeak-ng
```

**macOS:**
```bash
brew install espeak
# or
brew install espeak-ng
```

**Windows:**
- Download from http://espeak.sourceforge.net/
- Or install eSpeak-NG from https://github.com/espeak-ng/espeak-ng

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
coldvox-tts-espeak = { path = "../coldvox-tts-espeak", features = ["espeak"] }
```

Basic usage:

```rust
use coldvox_tts_espeak::EspeakEngine;
use coldvox_tts::{TtsEngine, TtsConfig, SynthesisEvent};

// Create and initialize eSpeak engine
let mut engine = EspeakEngine::new();

// Check availability
if !engine.is_available().await {
    eprintln!("eSpeak not available - please install espeak or espeak-ng");
    return;
}

// Initialize with configuration
let config = TtsConfig {
    enabled: true,
    speech_rate: Some(180),  // Words per minute
    pitch: Some(1.0),        // Normal pitch
    volume: Some(0.8),       // 80% volume
    ..Default::default()
};

engine.initialize(config).await?;

// List available voices
let voices = engine.list_voices().await?;
println!("Available voices:");
for voice in &voices[..5] {  // Show first 5
    println!("  {}: {}", voice.id, voice.name);
}

// Synthesize speech
match engine.synthesize("Hello from ColdVox TTS!", None).await? {
    SynthesisEvent::AudioData { data, sample_rate, channels, .. } => {
        println!("Generated {} bytes of audio", data.len());
        println!("Format: {} Hz, {} channels", sample_rate, channels);
        
        // Save to file
        std::fs::write("output.wav", &data)?;
    }
    SynthesisEvent::Failed { error, .. } => {
        eprintln!("Synthesis failed: {}", error);
    }
    _ => {}
}
```

## Configuration Options

### Voice Selection

```rust
// Set default voice
engine.set_voice("en-gb").await?;

// Or specify per synthesis
let options = SynthesisOptions {
    voice: Some("fr".to_string()),
    ..Default::default()
};
```

### Speech Parameters

```rust
let options = SynthesisOptions {
    speech_rate: Some(120),  // Slower speech (80-400 WPM)
    pitch: Some(1.5),        // Higher pitch (0.5-2.0)
    volume: Some(0.9),       // Louder volume (0.0-1.0)
    ..Default::default()
};

let result = engine.synthesize("Custom speech", Some(options)).await?;
```

## Voice Support

eSpeak supports numerous languages and variants:

- **English**: en, en-gb, en-us, en-au, en-ca, etc.
- **European**: fr, de, es, it, pt, nl, sv, no, da, etc.
- **Asian**: zh, ja, ko, hi, etc.
- **Many others**: See `espeak --voices` for complete list

Voice naming follows eSpeak conventions:
- Language codes: `en`, `fr`, `de`, `es`
- Regional variants: `en-us`, `en-gb`, `fr-ca`
- Special voices: `whisper`, `croak`, etc.

## Audio Output

eSpeak generates WAV format audio with these typical characteristics:

- **Sample Rate**: 22050 Hz (default)
- **Bit Depth**: 16-bit
- **Channels**: 1 (mono)
- **Format**: Microsoft WAV format

The audio data is returned as `Vec<u8>` containing the complete WAV file, including headers.

## Performance Notes

- eSpeak is optimized for speed over quality
- Synthesis is typically 10-50x faster than real-time
- Memory usage is minimal
- CPU usage is low
- Startup time is very fast

## Limitations

- **Voice Quality**: eSpeak voices are robotic compared to neural TTS
- **No SSML**: Limited markup language support
- **No Streaming**: Each synthesis call generates complete audio
- **Process Based**: Uses subprocess calls to eSpeak binary

## Error Handling

Common errors and solutions:

```rust
use coldvox_tts::TtsError;

match result {
    Err(TtsError::EngineNotAvailable(_)) => {
        println!("Install espeak: sudo apt install espeak");
    }
    Err(TtsError::VoiceNotFound(voice)) => {
        println!("Voice {} not found. Use --voices to list available", voice);
    }
    Err(TtsError::SynthesisError(msg)) => {
        println!("Synthesis error: {}", msg);
    }
    _ => {}
}
```

## Testing

```bash
# Run unit tests
cargo test

# Test with actual eSpeak (requires espeak installed)
cargo test --features espeak

# Run example
cargo run --example basic_synthesis --features espeak
```

## Feature Flags

- `default`: Enables eSpeak functionality
- `espeak`: Core eSpeak integration (enabled by default)

## Compatibility

- **eSpeak**: Original eSpeak (1.48+)
- **eSpeak-NG**: Enhanced Next Generation fork (1.49+)
- **Platforms**: Linux, macOS, Windows
- **Rust**: 1.75+ (async/await support)

## Related Crates

- `coldvox-tts`: Core TTS abstractions and traits  
- `coldvox-app`: Main application that uses this implementation
- `coldvox-text-injection`: Text injection system with TTS integration