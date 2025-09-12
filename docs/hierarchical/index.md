# ColdVox Hierarchical Documentation Index

## Vision
- [COLDVOX-VSN0-001-voice-ai-pipeline](VSN0/COLDVOX-VSN0-001-voice-ai-pipeline.md) - Voice AI Pipeline

## Pillars
- [COLDVOX-PIL1-001-realtime-audio-processing](PIL1/COLDVOX-PIL1-001-realtime-audio-processing.md) - Real-time Audio Processing
- [COLDVOX-PIL1-002-voice-activity-detection](PIL1/COLDVOX-PIL1-002-voice-activity-detection.md) - Voice Activity Detection
- [COLDVOX-PIL1-003-speech-to-text](PIL1/COLDVOX-PIL1-003-speech-to-text.md) - Speech-to-Text Transcription
- [COLDVOX-PIL1-004-text-injection](PIL1/COLDVOX-PIL1-004-text-injection.md) - Cross-platform Text Injection

## Domains
- [COLDVOX-DOM2-001-audio-capture](DOM2/COLDVOX-DOM2-001-audio-capture.md) - Audio Capture & Device Management
- [COLDVOX-DOM2-002-audio-processing](DOM2/COLDVOX-DOM2-002-audio-processing.md) - Audio Processing Pipeline
- [COLDVOX-DOM2-003-vad-engine](DOM2/COLDVOX-DOM2-003-vad-engine.md) - Voice Activity Detection Engine
- [COLDVOX-DOM2-004-stt-engine](DOM2/COLDVOX-DOM2-004-stt-engine.md) - Speech-to-Text Engine
- [COLDVOX-DOM2-005-text-injection](DOM2/COLDVOX-DOM2-005-text-injection.md) - Text Injection System
- [COLDVOX-DOM2-006-foundation](DOM2/COLDVOX-DOM2-006-foundation.md) - Foundation Infrastructure
- [COLDVOX-DOM2-007-telemetry](DOM2/COLDVOX-DOM2-007-telemetry.md) - Telemetry & Metrics
- [COLDVOX-DOM2-008-gui](DOM2/COLDVOX-DOM2-008-gui.md) - Graphical User Interface

## Systems
- [COLDVOX-SYS4-001-audio-capture-thread](SYS4/COLDVOX-SYS4-001-audio-capture-thread.md) - Dedicated Audio Capture Thread
- [COLDVOX-SYS4-002-resampler](SYS4/COLDVOX-SYS4-002-resampler.md) - Audio Resampler
- [COLDVOX-SYS4-003-chunker](SYS4/COLDVOX-SYS4-003-chunker.md) - Audio Chunker
- [COLDVOX-SYS4-004-vad-processor](SYS4/COLDVOX-SYS4-004-vad-processor.md) - VAD Processor
- [COLDVOX-SYS4-005-stt-processor](SYS4/COLDVOX-SYS4-005-stt-processor.md) - STT Processor
- [COLDVOX-SYS4-006-injection-manager](SYS4/COLDVOX-SYS4-006-injection-manager.md) - Text Injection Manager
- [COLDVOX-SYS3-007-hotkey-system](SYS3/COLDVOX-SYS3-007-hotkey-system.md) - Global Hotkey System

## Specifications
- [COLDVOX-SPEC5-001-audio-device-interface](SPEC5/COLDVOX-SPEC5-001-audio-device-interface.md) - Audio Device Interface Specification
- [COLDVOX-SPEC5-002-ring-buffer-interface](SPEC5/COLDVOX-SPEC5-002-ring-buffer-interface.md) - Ring Buffer Interface Specification
- [COLDVOX-SPEC5-003-vad-engine-interface](SPEC5/COLDVOX-SPEC5-003-vad-engine-interface.md) - VAD Engine Interface Specification
- [COLDVOX-SPEC5-004-stt-engine-interface](SPEC5/COLDVOX-SPEC5-004-stt-engine-interface.md) - STT Engine Interface Specification
- [COLDVOX-SPEC5-005-injection-backend-interface](SPEC5/COLDVOX-SPEC5-005-injection-backend-interface.md) - Text Injection Backend Interface Specification

## Implementations
- [CODE:repo://crates/coldvox-audio/src/device.rs](IMP6/CODE:repo://crates/coldvox-audio/src/device.rs.md) - Audio Device Management Implementation
- [CODE:repo://crates/coldvox-audio/src/ring_buffer.rs](IMP6/CODE:repo://crates/coldvox-audio/src/ring_buffer.rs.md) - Ring Buffer Implementation
- [CODE:repo://crates/coldvox-vad-silero/src/silero_wrapper.rs](IMP6/CODE:repo://crates/coldvox-vad-silero/src/silero_wrapper.rs.md) - Silero VAD Implementation
- [CODE:repo://crates/coldvox-stt-vosk/src/vosk_transcriber.rs](IMP6/CODE:repo://crates/coldvox-stt-vosk/src/vosk_transcriber.rs.md) - Vosk STT Implementation
- [CODE:repo://crates/coldvox-text-injection/src/manager.rs](IMP6/CODE:repo://crates/coldvox-text-injection/src/manager.rs.md) - Text Injection Manager Implementation

## Tests
- [COLDVOX-TST6-001-audio-capture-tests](TST6/COLDVOX-TST6-001-audio-capture-tests.md) - Audio Capture Thread Tests
- [COLDVOX-TST6-002-resampler-tests](TST6/COLDVOX-TST6-002-resampler-tests.md) - Audio Resampler Tests
- [COLDVOX-TST6-003-chunker-tests](TST6/COLDVOX-TST6-003-chunker-tests.md) - Audio Chunker Tests
- [COLDVOX-TST6-004-vad-processor-tests](TST6/COLDVOX-TST6-004-vad-processor-tests.md) - VAD Processor Tests
- [COLDVOX-TST6-005-stt-processor-tests](TST6/COLDVOX-TST6-005-stt-processor-tests.md) - STT Processor Tests
- [COLDVOX-TST6-006-injection-manager-tests](TST6/COLDVOX-TST6-006-injection-manager-tests.md) - Text Injection Manager Tests

## Architectural Decision Records
- [COLDVOX-ADR3-001-vosk-model-distribution](ADR3/COLDVOX-ADR3-001-vosk-model-distribution.md) - Vosk Model Distribution Strategy
- [COLDVOX-ADR3-002-hybrid-threading-model](ADR3/COLDVOX-ADR3-002-hybrid-threading-model.md) - Hybrid Threading Model for Audio Processing
- [COLDVOX-ADR3-003-adaptive-injection-strategy](ADR3/COLDVOX-ADR3-003-adaptive-injection-strategy.md) - Adaptive Text Injection Strategy
- [COLDVOX-ADR3-004-build-time-platform-detection](ADR3/COLDVOX-ADR3-004-build-time-platform-detection.md) - Build-Time Platform and Desktop Environment Detection
- [COLDVOX-ADR3-005-stt-plugin-architecture](ADR3/COLDVOX-ADR3-005-stt-plugin-architecture.md) - Speech-to-Text Plugin Architecture
- [COLDVOX-ADR3-006-logging-and-tui-integration](ADR3/COLDVOX-ADR3-006-logging-and-tui-integration.md) - Logging Architecture and TUI Integration