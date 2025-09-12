---
id: SPEC-002
title: Transcription API Specification
level: specification
status: drafting
owners:
  - CDIS
criticality: 4
parent: SYS-002
pillar_trace:
  - PIL-002
  - DOM-002
  - SUB-002
  - SYS-002
  - SPEC-002
implements:
  - "IMP-002"
verified_by:
  - "TST-002"
---

# Transcription API Specification [SPEC-002]

## 1. Overview

This specification defines the public API for all speech-to-text engines in ColdVox. The contract is defined by the `EventBasedTranscriber` trait, which ensures that any STT implementation can be used interchangeably by the rest of the application.

## 2. Core Trait: `EventBasedTranscriber`

Any component that provides transcription services MUST implement this trait.

**Public API:**
```rust
// Simplified for specification from coldvox-stt/src/plugin.rs
pub trait EventBasedTranscriber {
    /// Creates a new transcriber instance with the given configuration.
    fn new(config: TranscriptionConfig, sample_rate: f32) -> Result<Self, SttError>
    where
        Self: Sized;

    /// Accepts a frame of audio samples for processing.
    /// Returns an optional `TranscriptionEvent` if a result is available.
    fn accept_frame(&mut self, pcm: &[i16]) -> Result<Option<TranscriptionEvent>, SttError>;

    /// Signals the end of an utterance, flushing any remaining audio.
    fn utterance_end(&mut self) -> Result<Option<TranscriptionEvent>, SttError>;
}
```

## 3. Core Enum: `TranscriptionEvent`

This enum represents all possible outcomes from a transcription engine.

**Variants:**
```rust
// Simplified for specification from coldvox-stt/src/types.rs
pub enum TranscriptionEvent {
    /// A partial, intermediate transcription result.
    Partial { text: String, ... },

    /// A final, complete transcription result for an utterance.
    Final { text: String, words: Vec<WordInfo>, ... },

    /// An error occurred during transcription.
    Error { message: String, ... },
}
```

## 4. Configuration: `TranscriptionConfig`

All transcribers MUST be configurable via the `TranscriptionConfig` struct.

**Key Fields:**
- `enabled`: `bool`
- `model_path`: `String`
- `partial_results`: `bool`
- `include_words`: `bool`

## 5. Data Flow

1.  An STT engine (e.g., `VoskTranscriber`) is created with a `TranscriptionConfig`.
2.  The application continuously calls `accept_frame()` with new audio data.
3.  The engine processes the audio and may return `Some(TranscriptionEvent::Partial { ... })`.
4.  When the VAD signals the end of speech, the application calls `utterance_end()`.
5.  The engine flushes its internal buffer and returns `Some(TranscriptionEvent::Final { ... })`.
