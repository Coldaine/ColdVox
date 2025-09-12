---
id: COLDVOX-SPEC5-003-vad-engine-interface
type: SPEC
level: 4
title: VAD Engine Interface Specification
status: Approved
owner: @team-audio
subsystem: coldvox
updated: 2025-09-11
version: 2
last_reviewed: 2025-09-11
parent: COLDVOX-SYS4-004-vad-processor
links:
  satisfies: [COLDVOX-SYS4-004-vad-processor]
  depends_on: []
  implements: [CODE:repo://crates/coldvox-vad/src/engine.rs]
  verified_by: [COLDVOX-TST6-004-vad-processor-tests]
  related_to: []
---

## Summary
Define the interface for VAD engine implementations.

## Description
This specification defines the interface for VAD engine implementations, allowing for pluggable VAD algorithms.

## Interface
```rust
pub trait VadEngine {
    fn new(config: &UnifiedVadConfig) -> Result<Self, VadError>
    where
        Self: Sized;
    
    fn process_frame(&mut self, frame: &[f32]) -> Result<VadEvent, VadError>;
    
    fn reset(&mut self);
    
    fn set_threshold(&mut self, threshold: f32);
}

pub enum VadEvent {
    SpeechStart { confidence: f32 },
    SpeechEnd { confidence: f32 },
    Noise,
}

pub struct UnifiedVadConfig {
    pub threshold: f32,
    pub min_speech_duration: Duration,
    pub min_silence_duration: Duration,
    pub mode: VadMode,
}
```

## Requirements
- Pluggable engine design
- Standardized event interface
- Configurable parameters
- Proper error handling

---
satisfies: COLDVOX-SYS4-004-vad-processor  
depends_on:  
implements: CODE:repo://crates/coldvox-vad/src/engine.rs  
verified_by: COLDVOX-TST6-004-vad-processor-tests  
related_to: