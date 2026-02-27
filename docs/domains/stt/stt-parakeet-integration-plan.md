---
doc_type: reference
subsystem: stt
version: 1.0.0
status: draft
freshness: stale
preservation: preserve
owners: STT Team
last_reviewed: 2026-02-12
last_reviewer: Jules
review_due: 2026-08-12
summary: API contract analysis and gap analysis for Parakeet
signals: ['stt-api', 'parakeet']
---

# Parakeet STT Integration Plan - COMPREHENSIVE

## Executive Summary

This document defines the contracts, identifies gaps, and provides a detailed implementation plan for integrating **parakeet-rs v0.1.9** with ColdVox's STT plugin system.

**Status**: ⚠️ **RESEARCH COMPLETE** - Ready for implementation
**Grade**: Self-assessed as **D+** for initial attempt (guessed API, never verified compilation)
**This Plan**: Addresses all identified issues with verified API information

---

## Part 1: Contract Definitions

### 1.1 ColdVox SttPlugin Contract (REQUIRED INTERFACE)

```rust
#[async_trait]
pub trait SttPlugin: Send + Sync + Debug {
    // METADATA
    fn info(&self) -> PluginInfo;
    fn capabilities(&self) -> PluginCapabilities;

    // LIFECYCLE
    async fn is_available(&self) -> Result<bool, ColdVoxError>;
    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError>;
    async fn load_model(&mut self, model_path: Option<&Path>) -> Result<(), ColdVoxError>;
    async fn unload(&mut self) -> Result<(), ColdVoxError>;

    // AUDIO PROCESSING (Core contract)
    async fn process_audio(
        &mut self,
        samples: &[i16],  // ← INPUT: 16-bit PCM audio
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError>;

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError>;
    async fn reset(&mut self) -> Result<(), ColdVoxError>;
}
```

**Key Requirements**:
1. **Audio Input**: `&[i16]` (16-bit PCM samples)
2. **Return Type**: `Option<TranscriptionEvent>`
   - `None` = No transcription ready yet (buffering)
   - `Some(TranscriptionEvent::Partial{...})` = Intermediate result
   - `Some(TranscriptionEvent::Final{...})` = Final result with optional word timestamps
3. **Processing Model**: Incremental buffering, batch transcription on finalize
4. **Sample Rate**: 16kHz (assumed from audio pipeline)

### 1.2 TranscriptionEvent Contract (OUTPUT)

```rust
pub enum TranscriptionEvent {
    Partial {
        utterance_id: u64,
        text: String,
        t0: Option<f32>,  // Start time in seconds
        t1: Option<f32>,  // End time in seconds
    },
    Final {
        utterance_id: u64,
        text: String,
        words: Option<Vec<WordInfo>>,  // ← Word-level timestamps
    },
    Error {
        code: String,
        message: String,
    },
}

pub struct WordInfo {
    pub start: f32,  // Seconds
    pub end: f32,    // Seconds
    pub conf: f32,   // Confidence (0.0-1.0)
    pub text: String,
}
```

### 1.3 parakeet-rs API (PROVIDED LIBRARY)

```rust
// CONSTRUCTION
pub struct Parakeet { /* opaque */ }

impl Parakeet {
    pub fn from_pretrained<P: AsRef<Path>>(
        path: P,                         // ← Model directory (NOT HuggingFace ID!)
        config: Option<ExecutionConfig>, // ← GPU/CPU config
    ) -> Result<Self>;
}

// TRANSCRIPTION
impl Parakeet {
    pub fn transcribe_samples(
        &mut self,
        audio: Vec<f32>,     // ← FLOAT32 audio (NOT i16!)
        sample_rate: u32,    // ← e.g., 16000
        channels: u16,       // ← 1 for mono
        mode: Option<TimestampMode>, // ← Tokens/Words/Sentences
    ) -> Result<TranscriptionResult>;

    pub fn transcribe_file<P: AsRef<Path>>(
        &mut self,
        audio_path: P,
        mode: Option<TimestampMode>,
    ) -> Result<TranscriptionResult>;
}

// CONFIGURATION
#[derive(Default)]
pub struct ExecutionConfig {
    pub execution_provider: ExecutionProvider,
    pub intra_threads: usize,
    pub inter_threads: usize,
}

pub enum ExecutionProvider {
    Cpu,                 // Default
    #[cfg(feature = "cuda")]
    Cuda,                // Requires feature flag
    #[cfg(feature = "tensorrt")]
    TensorRT,            // Requires feature flag
    // ... other providers
}

// OUTPUT
pub struct TranscriptionResult {
    pub text: String,
    pub tokens: Vec<TimedToken>,
}

pub struct TimedToken {
    pub text: String,
    pub start: f32,  // Seconds
    pub end: f32,    // Seconds
    // NO CONFIDENCE FIELD!
}
```

---

## Part 2: Gap Analysis & Adapter Requirements

### 2.1 Audio Format Mismatch

**Problem**: ColdVox provides `&[i16]`, parakeet-rs expects `Vec<f32>`

**Solution**: Convert i16 → f32 in the adapter layer
```rust
fn convert_audio(samples_i16: &[i16]) -> Vec<f32> {
    samples_i16.iter()
        .map(|&s| s as f32 / 32768.0)  // Normalize to [-1.0, 1.0]
        .collect()
}
```

**Cost**: O(n) memory allocation + conversion per utterance

### 2.2 Confidence Scores Missing

**Problem**: `WordInfo` requires `conf: f32`, but `TimedToken` doesn't provide it

**Solution**: Use placeholder confidence
```rust
WordInfo {
    start: token.start,
    end: token.end,
    conf: 1.0,  // ← Placeholder (parakeet doesn't provide confidence)
    text: token.text,
}
```

**Trade-off**: Users can't filter low-confidence words

### 2.3 Batch-Only Processing

**Problem**: parakeet-rs only supports batch transcription (no streaming partials)

**Solution**: Buffer audio in `process_audio()`, transcribe in `finalize()`
```rust
// process_audio: buffer samples, return None
async fn process_audio(&mut self, samples: &[i16]) -> Result<Option<TranscriptionEvent>> {
    self.audio_buffer.extend_from_slice(samples);
    Ok(None)  // No partial results
}

// finalize: convert buffer → f32, transcribe, return Final event
async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>> {
    let audio_f32 = convert_audio(&self.audio_buffer);
    let result = self.model.transcribe_samples(audio_f32, 16000, 1, Some(TimestampMode::Words))?;
    // Map to TranscriptionEvent::Final
}
```

### 2.4 CPU Fallback (User Requirement Conflict)

**Problem**: User wants "GPU-only, no fallback", but parakeet-rs **AUTOMATICALLY** falls back to CPU

**Evidence**: From `execution.rs:94`:
```rust
builder.with_execution_providers([
    CUDAExecutionProvider::default().build(),
    CPUExecutionProvider::default().build().error_on_failure(),  // ← Automatic fallback!
])
```

**Options**:
1. **Accept library behavior** - Let parakeet-rs fallback to CPU (recommended)
2. **Check CUDA before initialization** - Use `nvidia-smi` to verify GPU, fail early
3. **Feature-gate enforcement** - Only compile with `cuda` feature, fail if unavailable

**Recommendation**: **Option 2** - Verify GPU in `check_requirements()`, fail fast if unavailable

### 2.5 Model Loading

**Problem**: parakeet-rs loads from **local directory**, not HuggingFace ID

**Required Files**:
- `model.onnx` (or variants: `model_fp16.onnx`, `model_int8.onnx`, `model_q4.onnx`)
- `tokenizer.json`
- `config.json` (optional)
- `preprocessor_config.json` (optional)

