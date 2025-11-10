---
doc_type: plan
subsystem: stt
version: 1.1.0
status: draft
owners: STT Team
last_reviewed: 2025-11-09
---

# Candle Whisper Integration Plan

This document outlines the plan for migrating ColdVox's speech-to-text (STT) backend from `faster-whisper-rs` to the Candle ML framework's Whisper implementation. The primary goal is to leverage Candle's performance, Rust-native ecosystem, and flexibility while mitigating the risks of its alpha-stage API.

## 1. Architectural Approach: Direct Embedding

Based on analysis of Candle's rapid development and the tight integration required, we will forgo a separate plugin crate. Instead, we will embed the Whisper engine directly within the `coldvox-stt` crate.

This approach offers:
-   **Reduced Overhead:** Eliminates the complexity of maintaining a separate crate and its API boundary.
-   **Faster Iteration:** Allows for quicker adaptation to upstream changes in the Candle API during its alpha phase.
-   **Simplified Dependencies:** ColdVox will depend directly on the `candle` crates.

The new implementation will live in a `coldvox_stt::candle` module and will present a clean internal facade, `WhisperEngine`, to the rest of the application.

## 2. Direct Code Verification (2025-11-09)

A direct review of the `candle-transformers` source code for the Whisper model (`model.rs`) has verified the following assumptions:

*   **Multi-lingual Support:** The model's `forward` pass accepts arbitrary token sequences. Language control is achieved by injecting language-specific tokens into this input sequence *before* starting the decoding process. The model itself is language-agnostic.
*   **Timestamp Generation:** The core model does **not** have any built-in logic for generating timestamps. The timestamp tokens are predicted by the model just like any other token. The logic for interpreting these tokens and converting them into time values resides in the example application's decoding loop (`whisper/main.rs`). This confirms that we must port this logic ourselves.
*   **Decoding Process:** The `Decoder`'s `forward` method confirms a token-by-token decoding loop is required. The key-value cache is explicitly managed by passing it into and out of the `forward` method on each iteration, which is typical for efficient auto-regressive decoding.

These findings confirm that the high-level plan to port logic from the example application is correct, as the core `candle-transformers` crate provides the model's building blocks but not the full transcription pipeline logic.

## 3. Core Implementation Phases

The implementation will be broken down into the following phases, creating a new module structure within `crates/coldvox-stt/src/candle/`:

1.  **Audio Preprocessing (`audio.rs`):**
    *   Port the `log_mel_spectrogram` function and related audio processing logic directly from the Candle Whisper examples.
    *   Ensure the implementation correctly converts ColdVox's 16kHz mono PCM audio stream into the required input format for the model.
    *   Maintain attribution to the original Candle source code.

2.  **Model Loading (`loader.rs`):**
    *   Implement a unified model loader that can handle both standard (`safetensors`) and quantized (`.gguf`) model formats.
    *   The loader will also be responsible for loading the tokenizer configuration and the model's JSON config.
    *   It will expose a simple function to instantiate the model and tokenizer based on file paths.

3.  **Core Decoding (`decode.rs`):**
    *   Adapt the token-by-token decoding loop from the Candle Whisper examples.
    *   Implement the core logic for running the encoder and using the decoder's key-value cache.
    *   Initially, focus on greedy sampling, with support for temperature-based fallback.

4.  **Timestamp Generation (`timestamps.rs`):**
    *   Port the timestamp heuristics (`apply_timestamp_rules`) from the Candle examples.
    *   This module will be responsible for translating token-level timestamps into segment-level start and end times.

5.  **Types and Facade (`types.rs`, `mod.rs`):**
    *   Define the public data structures for the engine (e.g., `Transcript`, `Segment`, `WhisperEngineConfig`).
    *   Create the primary `WhisperEngine` facade in `mod.rs` to expose a clean and simple API (`new`, `transcribe`) to the rest of ColdVox.

## 4. Lines of Inquiry for Follow-up

This section details the four primary areas of investigation that must be completed to ensure a successful migration.

### Inquiry 1: `WhisperEngine` Facade API Definition

A clear, stable internal API is critical to insulate the rest of ColdVox from Candle's API churn. The proposed facade is as follows:

