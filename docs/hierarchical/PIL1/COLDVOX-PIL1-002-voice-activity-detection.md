---
id: COLDVOX-PIL1-002-voice-activity-detection
type: PIL
level: 1
title: Voice Activity Detection
status: approved
owner: @team-audio
subsystem: coldvox
updated: 2025-09-11
version: 2
last_reviewed: 2025-09-11
parent: COLDVOX-VSN0-001-voice-ai-pipeline
links:
  satisfies: [COLDVOX-VSN0-001-voice-ai-pipeline]
  depends_on: [COLDVOX-PIL1-001-realtime-audio-processing]
  verified_by: []
  related_to: []
---

## Summary
Implement accurate voice activity detection to distinguish speech from non-speech audio segments, reducing computational load and improving transcription quality by gating the speech-to-text processing pipeline.

## Description
This pillar focuses on detecting when speech is present in the audio stream to enable efficient processing resources and improve transcription accuracy. The implementation uses a pluggable architecture to support multiple VAD algorithms with the Silero VAD model as the primary implementation.

## Key Requirements
- High accuracy in distinguishing speech from noise (>95%)
- Low false positive and false negative rates (<5% each)
- Configurable sensitivity thresholds for different environments
- Support for multiple VAD algorithms with consistent interface
- Real-time processing capabilities with minimal latency (<10ms)
- Event-based output (SpeechStart, SpeechEnd) with confidence metrics

## Success Metrics
- VAD accuracy: > 95%
- False positive rate: < 5%
- False negative rate: < 5%
- Processing latency: < 10ms
- Event detection precision: < 100ms

## Technical Approach
The VAD system implements a trait-based architecture:
1. **VAD Engine Trait**: Standardized interface for VAD implementations
2. **Silero Implementation**: Primary VAD engine using ONNX-based Silero model
3. **Configuration Management**: Unified configuration for all VAD engines
4. **Event Generation**: Produce SpeechStart/SpeechEnd events with debouncing

## Supported Algorithms
- **Silero VAD**: Primary implementation using ONNX model (default)
- **Energy-based Detection**: RMS-based detection (future expansion)
- **Custom Implementations**: Extensible for third-party VAD engines

## Dependencies
- voice_activity_detector crate for ONNX inference
- serde for configuration serialization
- thiserror for structured error handling

## Integration Points
- Receives processed audio frames from the audio pipeline
- Gates the speech-to-text processing pipeline
- Provides events to downstream components

---
satisfies: COLDVOX-VSN0-001-voice-ai-pipeline  
depends_on: COLDVOX-PIL1-001-realtime-audio-processing  
verified_by:  
related_to: