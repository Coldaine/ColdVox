# Agent Prompt: STT Pipeline Deep Inspection

## Mission
You are tasked with performing a comprehensive inspection and analysis of the ColdVox STT (Speech-to-Text) pipeline. Your goal is to trace the complete data flow from microphone input to transcription output, identify potential bottlenecks, verify correct operation, and document any issues or optimization opportunities.

## Context
ColdVox is a Rust-based real-time voice AI system using:
- Lock-free ring buffers for audio capture
- Dual VAD system (Silero ML + Energy fallback)
- Vosk for speech-to-text transcription
- Tokio async runtime with broadcast channels

## Primary Objectives

### 1. Trace Audio Data Flow
Starting from `crates/app/src/main.rs`, trace the complete path of audio data:
- **Entry Point**: AudioCaptureThread initialization (main.rs:43-44)
- **Buffer Transfer**: Ring buffer producer/consumer split
- **Chunking**: AudioChunker converting to 512-sample frames
- **Distribution**: Broadcast channel fan-out to multiple consumers
- **VAD Processing**: Frame analysis for speech detection
- **STT Gating**: Audio processing controlled by VAD events
- **Transcription**: Vosk engine processing and result generation

Document each transformation step with:
- Input format and size
- Output format and size
- Processing latency
- Memory allocations
- Thread boundaries

### 2. Analyze Component Interactions

#### Critical Interaction Points:
1. **AudioCapture → RingBuffer** (`audio/capture.rs`)
   - Verify producer write patterns
   - Check overflow handling
   - Monitor watchdog epoch management

2. **FrameReader → AudioChunker** (`audio/chunker.rs:97-103`)
   - Validate frame reading strategy
   - Check buffer accumulation logic
   - Verify timestamp calculation

3. **Broadcast Channel Distribution** (`main.rs:64-70`)
   - Count active subscribers
   - Monitor channel capacity usage
   - Track dropped frames

4. **VAD → STT Synchronization** (`stt/processor.rs:136-159`)
   - Verify event ordering
   - Check state machine transitions
   - Validate audio frame correlation

5. **STT → Vosk Interface** (`stt/vosk.rs:57-91`)
   - Confirm PCM format (i16)
   - Monitor decoding states
   - Track result generation

### 3. Identify Critical Code Paths

Focus inspection on:
```rust
// Key files to examine in detail:
crates/app/src/audio/capture.rs        // Hardware interface
crates/app/src/audio/ring_buffer.rs    // Lock-free buffer
crates/app/src/audio/chunker.rs        // Frame preparation
crates/app/src/audio/vad_processor.rs  // VAD integration
crates/app/src/stt/processor.rs        // STT orchestration
crates/app/src/stt/vosk.rs            // Engine wrapper
crates/app/src/vad/silero.rs          // Primary VAD
```

### 4. Verify Error Handling

For each component, verify:
- Error propagation paths
- Recovery mechanisms
- Logging coverage
- Resource cleanup

Pay special attention to:
- Device disconnection handling
- Buffer overflow scenarios
- VAD processing failures
- Vosk model errors
- Channel closure conditions

### 5. Performance Analysis

Measure and document:
- **Latency**: End-to-end from mic to transcription
- **Throughput**: Frames/second at each stage
- **Memory**: Buffer usage and allocations
- **CPU**: Thread utilization patterns

Use these inspection commands:
```bash
# Run with debug logging
RUST_LOG=debug cargo run --features vosk

# Monitor with TUI dashboard
cargo run --bin tui_dashboard

# Test specific components
cargo run --example mic_probe
cargo run --example vad_demo
```

### 6. Configuration Impact

Analyze how these configurations affect the pipeline:
```rust
// Key parameters to evaluate:
- Ring buffer size: 16384 * 4
- Chunk size: 512 samples
- Broadcast capacity: 200 frames
- VAD frame size: 512 samples
- VAD sample rate: 16000 Hz
- Event channel capacity: 100
```

### 7. Concurrency Analysis

Examine:
- Thread spawn points and lifecycle
- Channel ownership and sharing
- Atomic operations and synchronization
- Potential race conditions
- Deadlock scenarios

Key async boundaries:
- `AudioCaptureThread::spawn` (blocking thread)
- `AudioChunker::spawn` (tokio task)
- `VadProcessor::spawn` (tokio task)
- `SttProcessor::run` (tokio task)

## Deliverables

### 1. Data Flow Diagram
Create a detailed diagram showing:
- Component boundaries
- Data transformations
- Buffer/channel capacities
- Thread assignments
- Error paths

### 2. Bottleneck Analysis
Identify and rank potential bottlenecks:
- Location in pipeline
- Impact severity
- Triggering conditions
- Mitigation strategies

### 3. Issue Report
Document any discovered issues:
- Issue description
- Reproduction steps
- Impact assessment
- Suggested fixes

### 4. Optimization Recommendations
Propose improvements for:
- Latency reduction
- Throughput increase
- Memory efficiency
- Error resilience

### 5. Test Coverage Gaps
Identify untested scenarios:
- Edge cases
- Error conditions
- Performance limits
- Integration points

## Investigation Methodology

### Phase 1: Static Analysis
1. Read all relevant source files
2. Map function call graphs
3. Identify data structures
4. Document threading model

### Phase 2: Dynamic Analysis
1. Add strategic logging points
2. Run with sample audio
3. Monitor metrics
4. Stress test components

### Phase 3: Integration Testing
1. Test error injection
2. Verify recovery paths
3. Measure performance limits
4. Validate configuration changes

## Special Focus Areas

### Memory Safety
- Verify all unsafe blocks
- Check buffer boundary conditions
- Validate pointer arithmetic
- Confirm drop implementations

### Real-time Constraints
- Identify blocking operations
- Verify lock-free guarantees
- Check allocation patterns
- Monitor GC pressure points

### Resilience
- Test device hot-plug scenarios
- Verify exponential backoff
- Check resource leak potential
- Validate shutdown sequences

## Success Criteria

Your inspection is complete when you can:
1. Explain every audio sample's journey through the system
2. Identify the slowest component in the pipeline
3. Predict failure modes and recovery behavior
4. Recommend specific, actionable improvements
5. Provide reproduction steps for any issues found

## Additional Resources

- Project documentation: `docs/PROJECT_STATUS.md`
- Technical specification: `docs/1_foundation/EnhancedPhasePlanV2.md`
- Test scripts: `scripts/run_phase1_tests.sh`
- Example recordings: `test_data/` directory

Remember: Focus on understanding the "why" behind each design decision, not just the "what" of the implementation. The goal is to ensure the STT pipeline is robust, efficient, and maintainable.