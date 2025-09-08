# NVIDIA Parakeet STT Integration Plan for ColdVox (Revised)

## Executive Summary

This document outlines the revised plan for integrating NVIDIA Parakeet TDT-0.6B as an STT plugin using **ONNX Direct integration**. With the discovery of pre-exported ONNX models, we can eliminate Python dependencies entirely while achieving superior performance.

### Key Decision: ONNX Direct Architecture

**We are implementing ONNX Direct integration only.** No subprocess, no shared memory, no Python.

**Justification:**
- Pre-exported models available at `istupakov/parakeet-tdt-0.6b-v2-onnx` eliminate export risk
- Zero IPC overhead vs 10-20ms for subprocess
- Single binary deployment without Python environment complexity
- `ort` crate is mature with 500K+ downloads
- Simpler than subprocess + faster than any IPC variant

### Goals (Revised)
- **Primary**: Direct ONNX integration with `ort` crate
- **Secondary**: GPU acceleration with CPU fallback
- **Tertiary**: Single binary deployment without Python dependencies

### Success Criteria (Revised)
- [ ] Functional Parakeet plugin via ONNX Runtime
- [ ] Improved WER compared to Vosk baseline
- [ ] Real-time viable processing latency
- [ ] Automatic GPU/CPU fallback
- [ ] No Python dependencies in deployment

## Architectural Decisions & Justifications

### 1. Runtime Architecture: ONNX Direct ✅

**Decision:** Use ONNX Runtime via `ort` crate exclusively.

**Justification:**
- **Performance**: Eliminates IPC overhead inherent in subprocess approaches
- **Simplicity**: Single process, no subprocess management, no crash recovery needed
- **Deployment**: One Rust binary, no Python environment to manage
- **Proven**: Pre-exported models at `istupakov/parakeet-tdt-0.6b-v2-onnx` work today
- **Maintenance**: No version sync between Python/Rust, just update ONNX model file

**Rejected Alternatives:**
- ❌ **Subprocess**: Adds IPC latency and process management complexity
- ❌ **Shared Memory**: Marginal improvement over subprocess with significant complexity
- ❌ **PyO3**: Python GIL issues, complex dependency management

### 2. Model Selection: TDT-0.6B-v2 ✅

**Decision:** Use Parakeet TDT (Token-and-Duration Transducer) 0.6B v2 model.

**Justification:**
- **Available Now**: Pre-exported ONNX at `istupakov/parakeet-tdt-0.6b-v2-onnx`
- **Best Accuracy**: 6.05% WER on HuggingFace leaderboard
- **Versatile**: Works for both batch (VAD segments) and streaming
- **Proven**: 12,798 downloads last month, battle-tested

**Rejected Alternatives:**
- ❌ **CTC**: Would need manual export, simpler but less accurate
- ❌ **RNNT**: Would need export, more complex for marginal streaming benefit

### 3. Processing Mode: Batch-First ✅

**Decision:** Start with batch processing of complete VAD segments.

**Justification:**
- **Aligns with VAD**: Silero already provides complete speech segments
- **Simpler**: No streaming state management initially
- **Sufficient**: VAD segments are typically 1-5 seconds, fine for dictation
- **Iterative**: Can add streaming later if needed

### 4. Precision Strategy: FP16 on GPU, FP32 on CPU ✅

**Decision:** Use FP16 for GPU inference, FP32 for CPU fallback.

**Justification:**
- **RTX 3090 Optimized**: Tensor cores accelerate FP16 significantly
- **Memory Efficient**: Halves VRAM usage (important for 24GB limit)
- **No Accuracy Loss**: Negligible impact on WER
- **CPU Compatible**: FP32 ensures compatibility when GPU unavailable

### 5. Model Loading: Lazy Download + Caching ✅

**Decision:** Download models on first run, cache in `~/.cache/coldvox/parakeet/`.

**Justification:**
- **User-Friendly**: No manual model download required
- **Efficient**: One-time 600MB download, reused forever
- **Standard**: Follows XDG base directory specification
- **Flexible**: Allows manual placement for offline deployments


## Simplified Implementation Architecture

