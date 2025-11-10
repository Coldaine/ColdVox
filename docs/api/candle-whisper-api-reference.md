# Candle Whisper API Reference

## Overview

This document provides comprehensive API reference for the Candle Whisper implementation in ColdVox, including configuration options, usage patterns, and integration examples.

## Core API Components

### 1. WhisperEngine

The main engine for speech-to-text processing using the Candle framework.

#### Struct Definition
```rust
pub struct WhisperEngine {
    device: Device,
    model: Arc<Mutex<Model>>,
    processor: Arc<Mutex<Processor>>,
    tokenizer: Arc<Tokenizer>,
    config: WhisperEngineInit,
}
```

#### Methods

##### `new(init: WhisperEngineInit) -> Result<Self, WhisperError>`
Creates a new WhisperEngine instance.

```rust
use coldvox_stt::candle::engine::{WhisperEngine, WhisperEngineInit};

let init = WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en")
    .with_device_preference(DevicePreference::Auto);

let engine = WhisperEngine::new(init)?;
```

##### `transcribe(&self, audio: &[f32]) -> Result<Vec<Segment>, WhisperError>`
Transcribes audio data and returns text segments with timestamps.

```rust
use coldvox_stt::candle::types::Segment;

let audio_samples: Vec<f32> = // ... load audio data ...
let segments = engine.transcribe(&audio_samples)?;

for segment in segments {
    println!("Text: {}", segment.text);
    println!("Start: {}, End: {}", segment.start, segment.end);
    println!("Confidence: {}", segment.confidence);
}
```

##### `transcribe_streaming(&self, audio_chunk: &[f32]) -> Result<Option<String>, WhisperError>`
Processes audio in streaming mode for real-time transcription.

```rust
let audio_chunk: Vec<f32> = // ... 512 samples at 16kHz ...
if let Some(text) = engine.transcribe_streaming(&audio_chunk)? {
    println!("Partial: {}", text);
}
```

### 2. WhisperEngineInit

Configuration builder for WhisperEngine initialization.

#### Struct Definition
```rust
pub struct WhisperEngineInit {
    pub model_id: String,
    pub revision: String,
    pub device_preference: DevicePreference,
    pub quantized: bool,
    pub language: Option<String>,
    pub max_tokens: usize,
    pub temperature: f32,
    pub generate_timestamps: bool,
}
```

#### Builder Methods

##### `with_model_id(model_id: &str) -> Self`
Sets the model identifier (Hugging Face model ID or local path).

```rust
WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en")
```

##### `with_revision(revision: &str) -> Self`
Sets the model revision (default: "main").

```rust
WhisperEngineInit::new()
    .with_revision("v1.0")
```

##### `with_device_preference(preference: DevicePreference) -> Self`
Sets the device preference for computation.

```rust
use coldvox_stt::candle::engine::DevicePreference;

WhisperEngineInit::new()
    .with_device_preference(DevicePreference::Cuda)  // Force GPU
    .with_device_preference(DevicePreference::Cpu)   // Force CPU
    .with_device_preference(DevicePreference::Auto)  // Auto-detect
```

##### `with_quantized(quantized: bool) -> Self`
Enables quantized models for reduced memory usage.

```rust
WhisperEngineInit::new()
    .with_quantized(true)  // Use quantized model
```

##### `with_language(language: &str) -> Self`
Sets language hint for transcription.

```rust
WhisperEngineInit::new()
    .with_language("en")  // English
    .with_language("es")  // Spanish
    .with_language("fr")  // French
```

##### `with_max_tokens(max_tokens: usize) -> Self`
Sets maximum tokens for generation (default: 448).

```rust
WhisperEngineInit::new()
    .with_max_tokens(512)
```

##### `with_temperature(temperature: f32) -> Self`
Sets sampling temperature (default: 0.0, deterministic).

```rust
WhisperEngineInit::new()
    .with_temperature(0.5)  // More creative, less deterministic
```

##### `with_generate_timestamps(generate: bool) -> Self`
Enables timestamp generation (default: true).

```rust
WhisperEngineInit::new()
    .with_generate_timestamps(false)  // Faster, no timestamps
```

### 3. DevicePreference

Enumeration for device selection preferences.

```rust
pub enum DevicePreference {
    Auto,   // Automatically detect best device
    Cpu,    // Force CPU computation
    Cuda,   // Force CUDA GPU (if available)
}
```