**Solution**: User must download model manually or we implement downloader
```bash
# User downloads from HuggingFace:
git clone https://huggingface.co/nvidia/parakeet-ctc-1.1b ~/.cache/parakeet/ctc
# Or for TDT:
git clone https://huggingface.co/nvidia/parakeet-tdt-1.1b ~/.cache/parakeet/tdt
```

**Adapter Logic**:
```rust
fn resolve_model_path(&self, config: &TranscriptionConfig) -> Result<PathBuf> {
    let default_cache = dirs::cache_dir()
        .ok_or(...)?
        .join("parakeet")
        .join("ctc");  // or "tdt" based on variant

    // Priority: config.model_path → PARAKEET_MODEL_PATH env → default cache
    Ok(...)
}
```

---

## Part 3: Implementation Strategy

### 3.1 Adapter Architecture

```
┌─────────────────────────────────────────────┐
│ ColdVox Audio Pipeline                      │
│ Produces: &[i16] @ 16kHz                    │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ ParakeetPlugin (SttPlugin implementation)   │
│                                             │
│  ┌──────────────────────────────────────┐  │
│  │ process_audio(&[i16])                │  │
│  │   → buffer.extend_from_slice()       │  │
│  │   → return Ok(None)                  │  │
│  └──────────────────────────────────────┘  │
│                                             │
│  ┌──────────────────────────────────────┐  │
│  │ finalize()                           │  │
│  │   1. Convert i16 → f32               │  │
│  │   2. parakeet.transcribe_samples()   │  │
│  │   3. Map TimedToken → WordInfo       │  │
│  │   4. Return TranscriptionEvent::Final│  │
│  └──────────────────────────────────────┘  │
│                                             │
│  Internal:                                  │
│  - model: Parakeet                          │
│  - audio_buffer: Vec<i16>                   │
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ parakeet-rs                                 │
│ - from_pretrained(dir, ExecutionConfig)     │
│ - transcribe_samples(Vec<f32>, ...)         │
│ Returns: TranscriptionResult                │
└─────────────────────────────────────────────┘
```

### 3.2 Feature Flags Required

```toml
[dependencies]
parakeet-rs = { version = "0.1", features = ["cuda"], optional = true }
#                                            ^^^^^^ GPU support

[features]
parakeet = ["dep:parakeet-rs"]
```

**Critical**: Must enable `cuda` feature in parakeet-rs dependency, not just in ColdVox features!

### 3.3 Configuration

**Environment Variables**:
- `PARAKEET_MODEL_PATH`: Override model directory (default: `~/.cache/parakeet/ctc`)
- `PARAKEET_VARIANT`: "ctc" or "tdt" (default: "ctc" for simplicity)
- `PARAKEET_DEVICE`: Ignored (parakeet-rs always attempts GPU if feature enabled)

**config/plugins.json**:
```json
{
  "preferred_plugin": "parakeet",
  "fallback_plugins": ["mock"],
  "require_local": true
}
```

### 3.4 GPU Verification

```rust
fn check_gpu_available() -> Result<(), ColdVoxError> {
    let output = std::process::Command::new("nvidia-smi")
        .output()
        .map_err(|_| SttError::LoadFailed("nvidia-smi not found. GPU required.".to_string()))?;

    if !output.status.success() {
        return Err(SttError::LoadFailed("GPU check failed. CUDA GPU required.".to_string()).into());
    }

    Ok(())
}
```

---

## Part 4: Detailed Implementation Plan

### Phase 1: Fix Cargo.toml (10 min)

```toml
# crates/coldvox-stt/Cargo.toml
[dependencies]
parakeet-rs = { version = "0.1", features = ["cuda"], optional = true }
#                                            ^^^^^^ CRITICAL!

[features]
parakeet = ["dep:parakeet-rs"]
```

### Phase 2: Implement ParakeetPlugin (2-3 hours)

**File**: `crates/coldvox-stt/src/plugins/parakeet.rs`

