---
doc_type: index
subsystem: stt
version: 1.0.0
status: approved
freshness: current
preservation: preserve
owners: STT Team
last_reviewed: 2026-02-12
last_reviewer: Jules
review_due: 2026-08-12
domain_code: stt
---

# Speech-to-Text (STT) Overview

Core speech-to-text abstraction layer and plugin system for ColdVox.

## Purpose

The STT domain is responsible for converting processed audio frames into text. It provides a flexible, plugin-based architecture that supports multiple backends, allowing ColdVox to adapt to different hardware capabilities and accuracy requirements.

## Key Components

- **STT Plugin Trait**: The core interface that all transcription backends must implement.
- **Plugin Manager**: Orchestrates the lifecycle of STT plugins (loading, initialization, unloading).
- **Transcription Events**: Standardized output format for both partial and final transcription results.
- **SttProcessor**: A high-level component that connects VAD-gated audio streams to the active STT plugin.

## Supported Backends

- **Moonshine**: Current primary backend (Python-based, supports CPU and GPU).
- **Parakeet**: Planned high-performance backend using NVIDIA's Parakeet models.
- **Whisper**: Legacy/removed path (not recommended for active use).

## Documentation

- [Parakeet Integration Plan](stt-parakeet-integration-plan.md): Detailed analysis and implementation strategy for Parakeet support.
- [Reference: coldvox-stt](../../reference/crates/coldvox-stt.md): Crate-level index linking to the core implementation.

## Crate Links

- [coldvox-stt](../../../crates/coldvox-stt/README.md): Core abstraction layer.
