---
id: IMP-002
title: Vosk Transcription Module Implementation
level: implementation
status: drafting
owners:
  - CDIS
criticality: 3
parent: SYS-002
pillar_trace:
  - PIL-002
  - DOM-002
  - SUB-002
  - SYS-002
implements:
  - "SPEC-002"
---

# Vosk Transcription Module Implementation [IMP-002]

## 1. Overview

This document describes the Vosk-based implementation of the Transcription API, as defined in [SPEC-002](SPEC-002-transcription-api.md). The core logic is located in the `coldvox-stt-vosk` crate and implements the `EventBasedTranscriber` trait.

## 2. Code-level Traceability

This implementation directly maps to the following source code files and symbols:

-   **Primary Transcriber Logic**: `CODE:repo://crates/coldvox-stt-vosk/src/vosk_transcriber.rs#symbol=VoskTranscriber`
-   **Vosk Model Management**: `CODE:repo://crates/coldvox-stt-vosk/src/model.rs`
-   **Core STT Trait**: `CODE:repo://crates/coldvox-stt/src/plugin.rs#symbol=EventBasedTranscriber`

## 3. Key Components

### `VoskTranscriber`

This is the main struct that wraps the `vosk::Recognizer` and implements the `EventBasedTranscriber` trait.

**Key functions:**
- `new()`: Initializes the Vosk model and recognizer based on the provided `TranscriptionConfig`.
- `accept_frame()`: Feeds PCM audio samples into the Vosk recognizer and checks for partial or final results.

```rust
// From: crates/coldvox-stt-vosk/src/vosk_transcriber.rs
pub struct VoskTranscriber {
    recognizer: vosk::Recognizer,
    // ... other fields
}

impl EventBasedTranscriber for VoskTranscriber {
    fn accept_frame(&mut self, pcm: &[i16]) -> Result<Option<TranscriptionEvent>, SttError> {
        // ... implementation ...
    }
}
```

## 4. Dependencies

-   `vosk`: The official Rust bindings for the `libvosk` library.
-   `coldvox-stt`: Provides the core traits and types that this crate implements.
-   `log`: For logging transcription events and errors.
