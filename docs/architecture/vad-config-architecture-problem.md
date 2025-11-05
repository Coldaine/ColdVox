# VAD Configuration Architecture Problem

**Date**: 2025-11-05
**Issue**: VAD configuration is hardcoded in runtime, not configurable
**Severity**: High - Causes test/production drift, duplication, poor user experience

---

## The Problem

The VAD configuration is **hardcoded in two places** in `runtime.rs`:

### Location 1: `runtime::start()` (lines 431-439)
```rust
let vad_cfg = UnifiedVadConfig {
    mode: VadMode::Silero,
    frame_size_samples: FRAME_SIZE_SAMPLES,
    sample_rate_hz: SAMPLE_RATE_HZ,
    silero: SileroConfig {
        threshold: 0.1,
        min_speech_duration_ms: 100,
        min_silence_duration_ms: 500,  // ← Critical tuning
        window_size_samples: FRAME_SIZE_SAMPLES,
    },
};
```

### Location 2: `AppHandle::set_activation_mode()` (lines 257-266)
```rust
let vad_cfg = UnifiedVadConfig {
    mode: VadMode::Silero,
    frame_size_samples: FRAME_SIZE_SAMPLES,
    sample_rate_hz: SAMPLE_RATE_HZ,
    silero: SileroConfig {
        threshold: 0.1,
        min_speech_duration_ms: 100,
        min_silence_duration_ms: 500,  // ← Duplicated!
        window_size_samples: FRAME_SIZE_SAMPLES,
    },
};
```

**Same config, duplicated verbatim.** Includes 40+ lines of comments explaining why 500ms is important.

---

## What's Missing

### Not in `AppRuntimeOptions`
```rust
pub struct AppRuntimeOptions {
    pub device: Option<String>,
    pub resampler_quality: ResamplerQuality,  // ← Configurable
    pub activation_mode: ActivationMode,
    pub stt_selection: Option<...>,
    pub injection: Option<InjectionOptions>,
    // ❌ NO VAD CONFIG
}
```

### Not in `Settings`
```rust
pub struct Settings {
    pub device: Option<String>,
    pub resampler_quality: String,  // ← Configurable
    pub enable_device_monitor: bool,
    pub activation_mode: String,
    pub audio: AudioSettings,  // Only has capture_buffer_samples
    pub injection: InjectionSettings,
    pub stt: SttSettings,
    // ❌ NO VAD CONFIG
}
```

### Not in Config Files
```toml
# config/default.toml (presumably)
[audio]
capture_buffer_samples = 65536

# ❌ NO [vad] SECTION
```

---

## Consequences of This Architecture

### 1. Code Duplication
- Same VAD config appears **twice** in runtime.rs
- 40+ lines of documentation duplicated
- Easy to update one and forget the other

### 2. Test/Production Drift ⚠️
- Tests have no way to use "production VAD config"
- Tests default to `Default::default()` which is **completely different**:
  - Production: threshold=0.1, silence=500ms
  - Test default: threshold=0.3, silence=100ms
- **This is how we got the current mismatch**

### 3. No User Configurability
- Users can't tune VAD without editing code
- Can't experiment with different thresholds
- Can't adjust for different use cases (quiet room vs noisy)

### 4. Poor Testability
- Can't easily test different VAD configs
- Can't benchmark config changes
- Can't A/B test threshold values

### 5. Fragile Documentation
- Critical knowledge (why 500ms?) is in code comments
- Not in user-facing docs
- Not in config schema

---

## Comparison to Other Configurable Components

### Resampler Quality (✅ Done Right)
```rust
// Configurable via AppRuntimeOptions
pub struct AppRuntimeOptions {
    pub resampler_quality: ResamplerQuality,  // ✅
}

// Configurable via Settings
pub struct Settings {
    pub resampler_quality: String,  // ✅
}

// Used in runtime
let chunker_cfg = ChunkerConfig {
    resampler_quality: opts.resampler_quality,  // ✅
};
```

### Injection Config (✅ Done Right)
```rust
// Configurable via AppRuntimeOptions
pub struct AppRuntimeOptions {
    pub injection: Option<InjectionOptions>,  // ✅
}

// Detailed InjectionOptions structure
pub struct InjectionOptions {
    pub enable: bool,
    pub allow_kdotool: bool,
    pub allow_enigo: bool,
    pub max_total_latency_ms: Option<u64>,
    // ... lots of options
}
```

### VAD Config (❌ Wrong)
```rust
// NOT in AppRuntimeOptions
// NOT in Settings
// Hardcoded in runtime.rs (2 places)
```

---

## Recommended Architecture

### Option A: Add to Settings (Preferred)

```rust
// crates/app/src/lib.rs
#[derive(Debug, Deserialize)]
pub struct VadSettings {
    pub mode: String,  // "silero" (future: "webrtc", etc.)
    pub threshold: f32,
    pub min_speech_duration_ms: u64,
    pub min_silence_duration_ms: u64,
}

impl Default for VadSettings {
    fn default() -> Self {
        Self {
            mode: "silero".to_string(),
            threshold: 0.1,
            min_speech_duration_ms: 100,
            min_silence_duration_ms: 500,  // Production default
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub device: Option<String>,
    pub resampler_quality: String,
    pub activation_mode: String,
    pub audio: AudioSettings,
    pub vad: VadSettings,  // ← NEW
    pub injection: InjectionSettings,
    pub stt: SttSettings,
}
```

