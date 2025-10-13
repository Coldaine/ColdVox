# Detailed ColdVox STT Migration Plan

This plan outlines the step-by-step migration from the current Vosk plugin-based STT architecture to direct integrations with Whisper and NVIDIA Parakeet/Canary, including noise reduction. It is based on the research in [STT_MIGRATION_AND_NOISE_RESEARCH.md](STT_MIGRATION_AND_NOISE_RESEARCH.md) and evaluation in [STT_Migration_Evaluation.md](STT_Migration_Evaluation.md). The goal is simplicity, performance, and GPU support while maintaining the real-time audio pipeline.

## Phase 1: Preparation and Cleanup (1-2 days)
### Objectives
- Remove Vosk and plugin overhead.
- Set up backend selection mechanism.
- Ensure pipeline compatibility.

### Code Changes
1. **Remove Plugin System** (`crates/coldvox-stt`):
   - Delete: `src/plugin.rs`, `src/plugin_types.rs`, `src/plugins/` directory, `src/plugin_adapter.rs`.
   - Update `src/lib.rs`: Remove plugin exports; retain `StreamingStt`, `EventBasedTranscriber` traits, `TranscriptionConfig`, `TranscriptionEvent`.
   - Delete `SttPluginRegistry` and related configs (`PluginSelectionConfig`, `FailoverConfig`, etc.).
   - Effort: ~100 LOC deletion; refactor `SttProcessor` to use direct `StreamingStt` impl.

2. **Remove Vosk Crate**:
   - Delete entire `crates/coldvox-stt-vosk/` directory.
   - Update `Cargo.toml` in root and `coldvox-stt`: Remove `vosk` feature and deps (vosk-api, etc.).
   - Clean configs: Remove `preferred_plugin: "vosk"` from `config/default.toml`.

3. **Introduce Backend Enum** (`crates/coldvox-stt/src/backend.rs` - new file):
   ```rust
   use crate::{StreamingStt, TranscriptionConfig, TranscriptionEvent};
   use async_trait::async_trait;

   #[derive(Debug, Clone, Copy, PartialEq)]
   pub enum SttBackend {
       Whisper,
       Parakeet,
       Mock, // For tests
   }

   impl SttBackend {
       pub fn from_config(config: &TranscriptionConfig) -> Self {
           match config.backend.as_deref() {
               Some("whisper") => SttBackend::Whisper,
               Some("parakeet") => SttBackend::Parakeet,
               _ => SttBackend::Whisper, // Default
           }
       }
   }

   pub struct BackendStt {
       backend: SttBackend,
       whisper_ctx: Option<whisper_rs::WhisperContext>, // From whisper-rs crate
       parakeet_session: Option<ort::Session>, // From ort crate
       config: TranscriptionConfig,
   }

   #[async_trait]
   impl StreamingStt for BackendStt {
       async fn on_speech_frame(&mut self, samples: &[i16]) -> Option<TranscriptionEvent> {
           match self.backend {
               SttBackend::Whisper => self.whisper_process(samples),
               SttBackend::Parakeet => self.parakeet_process(samples),
               SttBackend::Mock => Some(TranscriptionEvent::Partial { text: "mock".to_string(), ..Default::default() }),
           }
       }

       async fn on_speech_end(&mut self) -> Option<TranscriptionEvent> {
           // Finalize logic per backend
           match self.backend {
               SttBackend::Whisper => self.whisper_finalize(),
               // ...
           }
       }

       async fn reset(&mut self) {
           // Reset state per backend
       }
   }

   // Placeholder methods - implement in Phase 2
   impl BackendStt {
       fn whisper_process(&mut self, _samples: &[i16]) -> Option<TranscriptionEvent> { None }
       fn parakeet_process(&mut self, _samples: &[i16]) -> Option<TranscriptionEvent> { None }
       fn whisper_finalize(&mut self) -> Option<TranscriptionEvent> { None }
   }
   ```
   - Update `SttProcessor`: Replace plugin with `BackendStt::new(config)`.

4. **Update TranscriptionConfig** (`crates/coldvox-stt/src/types.rs`):
   - Add `backend: Option<String>`, `enable_denoise: bool`, `denoiser_type: Option<String>`.

