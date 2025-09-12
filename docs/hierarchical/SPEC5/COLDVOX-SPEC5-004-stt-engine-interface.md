---
id: COLDVOX-SPEC5-004-stt-engine-interface
type: SPEC
level: 4
title: STT Engine Interface Specification
status: Approved
owner: @team-stt
subsystem: coldvox
updated: 2025-09-11
version: 2
last_reviewed: 2025-09-11
parent: COLDVOX-SYS4-005-stt-processor
links:
  satisfies: [COLDVOX-SYS4-005-stt-processor]
  depends_on: []
  implements: [CODE:repo://crates/coldvox-stt/src/plugin.rs]
  verified_by: [COLDVOX-TST6-005-stt-processor-tests]
  related_to: []
---

## Summary
Define the interface for STT engine implementations.

## Description
This specification defines the interface for STT engine implementations, allowing for pluggable speech-to-text algorithms.

## Interface
```rust
pub trait Transcriber {
    fn new(config: &SttConfig) -> Result<Self, SttError>
    where
        Self: Sized;
    
    fn transcribe(&mut self, audio: &[i16]) -> Result<TranscriptionEvent, SttError>;
    
    fn reset(&mut self);
}

pub enum TranscriptionEvent {
    Partial { text: String, confidence: f32 },
    Final { text: String, confidence: f32 },
    Error { error: SttError },
}

pub struct SttConfig {
    pub sample_rate: u32,
    pub language: String,
    pub model_path: Option<String>,
}
```

## Requirements
- Pluggable engine design
- Standardized event interface
- Support for partial and final results
- Proper error handling

---
satisfies: COLDVOX-SYS4-005-stt-processor  
depends_on:  
implements: CODE:repo://crates/coldvox-stt/src/plugin.rs  
verified_by: COLDVOX-TST6-005-stt-processor-tests  
related_to: