# STT Plugin Architecture

## Overview

ColdVox uses a plugin-based architecture for Speech-to-Text (STT) functionality, allowing seamless switching between different STT backends without requiring system dependencies for unused plugins. While the architecture is designed for modularity, the current practical implementation is focused on the single `vosk` plugin as part of the default pipeline.

## Motivation

Previously, ColdVox had a hard dependency on Vosk, which required:
- System-level `libvosk` library installation
- Large model downloads (500MB+)
- Complex CI/CD setup
- Build failures when libvosk wasn\'t available

The plugin architecture solves these issues by:
- Making STT backends optional and pluggable
- Allowing runtime backend selection
- Providing fallback mechanisms
- Enabling easy testing without heavy dependencies

## Architecture

### Core Components

#### 1. **SttPlugin Trait** (`coldvox-stt/src/plugin.rs`)
The main interface all STT plugins must implement:

```rust
#[async_trait]
pub trait SttPlugin: Send + Sync + Debug {
    fn info(&self) -> PluginInfo;
    fn capabilities(&self) -> PluginCapabilities;
    async fn is_available(&self) -> Result<bool, SttPluginError>;
    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), SttPluginError>;
    async fn process_audio(&mut self, samples: &[i16]) -> Result<Option<TranscriptionEvent>, SttPluginError>;
    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError>;
    async fn reset(&mut self) -> Result<(), SttPluginError>;
}
```

#### 2. **Plugin Registry** (`SttPluginRegistry`)
Manages available plugins and handles selection:
- Registers plugin factories
- Checks plugin availability
- Creates plugin instances
- Implements fallback logic

#### 3. **Plugin Manager** (`crates/app/src/stt/plugin_manager.rs`)
High-level manager for the application:
- Initializes plugins at startup
- Handles plugin switching
- Manages plugin lifecycle
- Provides unified API to the app

### Built-in Plugins

#### NoOp Plugin
- **Purpose**: Fallback when no STT is available
- **Dependencies**: None
- **Use case**: Testing, audio-only processing

#### Mock Plugin
- **Purpose**: Testing and development
- **Dependencies**: None
- **Features**: Configurable responses, delays, failures

#### Vosk Plugin
- **Purpose**: Offline speech recognition
- **Dependencies**: `libvosk` system library
- **Features**: Multiple languages, word timestamps
- **Status**: Default, feature-gated

#### Potential Future Plugins
- **Whisper**: Local AI-based STT
- **Google Cloud STT**: Cloud-based recognition
- **Azure Speech**: Microsoft\'s cloud STT
- **OpenAI Whisper API**: Cloud-based Whisper

## Usage

### Basic Usage

```rust
use coldvox_app::stt::plugin_manager::SttPluginManager;

// Create plugin manager
let mut manager = SttPluginManager::new();

// Initialize with best available plugin
let plugin_id = manager.initialize().await?;
println!("Using STT plugin: {}", plugin_id);

// Process audio
let transcription = manager.process_audio(&audio_samples).await?;
```

### Plugin Selection

```rust
// Configure plugin preferences
let mut config = PluginSelectionConfig {
    preferred_plugin: Some("vosk".to_string()),
    fallback_plugins: vec!["whisper".to_string(), "mock".to_string()],
    require_local: true,  // No cloud services
    max_memory_mb: Some(1000),
    required_language: Some("en".to_string()),
};

manager.set_selection_config(config);
manager.initialize().await?;
```

### Runtime Plugin Switching

```rust
// List available plugins
let plugins = manager.list_plugins().await;
for plugin in plugins {
    println!("{}: {} (Available: {})",
        plugin.id, plugin.description, plugin.is_available);
}

// Switch to a different plugin
manager.switch_plugin("whisper").await?;
```

## Creating New Plugins

### Step 1: Implement the Plugin

```rust
use async_trait::async_trait;
use coldvox_stt::plugin::*;

#[derive(Debug)]
struct MyCustomPlugin {
    // Plugin state
}

#[async_trait]
impl SttPlugin for MyCustomPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "custom".to_string(),
            name: "My Custom STT".to_string(),
            // ...
        }
    }

    async fn process_audio(&mut self, samples: &[i16]) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        // Your STT logic here
    }

    // ... implement other required methods
}
```

### Step 2: Create a Factory

```rust
struct MyCustomPluginFactory;

impl SttPluginFactory for MyCustomPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        Ok(Box::new(MyCustomPlugin::new()))
    }

    fn check_requirements(&self) -> Result<(), SttPluginError> {
        // Check if your plugin\'s dependencies are available
    }
}
```