### Tests
- Run existing `cargo test -p coldvox-stt` to ensure cleanup doesn't break traits.
- Add unit test for `SttBackend::from_config`.
- Temporary mock backend tests to verify pipeline.

### CI Updates
- Update `.github/workflows/ci.yml`: Remove `vosk` feature builds; add conditional GPU tests (if NVIDIA runner available).
- Add linter check for removed code.

### Documentation
- Update `crates/coldvox-stt/README.md`: Describe new direct backends.
- Add note in root README: "STT migration in progress - Vosk deprecated."

## Phase 2: Whisper Integration (2-3 days)
### Objectives
- Implement Whisper as primary backend.
- Enable GPU via whisper-rs.

### Code Changes
1. **Add Dependencies** (`crates/coldvox-stt/Cargo.toml`):
   ```
   [dependencies]
   whisper-rs = "0.9"  # For Whisper.cpp bindings
   # Optional: cuda for GPU
   ```

2. **Implement Whisper in BackendStt**:
   - In `backend.rs`:
     ```rust
     use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

     impl BackendStt {
         pub fn new(config: TranscriptionConfig) -> Self {
             let backend = SttBackend::from_config(&config);
             let mut ctx = None;
             if backend == SttBackend::Whisper {
                 let path = config.model_path.unwrap_or("models/ggml-base.en.bin".to_string());
                 ctx = Some(WhisperContext::new(&path).expect("Failed to load Whisper model"));
             }
             Self { backend, whisper_ctx: ctx, parakeet_session: None, config }
         }

         fn whisper_process(&mut self, samples: &[i16]) -> Option<TranscriptionEvent> {
             if let Some(ctx) = &mut self.whisper_ctx {
                 let mut params = FullParams::new(SamplingStrategy::Greedy);
                 params.set_translate(false);
                 params.set_language(Some("en"));
                 let res = ctx.full(params, &samples);
                 if let Ok(Some(text)) = res.get_full_text() {
                     if !text.is_empty() {
                         return Some(TranscriptionEvent::Partial { text, confidence: None, words: None });
                     }
                 }
             }
             None
         }

         fn whisper_finalize(&mut self) -> Option<TranscriptionEvent> {
             // Whisper full() handles finalize; emit final if partial was sent
             None // Adjust based on incremental mode
         }
     }
     ```
   - For real-time: Use incremental feeding if supported; otherwise, buffer as current.

3. **Model Management**:
   - Add download script `scripts/download_whisper.sh`: Use curl to fetch ggml models from HuggingFace.
   - Update config: Default `model_path = "models/ggml-base.en.bin"`.

### Tests
- Add integration test: `tests/whisper_integration.rs` – Load model, process sample WAV, assert text output.
- End-to-end: Update `cargo test -p coldvox-app test_end_to_end_wav` to use Whisper backend.
- GPU test: Conditional on CUDA env var.

### CI Updates
- Add step: Download Whisper model in CI; test with `--features whisper`.
- Use matrix for CPU/GPU if runner supports.

### Documentation
- `crates/coldvox-stt/README.md`: Whisper setup, model download.
- Add GPU requirements section.

## Phase 3: Parakeet/Canary Integration (3-4 days)
### Objectives
- Add NVIDIA backend via ONNX.
- GPU detection.

### Code Changes
1. **Dependencies**:
   ```
   ort = "1.18"  # ONNX Runtime
   nvml-wrapper = "0.9"  # GPU detection
   ```

2. **Implement Parakeet**:
   - Export NeMo model to ONNX (manual step: Python script in docs).
   - In `backend.rs`:
     ```rust
     use ort::{Environment, SessionBuilder, GraphOptimizationLevel};

     impl BackendStt {
         fn parakeet_process(&mut self, samples: &[i16]) -> Option<TranscriptionEvent> {
             if let Some(session) = &mut self.parakeet_session {
                 // Convert samples to tensor
                 let input_tensor = /* tensor from samples */;
                 let outputs = session.run(vec![input_tensor]).unwrap();
                 let text = /* decode output */;
                 Some(TranscriptionEvent::Partial { text, ..Default::default() })
             } else {
                 None
             }
         }
     }

     // In new():
     if backend == SttBackend::Parakeet {
         let env = Environment::builder().build().unwrap();
         let session = SessionBuilder::new(&env)
             .with_optimization_level(GraphOptimizationLevel::Basic)
             .commit_from_file("models/parakeet.onnx")?;
         parakeet_session = Some(session);
     }
     ```
   - GPU: Use `nvml-wrapper` to detect NVIDIA and set ORT execution provider to CUDA.

