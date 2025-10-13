# Evaluation of STT Backend Migration & Noise Reduction Plan

## Overview
This evaluation assesses the feasibility, risks, benefits, and recommendations for migrating the ColdVox STT backend from the current Vosk plugin-based architecture to direct integrations with Whisper and NVIDIA Parakeet/Canary, while incorporating noise reduction. The analysis is based on the research summary in [STT_MIGRATION_AND_NOISE_RESEARCH.md](STT_MIGRATION_AND_NOISE_RESEARCH.md), codebase analysis, and alignment with project goals (simplicity, performance, local/GPU support).

## Feasibility Assessment

### 1. Codebase Structure and Migration Path
- **Current Architecture**:
  - `coldvox-stt` provides a plugin system via `SttPlugin` trait, registry (`SttPluginRegistry`), and event-based processing (`EventBasedTranscriber`, `StreamingStt`).
  - Vosk integration in `coldvox-stt-vosk` implements these traits, with model loading in `plugin.rs` and transcription in `vosk_transcriber.rs`.
  - Audio pipeline in `SttProcessor` buffers VAD-gated frames (16kHz, 512-sample) and feeds to the STT engine on speech end.
  - Dependencies: Relies on `coldvox-audio` for capture/resampling, `coldvox-vad` for gating.

- **Target Architecture Feasibility**:
  - **High Feasibility (8/10)**: The plugin abstraction adds indirection (registry, factories, failover config) but is not deeply entangled. Core pipeline (`SttProcessor`) uses trait objects (`StreamingStt`), making backend swaps straightforward.
  - **Migration Steps**:
    - Remove `plugin.rs`, `plugin_types.rs`, registry, and Vosk-specific code (~200-300 LOC deletion).
    - Introduce enum `SttBackend { Whisper, Parakeet, Mock }` with implementations of `StreamingStt`.
    - For Whisper: Use `whisper-rs` crate – load context once, feed frames via `full_transcribe` or incremental mode. Supports GPU via CUDA if available.
    - For Parakeet/Canary: Use `ort` (ONNX Runtime Rust) for model inference; export NeMo models to ONNX. FFI via PyO3 if needed for Riva SDK (adds ~50-100ms startup but enables streaming gRPC).
    - Selection: Build-time Cargo features (`--features whisper parakeet`) or runtime enum from config.toml. Retain `TranscriptionConfig` for shared params (e.g., language, partials).
    - Preserve event interface: Map backend outputs to `TranscriptionEvent` (partials/finals with timestamps).
  - **Effort Estimate**: 2-3 days for Whisper (direct crate), 4-5 days for Parakeet (ONNX/FFI setup). Total refactor: 1 week, including tests.
  - **Risks**: 
    - Real-time latency: Whisper.cpp bindings achieve ~0.5-1x RTFx on GPU; test with 512-sample frames to match current chunking.
    - Model management: Vosk auto-extraction logic reusable for Whisper models (download/extract via reqwest/zip).
    - Compatibility: Ensure 16kHz mono PCM input; resample if needed in `coldvox-audio`.

### 2. Performance and Hardware Considerations
- **Latency & Accuracy**:
  - Whisper (large-v3): 10-20% better WER than Vosk on noisy data; ~1s for 10s utterance on RTX 3060.
  - Parakeet (0.6B): Superior in noise (15% WER improvement over Whisper); <0.5x RTFx on A100/RTX 40-series.
  - Current Vosk: ~2-3x RTFx on CPU; migration enables GPU acceleration, reducing end-to-end latency by 50-70%.
- **Hardware**:
  - CPU-only fallback: Whisper.cpp viable; Parakeet requires NVIDIA GPU (CUDA 11.8+).
  - Detection: Use `nvml-wrapper` crate to check GPU availability at runtime.
- **Benefits**: 2-3x speed-up on GPU; better multilingual support (Canary). Reduces deps (no Vosk lib).

### 3. Noise Reduction Integration
- **Feasibility (9/10)**: Add as pre-processing stage in `coldvox-audio` pipeline (after capture, before VAD).
  - **RNNoise**: Rust crate `rnnoise-sys`; CPU real-time (<1ms/frame), 20-30dB suppression for fans/keyboard. Integrate via FFI: process each frame in `chunker.rs`.
  - **Demucs**: PyTorch-based; GPU, but 50-100ms latency – suitable for batch, not streaming.
  - **NeMo Denoiser**: GPU-optimized; chain with Parakeet via ONNX. ~10ms latency, 15-25% WER gain.
- **Pipeline Update**: `AudioFrame` -> Denoise -> VAD -> Buffer -> STT. Config flag `enable_denoise: bool` in `TranscriptionConfig`.
- **Benchmarks**: RNNoise + Whisper: 10-15% WER drop, <5% latency overhead (test with LibriSpeech + noise overlays). NeMo + Parakeet: Sub-100ms E2E on GPU.
- **Risks**: CPU overhead for RNNoise (~5-10% on low-end); GPU sync for NeMo.

### 4. Testing, CI, and Documentation
- **Tests**: Update `SttProcessor` integration tests to use new backends (mock WAV inputs). Add GPU CI via GitHub Actions (self-hosted runner with NVIDIA).
- **CI Changes**: Feature-gated builds (`vosk` -> `whisper`, `parakeet`); model download in CI scripts.
- **Docs**: Update READMEs, add GPU setup guide. Remove Vosk-specific sections.

## Risks and Mitigations
- **High Risk**: Parakeet FFI complexity – Mitigate: Prototype with ONNX first; fallback to Whisper.
- **Medium Risk**: Breaking changes to event interface – Mitigate: Keep `StreamingStt` trait; deprecate plugins gradually.
- **Low Risk**: Model licensing (CC-BY-NC for Parakeet) – Suitable for local use; document restrictions.
- **Overall Risk**: Low-Medium; modular design allows phased rollout (Whisper first).

## Recommendations
- **Prioritize**: Whisper for broad compatibility; Parakeet for NVIDIA users (feature flag).
- **Phased Approach**: 1) Remove Vosk/plugins. 2) Add Whisper. 3) Noise reduction. 4) Parakeet.
- **Next**: Proceed to detailed plan with code sketches and diagrams. Aligns well with goals of simplicity and performance.

## Alignment with Vision
The proposed migration reduces abstraction overhead while enabling modern, GPU-accelerated STT. It maintains real-time pipeline integrity and adds denoising for robustness in noisy environments (e.g., keyboards, fans). No major deviations needed unless specific priorities (e.g., CPU-only) are emphasized.