```
┌─────────────────────────────────────────────────┐
│            ColdVox Single Process               │
├─────────────────────────────────────────────────┤
│  Audio Pipeline (16kHz, i16)                    │
│       ↓                                         │
│  VAD (Silero on CPU)                           │
│       ↓ (complete segments)                    │
│  ParakeetPlugin                                │
│   ├── i16 → f32 conversion                     │
│   ├── ort::Session (ONNX Runtime)              │
│   ├── CUDA EP (primary) / CPU EP (fallback)    │
│   └── TDT decoding → text + timestamps         │
└─────────────────────────────────────────────────┘
```

## Revised Implementation Plan

### Phase 1: Foundation & Setup

#### Goals
- Crate structure with ONNX Runtime integration
- Model downloading and caching
- Basic plugin trait implementation

#### Deliverables

**1.1 Crate Structure**
```toml
# Cargo.toml
[dependencies]
ort = { version = "2.0", features = ["cuda", "tensorrt"] }
ndarray = "0.15"
hf-hub = "0.3"
tokio = { version = "1.35", features = ["fs", "macros"] }
anyhow = "1.0"
tracing = "0.1"
```

**1.2 Core Components**
- [ ] `model_loader.rs`: HuggingFace model download with progress
- [ ] `onnx_session.rs`: ONNX Runtime session management
- [ ] `gpu_detector.rs`: CUDA availability checking
- [ ] `plugin.rs`: SttPlugin trait implementation

### Phase 2: Core Inference

#### Goals
- Audio pipeline integration
- ONNX inference implementation
- Basic transcription working end-to-end

#### Deliverables

**2.1 Audio Processing**
- [ ] `audio_converter.rs`: i16 → f32 normalization [-1, 1]
- [ ] Tensor preparation with batch dimension [1, samples]
- [ ] Buffer management for VAD segments

**2.2 Inference Pipeline**
```rust
// Simplified inference flow
let audio_tensor = convert_to_f32(&samples);
let input = Value::from_array(&session.allocator(), &audio_tensor)?;
let outputs = session.run(vec![input])?;
let text = decode_tdt_output(&outputs)?;
```

### Phase 3: Optimization & Polish

#### Goals
- GPU optimization (FP16, batching)
- Robust error handling
- Performance profiling

#### Deliverables

**3.1 Performance**
- [ ] FP16 inference on RTX 3090
- [ ] Warm-up inference to pre-JIT
- [ ] Memory pooling for repeated allocations

**3.2 Robustness**
- [ ] Automatic GPU→CPU fallback
- [ ] Timeout protection (5s max)
- [ ] Graceful degradation on errors

### Phase 4: Testing & Integration

#### Goals
- Comprehensive testing suite
- Integration with ColdVox pipeline
- Performance validation

#### Deliverables

**4.1 Testing**
- [ ] Unit tests for audio conversion
- [ ] Integration tests with real audio files
- [ ] Benchmark suite for performance tracking
- [ ] WER evaluation against LibriSpeech

**4.2 Integration**
- [ ] Wire into ColdVox STT plugin system
- [ ] Test with VAD pipeline
- [ ] Validate text injection flow


## Risk Analysis (Revised)

### Risks & Mitigation Strategies

| Risk | Probability | Impact | Mitigation | Justification |
|------|------------|--------|------------|---------------|
| **ONNX model incompatibility** | Low | High | Test with pre-exported model first, keep Whisper as backup | Pre-exported models proven to work |
| **GPU memory exhaustion** | Low | Medium | Implement 2GB VRAM limit, auto-fallback to CPU | RTX 3090 has 24GB, plenty of headroom |
| **Model download failures** | Low | Low | Retry with exponential backoff, allow manual placement | Standard practice, HuggingFace reliable |
| **CPU fallback too slow** | Medium | Low | Accept 500ms latency on CPU as acceptable | Users with GPU get full speed |
| **ONNX Runtime bugs** | Low | Medium | Pin to stable version 1.16, extensive testing | Mature library with wide adoption |

### Why These Risks Are Manageable

1. **Pre-exported models** eliminate the biggest risk (ONNX export complexity)
2. **ort crate** is battle-tested in production (Twitter, Bloop)
3. **Fallback options** exist (CPU mode, Whisper plugin, Vosk)
4. **Simple architecture** reduces failure modes (no IPC, no subprocess)

## Performance Considerations

### Resource Requirements

