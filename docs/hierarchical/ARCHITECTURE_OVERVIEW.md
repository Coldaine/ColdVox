# ColdVox Architecture Overview

This document provides a comprehensive overview of the ColdVox architecture, showing how the vision, pillars, domains, and systems work together to create a robust voice AI pipeline.

## Architecture Flow

The ColdVox architecture follows a linear pipeline flow with feedback loops for reliability and optimization:

```
Audio Input → Real-time Processing → Voice Activity Detection → Speech-to-Text → Text Injection
    ↓              ↓                        ↓                      ↓                ↓
[Device Mgmt]  [Resampling]          [Silero Model]        [Vosk Engine]    [Adaptive Strategy]
    ↓              ↓                        ↓                      ↓                ↓
[CPAL Capture]  [Chunking]           [Event Generation]   [Transcription]   [Multiple Backends]
    ↓              ↓                        ↓                      ↓                ↓
[Ring Buffer]  [Watchdog]              [Debouncing]        [Partial Results]  [Success Tracking]
```

## Core Vision Implementation

The [Vision](VSN0/COLDVOX-VSN0-001-voice-ai-pipeline.md) establishes ColdVox as a real-time voice AI pipeline. This is implemented through four foundational [Pillars](PIL1/):

1. **[Real-time Audio Processing](PIL1/COLDVOX-PIL1-001-realtime-audio-processing.md)** provides the foundation for all other capabilities
2. **[Voice Activity Detection](PIL1/COLDVOX-PIL1-002-voice-activity-detection.md)** optimizes resource usage and accuracy
3. **[Speech-to-Text Transcription](PIL1/COLDVOX-PIL1-003-speech-to-text.md)** converts speech to text with high accuracy
4. **[Cross-platform Text Injection](PIL1/COLDVOX-PIL1-004-text-injection.md)** delivers transcribed text to target applications

## Domain Organization

The architecture is organized into five key [Domains](DOM2/):

1. **[Audio Capture & Device Management](DOM2/COLDVOX-DOM2-001-audio-capture.md)** - Handles device discovery and real-time capture
2. **[Audio Processing Pipeline](DOM2/COLDVOX-DOM2-002-audio-processing.md)** - Processes audio for downstream consumption
3. **[Voice Activity Detection Engine](DOM2/COLDVOX-DOM2-003-vad-engine.md)** - Detects speech segments in audio
4. **[Speech-to-Text Engine](DOM2/COLDVOX-DOM2-004-stt-engine.md)** - Converts speech to text
5. **[Text Injection System](DOM2/COLDVOX-DOM2-005-text-injection.md)** - Injects text into target applications

## Key Technical Decisions

Three critical architectural decisions shape the implementation:

1. **[Hybrid Threading Model](ADR3/COLDVOX-ADR3-002-hybrid-threading-model.md)** - Dedicated real-time thread for audio capture prevents priority inversion
2. **[Vosk Model Distribution](ADR3/COLDVOX-ADR3-001-vosk-model-distribution.md)** - Committing models to the repository ensures deterministic CI
3. **[Adaptive Injection Strategy](ADR3/COLDVOX-ADR3-003-adaptive-injection-strategy.md)** - Learning from past attempts maximizes injection success rates

## Integration Points

The architecture provides several key integration points:

- **[Audio Device Interface](SPEC5/COLDVOX-SPEC5-001-audio-device-interface.md)** - Standardized device management
- **[Ring Buffer Interface](SPEC5/COLDVOX-SPEC5-002-ring-buffer-interface.md)** - Lock-free communication between threads
- **[VAD Engine Interface](SPEC5/COLDVOX-SPEC5-003-vad-engine-interface.md)** - Pluggable VAD algorithms
- **[STT Engine Interface](SPEC5/COLDVOX-SPEC5-004-stt-engine-interface.md)** - Pluggable transcription engines
- **[Injection Backend Interface](SPEC5/COLDVOX-SPEC5-005-injection-backend-interface.md)** - Pluggable injection methods

## Reliability Features

Several systems work together to ensure reliability:

- **[Watchdog Timer](SYS4/COLDVOX-SYS4-001-audio-capture-thread.md)** - Monitors for stream issues and triggers recovery
- **[Adaptive Strategy Manager](SYS4/COLDVOX-SYS4-006-injection-manager.md)** - Learns from past injection attempts
- **[Error Handling](IMP6/)** - Structured error types and recovery mechanisms throughout

## Platform Support

ColdVox provides comprehensive cross-platform support through platform-specific implementations:

- **Linux**: AT-SPI, wl-clipboard, ydotool, kdotool
- **Windows/macOS**: Enigo-based input simulation
- **Build-time Detection**: Automatic backend selection based on platform capabilities

## Performance Characteristics

The architecture is designed with specific performance targets:

- **Latency**: <50ms from microphone to injection
- **Accuracy**: <10% word error rate for transcription
- **Reliability**: >99% injection success rate
- **Availability**: >99.9% uptime with automatic recovery

This comprehensive architecture enables ColdVox to provide a robust, efficient, and reliable voice AI pipeline across multiple platforms while maintaining extensibility for future enhancements.