```rust
// In crates/coldvox-stt/src/candle/mod.rs

pub struct WhisperEngine { /* ... internal fields ... */ }

pub struct WhisperEngineInit {
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub config_path: PathBuf,
    pub quantized: bool,
    pub device: WhisperDevice, // Enum for Cpu | Cuda(id)
}

pub struct TranscribeOptions {
    pub language: Option<String>,
    pub task: WhisperTask, // Enum for Transcribe | Translate
    pub temperature: f32, // 0.0 for greedy, > 0.0 for sampling
    pub enable_timestamps: bool,
}

pub struct Transcript {
    pub segments: Vec<Segment>,
    // ... other metadata ...
}

pub struct Segment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub text: String,
    pub avg_logprob: f64,
    pub no_speech_prob: f64,
}

impl WhisperEngine {
    /// Creates a new instance of the Whisper engine.
    pub fn new(init: WhisperEngineInit) -> anyhow::Result<Self>;

    /// Transcribes a 16kHz mono f32 PCM audio slice.
    pub fn transcribe(&self, pcm_audio: &[f32], opts: &TranscribeOptions) -> anyhow::Result<Transcript>;
}
```
**Follow-up Action:** Solidify this API contract and present it for architectural review before beginning the `decode.rs` implementation.

### Inquiry 2: Benchmarking Plan

To validate this migration, we must quantitatively measure performance against the existing `faster-whisper` implementation.

**Benchmarking Protocol:**
1.  **Test Dataset:** Curate a standardized set of 5-10 audio files of varying lengths (3s, 10s, 30s) and complexities (clean speech, background noise).
2.  **Metrics:**
    *   **Word Error Rate (WER):** To ensure transcription accuracy is not regressing.
    *   **Real-Time Factor (RTF):** `(Processing Time / Audio Duration)`. This is the primary measure of transcription speed.
    *   **Latency:** Time from the end of audio input to the delivery of the final transcript.
    *   **Peak Memory Usage (RSS):** To monitor the memory footprint.
3.  **Tooling:** Create a simple `cargo bench` or a dedicated example binary that runs both the old and new implementations against the test dataset and reports the metrics above.
4.  **Hardware:** Run benchmarks on both CPU and, if available, a CUDA-enabled GPU.

**Follow-up Action:** Create the benchmark harness and the standardized audio dataset. Run an initial benchmark as soon as a working prototype of the `transcribe` function is available.

### Inquiry 3: Strategy for Multi-Lingual Models

Our code review confirms this is handled by injecting tokens. Our plan is therefore updated with this verification.

**Implementation Plan:**
1.  **Model Loading:** Ensure `loader.rs` can load multi-lingual models (e.g., `large-v3`).
2.  **Language Detection:**
    *   The `transcribe` function will accept an optional language code.
    *   If `None`, the initial token sequence passed to the decoder will *not* include a language token, triggering the model's automatic detection.
    *   The detected language should be extracted from the output tokens and reported back in the final `Transcript`.
3.  **Language Specification:**
    *   If a language code is provided (e.g., `Some("es")`), the corresponding token will be looked up in the tokenizer and prepended to the decoder's initial token sequence.

**Follow-up Action:** Add tests to the benchmark harness that specifically target multi-lingual transcription and language detection.

### Inquiry 4: Feasibility of Word-Level Timestamps

Our code review confirms that the core model only predicts timestamp *tokens*. The logic for converting these to seconds is in the example application.

**Investigation Plan:**
1.  **Port Existing Logic:** The first step is to faithfully port the token-to-timestamp conversion logic from the Candle Whisper example. This will provide segment-level timestamps.
2.  **Explore Token-Level Aggregation:** Once we have segment-level timestamps, investigate the feasibility of using the individual timestamp tokens to derive word boundaries. This will likely be heuristic-based and may have accuracy limitations.
3.  **External Aligners (Future Work):** If token-level aggregation is insufficient, schedule a follow-up research spike to evaluate external forced-alignment models as a post-processing step. This will not be part of the initial migration.

**Follow-up Action:** The initial implementation will focus on porting the segment-level timestamp logic. A separate task will be created in `docs/todo.md` to track the investigation into word-level timestamps.

## Relationship to Current Configuration System

This migration will replace the current `faster-whisper-rs` implementation while maintaining full compatibility with ColdVox's existing model configuration system. The new Candle-based implementation will support all current configuration options:

- Environment-based model size selection (via `WHISPER_MODEL_SIZE`)
- Memory-based model size selection
- Environment-specific defaults (CI, development, production)
- Custom model paths and device selection
- Compute type configuration (int8, float16, float32)

For current model configuration details, see: [Whisper Model Configuration Guide](../whisper-model-configuration.md)

The migration is designed to be transparent to users while providing improved performance and a more maintainable codebase.