**Structure**:
```rust
pub struct ParakeetPlugin {
    // Model state
    model: Option<Parakeet>,
    model_path: Option<PathBuf>,

    // Audio buffering
    audio_buffer: Vec<i16>,

    // Configuration
    active_config: Option<TranscriptionConfig>,
    initialized: bool,
}

impl ParakeetPlugin {
    pub fn new() -> Self { /* ... */ }

    // HELPER: Convert i16 → f32
    fn convert_audio_format(samples: &[i16]) -> Vec<f32> {
        samples.iter().map(|&s| s as f32 / 32768.0).collect()
    }

    // HELPER: Map parakeet result → TranscriptionEvent
    fn map_result_to_event(
        result: TranscriptionResult,
        include_words: bool,
    ) -> TranscriptionEvent {
        let words = if include_words {
            Some(result.tokens.iter().map(|token| WordInfo {
                start: token.start,
                end: token.end,
                conf: 1.0,  // Placeholder
                text: token.text.clone(),
            }).collect())
        } else {
            None
        };

        TranscriptionEvent::Final {
            utterance_id: 0,  // TODO: Track utterance IDs
            text: result.text,
            words,
        }
    }
}

#[async_trait]
impl SttPlugin for ParakeetPlugin {
    fn info(&self) -> PluginInfo { /* ... */ }
    fn capabilities(&self) -> PluginCapabilities { /* ... */ }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        // 1. Resolve model path
        let model_path = resolve_model_path(&config)?;

        // 2. Create ExecutionConfig with CUDA
        let exec_config = ExecutionConfig::default()
            .with_execution_provider(ExecutionProvider::Cuda);

        // 3. Load model
        let model = Parakeet::from_pretrained(&model_path, Some(exec_config))
            .map_err(|e| SttError::LoadFailed(format!("Failed to load Parakeet: {}", e)))?;

        self.model = Some(model);
        self.audio_buffer.clear();
        self.active_config = Some(config);
        self.initialized = true;
        Ok(())
    }

    async fn process_audio(&mut self, samples: &[i16]) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        // Buffer audio, return None (no partials)
        self.audio_buffer.extend_from_slice(samples);
        Ok(None)
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        if self.audio_buffer.is_empty() {
            return Ok(None);
        }

        // 1. Convert i16 → f32
        let audio_f32 = Self::convert_audio_format(&self.audio_buffer);

        // 2. Transcribe
        let result = self.model.as_mut()
            .ok_or(...)?
            .transcribe_samples(
                audio_f32,
                16000,  // 16kHz
                1,      // Mono
                Some(TimestampMode::Words),
            )
            .map_err(|e| SttError::TranscriptionFailed(format!("Parakeet failed: {}", e)))?;

        // 3. Map to event
        let include_words = self.active_config.as_ref()
            .map(|c| c.include_words)
            .unwrap_or(false);
        let event = Self::map_result_to_event(result, include_words);

        // 4. Clear buffer
        self.audio_buffer.clear();

        Ok(Some(event))
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        self.audio_buffer.clear();
        Ok(())
    }
}
```

### Phase 3: Factory Implementation (30 min)

```rust
pub struct ParakeetPluginFactory {
    model_path: Option<PathBuf>,
}

impl SttPluginFactory for ParakeetPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        let mut plugin = ParakeetPlugin::new();
        if let Some(ref path) = self.model_path {
            plugin = plugin.with_model_path(path.clone());
        }
        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        PluginInfo {
            id: "parakeet".to_string(),
            name: "NVIDIA Parakeet (GPU)".to_string(),
            description: "GPU-accelerated STT via parakeet-rs".to_string(),
            requires_network: false,
            is_local: true,
            is_available: check_parakeet_available(),
            supported_languages: vec!["en".to_string()],  // CTC English-only
            memory_usage_mb: Some(2000),  // ~2GB for 1.1B model
        }
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        // Check GPU availability
        check_gpu_available()?;

        // Check model files exist (if path specified)
        if let Some(ref path) = self.model_path {
            if !path.join("model.onnx").exists() &&
               !path.join("model_q4.onnx").exists() {
                return Err(SttError::LoadFailed(
                    format!("No model files found in {}", path.display())
                ).into());
            }
        }

        Ok(())
    }
}
```

