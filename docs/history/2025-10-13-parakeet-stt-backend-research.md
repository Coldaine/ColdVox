---
doc_type: history
subsystem: stt
status: archived
freshness: historical
preservation: permanent
last_reviewed: 2026-02-12
owners: Coldaine
version: 1.0.0
---

# Parakeet STT Backend Research (2025-10-13)

## Context

Research into implementing Parakeet-TDT-1.1B model in Rust, comparing two primary open-source stacks for integration: sherpa-onnx (with sherpa-rs bindings) and transcribe-rs.

## Key Architectural Findings

### sherpa-onnx vs transcribe-rs Comparison

**sherpa-onnx**:
- C++ core with Rust bindings (sherpa-rs crate)
- Production-grade with VAD, streaming, batching, speaker diarization, TTS
- Mature ecosystem with active development
- CUDA GPU support via ONNX Runtime
- Complex dependency chain (ONNX Runtime, CUDA toolkit, cuDNN)
- **Key feature**: `download-binaries` flag bypasses native compilation

**transcribe-rs**:
- Pure Rust library, minimal API
- Focus on simple batch transcription
- Vulkan GPU support on Linux via onnxruntime crate
- No streaming VAD or advanced pipeline features
- Simpler but places full burden of native deps on user

### GPU Acceleration Details

**sherpa-onnx CUDA path**:
- Requires glibc >= 2.28
- CUDA 11.8 compatibility (sherpa-onnx v1.17.1 built against CUDA 11.8)
- Critical finding: GPU only benefits parallel/streaming workloads
- Single utterance: CPU faster due to GPU transfer overhead
- Must "warm up" model before seeing GPU gains

**transcribe-rs Vulkan path**:
- Native Linux Vulkan support via onnxruntime crate
- Performance: 20x realtime on Zen 3 CPU, 30x on M4 Max
- More cross-platform but less vendor control

### Dependency Management Strategy

**sherpa-rs solution to dependency hell**:
```toml
[dependencies]
sherpa-rs = { version = "0.6.8", features = ["download-binaries", "cuda"] }
```

The `download-binaries` feature:
- Downloads pre-built sherpa-onnx binaries
- Bypasses complex native compilation
- Avoids manual CUDA/cuDNN installation
- Direct solution to "dependency heavy builds" problem

**Trade-off**: Locked to maintainer's build configuration, can't customize without rebuilding native lib.

## Technical Implementation Patterns

### Audio Preprocessing for Parakeet

Parakeet-TDT-1.1B expects 16kHz mono audio:

```rust
fn normalize_i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter()
        .map(|&sample| sample as f32 / 32768.0)
        .collect()
}

// Create tensor with batch dimension [1, num_samples]
let audio_array = Array2::from_shape_vec((1, normalized.len()), normalized)?;
```

### CTC Decoding Options

1. **ctclib-pp** - Rust native with KenLM support (recommended)
2. **fast-ctc-decode** - Based on nanoporetech implementation
3. **Greedy decoder** - Simple fallback without language model

### ONNX Runtime Setup

```rust
use ort::{
    execution_providers::{CUDAExecutionProvider, CPUExecutionProvider},
    session::Session,
    GraphOptimizationLevel,
};

Session::builder()?
    .with_execution_providers([
        CUDAExecutionProvider::default().build(),
        CPUExecutionProvider::default().build(),
    ])?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .commit_from_file(model_path)?
```

### Parakeet Model Artifacts

Parakeet-TDT-1.1B uses SentencePiece Unigram tokenizer with 1024 tokens.

Vocabulary embedded in `.nemo` file:
- `tokenizer.model` - SentencePiece model
- `tokenizer.vocab` - Vocabulary mappings
- `vocab.txt` - Plain text vocab list

Output tensor: `[batch_size, sequence_length, vocab_size]` where vocab_size=1024

## Strategic Recommendation

**Chosen path**: sherpa-onnx with `download-binaries` feature

**Rationale**:
1. Superior GPU acceleration story (CUDA + TensorRT options)
2. Direct solution to dependency pain via pre-built binaries
3. Production-grade streaming architecture scales with GPU parallelism
4. Comprehensive pipeline features (VAD, streaming) needed for high-performance app

**Implementation priority**:
1. Environment setup (glibc >= 2.28, CUDA 11.x, cuDNN)
2. sherpa-rs with download-binaries + cuda features
3. OnlineRecognizer API for streaming (capitalize on GPU parallelism)
4. Resource management (wrap C++ objects, call Delete in Drop trait)

## Performance Expectations

- GPU benefits manifest in parallel/multi-stream scenarios
- Single utterance: CPU competitive due to transfer overhead
- Target: Design for streaming architecture to fully utilize GPU
- Memory management: Explicit cleanup required for C++ resources

## Key Lessons

1. **GPU performance is workload-dependent** — single-threaded inference may be slower on GPU due to transfer overhead
2. **Dependency management via feature flags** — pre-built binaries eliminate native compilation complexity
3. **Python ecosystem coupling risks** — `onnxruntime` version compatibility with `uv` package manager had known issues
4. **Native library version constraints** — CUDA 11.8 requirement vs 12.x created deployment challenges

## Applicability

This research informed the decision to explore Parakeet integration. ColdVox ultimately moved to parakeet-rs (pure Rust ONNX wrapper) rather than sherpa-onnx, and Moonshine is the current production STT backend. The architectural insights about GPU utilization, streaming design, and dependency management remain relevant for future model evaluations.

## References

- sherpa-rs crate: https://crates.io/crates/sherpa-rs
- Parakeet-TDT-1.1B: NVIDIA NeMo model
- ONNX Runtime execution providers documentation
