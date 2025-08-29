# Agent Prompt: Implement Essential STT Pipeline Tests

## Mission
Implement 6 comprehensive tests for the ColdVox STT pipeline. Focus on functional correctness, not performance. Each test should be self-contained, runnable, and provide clear pass/fail results.

## Context
ColdVox is a Rust STT pipeline with:
- Audio capture from microphone → 16kHz mono conversion
- 512-sample chunking for processing
- Silero VAD for speech detection
- Vosk for transcription
- Generous buffers (4+ seconds) throughout

## Required Tests

### Test 1: End-to-End Pipeline Test
**File**: `crates/app/tests/pipeline_integration.rs`

**Implementation**:
```rust
#[test]
fn test_pipeline_known_audio() {
    // 1. Load test WAV file with known transcription
    // 2. Feed audio through complete pipeline
    // 3. Verify transcription matches expected text
    // 4. Verify no samples lost (input count == processed count)
}
```

**Test Data**: Create `test_data/pipeline_test.wav` with:
- Clear speech: "Hello world, testing one two three"
- 16kHz mono format
- Expected transcription in `test_data/pipeline_test.txt`

**Pass Criteria**: 
- Transcription accuracy > 90%
- All audio samples accounted for

### Test 2: VAD Accuracy Test
**File**: `crates/app/tests/vad_accuracy.rs`

**Implementation**:
```rust
#[test]
fn test_vad_detection_accuracy() {
    // 1. Load multi-segment test file:
    //    - 2 sec silence
    //    - 3 sec speech
    //    - 2 sec silence
    //    - 2 sec noisy speech
    //    - 1 sec silence
    // 2. Process through VAD
    // 3. Verify correct SpeechStart/End events
}
```

**Test Data**: Create `test_data/vad_test.wav` with labeled segments

**Pass Criteria**:
- No VAD triggers during silence segments
- VAD triggers during all speech segments
- Events within 200ms of actual boundaries

### Test 3: Error Recovery Test
**File**: `crates/app/tests/error_recovery.rs`

**Implementation**:
```rust
#[tokio::test]
async fn test_error_recovery() {
    // Scenario 1: Missing Vosk model
    // 1. Set invalid model path
    // 2. Start pipeline
    // 3. Verify graceful degradation (pipeline runs without STT)
    
    // Scenario 2: Device disconnection (mock)
    // 1. Start with mock audio device
    // 2. Simulate disconnect
    // 3. Verify watchdog detects and attempts recovery
}
```

**Pass Criteria**:
- System doesn't crash on errors
- Appropriate error logs generated
- Recovery attempted for device issues

### Test 4: System Health Test
**File**: `crates/app/tests/system_health.rs`

**Implementation**:
```rust
#[tokio::test]
async fn test_component_initialization() {
    // 1. Initialize all components with valid config
    // 2. Verify each component starts successfully:
    //    - AudioCaptureThread spawned
    //    - Ring buffer created
    //    - AudioChunker running
    //    - VadProcessor active
    //    - SttProcessor ready (if model available)
    // 3. Send shutdown signal
    // 4. Verify clean shutdown
}
```

**Pass Criteria**:
- All components initialize without error
- Clean shutdown within 5 seconds

### Test 5: Live Operation Test
**File**: `crates/app/examples/live_operation_test.rs`

**Implementation**:
```rust
// Run as: cargo run --example live_operation_test
async fn main() {
    // 1. Start full pipeline with real microphone
    // 2. Run for 30 seconds
    // 3. Track metrics:
    //    - Frames captured
    //    - VAD events generated
    //    - Transcriptions produced
    // 4. Verify no accumulating errors
}
```

**Pass Criteria**:
- Runs 30 seconds without crash
- Steady frame rate (~31.25 fps)
- No error accumulation

### Test 6: State Management Test
**File**: `crates/app/tests/state_transitions.rs`

**Implementation**:
```rust
#[test]
fn test_rapid_speech_transitions() {
    // 1. Create test audio with rapid on/off speech:
    //    - 0.5s speech, 0.5s silence (repeat 10x)
    // 2. Process through pipeline
    // 3. Verify:
    //    - Correct number of VAD events
    //    - STT processor state transitions match
    //    - No stuck states
    //    - Transcriber resets between utterances
}
```

**Test Data**: Generate programmatically or use `test_data/rapid_speech.wav`

**Pass Criteria**:
- All speech segments detected
- State machine never stuck
- Clean transitions

## Implementation Guidelines

### Test Structure
```rust
// Each test should follow this pattern:
#[test]
fn test_name() {
    // Setup
    let config = create_test_config();
    let pipeline = setup_test_pipeline(config);
    
    // Execute
    let result = pipeline.process(test_input);
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_output);
    
    // Cleanup (if needed)
    pipeline.shutdown();
}
```

### Required Test Utilities
Create `crates/app/src/test_utils.rs`:
```rust
pub fn create_test_config() -> AppConfig { /* ... */ }
pub fn load_test_audio(path: &str) -> Vec<i16> { /* ... */ }
pub fn compare_transcriptions(actual: &str, expected: &str) -> f32 { /* ... */ }
pub fn create_mock_audio_device() -> MockAudioDevice { /* ... */ }
```

### Test Data Organization
```
test_data/
├── pipeline_test.wav       # Known transcription test
├── pipeline_test.txt       # Expected transcription
├── vad_test.wav           # Multi-segment VAD test
├── vad_test.json          # Segment labels
├── rapid_speech.wav       # State transition test
└── README.md              # Test data documentation
```

## Success Criteria

Your implementation is complete when:
1. All 6 tests compile and run
2. Tests provide clear pass/fail output
3. Test coverage includes all major code paths
4. Tests can run in CI/CD environment
5. Total test runtime < 60 seconds (excluding live test)

## Important Notes

- **DO NOT** test performance/throughput/latency
- **DO NOT** create redundant variations of the same test
- **DO** use existing test utilities where available
- **DO** make tests deterministic (except live test)
- **DO** provide helpful assertion messages on failure

## Existing Test Infrastructure

Check these existing test utilities:
- `crates/app/src/probes/` - Live hardware testing utilities
- `crates/app/examples/` - Example programs that can inspire test code
- `Forks/ColdVox-voice_activity_detector/tests/` - VAD-specific tests

## Running the Tests

```bash
# Run all tests
cargo test --workspace

# Run specific test
cargo test test_pipeline_known_audio

# Run with logging
RUST_LOG=debug cargo test -- --nocapture

# Run live operation test
cargo run --example live_operation_test
```

Remember: These 6 tests should comprehensively validate the STT pipeline works correctly. Quality over quantity.