---
id: SYS-002
title: Transcription Engine System
level: system
status: drafting
owners:
  - CDIS
criticality: 4
parent: SUB-002
pillar_trace:
  - PIL-002
  - DOM-002
  - SUB-002
  - SYS-002
---

# Transcription Engine System [SYS-002]

The Transcription Engine System is the technical component that provides a concrete implementation of the speech recognition service. In the default ColdVox configuration, this system is powered by the Vosk toolkit.

This system is primarily implemented by the `VoskTranscriber` component within the `coldvox-stt-vosk` crate.

Key components:
- **`VoskTranscriber`**: The main struct that implements the `EventBasedTranscriber` trait and interfaces with the underlying `libvosk` library.
- **Model Loader**: Logic responsible for loading the Vosk model from the configured path into memory.
- **Audio Buffer**: An internal buffer that accumulates audio samples between calls to the STT engine.
- **Feature Gating**: The entire system is conditionally compiled based on the `vosk` feature flag, allowing it to be excluded from builds if not needed.
