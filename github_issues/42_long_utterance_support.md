# [STT] Implement support for long utterance processing

**Priority:** Medium

## Problem Description
The current STT implementation has limitations with long utterances, causing memory issues, processing timeouts, and degraded accuracy for extended speech segments. The system is optimized for short utterances and struggles with continuous speech.

## Impact
- **Medium**: Memory exhaustion with long audio segments
- Processing timeouts for extended speech
- Degraded transcription accuracy over time
- Limited support for continuous speech scenarios
- Poor performance with long-form content

## Reproduction Steps
1. Test transcription with 5+ minute audio segments
2. Monitor memory usage during long utterance processing
3. Check for processing timeouts with extended speech
4. Test accuracy degradation over time
5. Examine buffer management for long audio streams

## Expected Behavior
The system should:
- Handle long utterances (10+ minutes) efficiently
- Maintain consistent accuracy throughout long segments
- Manage memory usage effectively for extended processing
- Support streaming processing for continuous speech
- Provide progress feedback for long transcriptions

## Current Behavior
The system exhibits:
- Memory exhaustion with long audio segments
- Processing timeouts for extended speech
- Accuracy degradation over time
- Inefficient buffer management
- Limited support for continuous speech

## Proposed Solution
1. Implement streaming processing for long utterances
2. Add memory-efficient buffer management
3. Create utterance segmentation for long audio
4. Implement progress tracking and feedback
5. Optimize memory usage for extended processing

## Implementation Steps
1. Analyze current memory and processing limitations
2. Implement streaming transcription for long audio
3. Add utterance segmentation and chunking
4. Create memory-efficient buffer management
5. Implement progress tracking and user feedback
6. Optimize STT engine for long-form content

## Acceptance Criteria
- [ ] Support for 10+ minute audio segments
- [ ] Consistent accuracy throughout long utterances
- [ ] Efficient memory usage for extended processing
- [ ] Progress feedback for long transcriptions
- [ ] Streaming processing without memory exhaustion
- [ ] Optimized performance for long-form content

## Technical Details
- **Current Limit**: ~2-3 minutes before memory/timeout issues
- **Target**: 30+ minutes of continuous speech
- **Memory**: Streaming processing with bounded buffers
- **Segmentation**: Intelligent utterance chunking
- **Progress**: Real-time progress updates and feedback

## Memory Optimization Strategies
- **Streaming Processing**: Process audio in chunks rather than loading entirely
- **Buffer Management**: Circular buffers with size limits
- **Garbage Collection**: Efficient cleanup of processed segments
- **Memory Pooling**: Reuse buffers for repeated operations
- **Virtual Memory**: Support for memory-mapped files if needed

## Accuracy Maintenance
- **Context Preservation**: Maintain context across segment boundaries
- **Overlap Processing**: Process overlapping segments for continuity
- **Quality Monitoring**: Track accuracy throughout long utterances
- **Adaptive Segmentation**: Adjust chunk sizes based on content
- **Error Correction**: Cross-segment error detection and correction

## Performance Targets
- **Memory Usage**: < 500MB for 30-minute audio
- **Processing Time**: Linear scaling with audio length
- **Accuracy**: Maintain >95% accuracy throughout
- **Responsiveness**: Progress updates every 10 seconds
- **Resource Usage**: Efficient CPU utilization

## Related Files
- `crates/app/src/stt/processor.rs`
- `crates/coldvox-audio/src/chunker.rs`
- `crates/coldvox-audio/src/buffer.rs`
- `crates/coldvox-stt-vosk/src/vosk_transcriber.rs`