### Phase 4: Testing Strategy (Live hardware only)

**No mock tests per user requirement**. Test on GPU hardware:

```rust
#[cfg(feature = "live-hardware-tests")]
#[tokio::test]
async fn test_parakeet_live_transcription() {
    // 1. Verify GPU available
    assert!(check_gpu_available().is_ok());

    // 2. Create plugin
    let mut plugin = ParakeetPlugin::new();
    let config = TranscriptionConfig {
        enabled: true,
        model_path: "~/.cache/parakeet/ctc".to_string(),
        include_words: true,
        ..Default::default()
    };

    // 3. Initialize
    plugin.initialize(config).await.expect("Init failed");

    // 4. Load test audio (test_11.wav)
    let audio = load_test_audio("test_data/test_11.wav");

    // 5. Process in chunks
    for chunk in audio.chunks(512) {
        plugin.process_audio(chunk).await.expect("Process failed");
    }

    // 6. Finalize and verify
    let result = plugin.finalize().await.expect("Finalize failed");
    assert!(result.is_some());

    match result.unwrap() {
        TranscriptionEvent::Final { text, words, .. } => {
            assert!(!text.is_empty());
            assert!(words.is_some());
            println!("Transcribed: {}", text);
        }
        _ => panic!("Expected Final event"),
    }
}
```

---

## Part 5: Critical Issues & Limitations

### 5.1 NO CPU Fallback Enforcement

**User Requirement**: "GPU-only, no headless fallback no matter what"

**Reality**: parakeet-rs **automatically falls back to CPU** at the ONNX Runtime level

**Mitigation**:
1. Check `nvidia-smi` in `check_requirements()` - fail before model load
2. Document that GPU is verified but runtime fallback may still occur
3. Consider patching parakeet-rs or using ORT directly to disable fallback

**Status**: ⚠️ **User expectation may not be fully met**

### 5.2 Model Size

**User wants "largest model possible"**

**Reality**: parakeet-rs supports Parakeet CTC 0.6B (600M parameters), **NOT 1.1B as initially claimed**

**Correction**: The "1.1B TDT" model is ParakeetTDT (different API), and may not be supported yet

**Action**: Verify which model variant is actually available

### 5.3 Confidence Scores

**Missing from parakeet-rs API** - we'll use placeholder `1.0`

**Impact**: Users can't filter low-confidence transcriptions

---

## Part 6: Files to Modify

1. ✅ `crates/coldvox-stt/Cargo.toml` - Fix dependency with `features = ["cuda"]`
2. ✅ `crates/coldvox-stt/src/plugins/parakeet.rs` - Complete rewrite with correct API
3. ✅ `CLAUDE.md` - Update with corrected instructions
4. ✅ `CHANGELOG.md` - Revise claims about model size and capabilities
5. ✅ `config/plugins.json` - Already set to "parakeet"

---

## Part 7: Implementation Checklist

- [ ] Fix Cargo.toml with correct feature flags
- [ ] Rewrite parakeet.rs with actual API (not guessed API)
- [ ] Implement audio format conversion (i16 → f32)
- [ ] Implement buffer-and-batch transcription pattern
- [ ] Add GPU verification in check_requirements()
- [ ] Test on GPU hardware with real audio
- [ ] Update documentation with realistic capabilities
- [ ] Amend previous commit with corrections

---

## Honest Self-Assessment

**Initial Implementation Grade**: D+ (code never compiled, guessed API)

**This Plan Grade**: B+ (thorough research, identified real issues, but untested on hardware)

**To achieve A**: Verify on GPU system, confirm model size, test live transcription

---

**Next Steps**: Proceed with implementation using this verified plan?