**Config file**:
```toml
# config/default.toml
[vad]
mode = "silero"
threshold = 0.1
min_speech_duration_ms = 100
min_silence_duration_ms = 500  # Tuned for quality (see docs)
```

**Runtime usage**:
```rust
// runtime.rs
let vad_cfg = UnifiedVadConfig::from_settings(&settings.vad)?;
// OR
let vad_cfg = settings.vad.to_unified_vad_config()?;
```

**Benefits**:
- ✅ Single source of truth
- ✅ Configurable by users
- ✅ Tests can use production config
- ✅ No duplication
- ✅ Easy to A/B test
- ✅ Self-documenting (config schema)

### Option B: Extract to Factory Function (Minimal Fix)

```rust
// crates/coldvox-vad/src/config.rs or crates/app/src/vad_config.rs
impl UnifiedVadConfig {
    /// Production VAD configuration tuned for dictation quality.
    ///
    /// Key parameters:
    /// - threshold: 0.1 (sensitive detection)
    /// - min_silence: 500ms (stitches utterances, prevents fragmentation)
    ///
    /// See issue #61 for rationale on 500ms silence duration.
    pub fn production_default() -> Self {
        Self {
            mode: VadMode::Silero,
            frame_size_samples: FRAME_SIZE_SAMPLES,
            sample_rate_hz: SAMPLE_RATE_HZ,
            silero: SileroConfig {
                threshold: 0.1,
                min_speech_duration_ms: 100,
                min_silence_duration_ms: 500,
                window_size_samples: FRAME_SIZE_SAMPLES,
            },
        }
    }
}
```

**Usage**:
```rust
// runtime.rs (both locations)
let vad_cfg = UnifiedVadConfig::production_default();

// tests
let vad_cfg = UnifiedVadConfig::production_default();
```

**Benefits**:
- ✅ Single source of truth (no duplication)
- ✅ Tests can use production config
- ✅ Documentation in one place
- ❌ Still not user-configurable
- ❌ Still hardcoded (but at least not duplicated)

---

## Impact Assessment

### Current State
| Aspect | Status | Impact |
|--------|--------|--------|
| Duplication | 2 copies in runtime.rs | High - Easy to update one, forget other |
| Test drift | Tests use different config | **Critical** - False confidence |
| User config | Not possible | Medium - Power users can't tune |
| Testability | Hard to test variants | Medium - Can't benchmark configs |
| Documentation | In code comments | Low - Knowledge in wrong place |

### After Option A (Settings)
| Aspect | Status | Impact |
|--------|--------|--------|
| Duplication | None | ✅ Fixed |
| Test drift | Can use production config | ✅ Fixed |
| User config | Fully configurable | ✅ Fixed |
| Testability | Easy | ✅ Fixed |
| Documentation | In config schema + docs | ✅ Fixed |

### After Option B (Factory)
| Aspect | Status | Impact |
|--------|--------|--------|
| Duplication | None | ✅ Fixed |
| Test drift | Can use production config | ✅ Fixed |
| User config | Still not possible | ⚠️ Not fixed |
| Testability | Easier | ✅ Improved |
| Documentation | In one place | ✅ Fixed |

---

## Recommendation

**Implement Option A** (Add to Settings) because:

1. **Consistency**: Matches how resampler_quality, injection, STT are configured
2. **User value**: Power users can tune VAD for their environment
3. **Testability**: Easy to test different configs, A/B test improvements
4. **No duplication**: Single source of truth
5. **Self-documenting**: Config schema documents available options

**Short-term**: Implement Option B as immediate fix to prevent test drift
**Long-term**: Migrate to Option A for full configurability

---

## Implementation Checklist (Option B - Immediate Fix)

- [ ] Create `UnifiedVadConfig::production_default()` factory method
- [ ] Move 40+ lines of documentation to factory method docs
- [ ] Update `runtime::start()` to use factory
- [ ] Update `AppHandle::set_activation_mode()` to use factory
- [ ] Update all tests to use factory (fixes test/prod drift)
- [ ] Add test that verifies factory config matches expected values
- [ ] Remove duplicated documentation from runtime.rs

**Estimated time**: 1-2 hours

## Implementation Checklist (Option A - Long-term)

- [ ] Add `VadSettings` struct to `Settings`
- [ ] Add `[vad]` section to `config/default.toml`
- [ ] Add conversion `VadSettings -> UnifiedVadConfig`
- [ ] Add validation for VAD settings
- [ ] Update `AppRuntimeOptions` to accept optional `VadSettings` override
- [ ] Update `runtime::start()` to use settings
- [ ] Update `AppHandle::set_activation_mode()` to use settings
- [ ] Update all tests to use settings
- [ ] Document VAD settings in user docs
- [ ] Add migration guide for users with custom configs

**Estimated time**: 4-6 hours

---

## Related Issues

- Test/production VAD config mismatch (discovered in pipeline comparison)
- Issue #61 mentioned in comments (rationale for 500ms silence)
- General pattern of configuration management across ColdVox

## See Also

- `docs/testing/PIPELINE_COMPARISON.md` - Discovery of test/prod config mismatch
- `crates/app/src/runtime.rs:431-439` - Production VAD config location 1
- `crates/app/src/runtime.rs:257-266` - Production VAD config location 2 (duplicate)