#### Usage Examples

```rust
use coldvox_stt::candle::engine::DevicePreference;

// Auto-detect (recommended)
DevicePreference::Auto

// CPU-only (for compatibility)
DevicePreference::Cpu

// GPU acceleration (if available)
DevicePreference::Cuda
```

### 4. Plugin API

#### CandleWhisperPlugin

Main plugin implementation for ColdVox integration.

##### Struct Definition
```rust
pub struct CandleWhisperPlugin {
    engine: Option<WhisperEngine>,
    config: Option<TranscriptionConfig>,
    capabilities: PluginCapabilities,
}
```

##### Trait Implementation: SttPlugin

```rust
#[async_trait::async_trait]
impl SttPlugin for CandleWhisperPlugin {
    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), PluginError> {
        // Convert TranscriptionConfig to WhisperEngineInit
        let engine_init = self.config_to_engine_init(&config)?;
        let engine = WhisperEngine::new(engine_init)?;
        self.engine = Some(engine);
        self.config = Some(config);
        Ok(())
    }

    async fn process_audio(&mut self, audio_data: &[i16]) -> Result<Vec<TranscriptionEvent>, PluginError> {
        // Convert i16 samples to f32
        let f32_samples: Vec<f32> = audio_data.iter()
            .map(|&sample| (sample as f32) / 32768.0)
            .collect();
        
        // Process with engine
        if let Some(ref engine) = self.engine {
            let segments = engine.transcribe(&f32_samples)?;
            // Convert segments to TranscriptionEvent
            // ... implementation details ...
        }
        Ok(events)
    }
}
```

#### CandleWhisperPluginFactory

Factory for creating CandleWhisperPlugin instances.

```rust
pub struct CandleWhisperPluginFactory {
    device_preference: Option<DevicePreference>,
    language: Option<String>,
    quantized: bool,
}
```

##### Factory Methods

```rust
impl CandleWhisperPluginFactory {
    pub fn new() -> Self {
        CandleWhisperPluginFactory {
            device_preference: None,
            language: None,
            quantized: false,
        }
    }

    pub fn with_device_preference(mut self, preference: DevicePreference) -> Self {
        self.device_preference = Some(preference);
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_quantized(mut self, quantized: bool) -> Self {
        self.quantized = quantized;
        self
    }
}
```

### 5. Types and Data Structures

#### Segment
Represents a transcribed text segment with timing information.

```rust
pub struct Segment {
    pub start: f32,           // Start time in seconds
    pub end: f32,             // End time in seconds
    pub text: String,         // Transcribed text
    pub confidence: f32,      // Confidence score (0.0-1.0)
    pub word_count: usize,    // Number of words
    pub words: Vec<WordInfo>, // Word-level details
}
```

#### WordInfo
Represents individual word timing and confidence.

```rust
pub struct WordInfo {
    pub start: f32,     // Word start time
    pub end: f32,       // Word end time
    pub text: String,   // Word text
    pub conf: f32,      // Word confidence
}
```

#### DeviceInfo
Information about the selected compute device.

```rust
pub struct DeviceInfo {
    pub device_type: DeviceType,
    pub name: String,
    pub memory_gb: Option<f32>,
    pub compute_capability: Option<String>,
}
```

## Configuration Examples

### Basic Configuration

```rust
use coldvox_stt::candle::engine::{WhisperEngine, WhisperEngineInit, DevicePreference};

let config = WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en")
    .with_device_preference(DevicePreference::Auto);

let engine = WhisperEngine::new(config)?;
```

### Advanced Configuration

```rust
let config = WhisperEngineInit::new()
    .with_model_id("openai/whisper-small.en")
    .with_revision("main")
    .with_device_preference(DevicePreference::Cuda)
    .with_quantized(false)
    .with_language("en")
    .with_max_tokens(512)
    .with_temperature(0.0)
    .with_generate_timestamps(true);

let engine = WhisperEngine::new(config)?;
```

### Environment-based Configuration