| Resource | GPU Mode | CPU Mode | Notes |
|----------|----------|----------|-------|
| **Memory** | ~1GB VRAM | ~2GB RAM | Model + runtime overhead |
| **Disk** | ~600MB | ~600MB | Model storage |

### Optimization Strategies

1. **GPU Acceleration**: Utilize CUDA execution provider when available
2. **FP16 Inference**: Use half-precision on compatible GPUs
3. **Direct Memory Access**: No serialization/deserialization overhead
4. **Batch Processing**: Process complete VAD segments

## Contingency Plans (Only if ONNX Completely Fails)

### Fallback Option: Whisper.cpp Integration

**Only consider if ONNX integration is completely blocked.**

If Parakeet ONNX fails:
1. Use existing Whisper plugin stub as template
2. Integrate whisper.cpp (proven C++ library)
3. Similar accuracy improvements (though not as good as Parakeet)
4. Well-documented Rust FFI path

**Why this is unlikely to be needed:**
- Pre-exported ONNX models already validated
- `ort` crate proven in production
- Multiple successful Parakeet ONNX deployments exist

### Explicitly Rejected: Subprocess/IPC Approaches

**We will NOT implement:**
- ❌ Python subprocess with JSON IPC (adds latency)
- ❌ Shared memory IPC (complex for minimal gain)
- ❌ gRPC/HTTP microservice (network latency)
- ❌ PyO3 embedding (Python dependency hell)

**Justification:** ONNX Direct is superior in every metric. Any IPC approach adds permanent overhead that cannot be optimized away.

## Testing Strategy

### Test Categories

1. **Unit Tests** (`cargo test -p coldvox-stt-parakeet`)
   - Audio conversion accuracy
   - Protocol serialization
   - Error handling

2. **Integration Tests** (`tests/integration/`)
   - Full pipeline with mock Python service
   - Real Parakeet service integration
   - VAD→STT event flow

3. **Performance Tests** (`benches/`)
   - Latency benchmarks
   - Throughput measurements
   - Memory profiling

4. **Acceptance Tests**
   - WER evaluation on standard datasets
   - Real-world audio samples
   - User acceptance criteria

### Test Data Requirements

- LibriSpeech test-clean dataset for WER baseline
- Noisy audio samples for robustness testing
- Various accent and speaking rate samples
- Long-form audio for streaming tests

## Deployment & Operations

### Installation Flow

```bash
# 1. Check system requirements
./scripts/check_parakeet_requirements.sh

# 2. Install Python dependencies
cd crates/coldvox-stt-parakeet/python
pip install -r requirements.txt

# 3. Download model (automated on first run)
python download_model.py --model parakeet-tdt-0.6b-v3

# 4. Build with feature flag
cargo build --features parakeet

# 5. Run with Parakeet enabled
COLDVOX_STT_PLUGIN=parakeet cargo run
```

### Configuration

```toml
# coldvox.toml
[stt.parakeet]
enabled = true
model_size = "0.6b"  # or "1.1b"
device = "auto"      # "cuda", "cpu", or "auto"
model_path = "~/.cache/coldvox/parakeet/models"
python_path = "python3"  # or specific virtualenv
batch_size = 4
beam_size = 5
temperature = 0.0

[stt.parakeet.fallback]
plugin = "vosk"      # Fallback if Parakeet fails
```

### Monitoring

Key metrics to track:
- Transcription latency (p50, p95, p99)
- WER on production audio
- Process restarts per hour
- GPU memory utilization
- Model loading time
- Error rates by category

### Troubleshooting Guide

| Issue | Symptoms | Solution |
|-------|----------|----------|
| Process won't start | "Failed to spawn Python process" | Check Python path, verify dependencies |
| High latency | >2s for short segments | Check GPU availability, reduce batch size |
| Memory errors | OOM kills, VRAM exhaustion | Reduce batch size, use CPU mode |
| Poor accuracy | High WER on clear audio | Verify model version, check audio format |
| Frequent restarts | Process cycling in logs | Check Python errors, memory leaks |

## Success Metrics

### Quantitative Metrics
- [ ] WER < 10% on LibriSpeech test-clean (Vosk baseline: ~15%)
- [ ] P95 latency < 200ms for 3s segments (GPU)
- [ ] Memory usage < 4GB RSS
- [ ] Zero process crashes in 24-hour test
- [ ] >95% availability over 7-day period

