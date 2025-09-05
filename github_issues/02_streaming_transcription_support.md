# [STT] Implement streaming transcription support for real-time processing

**Priority:** High

## Problem Description
The current STT implementation processes entire utterances at once, which introduces significant latency and prevents real-time transcription capabilities. This batch processing approach means users must wait for complete speech segments before seeing any transcription results, making the system unsuitable for real-time applications.

## Impact
- **High**: Prevents real-time transcription capabilities
- Introduces significant latency in transcription results
- Makes the system unsuitable for live applications
- Poor user experience with delayed feedback
- Inefficient resource usage with batch processing

## Reproduction Steps
1. Examine `crates/app/src/stt/processor.rs` - note the batch processing in `process_audio_chunk`
2. Check `crates/coldvox-stt-vosk/src/vosk_transcriber.rs` - observe synchronous processing
3. Monitor transcription latency during normal usage
4. Test with continuous speech to see batch processing delays

## Expected Behavior
The STT system should:
- Process audio in streaming fashion with minimal latency
- Provide incremental transcription results as speech continues
- Support real-time transcription for live applications
- Maintain low latency (< 100ms) for transcription results
- Handle continuous speech without waiting for utterance completion

## Current Behavior
The system processes complete utterances in batches, causing:
- High latency before transcription results appear
- No incremental results during speech
- Poor real-time performance
- Inefficient processing of continuous audio streams

## Proposed Solution
1. Implement streaming transcription interface in STT plugins
2. Modify `SttProcessor` to handle streaming audio chunks
3. Add incremental result processing and buffering
4. Implement low-latency audio processing pipeline
5. Add configuration options for streaming vs batch modes

## Implementation Steps
1. Define streaming transcription trait for STT plugins
2. Update Vosk transcriber to support streaming mode
3. Modify `SttProcessor` to process audio chunks incrementally
4. Implement result buffering and incremental output
5. Add latency monitoring and optimization
6. Update VAD integration for streaming compatibility

## Acceptance Criteria
- [ ] Streaming transcription interface implemented
- [ ] Latency reduced to < 100ms for incremental results
- [ ] Support for continuous speech processing
- [ ] Incremental transcription results during speech
- [ ] Configuration options for streaming mode
- [ ] Performance benchmarks for latency improvements

## Technical Details
- Current: Batch processing with ~500ms+ latency
- Target: Streaming with <100ms incremental latency
- Audio chunk size: Optimize for 20-50ms chunks
- Memory: Implement efficient result buffering
- Threading: Ensure thread-safe streaming processing

## Related Files
- `crates/app/src/stt/processor.rs`
- `crates/coldvox-stt-vosk/src/vosk_transcriber.rs`
- `crates/app/src/audio/vad_processor.rs`
- `crates/coldvox-audio/src/chunker.rs`
