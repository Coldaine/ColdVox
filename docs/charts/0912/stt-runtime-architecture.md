# ColdVox STT Runtime Architecture (Post-Migration)

## Overview
The STT runtime in `crates/app/src/runtime.rs` supports dual architectures behind the `COLDVOX_STT_ARCH` env var ("legacy" default, "streaming" opt). Legacy uses batch `PluginSttProcessor` with direct `AudioFrame` (f32) and `VadEvent`. Streaming introduces async `StreamingSttProcessor` with format conversion (f32→i16 at 16kHz), monotonic timestamps, and `StreamingVadEvent` mapping via dedicated forwarder and fanout extensions. Both paths share audio/VAD sources, plugin_manager (always for streaming), and `stt_tx` mpsc<TranscriptionEvent> to consumers (TUI, injection). Plugin manager handles mock/noop/whisper/parakeet selection, failover, GC.

Key changes:
- Single branch in `start()` after audio/VAD setup.
- Streaming always initializes `SttPluginManager` (defaults if no `opts.stt_selection`).
- Audio forwarder: subscribes to `audio_tx`, converts to `StreamingAudioFrame { data: Vec<i16>, sample_rate: 16000, timestamp_ms: monotonic 32ms increments }`.
- VAD fanout: maps `VadEvent::SpeechStart/End` to `StreamingVadEvent` alongside broadcast.
- Tests verify both paths end-to-end with mock plugin (VAD/audio → final events, clean shutdown).

## Components & Data Flows
- **External Inputs**: Microphone → `AudioCaptureThread` → ring buffer → `AudioChunker` (resamples to 16kHz frames) → `audio_tx` broadcast<coldvox_audio::AudioFrame>.
- **Activation**: VAD (`VadProcessor`) or Hotkey → `raw_vad_tx` mpsc<VadEvent>.
- **STT Branch** (env-gated):
  - **Legacy**: If selection, `PluginSttProcessor` subscribes `audio_tx`/`stt_vad_rx` (from fanout), buffers/processes via plugin_manager → `stt_tx`.
  - **Streaming**: `ManagerStreamingAdapter` wraps plugin_manager; forwarder converts audio → `stream_audio_tx` broadcast<StreamingAudioFrame>; fanout maps VAD → `stream_vad_tx` mpsc<StreamingVadEvent>; `StreamingSttProcessor` processes incrementally → `stt_tx`.
- **Consumers**: `vad_tx` broadcast to TUI/UI; `stt_rx` to transcription handlers (injection, display); metrics shared via `PipelineMetrics`.
- **Shutdown**: `AppHandle::shutdown()` aborts tasks, unloads plugins, awaits joins for clean termination.

## Diagrams
See embedded SVGs below (generated from Mermaid .mmd files).

![Overall STT Pipeline](stt-overall-pipeline.svg)

![Legacy STT Path](stt-legacy-path.svg)

![Streaming STT Path](stt-streaming-path.svg)

## Evidence from Code (crates/app/src/runtime.rs)
- Env read: `env::var("COLDVOX_STT_ARCH").unwrap_or_else(|_| "legacy".to_string())`; log `info!("STT architecture: {}", stt_arch)`.
- Plugin manager: Always for "streaming" (`create_manager(opts.stt_selection.clone()).await?`); conditional for legacy.
- Audio forwarder: `tokio::spawn` subscribes `audio_tx`, converts `samples: Vec<f32>` → `data: Vec<i16>` (clamp * 32767), `timestamp_ms += 32`.
- VAD mapping: In fanout `tokio::spawn`, `match ev { SpeechStart { timestamp_ms } => send(StreamingVadEvent::SpeechStart { timestamp_ms }) }`.
- Processors: Legacy `PluginSttProcessor::new(stt_audio_rx, stt_vad_rx, stt_tx, plugin_manager, config)`; Streaming `StreamingSttProcessor::new(stream_audio_rx, stream_vad_rx, stt_tx, adapter, config { streaming: true })`.
- Tests: `end_to_end_legacy_stt_pipeline`/`end_to_end_streaming_stt_pipeline` set env, send VAD/audio via tx, assert events in `stt_rx`, `app.shutdown().await`.
- AppHandle: `stt_rx: Some(stt_rx)` (vosk), `plugin_manager` set for both when applicable.

Last reviewed: 2025-09-12