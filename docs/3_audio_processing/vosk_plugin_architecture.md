# Vosk Plugin Architecture

## Overview

The Vosk plugin provides offline speech recognition capabilities for ColdVox using the Vosk speech recognition toolkit. This document outlines the architecture, components, and integration points of the Vosk STT implementation.

## Architecture Components

### Core Components

#### 1. VoskTranscriber (`crates/coldvox-stt-vosk/src/vosk_transcriber.rs`)
The primary implementation of the Vosk speech recognition engine.

**Key Features:**
- Implements `EventBasedTranscriber` trait for modern event-driven interface
- Implements legacy `Transcriber` trait for backward compatibility
- Supports word-level timestamps and confidence scores
- Handles partial and final transcription results
- Feature-gated behind `vosk` Cargo feature

**Architecture:**
```rust
pub struct VoskTranscriber {
    recognizer: Recognizer,           // Vosk's main recognition engine
    config: TranscriptionConfig,      // Configuration settings
    current_utterance_id: u64,        // Unique ID for current utterance
}
```

#### 2. VoskPlugin (`crates/coldvox-stt/src/plugins/vosk_plugin.rs`)
Plugin wrapper that adapts VoskTranscriber to the STT plugin interface.

**Current Status:** Stub implementation - needs integration with actual VoskTranscriber

**Capabilities:**
- Streaming support (declared but not implemented)
- Word-level timestamps
- Confidence scores
- Multiple language support
- Custom vocabulary support

#### 3. SttProcessor (`crates/app/src/stt/processor.rs`)
Main audio processing pipeline that coordinates VAD events and transcription.

**Key Features:**
- Buffers audio during speech segments
- Processes entire utterance at once (batch mode)
- Maintains performance metrics
- Handles VAD events for speech start/end detection

**Processing Flow:**
```
VAD SpeechStart → Start buffering audio
Receive audio frames → Buffer in memory
VAD SpeechEnd → Process entire buffer → Generate transcription
```

#### 4. SttPluginManager (`crates/app/src/stt/plugin_manager.rs`)
Manages plugin lifecycle, selection, and fallback logic.

**Features:**
- Plugin registry and discovery
- Automatic fallback to available plugins
- Preferred plugin configuration
- Runtime plugin switching

## Data Flow Architecture

### Audio Processing Pipeline

```
Audio Input (f32 samples)
    ↓
AudioFrame (chunked)
    ↓
VAD Processing
    ↓
Speech Detection Events
    ↓
SttProcessor
    ↓
Buffer Management
    ↓
VoskTranscriber
    ↓
Transcription Events
    ↓
Text Injection
```

### Plugin System Integration

```
Application Startup
    ↓
SttPluginManager.initialize()
    ↓
Plugin Registry Scan
    ↓
VoskPluginFactory.create()
    ↓
VoskPlugin (wrapper)
    ↓
VoskTranscriber (actual engine)
    ↓
Ready for transcription
```

## Configuration Architecture

### TranscriptionConfig
```rust
pub struct TranscriptionConfig {
    pub enabled: bool,                    // Enable/disable transcription
    pub model_path: String,              // Path to Vosk model files
    pub partial_results: bool,           // Enable real-time partial results
    pub max_alternatives: u16,           // Number of alternative transcriptions
    pub include_words: bool,             // Include word-level timing
    pub buffer_size_ms: u64,             // Audio buffer size
}
```

### Environment Variables
- `VOSK_MODEL_PATH`: Override default model path
- `RUST_LOG`: Control logging verbosity

## Performance Characteristics

### Current Implementation
- **Mode**: Batch processing (entire utterance)
- **Latency**: High (waits for speech end)
- **Memory**: Buffers entire utterance in memory
- **CPU**: Processes all audio at once

### Target Optimizations
- **Streaming Mode**: Incremental processing
- **Reduced Latency**: Real-time transcription
- **Memory Optimization**: Circular buffers
- **Performance Metrics**: Detailed timing and resource usage

## Integration Points

### With VAD System
- Receives `VadEvent::SpeechStart` and `VadEvent::SpeechEnd`
- Uses speech boundaries to determine processing windows
- Coordinates buffering with speech activity

### With Text Injection
- Sends `TranscriptionEvent` variants (Partial, Final, Error)
- Provides word-level timing for advanced injection strategies
- Supports multiple alternatives for user selection

### With Plugin System
- Implements `SttPlugin` trait
- Supports plugin discovery and fallback
- Allows runtime configuration changes

## Model Management

### Model Loading
- Lazy loading on first use
- Model validation and existence checks
- Memory-mapped model files for efficiency

### Supported Models
- Multiple languages (en, ru, de, es, fr, etc.)
- Various sizes (small: ~40MB, large: ~1.8GB)
- Custom trained models support

## Error Handling

### Error Types
- `ModelLoadFailed`: Model file not found or invalid
- `InitializationFailed`: Vosk library initialization error
- `BackendError`: Vosk processing errors
- `NotAvailable`: Plugin requirements not met

### Recovery Strategies
- Automatic fallback to alternative plugins
- Graceful degradation (NoOp plugin)
- Detailed error reporting and logging

## Testing Architecture

### Unit Tests
- VoskTranscriber functionality
- Plugin interface compliance
- Configuration validation

### Integration Tests
- End-to-end audio processing
- Plugin manager operations
- VAD coordination

### Performance Benchmarks
- Latency measurements
- Memory usage profiling
- Accuracy validation

## Future Enhancements

### Streaming Support
- Incremental audio processing
- Real-time partial results
- Reduced memory footprint

### Advanced Features
- Speaker diarization
- Custom vocabulary
- Language detection
- Model hot-swapping

### Performance Optimizations
- GPU acceleration
- Model quantization
- Parallel processing
- Caching optimizations

## Dependencies

### Runtime Dependencies
- `libvosk.so` (Linux)
- `libvosk.dylib` (macOS)
- `vosk.dll` (Windows)

### Build Dependencies
- `vosk-sys` crate for FFI bindings
- Feature-gated compilation

## Configuration Examples

### Basic Configuration
```rust
let config = TranscriptionConfig {
    enabled: true,
    model_path: "models/vosk-model-small-en-us-0.15".to_string(),
    partial_results: true,
    max_alternatives: 1,
    include_words: true,
    buffer_size_ms: 512,
};
```

### Advanced Configuration
```rust
let config = TranscriptionConfig {
    enabled: true,
    model_path: "/custom/models/vosk-large-en".to_string(),
    partial_results: true,
    max_alternatives: 3,
    include_words: true,
    buffer_size_ms: 1024,
};
```

## Monitoring and Observability

### Metrics
- Frames processed per second
- Memory usage
- Error rates
- Latency distributions

### Logging
- Structured logging with `tracing`
- Debug information for audio processing
- Error details for troubleshooting

This architecture provides a solid foundation for offline speech recognition in ColdVox, with clear separation of concerns and extensibility for future enhancements.