### Qualitative Metrics
- [ ] User preference vs Vosk in A/B testing
- [ ] Improved punctuation and capitalization
- [ ] Better handling of technical vocabulary
- [ ] Reduced manual corrections needed

## Resource Requirements

### Development Resources
- Rust developer with ONNX Runtime experience
- Access to NVIDIA GPU for testing

### Infrastructure
- Development: Local workstation with GPU
- CI/CD: GPU-enabled runners for tests
- Storage: 5GB for models and test data

## Conclusion

### The Clear Path Forward: ONNX Direct Only

After comprehensive analysis, the decision is unequivocal:

**Implement Parakeet via ONNX Direct integration using the `ort` crate.**

### Why This Decision Is Final

1. **Pre-exported models exist**: `istupakov/parakeet-tdt-0.6b-v2-onnx` eliminates export risk
2. **Superior performance**: Zero IPC overhead, direct memory access
3. **Simpler deployment**: Single Rust binary, no Python environment
4. **Proven technology**: `ort` crate used in production by major companies
5. **Reduced complexity**: No subprocess management or error recovery needed

### What We're NOT Doing

- ❌ **No subprocess architecture** - Adds permanent latency
- ❌ **No shared memory "optimization"** - Complexity without meaningful benefit
- ❌ **No Python dependencies** - Deployment simplicity is paramount
- ❌ **No parallel implementation paths** - ONNX is the only path

### Expected Outcomes

| Metric | Target | Justification |
|--------|--------|---------------|
| **WER** | Improved vs Vosk | Parakeet SOTA model |
| **Latency** | Real-time viable | Direct inference, no IPC |
| **Development** | Simplified | Single integration point |
| **Maintenance** | Minimal | No subprocess management |

### Final Architecture

```
ColdVox → VAD → Parakeet ONNX → Text Output
         CPU    GPU/CPU
```

Simple. Fast. Maintainable.

**This plan supersedes all previous proposals. We are committed to ONNX Direct as the sole implementation strategy.**

## Appendices

### A. ONNX Rust Implementation Sketch

```rust
// plugin.rs - ONNX Direct Implementation
use async_trait::async_trait;
use ndarray::{Array1, Array2};
use ort::{Environment, SessionBuilder, Value, ExecutionProvider, CUDAExecutionProvider};

pub struct ParakeetPlugin {
    session: ort::Session,
    sample_rate: u32,
    buffer: Vec<f32>,
}

impl ParakeetPlugin {
    pub async fn new() -> Result<Self, ParakeetError> {
        // Initialize ONNX Runtime environment
        let environment = Environment::builder()
            .with_name("parakeet")
            .with_log_level(ort::LoggingLevel::Warning)
            .build()?;

        // Download model if not cached
        let model_path = Self::ensure_model_cached().await?;

        // Create session with GPU support if available
        let mut builder = SessionBuilder::new(&environment)?;

        // Try CUDA first, fallback to CPU
        if CUDAExecutionProvider::default().is_available() {
            builder = builder.with_execution_providers([
                CUDAExecutionProvider::default().build(),
            ])?;
        }

        let session = builder.with_model_from_file(model_path)?;

        Ok(Self {
            session,
            sample_rate: 16000,
            buffer: Vec::new(),
        })
    }

    async fn ensure_model_cached() -> Result<PathBuf, ParakeetError> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("coldvox/parakeet");

        let model_path = cache_dir.join("parakeet-tdt-0.6b-v2.onnx");

        if !model_path.exists() {
            // Download from HuggingFace
            let api = hf_hub::api::tokio::Api::new()?;
            let repo = api.model("istupakov/parakeet-tdt-0.6b-v2-onnx".to_string());
            repo.get("model.onnx").await?;
        }

        Ok(model_path)
    }

    fn prepare_audio(&self, samples: &[i16]) -> Array2<f32> {
        // Convert i16 to f32 normalized to [-1, 1]
        let float_samples: Vec<f32> = samples
            .iter()
            .map(|&s| s as f32 / 32768.0)
            .collect();

        // Create batch dimension [1, samples]
        Array2::from_shape_vec((1, float_samples.len()), float_samples)
            .expect("Failed to create audio tensor")
    }
}

#[async_trait]
impl SttPlugin for ParakeetPlugin {
    async fn process_audio(&mut self, samples: &[i16])
        -> Result<Option<TranscriptionEvent>, SttPluginError>
    {
        // Buffer audio for complete VAD segments
        let audio_tensor = self.prepare_audio(samples);

        // Prepare inputs
        let inputs = vec![Value::from_array(self.session.allocator(), &audio_tensor)?];

        // Run inference
        let outputs = self.session.run(inputs)?;

        // Decode outputs (model-specific)
        let logits = outputs[0].try_extract::<f32>()?;
        let text = self.decode_tdt_output(logits)?;

        Ok(Some(TranscriptionEvent::Final {
            utterance_id: self.utterance_counter,
            text,
            words: self.extract_word_timestamps(&outputs),
        }))
    }
}
```

