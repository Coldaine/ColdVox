---
id: TST-002
title: Transcription Tests
level: test
status: drafting
owners:
  - CDIS
criticality: 3
parent: null
pillar_trace:
  - PIL-002
  - DOM-002
  - SUB-002
  - SYS-002
verifies:
  - "SPEC-002"
---

# Transcription Tests [TST-002]

## 1. Overview

This document describes the test suite for the Transcription Engine System, which verifies the functionality defined in [SPEC-002](SPEC-002-transcription-api.md). The tests for the STT components are primarily end-to-end tests that process a WAV file and assert the transcribed text.

## 2. Code-level Traceability

The primary test case is located in the main application crate:

-   **End-to-End WAV Test**: `CODE:repo://crates/app/src/stt/tests/end_to_end_wav.rs`
-   *(Conceptual: Unit tests for the `VoskTranscriber` may exist as inline modules within the `coldvox-stt-vosk` crate.)*

## 3. Test Cases

This test suite includes the following key validation scenario:

### `end_to_end_wav.rs`

-   **`test_end_to_end_wav`**: This is an integration test that:
    1.  Loads a pre-recorded WAV file containing speech.
    2.  Initializes the full VAD-gated STT pipeline.
    3.  Processes the audio from the WAV file.
    4.  Asserts that the final transcription event from the `VoskTranscriber` matches the expected text.

```rust
// From: crates/app/src/stt/tests/end_to_end_wav.rs
#[test]
#[ignore] // Ignored by default as it requires a Vosk model
fn test_end_to_end_wav() {
    // ... test implementation ...
}
```

## 4. Test Strategy

-   **Unit Tests**: Individual components should have unit tests co-located with their source code. Due to the nature of STT, these are often difficult to write and are less common.
-   **Integration Tests**: The primary testing strategy relies on integration tests like `test_end_to_end_wav`, which validate the entire STT pipeline against known audio inputs and expected text outputs. These tests are marked as `#[ignore]` because they require a Vosk model to be present at `VOSK_MODEL_PATH`.
