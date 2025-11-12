# Candle Whisper Backend for ColdVox

## Overview

The Candle Whisper backend provides a pure Rust implementation of OpenAI's Whisper speech-to-text model, offering a fully local alternative to Python-based implementations. Built on the [Candle](https://github.com/huggingface/candle) machine learning framework, this backend delivers production-ready speech recognition without external dependencies.

## Key Features

- **100% Pure Rust**: No Python dependencies, compiled binaries only
- **Local Processing**: Complete offline speech recognition
- **GPU Acceleration**: Support for CUDA when available
- **Model Quantization**: Reduced memory usage with quantized models
- **Streaming Support**: Real-time transcription capabilities
- **Word-level Timestamps**: Precise timing information
- **Confidence Scores**: Quality metrics for transcription results
- **Multiple Model Sizes**: Support for tiny, base, small, medium, and large models

## Architecture

The Candle Whisper implementation consists of several key components:

```
candle/
├── engine.rs          # High-level WhisperEngine facade
├── decoder.rs         # Advanced decoding with temperature and token suppression
├── model.rs           # Model loading and building
├── audio.rs           # Audio preprocessing (mel filters, spectrograms)
├── timestamps.rs      # Timestamp extraction and segment processing
├── loader.rs          # Model artifact loading from HuggingFace
├── types.rs           # Domain types (Segment, Transcript, WordTiming)
└── decode.rs          # Token decoding and text generation
```

## Installation

### Enable the Candle Whisper Feature

Add the `candle-whisper` feature to your `Cargo.toml`:

```toml
[dependencies.coldvox-stt]
version = "0.1.0"
features = ["candle-whisper"]
```

### System Requirements

- **CPU**: Any modern x86_64 processor
- **Memory**: 2GB+ RAM (varies by model size)
- **GPU**: Optional CUDA-compatible GPU for acceleration
- **OS**: Linux, macOS, or Windows

### Model Download

The backend automatically downloads models from HuggingFace on first use, or you can specify a local path:

```rust
use coldvox_stt::candle::{WhisperEngine, WhisperEngineInit, DevicePreference};

// Download model automatically
let init = WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en");

// Or use local model
let init = WhisperEngineInit::new()
    .with_local_path("/path/to/local/model");
```

## Configuration

### Basic Configuration

```rust
use coldvox_stt::candle::{WhisperEngine, WhisperEngineInit, DevicePreference};

let init = WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en")  // Model to use
    .with_device_preference(DevicePreference::Auto)  // Auto-detect GPU
    .with_quantized(false)  // Use full precision
    .with_language("en")  // Language hint
    .with_max_tokens(448)  // Maximum tokens to generate
    .with_temperature(0.0)  // Deterministic output (0.0 = greedy)
    .with_generate_timestamps(true);  // Include timing information

let mut engine = WhisperEngine::new(init)?;
```

### Device Configuration

```rust
use coldvox_stt::candle::DevicePreference;

// Force CPU usage
let cpu_init = WhisperEngineInit::new()
    .with_device_preference(DevicePreference::Cpu);

// Force CUDA GPU
let cuda_init = WhisperEngineInit::new()
    .with_device_preference(DevicePreference::Cuda);

// Auto-select best device
let auto_init = WhisperEngineInit::new()
    .with_device_preference(DevicePreference::Auto);
```

### Model Sizes and Memory Usage

| Model | Memory | Quality | Speed |
|-------|--------|---------|-------|
| Tiny | ~150 MB | Low | Fastest |
| Base | ~300 MB | Medium | Fast |
| Small | ~900 MB | Good | Medium |
| Medium | ~2.9 GB | Better | Slow |
| Large | ~6.4 GB | Best | Slowest |

## Usage

### Basic Transcription

```rust
use coldvox_stt::candle::WhisperEngine;
use coldvox_stt::types::TranscriptionConfig;

// Initialize engine
let init = WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en");
let mut engine = WhisperEngine::new(init)?;

// Convert audio samples (i16) to f32
let audio_samples: Vec<f32> = audio_data
    .iter()
    .map(|&sample| sample as f32 / 32768.0)
    .collect();

// Transcribe
let transcript = engine.transcribe(&audio_samples)?;

for segment in transcript.segments {
    println!("{:.2}s - {:.2}s: {}", segment.start, segment.end, segment.text);
    if let Some(ref words) = segment.words {
        for word in words {
            println!("  Word: '{}' (confidence: {:.2})", word.text, word.confidence);
        }
    }
}
```

### Plugin Integration

```rust
use coldvox_stt::plugins::candle_whisper::{CandleWhisperPlugin, CandleWhisperPluginFactory};
use coldvox_stt::types::TranscriptionConfig;
use coldvox_stt::plugin::SttPluginRegistry;

// Create plugin factory
let factory = CandleWhisperPluginFactory::new()
    .with_device_preference(DevicePreference::Auto)
    .with_language("en".to_string());

// Register with plugin registry
let mut registry = SttPluginRegistry::new();
registry.register(Box::new(factory));

// Create plugin instance
let mut plugin = registry.create_plugin("candle-whisper")?;

// Initialize with configuration
let config = TranscriptionConfig {
    enabled: true,
    model_path: "openai/whisper-base.en".to_string(),
    include_words: true,
    streaming: true,
    ..Default::default()
};

plugin.initialize(config).await?;

// Process audio
let audio_samples = vec![1000i16, 2000i16, 3000i16]; // Example data
let result = plugin.process_audio(&audio_samples).await?;
```

