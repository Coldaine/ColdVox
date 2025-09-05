# Whisper STT Backend Implementation

## Issue Type
Feature Implementation

## Priority
Medium

## Component
`crates/coldvox-stt-whisper` (to be created)

## Description
Implement a functional Whisper speech-to-text backend to complement the existing Vosk implementation. Currently only a stub exists without actual transcription capabilities.

## Current State
- Stub Whisper plugin mentioned in PROJECT_STATUS.md
- Multi-backend STT framework in place
- Vosk backend fully functional as reference
- STT trait abstractions ready in coldvox-stt

## Implementation Options

### 1. whisper.cpp Binding
- **Pros**: Fast, low memory, C++ implementation
- **Cons**: Requires FFI bindings, build complexity
- **Crate**: whisper-rs (bindings to whisper.cpp)

### 2. Candle (Rust native)
- **Pros**: Pure Rust, good performance
- **Cons**: Larger memory footprint
- **Crate**: candle-whisper

### 3. ONNX Runtime
- **Pros**: Hardware acceleration, standardized format
- **Cons**: Requires ONNX model conversion
- **Crate**: ort + custom integration

### 4. OpenAI API
- **Pros**: No local model needed, latest improvements
- **Cons**: Network dependency, API costs, latency
- **Crate**: async-openai or reqwest

## Proposed Architecture
```rust
// crates/coldvox-stt-whisper/src/lib.rs
pub struct WhisperTranscriber {
    model: WhisperModel,
    config: WhisperConfig,
}

impl SttProcessor for WhisperTranscriber {
    async fn process_audio(&mut self, samples: &[i16]) -> Result<TranscriptionEvent> {
        // Implementation
    }
}
```

## Features to Implement
- [ ] Model loading (tiny, base, small, medium, large)
- [ ] Language detection or configuration
- [ ] Streaming transcription support
- [ ] Partial vs final transcripts
- [ ] Timestamp generation
- [ ] Confidence scores
- [ ] VAD integration (use existing segments)

## Configuration
```toml
[stt.whisper]
model_path = "models/whisper-base"
model_size = "base"  # tiny, base, small, medium, large
language = "en"      # or "auto"
beam_size = 5
temperature = 0.0
initial_prompt = ""
```

## Performance Targets
- Model loading: <5 seconds for base model
- Transcription latency: <500ms for 5-second audio
- Memory usage: <1GB for base model
- CPU usage: <50% single core during inference

## Integration Requirements
1. **Crate structure**:
   - Create crates/coldvox-stt-whisper
   - Implement SttProcessor trait
   - Add feature flag to main app

2. **Model management**:
   - Auto-download models option
   - Model path configuration
   - Model validation

3. **Audio preprocessing**:
   - Use existing 16kHz pipeline
   - Handle VAD segments properly
   - Buffer management for streaming

## Testing Requirements
- Unit tests with sample audio
- Integration test with full pipeline
- Performance benchmarks vs Vosk
- Accuracy comparison on test set
- Memory leak testing

## Documentation Needs
- Model selection guide
- Performance tuning tips
- Language configuration
- Example configurations

## Dependencies
```toml
[dependencies]
# Option 1: whisper.cpp
whisper-rs = "0.x"

# Option 2: Candle
candle-core = "0.x"
candle-whisper = "0.x"

# Option 3: ONNX
ort = "2.x"

# Common
tokio = { workspace = true }
tracing = { workspace = true }
```

## Acceptance Criteria
- [ ] Whisper backend crate created
- [ ] At least one implementation option working
- [ ] Feature flag integration in main app
- [ ] Model auto-download or clear setup docs
- [ ] Performance meets targets
- [ ] Tests passing
- [ ] Documentation complete
