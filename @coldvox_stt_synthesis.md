# ColdVox TTS (Text-to-Speech) Synthesis

This document describes the TTS synthesis functionality implemented for ColdVox.

## Overview

The TTS synthesis system provides text-to-speech functionality to complement ColdVox's existing speech-to-text capabilities. It follows the same plugin-based architecture pattern used by the STT system, allowing for multiple TTS engine implementations.

## Architecture

### Core Components

- **`coldvox-tts`**: Core TTS abstraction layer with traits and types
- **`coldvox-tts-espeak`**: eSpeak TTS engine implementation
- **Engine Interface**: `TtsEngine` trait for implementing TTS backends

### Key Types

```rust
// Synthesis configuration
pub struct TtsConfig {
    pub enabled: bool,
    pub default_voice: Option<String>,
    pub speech_rate: Option<u32>,     // WPM (100-300)
    pub pitch: Option<f32>,           // 0.0-2.0 (1.0 = normal)
    pub volume: Option<f32>,          // 0.0-1.0
    pub output_format: AudioFormat,
    pub engine_options: HashMap<String, String>,
}

// Synthesis events
pub enum SynthesisEvent {
    Started { synthesis_id: u64, text: String, voice_id: String },
    AudioData { synthesis_id: u64, data: Vec<u8>, sample_rate: u32, channels: u16 },
    Completed { synthesis_id: u64, total_duration_ms: u64 },
    Failed { synthesis_id: u64, error: String },
    Cancelled { synthesis_id: u64 },
}

// Voice information
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub language: String,
    pub gender: Option<VoiceGender>,
    pub age: Option<VoiceAge>,
    pub properties: HashMap<String, String>,
}
```

## Features

### Feature Flags

- **`tts`**: Enable TTS core functionality
- **`tts-espeak`**: Enable eSpeak TTS engine implementation

### Engine Support

#### eSpeak Engine

The eSpeak engine (`coldvox-tts-espeak`) provides:

- **Multiple voices**: Supports all voices available in the system's eSpeak installation
- **Voice parameters**: Configurable speech rate, pitch, and volume
- **Language support**: Supports all languages provided by eSpeak
- **Audio output**: Generates WAV audio data (typically 22050 Hz, 16-bit mono)

**Requirements:**
- `espeak` or `espeak-ng` must be installed on the system

## Usage

### Basic Example

```rust
use coldvox_tts::{TtsEngine, TtsConfig};
use coldvox_tts_espeak::EspeakEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and initialize engine
    let mut engine = EspeakEngine::new();
    
    if !engine.is_available().await {
        eprintln!("eSpeak not available");
        return Ok(());
    }
    
    let config = TtsConfig::default();
    engine.initialize(config).await?;
    
    // List voices
    let voices = engine.list_voices().await?;
    for voice in voices.iter().take(5) {
        println!("Voice: {} ({})", voice.name, voice.id);
    }
    
    // Synthesize text
    match engine.synthesize("Hello from ColdVox TTS!", None).await? {
        SynthesisEvent::AudioData { data, sample_rate, channels, .. } => {
            // Save or play the audio data
            std::fs::write("output.wav", &data)?;
            println!("Synthesized {} bytes at {} Hz", data.len(), sample_rate);
        }
        SynthesisEvent::Failed { error, .. } => {
            eprintln!("Synthesis failed: {}", error);
        }
        _ => {}
    }
    
    engine.shutdown().await?;
    Ok(())
}
```

### Custom Synthesis Options

```rust
use coldvox_tts::SynthesisOptions;

let options = SynthesisOptions {
    speech_rate: Some(150),  // Slower speech
    pitch: Some(1.2),        // Higher pitch
    volume: Some(0.9),       // Louder volume
    ..Default::default()
};

let event = engine.synthesize("Custom synthesis options", Some(options)).await?;
```

### Voice Selection

```rust
// Set default voice for engine
engine.set_voice("en-us").await?;

// Or override for specific synthesis
let options = SynthesisOptions {
    voice: Some("en-gb".to_string()),
    ..Default::default()
};
```

## Integration with ColdVox

The TTS system integrates with ColdVox's architecture:

