# [Audio] Fix memory allocations in audio capture callbacks

**Priority:** Medium

## Problem Description
The audio capture system performs memory allocations within the real-time audio callback, which can cause audio glitches, dropped samples, and performance degradation. This violates real-time audio processing best practices and can lead to unreliable audio capture under high CPU load.

## Impact
- **Medium**: Can cause audio glitches and dropped samples
- Performance degradation under high CPU load
- Potential audio artifacts and interruptions
- Violates real-time audio processing guidelines
- Memory fragmentation in long-running sessions

## Reproduction Steps
1. Examine `crates/coldvox-audio/src/capture.rs` - look for allocations in callback
2. Run application under high CPU load
3. Monitor for audio dropouts or glitches
4. Check memory usage patterns during audio capture
5. Profile memory allocations during callback execution

## Expected Behavior
Audio callbacks should:
- Perform no memory allocations
- Use pre-allocated buffers
- Maintain real-time performance guarantees
- Handle audio data without dynamic memory operations
- Support high CPU load without audio degradation

## Current Behavior
The audio capture callback performs memory allocations, causing:
- Potential audio dropouts under load
- Memory fragmentation
- Non-deterministic performance
- Violation of real-time constraints

## Proposed Solution
1. Pre-allocate audio buffers at initialization
2. Use ring buffers or fixed-size buffers for audio data
3. Move any processing requiring allocation outside the callback
4. Implement zero-allocation audio capture path
5. Add performance monitoring for callback timing

## Implementation Steps
1. Analyze current callback memory usage
2. Design pre-allocated buffer system
3. Refactor capture callback to avoid allocations
4. Implement ring buffer for audio data queuing
5. Add callback performance monitoring
6. Test under high load conditions

## Acceptance Criteria
- [ ] No memory allocations in audio capture callback
- [ ] Pre-allocated buffer system implemented
- [ ] Real-time performance maintained under load
- [ ] Audio quality preserved during high CPU usage
- [ ] Performance monitoring for callback timing
- [ ] Memory usage profiling shows no callback allocations

## Technical Details
- Current: Dynamic allocations in `cpal` callback
- Target: Zero-allocation callback processing
- Buffer Strategy: Pre-allocated ring buffer
- Monitoring: Callback execution time tracking
- Testing: High CPU load simulation

## Code Analysis
```rust
// Current problematic code in capture.rs
let buffer = vec![0.0; frame_count]; // Allocation in callback!
```

## Related Files
- `crates/coldvox-audio/src/capture.rs`
- `crates/coldvox-audio/src/buffer.rs`
- `crates/app/src/audio/capture.rs`