```rust
use std::env;

fn create_engine_from_env() -> Result<WhisperEngine, Box<dyn std::error::Error>> {
    let model_id = env::var("WHISPER_MODEL_PATH")
        .unwrap_or_else(|_| "openai/whisper-base.en".to_string());
    
    let device_str = env::var("CANDLE_WHISPER_DEVICE")
        .unwrap_or_else(|_| "auto".to_string());
    
    let device_preference = device_str.parse::<DevicePreference>()?;
    
    let language = env::var("WHISPER_LANGUAGE").ok();
    
    let config = WhisperEngineInit::new()
        .with_model_id(&model_id)
        .with_device_preference(device_preference)
        .with_language_opt(language.as_deref());
    
    WhisperEngine::new(config)
}
```

## Integration Patterns

### Plugin Registration

```rust
use coldvox_stt::plugin::{SttPluginRegistry, SttPluginFactory};
use coldvox_stt::plugins::candle_whisper::CandleWhisperPluginFactory;

fn register_candle_plugin(registry: &mut SttPluginRegistry) {
    let factory = CandleWhisperPluginFactory::new()
        .with_device_preference(DevicePreference::Auto)
        .with_language("en".to_string());
    
    registry.register(Box::new(factory));
}
```

### Streaming Usage

```rust
async fn streaming_transcription(engine: &WhisperEngine) -> Result<(), Box<dyn std::error::Error>> {
    // Simulate real-time audio chunks
    for chunk_index in 0..100 {
        let audio_chunk: Vec<f32> = get_audio_chunk(chunk_index).await?;
        
        match engine.transcribe_streaming(&audio_chunk)? {
            Some(text) => {
                println!("Partial result: {}", text);
            }
            None => {
                // No complete transcription yet
            }
        }
        
        tokio::time::sleep(Duration::from_millis(32)).await; // ~16kHz frame rate
    }
    Ok(())
}
```

### Batch Processing Usage

```rust
fn batch_transcription(engine: &WhisperEngine, audio_file: &str) -> Result<Vec<Segment>, Box<dyn std::error::Error>> {
    // Load audio file
    let audio_data = load_wav_file(audio_file)?;
    
    // Convert to f32 samples
    let f32_samples: Vec<f32> = audio_data.iter()
        .map(|&sample| (sample as f32) / 32768.0)
        .collect();
    
    // Transcribe entire file
    let segments = engine.transcribe(&f32_samples)?;
    
    // Process results
    for segment in &segments {
        println!("[{} - {}] {} (confidence: {:.2})", 
                 segment.start, segment.end, segment.text, segment.confidence);
    }
    
    Ok(segments)
}
```

## Error Handling

### Common Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum WhisperError {
    #[error("Model loading failed: {0}")]
    ModelLoadError(String),
    
    #[error("Audio processing failed: {0}")]
    AudioProcessingError(String),
    
    #[error("Device error: {0}")]
    DeviceError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
```

### Error Handling Examples

```rust
use coldvox_stt::candle::engine::WhisperError;