### B. Fallback Python Service (Only if ONNX blocked)

```rust
// plugin.rs
use async_trait::async_trait;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct ParakeetPlugin {
    process: Option<Child>,
    config: ParakeetConfig,
}

impl ParakeetPlugin {
    async fn ensure_process(&mut self) -> Result<(), ParakeetError> {
        if self.process.is_none() {
            let mut cmd = Command::new(&self.config.python_path);
            cmd.arg("-u")  // Unbuffered output
               .arg("parakeet_service.py")
               .stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .stderr(Stdio::piped());

            self.process = Some(cmd.spawn()?);
        }
        Ok(())
    }

    async fn send_request(&mut self, request: TranscribeRequest)
        -> Result<TranscribeResponse, ParakeetError>
    {
        self.ensure_process().await?;

        let process = self.process.as_mut().unwrap();
        let stdin = process.stdin.as_mut().unwrap();
        let stdout = process.stdout.as_mut().unwrap();

        // Send request
        let request_json = serde_json::to_string(&request)?;
        stdin.write_all(request_json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read response
        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: TranscribeResponse = serde_json::from_str(&response_line)?;
        Ok(response)
    }
}

#[async_trait]
impl SttPlugin for ParakeetPlugin {
    async fn process_audio(&mut self, samples: &[i16])
        -> Result<Option<TranscriptionEvent>, SttPluginError>
    {
        // Convert i16 to f32
        let float_samples: Vec<f32> = samples.iter()
            .map(|&s| s as f32 / 32768.0)
            .collect();

        let request = TranscribeRequest {
            id: uuid::Uuid::new_v4().to_string(),
            type_: "transcribe".to_string(),
            audio: Some(AudioData {
                samples: float_samples,
                sample_rate: 16000,
                channels: 1,
            }),
            config: None,
        };

        let response = self.send_request(request).await?;

        // Convert response to TranscriptionEvent
        match response.result {
            Some(result) => Ok(Some(TranscriptionEvent::Final {
                utterance_id: self.utterance_counter,
                text: result.text,
                words: result.words.map(|words| {
                    words.into_iter().map(|w| WordInfo {
                        text: w.text,
                        start: w.start,
                        end: w.end,
                        conf: w.confidence,
                    }).collect()
                }),
            })),
            None => Ok(None),
        }
    }
}
```

### C. Installation Script

```bash
#!/bin/bash
# scripts/setup_parakeet.sh

set -e

echo "Setting up Parakeet STT plugin..."

# Check Python version
python_version=$(python3 --version 2>&1 | grep -oP '\d+\.\d+')
if (( $(echo "$python_version < 3.8" | bc -l) )); then
    echo "Error: Python 3.8+ required, found $python_version"
    exit 1
fi

# Check CUDA (optional)
if command -v nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU detected:"
    nvidia-smi --query-gpu=name,memory.total --format=csv
else
    echo "No NVIDIA GPU detected, will use CPU mode"
fi

# Create virtual environment
echo "Creating Python virtual environment..."
python3 -m venv ~/.cache/coldvox/parakeet/venv

# Activate and install dependencies
source ~/.cache/coldvox/parakeet/venv/bin/activate
pip install --upgrade pip
pip install -r crates/coldvox-stt-parakeet/python/requirements.txt

# Download model
echo "Downloading Parakeet model..."
python3 -c "
import nemo.collections.asr as nemo_asr
model = nemo_asr.models.ASRModel.from_pretrained('nvidia/parakeet-tdt-0.6b-v3')
print('Model downloaded successfully')
"

echo "Parakeet setup complete!"
```