### Streaming Transcription

```rust
use coldvox_stt::StreamingStt;
use coldvox_stt::plugin::PluginAdapter;

// Wrap plugin in adapter for streaming interface
let plugin = registry.create_plugin("candle-whisper")?;
let mut stt = PluginAdapter::new(plugin);

// Simulate streaming audio processing
for chunk in audio_chunks {
    if let Some(event) = stt.on_speech_frame(&chunk).await {
        match event {
            TranscriptionEvent::Partial { text, .. } => {
                println!("Partial: {}", text);
            }
            TranscriptionEvent::Final { text, words, .. } => {
                println!("Final: {}", text);
                if let Some(word_list) = words {
                    for word in word_list {
                        println!("  Word: '{}' ({:.2}s - {:.2}s)", 
                                word.text, word.start, word.end);
                    }
                }
            }
            _ => {}
        }
    }
}

// Signal end of speech
if let Some(final_event) = stt.on_speech_end().await {
    // Handle final result
}
```

## Environment Variables

Configure behavior using environment variables:

```bash
# Device selection
export CANDLE_WHISPER_DEVICE="auto"  # auto, cpu, cuda

# Model configuration
export WHISPER_MODEL_PATH="/path/to/model"  # Local model path
export WHISPER_LANGUAGE="en"  # Language hint

# Engine configuration
export WHISPER_MAX_TOKENS=448
export WHISPER_TEMPERATURE=0.0
export WHISPER_QUANTIZED=false
```

## API Reference

### WhisperEngine

The main entry point for Candle Whisper functionality.

```rust
pub struct WhisperEngine {
    // Internal components (private)
}

impl WhisperEngine {
    /// Create new engine with configuration
    pub fn new(init: WhisperEngineInit) -> Result<Self, WhisperEngineError>;
    
    /// Transcribe audio samples
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<Transcript, WhisperEngineError>;
    
    /// Get device information
    pub fn device_info(&self) -> &DeviceInfo;
    
    /// Get underlying decoder for advanced configuration
    pub fn decoder(&self) -> &Decoder;
    
    /// Update suppression tokens at runtime
    pub fn update_suppression(&mut self, tokens: HashSet<u32>) -> Result<(), ColdVoxError>;
    
    /// Update temperature at runtime
    pub fn update_temperature(&mut self, temperature: f32);
    
    /// Get sample rate
    pub fn sample_rate(&self) -> u32;
    
    /// Get audio processing configuration
    pub fn audio_config(&self) -> &WhisperAudioConfig;
}
```

### WhisperEngineInit

Configuration builder for engine initialization.

```rust
#[derive(Debug, Clone)]
pub struct WhisperEngineInit {
    pub model_id: String,
    pub revision: String,
    pub local_path: Option<PathBuf>,
    pub device_preference: DevicePreference,
    pub quantized: bool,
    pub language: Option<String>,
    pub max_tokens: usize,
    pub temperature: f32,
    pub generate_timestamps: bool,
}

impl WhisperEngineInit {
    pub fn new() -> Self;
    pub fn with_model_id<S: Into<String>>(self, model_id: S) -> Self;
    pub fn with_device_preference(self, preference: DevicePreference) -> Self;
    pub fn with_quantized(self, quantized: bool) -> Self;
    pub fn with_language<S: Into<String>>(self, language: S) -> Self;
    // ... other builder methods
    pub fn validate(&self) -> Result<(), ColdVoxError>;
}
```

### DevicePreference

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DevicePreference {
    Cpu,      // Force CPU
    Cuda,     // Force CUDA GPU
    Auto,     // Auto-select (default)
}
```

### Transcript and Segments

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub segments: Vec<Segment>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub start: f32,        // Start time in seconds
    pub end: f32,          // End time in seconds
    pub text: String,      // Transcribed text
    pub confidence: f32,   // Confidence score (0.0-1.0)
    pub word_count: usize, // Number of words
    pub words: Option<Vec<WordTiming>>, // Word-level timing
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordTiming {
    pub text: String,      // Word text
    pub start: f32,        // Start time
    pub end: f32,          // End time
    pub confidence: f32,   // Confidence score
}
```

## Performance Optimization

### Memory Management

```rust
// Use quantized models for reduced memory usage
let init = WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en")
    .with_quantized(true);

// Monitor memory usage
let memory_usage = engine.estimate_memory_usage()?;
println!("Estimated memory usage: {} MB", memory_usage);
```

### GPU Acceleration

```rust
// Enable CUDA when available
let init = WhisperEngineInit::new()
    .with_device_preference(DevicePreference::Auto);  // Will use CUDA if available

// Check device info
let device_info = engine.device_info();
if device_info.is_cuda {
    println!("Using GPU: {}", device_info.device_type);
}
```

