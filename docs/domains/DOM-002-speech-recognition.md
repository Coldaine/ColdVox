---
id: DOM-002
title: Speech Recognition Domain
level: domain
status: drafting
owners:
  - CDIS
criticality: 4
parent: PIL-002
pillar_trace:
  - PIL-002
  - DOM-002
---

# Speech Recognition Domain [DOM-002]

The Speech Recognition Domain is responsible for the entire process of converting a stream of human speech into a sequence of words. It defines the core abstractions and event-based architecture for how the ColdVox system handles speech-to-text functionality.

Key responsibilities of this domain include:
- **STT Abstraction**: Defining a common interface (`EventBasedTranscriber` trait) that allows for multiple, interchangeable STT engines.
- **Event-Based Architecture**: Emitting transcription events (`TranscriptionEvent`) for partial results, final results, and errors.
- **Configuration Management**: Providing a unified configuration structure (`TranscriptionConfig`) for all STT implementations.
- **VAD Integration**: Working with Voice Activity Detection events to process audio only when speech is present.
- **Data Structures**: Defining standard data structures for transcription results, such as word-level timing and confidence (`WordInfo`).
