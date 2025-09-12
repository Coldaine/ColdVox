---
id: COLDVOX-SYS4-004-vad-processor
type: SYS
level: 3
title: VAD Processor
status: Approved
owner: @team-audio
subsystem: coldvox
updated: 2025-09-11
version: 2
last_reviewed: 2025-09-11
parent: COLDVOX-DOM2-003-vad-engine
links:
  satisfies: [COLDVOX-DOM2-003-vad-engine]
  implements: [CODE:repo:/crates/coldvox-vad/src/processor.rs]
  verified_by: [COLDVOX-TST6-004-vad-processor-tests]
  related_to: []
---

## Summary
Process audio frames through the VAD engine to detect speech activity.

## Description
This system processes audio frames through the VAD engine to detect speech activity and generate SpeechStart/SpeechEnd events for downstream consumption.

## Key Components
- Audio frame processing
- VAD engine integration
- Event generation and debouncing
- Configuration management

## Requirements
- Real-time processing
- Accurate event generation
- Configurable thresholds
- Proper debouncing

---
satisfies: COLDVOX-DOM2-003-vad-engine  
depends_on: COLDVOX-SYS4-003-chunker  
verified_by: COLDVOX-TST6-004-vad-processor-tests  
related_to: