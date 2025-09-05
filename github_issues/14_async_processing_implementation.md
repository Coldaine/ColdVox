# [Performance] Implement async processing for non-blocking STT operations

**Priority:** Medium

## Problem Description
The STT processing pipeline uses synchronous operations that block the main thread, causing UI freezes and poor responsiveness during transcription. Long-running STT operations prevent concurrent processing and degrade user experience.

## Impact
- **Medium**: UI freezing during STT operations
- Poor application responsiveness
- Inability to process multiple audio streams concurrently
- Resource underutilization during I/O operations
- Limited scalability for high-throughput scenarios

## Reproduction Steps
1. Examine `crates/app/src/stt/processor.rs` - check for synchronous operations
2. Test application responsiveness during transcription
3. Monitor thread usage during STT processing
4. Check for blocking I/O operations in STT pipeline
5. Test concurrent audio processing capabilities

## Expected Behavior
The STT system should:
- Process transcriptions asynchronously without blocking UI
- Support concurrent processing of multiple audio streams
- Maintain responsive UI during intensive STT operations
- Utilize system resources efficiently
- Handle high-throughput scenarios gracefully

## Current Behavior
The system exhibits:
- Synchronous processing that blocks the main thread
- UI freezing during transcription operations
- Sequential processing of audio streams
- Poor resource utilization
- Limited concurrent processing capabilities

## Proposed Solution
1. Convert STT processing to async/await patterns
2. Implement async transcription pipelines
3. Add concurrent processing capabilities
4. Optimize I/O operations for non-blocking behavior
5. Create async task management and coordination

## Implementation Steps
1. Analyze current synchronous bottlenecks
2. Convert STT processor to async implementation
3. Implement async transcription result handling
4. Add concurrent audio stream processing
5. Optimize model loading and initialization for async
6. Create async task coordination and error handling

## Acceptance Criteria
- [ ] STT processing converted to async operations
- [ ] UI remains responsive during transcription
- [ ] Support for concurrent audio stream processing
- [ ] Efficient resource utilization
- [ ] Non-blocking I/O operations throughout pipeline
- [ ] Performance benchmarks showing improved throughput

## Technical Details
- **Current**: Synchronous processing with thread blocking
- **Target**: Async processing with tokio/futures
- **Concurrency**: Support for multiple concurrent transcriptions
- **Resource**: Efficient thread pool utilization
- **Scalability**: Handle 10+ concurrent audio streams

## Async Conversion Scope
- **STT Processor**: Convert main processing loop to async
- **Model Loading**: Async model initialization and loading
- **Result Handling**: Async result processing and delivery
- **Error Handling**: Async error propagation and recovery
- **Resource Management**: Async cleanup and resource management

## Performance Improvements Expected
- **Responsiveness**: 100% improvement in UI responsiveness
- **Throughput**: 3-5x improvement in concurrent processing
- **Resource Usage**: Better CPU utilization during I/O operations
- **Scalability**: Linear scaling with available CPU cores
- **Latency**: Reduced latency for concurrent operations

## Related Files
- `crates/app/src/stt/processor.rs`
- `crates/coldvox-stt-vosk/src/vosk_transcriber.rs`
- `crates/app/src/audio/vad_processor.rs`
- `crates/coldvox-audio/src/capture.rs`
