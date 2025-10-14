---
reviser: GitHub Copilot
date: 2025-10-13
---

# Requirements Document

## Introduction

ColdVox is a modular Rust-based voice-to-text application that provides real-time audio capture, voice activity detection (VAD), speech-to-text (STT) transcription, and cross-platform text injection. The system captures audio from microphones, detects speech, transcribes it to text, and can automatically inject the text into active applications. This document captures the top-level requirements that govern the voice pipeline. Each statement is framed as a "shall" requirement to emphasize expected system behavior.

For long-range architectural intent (always-on intelligent listening, tiered STT memory management), see `docs/architecture.md`.

## Core Requirements

### 1. Audio Capture & Device Management

- The system shall initialize an audio capture stream from the specified device, or fall back to the default device when none is provided or when the target device is unavailable.
- The system shall capture audio in the device's native format (sample rate, bit depth, channels), convert samples to 16-bit signed integers, and resample/downmix to 16 kHz mono downstream in the chunker as needed for VAD and STT processing.
- The system shall expose device selection through configuration (CLI arguments and environment variables) and log any automatic fallbacks or recovery events.
- The system shall restart the capture stream automatically when no audio data is observed for the watchdog interval.


### 2. Voice Activity Detection

- The system shall evaluate incoming audio frames through a voice activity detector to distinguish speech from silence.
- The system shall emit SpeechStart and SpeechEnd events that respect minimum duration thresholds to reduce spurious activations.
- The system shall use Silero V5 ONNX-based VAD as the default and only voice activity detection engine, configured with a threshold of 0.1, minimum speech duration of 100ms, and minimum silence duration of 500ms to stitch together natural pauses in speech.
- The system shall suppress speech events that do not meet minimum speech duration requirements and ignore silence shorter than the configured minimum.

### 3. Speech Transcription

- The system shall buffer audio between SpeechStart and SpeechEnd and submit the buffered segment to the configured STT engine.
- The system shall load the active STT model from the configured path and emit partial as well as final transcription events.
- The system shall surface transcription errors, attempt recovery or failover to alternate STT plugins when available, and clearly report failures.
- The system shall support prioritized plugin ordering and failover thresholds to maintain transcription continuity.

### 4. Text Injection

- The system shall attempt to inject each final transcription into the currently focused application when text injection is compiled with the `text-injection` feature flag.
- The system shall select the appropriate injection backend based on platform (e.g., AT-SPI/clipboard for Wayland, kdotool/ydotool for X11) and provide fallbacks when a backend fails.
- The system shall support disabling text injection only at compile time by omitting the `text-injection` feature; runtime enable/disable is not currently supported.
- The system shall restore clipboard contents after injection operations that use the clipboard.

### 5. Activation & Control Modes

- The system shall offer both VAD-driven activation (continuous listening) and hotkey-driven activation (push-to-talk), configurable at runtime.
- The system shall register global hotkeys across supported desktop environments and use KGlobalAccel on KDE platforms.
- The system shall provide visible or logged feedback when the voice pipeline is activated or deactivated via hotkey.
- The system shall apply activation-mode changes without requiring application restart.

### 6. Observability & Health

- The system shall record and log pipeline metrics (capture, chunker, VAD, buffer utilization) at regular intervals.
- The system shall monitor pipeline health on a cadence and log detailed error information with appropriate severity when issues occur.
- The system shall provide a TUI dashboard that presents near-real-time pipeline status when the TUI binary is used.
- The system shall support file-based logging with daily rotation and enable verbose debugging output when requested.

### 7. Configuration & Extensibility

- The system shall accept configuration via command-line arguments and environment variables for device selection, resampler quality, STT plugins, and injection backends.
- The system shall validate configuration at startup, emit clear errors for invalid combinations, and exit gracefully when validation fails.
- The system shall support explicit selection of resampler quality modes (Fast, Balanced, Quality) and STT plugin limits (e.g., memory caps, fallback order).
- The system shall honor logging configuration (e.g., `RUST_LOG`) and expose sane defaults for common workflows.

### 8. Reliability & Shutdown

- The system shall handle shutdown signals by transitioning through defined application states (Running → Stopping → Stopped) and releasing resources in order.
- The system shall close audio streams, flush logs, and complete or cancel outstanding STT operations during shutdown.
- The system shall await the completion (or timeout) of background tasks and confirm successful shutdown with a final log entry and exit code.

## Future-Facing Requirements

These statements outline the aspirational direction that informs ongoing design. They supplement the core requirements above and may evolve as research continues.

- The system shall evolve toward an always-on intelligent listening mode with a dedicated listening thread that operates independently of STT processing threads.
- The system shall manage STT memory usage dynamically, unloading large models during extended idle periods while keeping lightweight engines ready for fast activation.
- The system shall support a tiered STT architecture (primary/secondary/tertiary engines) with context-aware engine selection and graceful degradation.
- The system shall expose privacy safeguards and user controls (e.g., opt-in consent, status indicators) whenever continuous listening capabilities are enabled.