### Text Injection Integration

TTS synthesis can complement the text injection system by providing audio feedback:

```rust
// After successful text injection
if let Some(injected_text) = injection_processor.get_last_injection() {
    tts_engine.synthesize(&format!("Injected: {}", injected_text), None).await?;
}
```

### GUI Accessibility

TTS supports accessibility features mentioned in the GUI roadmap:

- **Status announcements**: Speak system status changes
- **Transcript reading**: Read back transcribed text
- **Error notifications**: Audio feedback for errors

### CLI Integration

The main application can include TTS options:

```bash
coldvox --tts-enabled --tts-voice en-us --tts-rate 180
```

## Examples

### Running the TTS Example

```bash
cd crates/app
cargo run --features tts-espeak,examples --example tts_synthesis_example
```

This example demonstrates:
- Engine initialization and availability checking
- Voice listing
- Basic synthesis
- Custom synthesis options
- Audio output saving

### System Requirements

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install espeak espeak-data

# Or eSpeak NG (newer version)
sudo apt install espeak-ng espeak-ng-data

# Arch Linux
sudo pacman -S espeak-ng

# Fedora
sudo dnf install espeak espeak-ng
```

**macOS:**
```bash
brew install espeak
# or
brew install espeak-ng
```

**Windows:**
- Download and install eSpeak from http://espeak.sourceforge.net/
- Or install eSpeak-NG from https://github.com/espeak-ng/espeak-ng

## Architecture Decisions

### Plugin Pattern Consistency

The TTS system follows the same plugin architecture as the STT system:
- Core abstraction crate (`coldvox-tts`)
- Engine-specific implementation crates (`coldvox-tts-espeak`)
- Feature-gated optional dependencies
- Async trait interfaces

### Audio Format

TTS engines output audio data as `Vec<u8>` containing WAV data, allowing:
- Direct playback through audio systems
- File output for testing
- Further processing if needed

### Error Handling

Comprehensive error types cover:
- Engine availability issues
- Configuration errors
- Synthesis failures
- Voice selection problems

## Future Enhancements

### Additional Engines

The architecture supports additional TTS engines:
- **Festival**: University of Edinburgh's speech synthesis system
- **Piper**: Neural text-to-speech system
- **MaryTTS**: Modular, multilingual TTS platform
- **Azure Cognitive Services**: Cloud-based TTS
- **Google Cloud Text-to-Speech**: Cloud-based TTS

### Advanced Features

- **SSML Support**: Speech Synthesis Markup Language for advanced control
- **Voice Cloning**: Custom voice training and synthesis
- **Real-time Streaming**: Streaming synthesis for long texts
- **Audio Effects**: Post-processing effects (reverb, echo, etc.)
- **Emotion Control**: Emotional expression in synthesized speech

### Integration Enhancements

- **Hotkey Triggers**: Keyboard shortcuts for TTS functions
- **Text Selection Reading**: Read selected text in applications
- **Clipboard Reading**: Speak clipboard contents
- **Document Narration**: Read entire documents or web pages

## Testing

### Unit Tests

Each TTS crate includes unit tests:
```bash
cargo test -p coldvox-tts
cargo test -p coldvox-tts-espeak
```

### Integration Tests

Test TTS integration with the main application:
```bash
cargo test --features tts-espeak
```

### Manual Testing

Use the example to test TTS functionality:
```bash
cargo run --features tts-espeak,examples --example tts_synthesis_example
```

## Troubleshooting

### Common Issues

**"eSpeak not available"**
- Ensure eSpeak or eSpeak-NG is installed
- Check that the `espeak` command is in your PATH

**"No voices found"**
- Install eSpeak language data packages
- Verify eSpeak installation with `espeak --voices`

**"Synthesis failed"**
- Check input text is not empty
- Verify selected voice exists
- Check system audio permissions

### Debugging

Enable debug logging:
```bash
RUST_LOG=debug cargo run --features tts-espeak,examples --example tts_synthesis_example
```

### Performance

TTS synthesis performance depends on:
- Text length
- eSpeak voice complexity
- System CPU performance
- Audio output buffer size

Typical performance: 1-10x real-time synthesis speed.