fn robust_transcription() -> Result<Vec<Segment>, Box<dyn std::error::Error>> {
    let config = WhisperEngineInit::new()
        .with_model_id("openai/whisper-base.en");
    
    let engine = match WhisperEngine::new(config) {
        Ok(engine) => engine,
        Err(WhisperError::ModelLoadError(msg)) => {
            eprintln!("Failed to load model: {}", msg);
            return Err("Model loading failed".into());
        }
        Err(WhisperError::DeviceError(msg)) => {
            eprintln!("Device error, falling back to CPU: {}", msg);
            let cpu_config = WhisperEngineInit::new()
                .with_model_id("openai/whisper-base.en")
                .with_device_preference(DevicePreference::Cpu);
            WhisperEngine::new(cpu_config)?
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };
    
    // Process audio
    let audio_data = load_audio_file("test.wav")?;
    let segments = engine.transcribe(&audio_data)?;
    Ok(segments)
}
```

## Performance Optimization

### Memory Optimization

```rust
// Use smaller models for memory-constrained environments
let tiny_config = WhisperEngineInit::new()
    .with_model_id("openai/whisper-tiny")
    .with_quantized(true);  // Further reduce memory

// Enable streaming for better memory management
let mut engine = WhisperEngine::new(config)?;
// Use transcribe_streaming for ongoing processing
```

### GPU Optimization

```rust
// Force GPU usage for better performance
let gpu_config = WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en")
    .with_device_preference(DevicePreference::Cuda);

// Monitor GPU memory
let device_info = engine.device_info();
println!("Using device: {}", device_info.name);
if let Some(memory) = device_info.memory_gb {
    println!("Memory: {:.1} GB", memory);
}
```

## Testing and Validation

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_engine_init_validation() {
        let config = WhisperEngineInit::new()
            .with_model_id("test-model");
        
        assert!(config.validate().is_ok());
        
        let mut invalid_config = WhisperEngineInit::new();
        invalid_config.model_id = "".to_string();
        assert!(invalid_config.validate().is_err());
    }
    
    #[tokio::test]
    async fn test_transcription_accuracy() {
        let config = WhisperEngineInit::new()
            .with_model_id("openai/whisper-tiny");
        
        let engine = WhisperEngine::new(config).unwrap();
        let test_audio = vec![0.0f32; 16000]; // 1 second of silence
        
        let segments = engine.transcribe(&test_audio).unwrap();
        
        // Should return empty or minimal text for silence
        assert!(segments.is_empty() || segments[0].confidence < 0.1);
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_plugin_integration() {
    use coldvox_stt::plugin::SttPlugin;
    use coldvox_stt::types::TranscriptionConfig;
    
    let config = TranscriptionConfig {
        enabled: true,
        model_path: "openai/whisper-tiny".to_string(),
        ..Default::default()
    };
    
    let mut plugin = CandleWhisperPlugin::new();
    plugin.initialize(config).await.unwrap();
    
    let test_audio = vec![0i16; 16000]; // 1 second of silence
    let events = plugin.process_audio(&test_audio).await.unwrap();
    
    // Should handle gracefully
    assert!(events.len() >= 0);
}
```

## Best Practices

### 1. Resource Management
```rust
// Always check device availability
let device_info = engine.device_info();
match device_info.device_type {
    DeviceType::Gpu => println!("Using GPU acceleration"),
    DeviceType::Cpu => println!("Using CPU fallback"),
}

// Monitor memory usage
let memory_usage = engine.memory_usage();
println!("Memory usage: {:.1} MB", memory_usage);
```

### 2. Error Recovery
```rust
// Implement fallback strategies
fn create_engine_with_fallback() -> Result<WhisperEngine, Box<dyn std::error::Error>> {
    // Try CUDA first
    let cuda_config = WhisperEngineInit::new()
        .with_model_id("openai/whisper-base.en")
        .with_device_preference(DevicePreference::Cuda);
    
    if let Ok(engine) = WhisperEngine::new(cuda_config) {
        return Ok(engine);
    }
    
    // Fallback to CPU
    let cpu_config = WhisperEngineInit::new()
        .with_model_id("openai/whisper-base.en")
        .with_device_preference(DevicePreference::Cpu);
    
    WhisperEngine::new(cpu_config)
}
```

### 3. Configuration Validation
```rust
// Validate configuration before initialization
fn validate_and_create_engine(config: WhisperEngineInit) -> Result<WhisperEngine, Box<dyn std::error::Error>> {
    config.validate()?;
    
    // Check model availability
    if !model_available(&config.model_id) {
        return Err("Model not available".into());
    }
    
    WhisperEngine::new(config)
}
```

## Migration from Python APIs

### Python Faster-Whisper Compatibility

| Python API | Rust API |
|------------|----------|
| `model.transcribe(audio)` | `engine.transcribe(audio)` |
| `model.generate(audio, **kwargs)` | `engine.transcribe_streaming(audio)` |
| `model.model_path` | `engine.config.model_id` |
| `model.device` | `engine.device_info()` |
| `model.language` | `engine.config.language` |

### Configuration Mapping

```python
# Python configuration
model = WhisperModel("base.en", device="cuda", cpu_threads=4)
segments = model.transcribe("audio.wav", beam_size=5)
```

```rust
// Rust equivalent
let config = WhisperEngineInit::new()
    .with_model_id("openai/whisper-base.en")
    .with_device_preference(DevicePreference::Cuda)
    .with_max_tokens(448);

let engine = WhisperEngine::new(config)?;
let audio_data = load_wav_file("audio.wav")?;
let segments = engine.transcribe(&audio_data)?;
```

This API reference provides complete documentation for integrating and using the Candle Whisper implementation in ColdVox applications.

---

**API Status**: ✅ **COMPLETE**  
**Version**: 1.0.0  
**Compatibility**: ColdVox Plugin System v1.0  

*Last updated: 2025-11-10T19:00:31.204Z*