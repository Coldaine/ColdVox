---
id: COLDVOX-PIL1-003-speech-to-text
type: PIL
level: 1
title: Speech-to-Text Transcription
status: approved
owner: @team-stt
subsystem: coldvox
updated: 2025-09-11
version: 2
last_reviewed: 2025-09-11
parent: COLDVOX-VSN0-001-voice-ai-pipeline
links:
  satisfies: [COLDVOX-VSN0-001-voice-ai-pipeline]
  depends_on: [COLDVOX-PIL1-002-voice-activity-detection]
  verified_by: []
  related_to: [COLDVOX-ADR3-001-vosk-model-distribution]
---

## Summary
Provide accurate, offline-capable speech-to-text transcription with support for multiple engines, real-time partial results, and configurable quality settings.

## Description
This pillar implements speech-to-text transcription capabilities with a focus on offline operation and high accuracy. The system uses a pluggable architecture to support multiple transcription engines, with Vosk as the primary implementation for offline recognition.

## Key Requirements
- High transcription accuracy (word error rate < 10%)
- Support for offline operation without internet connectivity
- Multiple engine support (Vosk, Whisper planned)
- Real-time partial results with low latency
- Configurable quality settings for performance/accuracy trade-offs
- Event-based output (Partial, Final, Error) with confidence metrics

## Success Metrics
- Word error rate: < 10%
- Partial result latency: < 200ms
- Final result latency: < 500ms
- Offline operation availability: 100%
- Model loading time: < 2 seconds

## Technical Approach
The STT system implements a trait-based architecture:
1. **Transcriber Trait**: Standardized interface for STT implementations
2. **Vosk Implementation**: Primary STT engine for offline recognition
3. **Plugin System**: Extensible architecture for additional engines
4. **Event Generation**: Produce Partial/Final/Error events with metadata

## Supported Engines
- **Vosk**: Primary implementation for offline speech recognition
- **Whisper**: Planned cloud-based alternative
- **Mock/Noop**: Testing implementations
- **Custom Implementations**: Extensible for third-party STT engines

## Dependencies
- vosk crate for Vosk library integration
- serde for configuration serialization
- thiserror for structured error handling
- tokio for async processing

## Integration Points
- Receives speech segments from VAD events
- Produces transcription events for text injection
- Integrates with telemetry for performance monitoring

## Related Architectural Decisions
- [COLDVOX-ADR3-001-vosk-model-distribution](../../ADR3/COLDVOX-ADR3-001-vosk-model-distribution.md): Vosk model distribution strategy

---
satisfies: COLDVOX-VSN0-001-voice-ai-pipeline  
depends_on: COLDVOX-PIL1-002-voice-activity-detection  
verified_by:  
related_to: COLDVOX-ADR3-001-vosk-model-distribution