### Batch Processing

```rust
// Process large audio files in chunks
fn process_large_audio(engine: &mut WhisperEngine, audio_data: &[i16], chunk_size: usize) -> Result<Vec<Transcript>> {
    let mut results = Vec::new();
    
    for chunk in audio_data.chunks(chunk_size) {
        let f32_chunk: Vec<f32> = chunk.iter()
            .map(|&sample| sample as f32 / 32768.0)
            .collect();
        
        let transcript = engine.transcribe(&f32_chunk)?;
        results.push(transcript);
    }
    
    Ok(results)
}
```

## Error Handling

```rust
use coldvox_stt::candle::WhisperEngineError;

match WhisperEngine::new(init) {
    Ok(mut engine) => {
        match engine.transcribe(&audio_samples) {
            Ok(transcript) => {
                // Handle successful transcription
                for segment in transcript.segments {
                    println!("{}", segment.text);
                }
            }
            Err(WhisperEngineError::TranscriptionFailed(msg)) => {
                eprintln!("Transcription failed: {}", msg);
            }
            Err(WhisperEngineError::AudioProcessing(msg)) => {
                eprintln!("Audio processing error: {}", msg);
            }
            Err(e) => {
                eprintln!("Engine error: {}", e);
            }
        }
    }
    Err(WhisperEngineError::Config(e)) => {
        eprintln!("Configuration error: {}", e);
    }
    Err(WhisperEngineError::Model(e)) => {
        eprintln!("Model loading error: {}", e);
    }
    Err(e) => {
        eprintln!("Initialization error: {}", e);
    }
}
```

## Troubleshooting

### Common Issues

1. **CUDA not detected**
   - Ensure CUDA drivers are installed
   - Check `nvidia-smi` to verify GPU availability
   - Fall back to CPU with `DevicePreference::Cpu`

2. **Model download fails**
   - Check internet connection
   - Verify HuggingFace access
   - Use local model path with `with_local_path()`

3. **Out of memory**
   - Use smaller model (tiny, base, small)
   - Enable quantization with `with_quantized(true)`
   - Process audio in smaller chunks

4. **Poor transcription quality**
   - Check audio sample rate (should be 16kHz)
   - Verify audio normalization
   - Try different model sizes
   - Adjust temperature setting

### Debug Logging

Enable detailed logging:

```rust
use tracing::{info, warn, error};

// Enable tracing subscriber
tracing_subscriber::fmt::init();

// Check device information
let device_info = engine.device_info();
info!("Using device: {}", device_info.device_type);
info!("CUDA enabled: {}", device_info.is_cuda);
info!("Quantized: {}", device_info.is_quantized);
```

## Integration Examples

### With ColdVox Audio Pipeline

```rust
use coldvox_audio::{AudioCapture, RingBuffer};
use coldvox_stt::candle::WhisperEngine;

// Set up audio capture
let mut audio_capture = AudioCapture::new()?;
let mut ring_buffer = RingBuffer::new(16000 * 10); // 10 seconds buffer

// Initialize Whisper engine
let init = WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en")
    .with_generate_timestamps(true);
let mut engine = WhisperEngine::new(init)?;

// Process audio in real-time
loop {
    if let Some(audio_chunk) = audio_capture.read_chunk()? {
        ring_buffer.write(&audio_chunk);
        
        // Process when we have enough audio
        if ring_buffer.len() >= 16000 { // 1 second at 16kHz
            let mut samples = vec![0.0f32; 16000];
            ring_buffer.read_exact(&mut samples);
            
            match engine.transcribe(&samples) {
                Ok(transcript) => {
                    for segment in transcript.segments {
                        println!("{}", segment.text);
                    }
                }
                Err(e) => eprintln!("Transcription error: {}", e),
            }
        }
    }
}
```

### Custom Audio Preprocessing

```rust
use coldvox_stt::candle::audio::{pcm_to_mel, mel_filters, WhisperAudioConfig};

fn preprocess_audio(audio_data: &[i16], config: &WhisperAudioConfig) -> Result<Vec<f32>, ColdVoxError> {
    // Convert i16 to f32 and normalize
    let f32_data: Vec<f32> = audio_data
        .iter()
        .map(|&sample| (sample as f32) / 32768.0)
        .collect();
    
    // Apply pre-emphasis filter
    let mut pre_emphasized = vec![0.0; f32_data.len()];
    pre_emphasized[0] = f32_data[0];
    for i in 1..f32_data.len() {
        pre_emphasized[i] = f32_data[i] - 0.97 * f32_data[i-1];
    }
    
    // Convert to mel spectrogram
    let filters = mel_filters(config.num_mel_bins)?;
    let mel_spectrogram = pcm_to_mel(config, &pre_emphasized, &filters)?;
    
    // Flatten for input to Whisper
    Ok(mel_spectrogram)
}
```

## Contributing

The Candle Whisper backend is part of the ColdVox project. See the main project contributing guidelines for information on how to contribute to this implementation.

## License

This implementation is part of the ColdVox project and inherits its license. See the main project LICENSE file for details.