### Step 3: Register the Plugin

```rust
// In plugin_manager.rs
registry.register(Box::new(MyCustomPluginFactory));
```

## Feature Flags

Control which plugins are compiled:

```toml
[features]
default = ["silero", "vosk", "text-injection"]  # Vosk is now part of the default pipeline
vosk = ["dep:coldvox-stt-vosk"]
whisper = ["dep:whisper-rs"]
all-stt = ["vosk", "whisper"]           # Enable all STT backends
```

## Testing

### Unit Tests
Use the Mock plugin for deterministic testing:

```rust
#[test]
async fn test_transcription_pipeline() {
    let plugin = MockPlugin::with_transcription("Hello world".to_string());
    let result = plugin.process_audio(&audio).await.unwrap();
    assert_eq!(result.unwrap().text, "Hello world");
}
```

### Integration Tests
Test with NoOp plugin to avoid dependencies:

```rust
#[test]
async fn test_audio_pipeline_without_stt() {
    let plugin = NoOpPlugin::new();
    // Test that audio flows through even without transcription
}
```

### Feature Testing
The testing framework now works without STT dependencies:

```bash
# Test without any STT
./test-features.py -p coldvox-app --strategy default-only

# Test with Vosk enabled (requires libvosk)
./test-features.py -p coldvox-app -- --features vosk
```

## Benefits

### For Developers
- ✅ **No mandatory system dependencies**: Build and test without libvosk
- ✅ **Faster CI/CD**: No need to install STT libraries for unrelated tests
- ✅ **Easy testing**: Use Mock plugin for predictable tests
- ✅ **Modular development**: Work on VAD without STT dependencies

### For Users
- ✅ **Choice**: Select the STT backend that fits their needs
- ✅ **Graceful degradation**: Falls back to available options
- ✅ **Privacy options**: Choose between local and cloud STT
- ✅ **Performance tuning**: Select based on speed/accuracy tradeoffs

### For the Project
- ✅ **Future-proof**: Easy to add new STT backends
- ✅ **Maintainable**: Clear separation of concerns
- ✅ **Testable**: Comprehensive testing without heavy dependencies
- ✅ **Flexible**: Runtime configuration and switching

## Migration Guide

### From Hard-coded Vosk

**Before** (hard dependency):
```rust
let transcriber = VoskTranscriber::new(config)?;
let result = transcriber.process(audio)?;
```

**After** (plugin-based):
```rust
let mut manager = SttPluginManager::new();
manager.initialize().await?;
let result = manager.process_audio(audio).await?;
```

### Feature Flags

**Before**:
```toml
[features]
default = ["silero", "text-injection", "vosk"]  # Vosk always required
```

**After**:
```toml
[features]
default = ["silero", "vosk", "text-injection"]  # Vosk is now part of the default pipeline
vosk = ["dep:coldvox-stt-vosk"]
```

## Troubleshooting

### No STT Available
```
Warning: No STT plugins available, using NoOp plugin
```
**Solution**: Install an STT backend or explicitly configure NoOp for audio-only processing.

### Vosk Not Found
```
Error: Plugin \'vosk\' not available: libvosk not found on system
```
**Solution**: Install libvosk or use a different STT backend.

### Performance Issues
- Check plugin capabilities and memory usage
- Consider using streaming mode if supported
- Adjust buffer sizes in TranscriptionConfig

## Future Enhancements

### Planned Features
1. **Dynamic plugin loading**: Load plugins from external libraries
2. **Plugin marketplace**: Community-contributed plugins
3. **Hybrid mode**: Use multiple plugins simultaneously
4. **Quality metrics**: Automatic plugin selection based on performance
5. **Caching layer**: Cache transcriptions across plugins

### Potential Plugins
- **Whisper.cpp**: Lightweight C++ Whisper implementation
- **SpeechRecognition API**: Web browser speech API
- **Watson STT**: IBM\'s speech recognition
- **Kaldi**: Open-source speech recognition toolkit
- **DeepSpeech**: Mozilla\'s neural network STT

## Conclusion

The plugin architecture transforms ColdVox from a monolithic system into a flexible, extensible platform. It eliminates hard dependencies, improves testability, and provides users with choice while maintaining a simple, unified API.

This architectural change demonstrates the principle of **dependency inversion**: high-level modules (the VAD pipeline) no longer depend on low-level modules (specific STT implementations), but both depend on abstractions (the plugin interface).