3. **Riva Alternative**: If ONNX complex, add gRPC client option via `tonic`.

### Tests
- Mock ONNX session for unit tests.
- Integration: Process noisy WAV, compare WER.

### CI Updates
- NVIDIA runner: Test Parakeet with Docker NVIDIA image.
- Feature: `--features parakeet`.

### Documentation
- Guide: Model export from NeMo, ONNX setup.
- Licensing note: CC-BY-NC for models.

## Phase 4: Noise Reduction (2 days)
### Objectives
- Integrate RNNoise primary, NeMo secondary.

### Code Changes
1. **Dependencies** (`crates/coldvox-audio/Cargo.toml`):
   ```
   nnnoiseless = "0.1"  # RNNoise
   ort = "1.18"  # For NeMo ONNX
   ```

2. **New Module** (`crates/coldvox-audio/src/denoiser.rs`):
   ```rust
   use nnnoiseless::Denoise;
   use crate::AudioFrame;

   pub enum DenoiserType { RNNoise, NeMo, None }

   pub struct AudioDenoiser {
       rnr: Option<Denoise>,
       nemo_session: Option<ort::Session>,
       typ: DenoiserType,
   }

   impl AudioDenoiser {
       pub fn new(typ: DenoiserType, sample_rate: u32) -> Self {
           let mut rnr = None;
           if typ == DenoiserType::RNNoise {
               rnr = Some(Denoise::new(sample_rate));
           }
           // Nemo setup similar to Parakeet
           Self { rnr, nemo_session: None, typ }
       }

       pub fn process(&mut self, frame: &mut AudioFrame) {
           match self.typ {
               DenoiserType::RNNoise => {
                   if let Some(r) = &mut self.rnr {
                       let denoised_f32: Vec<f32> = r.process(&frame.data.iter().map(|&s| s as f32 / 32768.0).collect::<Vec<_>>());
                       frame.data = denoised_f32.iter().map(|&f| (f * 32767.0) as i16).collect();
                   }
               }
               // NeMo ONNX process
               _ => {}
           }
       }
   }
   ```

3. **Pipeline Update** (`crates/coldvox-audio/src/chunker.rs` or similar):
   - Create `AudioDenoiser` from config.
   - Call `denoiser.process(&mut frame)` after resampling.

4. **Config**: Add to `config/default.toml`: `denoise_type = "rnnoise"`.

### Tests
- Unit: Denoise sample noisy frame, assert SNR improvement.
- Integration: End-to-end with STT, measure WER on noisy inputs.

### CI Updates
- Add noise benchmark script: Process DNS samples, compute WER/latency.

### Documentation
- README: Denoising setup, benchmarks.

## Phase 5: Testing, CI, and Polish (1-2 days)
### Comprehensive Tests
- Unit: Backend methods, denoiser frames.
- Integration: Full pipeline with WAV inputs (noisy/clean).
- Performance: Benchmark latency/WER on CPU/GPU.
- E2E: `cargo test -p coldvox-app --features whisper parakeet`.

### CI Enhancements
- Matrix: OS (Linux), features (whisper, parakeet, rnnoise).
- GPU: Use self-hosted NVIDIA runner; download models.
- Benchmarks: Run with criterion or custom script; threshold failures.

### Documentation
- Update all READMEs, add migration guide.
- User docs: GPU setup, model downloads.
- Deprecate old plugin docs.

## Risks and Rollback
- Risk: Backend incompat – Rollback to mock/Vosk branch.
- Total Effort: 9-13 days.
- Success Metrics: <1s latency for 10s speech, >15% WER improvement with denoising.

Approve to proceed to diagrams and Code mode.