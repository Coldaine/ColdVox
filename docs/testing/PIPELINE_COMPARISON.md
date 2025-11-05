# Pipeline Wiring Comparison: Test vs Production

**Date**: 2025-11-05 (Initial analysis)
**Updated**: 2025-11-05 (Resolution)
**Purpose**: Verify that `end_to_end_wav.rs` test pipeline matches production `runtime.rs` pipeline

---

## 🎯 RESOLUTION (2025-11-05)

**Status**: ✅ **CRITICAL ISSUE FIXED**

The most critical mismatch (VAD configuration drift) has been resolved:

### What Was Fixed
1. **Created `UnifiedVadConfig::production_default()` factory method**
   - Single source of truth for production VAD configuration
   - Comprehensive documentation explaining tuned values

2. **Updated production code** (`runtime.rs`)
   - Both locations now use `production_default()`
   - Removed 40+ lines of duplicated documentation

3. **Updated E2E test** (`end_to_end_wav.rs`)
   - Now uses `production_default()` instead of `Default::default()`
   - Test VAD config **now matches production exactly**

4. **Added regression tests**
   - Verify production config values don't drift
   - Ensure production differs from conservative defaults

### Impact
- ✅ Test and production VAD behavior now **identical**
- ✅ No more 5x difference in silence duration (500ms production vs 100ms test)
- ✅ No more 3x difference in threshold (0.1 production vs 0.3 test)
- ✅ Future config changes automatically propagate to tests

### Remaining Minor Differences
See comparison below - remaining differences are acceptable:
- Metrics collection (test doesn't verify metrics - could be improved)
- Arc/Mutex wrapping (test doesn't need thread-safety)

---

## Component-by-Component Comparison

### 1. Audio Ring Buffer

#### Production (`runtime.rs:299-305`)
```rust
let audio_config = AudioConfig {
    silence_threshold: 100,
    capture_buffer_samples: opts.capture_buffer_samples, // 65_536 default
};
let ring_buffer = AudioRingBuffer::new(audio_config.capture_buffer_samples);
let (audio_producer, audio_consumer) = ring_buffer.split();
let audio_producer = Arc::new(Mutex::new(audio_producer));
```

#### Test (`end_to_end_wav.rs:146-148`)
```rust
let audio_config = AudioConfig::default();
let ring_buffer = AudioRingBuffer::new(audio_config.capture_buffer_samples);
let (audio_producer, audio_consumer) = ring_buffer.split();
```

**Differences**:
- ✅ Same buffer creation pattern
- ⚠️ Test uses `AudioConfig::default()` - need to check what that is
- ⚠️ Production wraps producer in `Arc<Mutex<>>`, test doesn't

**Verdict**: **Minor difference** - Test might have different buffer size

---

### 2. Audio Chunker Setup

#### Production (`runtime.rs:372-402`)
```rust
let frame_reader = FrameReader::new(
    audio_consumer,
    device_cfg.sample_rate,
    device_cfg.channels,
    audio_config.capture_buffer_samples,
    Some(metrics.clone()),  // ← Has metrics
);

let chunker_cfg = ChunkerConfig {
    frame_size_samples: FRAME_SIZE_SAMPLES,
    sample_rate_hz: SAMPLE_RATE_HZ,
    resampler_quality: opts.resampler_quality,  // ← Configurable
};

let (audio_tx, _) = broadcast::channel::<SharedAudioFrame>(200);

let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg)
    .with_metrics(metrics.clone())  // ← Has metrics
    .with_device_config(device_config_rx_for_chunker);  // ← Device config updates

let chunker_handle = chunker.spawn();
```

#### Test (`end_to_end_wav.rs:154-179`)
```rust
let (audio_tx, _) = broadcast::channel::<AudioFrame>(200);

// Emulate device config broadcast like live capture
let (cfg_tx, cfg_rx) = broadcast::channel::<DeviceConfig>(8);
let _ = cfg_tx.send(DeviceConfig {
    sample_rate: wav_loader.sample_rate(),
    channels: wav_loader.channels(),
});

let frame_reader = coldvox_audio::frame_reader::FrameReader::new(
    audio_consumer,
    wav_loader.sample_rate(),
    wav_loader.channels(),
    audio_config.capture_buffer_samples,
    None,  // ← NO METRICS
);

let chunker_cfg = ChunkerConfig {
    frame_size_samples: FRAME_SIZE_SAMPLES,
    sample_rate_hz: SAMPLE_RATE_HZ,
    resampler_quality: coldvox_audio::chunker::ResamplerQuality::Balanced,  // ← Hardcoded
};

let chunker = AudioChunker::new(frame_reader, audio_tx.clone(), chunker_cfg)
    .with_device_config(cfg_rx);  // ← Manual device config, no metrics

let chunker_handle = chunker.spawn();
```

**Differences**:
- ❌ Test has **NO METRICS** on FrameReader (line 168: `None`)
- ❌ Test has **NO METRICS** on AudioChunker (missing `.with_metrics()`)
- ⚠️ Test hardcodes `ResamplerQuality::Balanced`, production uses `opts.resampler_quality`
- ✅ Both use same channel size (200)
- ✅ Both use same frame config

**Verdict**: **SIGNIFICANT DIFFERENCE** - Test doesn't collect chunker metrics

---

### 3. VAD Processor Setup

#### Production (`runtime.rs:431-443`)
```rust
let vad_cfg = UnifiedVadConfig {
    mode: VadMode::Silero,
    frame_size_samples: FRAME_SIZE_SAMPLES,
    sample_rate_hz: SAMPLE_RATE_HZ,
    silero: SileroConfig {
        threshold: 0.1,
        min_speech_duration_ms: 100,
        min_silence_duration_ms: 500,  // ← 500ms (important!)
        window_size_samples: FRAME_SIZE_SAMPLES,
    },
};

let vad_audio_rx = self.audio_tx.subscribe();
crate::audio::vad_processor::VadProcessor::spawn(
    vad_cfg,
    vad_audio_rx,
    self.raw_vad_tx.clone(),
    Some(self.metrics.clone()),  // ← Has metrics
)?
```

#### Test (`end_to_end_wav.rs:182-202`)
```rust
let vad_cfg = UnifiedVadConfig {
    mode: VadMode::Silero,
    frame_size_samples: FRAME_SIZE_SAMPLES,
    sample_rate_hz: SAMPLE_RATE_HZ,
    silero: Default::default(),  // ← Uses default config!
};

let (vad_event_tx, vad_event_rx) = mpsc::channel::<VadEvent>(100);
let vad_audio_rx = audio_tx.subscribe();
let vad_handle = match crate::audio::vad_processor::VadProcessor::spawn(
    vad_cfg,
    vad_audio_rx,
    vad_event_tx,
    None,  // ← NO METRICS
) {
    Ok(handle) => handle,
    Err(e) => {
        eprintln!("Failed to spawn VAD processor: {}", e);
        return;
    }
};
```

**Differences**:
- ❌ Test uses `silero: Default::default()` - **WRONG CONFIG**
- ❌ Test has **NO METRICS** on VAD (line 195: `None`)

**ACTUAL VALUES**:

| Parameter | Production | Test (Default) | Difference |
|-----------|-----------|----------------|------------|
| threshold | **0.1** | **0.3** | 3x more sensitive in test |
| min_speech_duration_ms | **100** | **250** | 2.5x longer in test |
| min_silence_duration_ms | **500** | **100** | 5x shorter in test |

**Verdict**: **CRITICAL DIFFERENCE** - Test uses COMPLETELY DIFFERENT VAD config!

**Impact**:
- Test requires **3x more energy** to detect speech (threshold 0.3 vs 0.1)
- Test requires speech to be **2.5x longer** to register (250ms vs 100ms)
- Test splits utterances **5x more frequently** (100ms silence vs 500ms)

This means the test is testing a **fundamentally different VAD behavior** than production!

---

### 4. STT Processor Setup

#### Production (`runtime.rs:459-510`)
```rust
// Set up Plugin Manager
let mut plugin_manager = SttPluginManager::new();
plugin_manager.initialize().await?;
let plugin_manager = Arc::new(RwLock::new(plugin_manager));

// Session events from VAD
let (session_tx, session_rx) = mpsc::channel::<SessionEvent>(200);

// Spawn translator from raw VAD events to session events
tokio::spawn({
    let session_tx = session_tx.clone();
    let vad_tx = vad_tx.clone();
    async move {
        let mut rx = vad_tx.subscribe();
        while let Ok(event) = rx.recv().await {
            let se = match event {
                VadEvent::SpeechStart { .. } => {
                    SessionEvent::Start(SessionSource::Vad, Instant::now())
                }
                VadEvent::SpeechEnd { .. } => {
                    SessionEvent::End(SessionSource::Vad, Instant::now())
                }
            };
            if session_tx.send(se).await.is_err() {
                break;
            }
        }
    }
});

let stt_config = TranscriptionConfig {
    enabled: true,
    model_path: /* ... model selection logic ... */,
    partial_results: true,
    max_alternatives: 1,
    include_words: false,
    buffer_size_ms: 512,
    streaming: false,
    auto_extract_model: false,
};

let (stt_tx, stt_rx) = mpsc::channel::<TranscriptionEvent>(100);
let stt_audio_rx = audio_tx.subscribe();

let processor = PluginSttProcessor::new(
    stt_audio_rx,
    session_rx,
    stt_tx,
    plugin_manager.clone(),
    stt_config,
    Settings::default(),
);

let stt_handle = tokio::spawn(async move {
    processor.run().await;
});
```

#### Test (`end_to_end_wav.rs:204-274`)
```rust
let (stt_transcription_tx, stt_transcription_rx) = mpsc::channel::<TranscriptionEvent>(100);

let stt_config = TranscriptionConfig {
    enabled: true,
    model_path: resolve_whisper_model_identifier(),  // ← Different model selection
    partial_results: true,
    max_alternatives: 1,
    include_words: false,
    buffer_size_ms: 512,
    streaming: false,
    auto_extract_model: false,
};

let stt_audio_rx = audio_tx.subscribe();

// Set up Plugin Manager
let mut plugin_manager = SttPluginManager::new();
plugin_manager.initialize().await.unwrap();
let plugin_manager = Arc::new(tokio::sync::RwLock::new(plugin_manager));

// Set up SessionEvent channel and translator
let (session_tx, session_rx) = mpsc::channel::<SessionEvent>(100);
tokio::spawn(async move {
    let mut vad_rx = vad_event_rx;
    while let Some(event) = vad_rx.recv().await {
        let session_event = match event {
            VadEvent::SpeechStart { .. } => {
                Some(SessionEvent::Start(SessionSource::Vad, Instant::now()))
            }
            VadEvent::SpeechEnd { .. } => {
                Some(SessionEvent::End(SessionSource::Vad, Instant::now()))
            }
        };
        if let Some(se) = session_event {
            if session_tx.send(se).await.is_err() {
                break;
            }
        }
    }
});

let stt_processor = PluginSttProcessor::new(
    stt_audio_rx,
    session_rx,
    stt_transcription_tx,
    plugin_manager,
    stt_config,
    Settings::default(),
);

let stt_handle = tokio::spawn(async move {
    stt_processor.run().await;
});
```

**Differences**:
- ✅ Same plugin manager setup
- ✅ Same session event translation logic
- ✅ Same STT config (except model path selection logic)
- ⚠️ Different model selection: test uses `resolve_whisper_model_identifier()`, production uses different logic
- ⚠️ Test channel is 100, production uses 200 for sessions

**Verdict**: **MOSTLY ALIGNED** - STT setup is very similar

---

### 5. Text Injection Setup

#### Production (`runtime.rs:523-545` - if enabled)
```rust
#[cfg(feature = "text-injection")]
let injection_handle = if let Some(injection_opts) = opts.injection {
    if injection_opts.enable {
        let injection_config = crate::text_injection::InjectionConfig {
            allow_kdotool: injection_opts.allow_kdotool,
            allow_enigo: injection_opts.allow_enigo,
            inject_on_unknown_focus: injection_opts.inject_on_unknown_focus,
            max_total_latency_ms: injection_opts.max_total_latency_ms.unwrap_or(800),
            per_method_timeout_ms: injection_opts.per_method_timeout_ms.unwrap_or(500),
            cooldown_initial_ms: injection_opts.cooldown_initial_ms.unwrap_or(500),
            ..Default::default()
        };

        let injection_processor = crate::text_injection::AsyncInjectionProcessor::new(
            injection_config,
            stt_rx,
            shutdown_rx,
            Some(metrics.clone()),  // ← Has metrics
        )
        .await;

        Some(tokio::spawn(async move {
            injection_processor.run().await;
        }))
    } else {
        None
    }
} else {
    None
};
```

#### Test (`end_to_end_wav.rs:297-350`)
```rust
let mut injection_config = InjectionConfig {
    allow_kdotool: false,
    allow_enigo: false,
    inject_on_unknown_focus: false,
    require_focus: true,
    ..Default::default()
};

if terminal.is_none() {
    // Relax focus requirements in headless mode
    injection_config.require_focus = false;
    injection_config.inject_on_unknown_focus = true;
}

// Tee transcription events (lines 311-345 - complex splitting logic)

let injection_processor = AsyncInjectionProcessor::new(
    injection_config,
    inj_rx,  // ← Receives from tee, not direct from STT
    shutdown_rx,
    None,  // ← NO METRICS
).await;

let injection_handle = tokio::spawn(async move {
    injection_processor.run().await
});
```

**Differences**:
- ❌ Test has **NO METRICS** on injection processor
- ❌ Test has complex "tee" logic to split transcription events (100+ lines)
- ❌ Test disables kdotool and enigo (testing specific backends)
- ⚠️ Production uses different default config values

**Verdict**: **DIFFERENT PURPOSE** - Test is specifically testing certain backends

---

## Summary of Discrepancies

### Critical Issues (May Cause False Positives/Negatives)

| Component | Issue | Impact |
|-----------|-------|--------|
| **VAD Config** | Test uses `Default::default()` instead of production config | ❌ **CRITICAL** - Test uses different silence duration (likely 100ms vs 500ms) |
| **Metrics** | Test has NO metrics on: FrameReader, Chunker, VAD, Injection | ❌ **SIGNIFICANT** - Can't verify metrics work in real pipeline |

### Minor Issues

| Component | Issue | Impact |
|-----------|-------|--------|
| AudioConfig | Test uses `default()`, production uses explicit values | ⚠️ Minor - May have different buffer size |
| ResamplerQuality | Test hardcodes `Balanced`, production configurable | ⚠️ Minor - Acceptable for test |
| Channel sizes | Session channel: test=100, prod=200 | ⚠️ Minor - Unlikely to matter |
| Model selection | Different logic for Whisper model | ✅ OK - Test-specific override |

### Intentional Differences (Acceptable)

| Component | Difference | Reason |
|-----------|------------|--------|
| Injection backends | Test disables kdotool/enigo | Testing specific backends |
| Transcription tee | Test splits events for verification | Need to capture output for assertions |
| Terminal handling | Test-specific terminal spawn | Needed for verification |

---

## Recommendations

### 1. Fix VAD Config Immediately ⚠️

**Problem**: Test uses `Default::default()` for Silero config, production uses carefully tuned values.

**Fix**:
```rust
// end_to_end_wav.rs:182
let vad_cfg = UnifiedVadConfig {
    mode: VadMode::Silero,
    frame_size_samples: FRAME_SIZE_SAMPLES,
    sample_rate_hz: SAMPLE_RATE_HZ,
    silero: SileroConfig {
        threshold: 0.1,
        min_speech_duration_ms: 100,
        min_silence_duration_ms: 500,  // ← MUST MATCH PRODUCTION
        window_size_samples: FRAME_SIZE_SAMPLES,
    },
};
```

### 2. Add Metrics Throughout

**Problem**: Test doesn't verify metrics collection works.

**Options**:
A) Add metrics to test (verify they're incremented)
B) Document that metrics aren't tested in E2E (acceptable if tested elsewhere)

**Recommendation**: Option A - Add metrics and verify basic counters work

### 3. Consider Extracting Pipeline Factory

**Problem**: Test duplicates 200+ lines of pipeline wiring.

**Solution**: Create a shared pipeline builder:
```rust
// crates/app/src/pipeline_builder.rs
pub struct PipelineBuilder {
    audio_config: AudioConfig,
    vad_config: UnifiedVadConfig,
    stt_config: TranscriptionConfig,
    // ...
}

impl PipelineBuilder {
    pub fn new() -> Self { /* ... */ }

    pub fn with_audio_source(mut self, source: AudioSource) -> Self { /* ... */ }

    pub async fn build(self) -> Pipeline { /* ... */ }
}

// Use in both runtime.rs and tests
let pipeline = PipelineBuilder::new()
    .with_audio_source(AudioSource::WavFile("test.wav"))
    .with_vad_config(production_vad_config())
    .build()
    .await?;
```

---

## Verdict

**Overall Assessment**: **MISALIGNED** 🔴

The test pipeline has **critical differences** from production:

1. ❌ **VAD config is wrong** - Uses defaults instead of production tuning
2. ❌ **No metrics verification** - Can't confirm metrics work
3. ⚠️ **Duplicated wiring** - 200+ lines recreating what runtime.rs does

**Does this explain passing test with failing production?**

- **Possibly**: If VAD config differences cause production to segment differently than test
- **Possibly**: If metrics collection causes production performance issues not seen in test
- **Unlikely**: Core pipeline flow is similar enough that major bugs would affect both

**Recommended Actions**:

1. **Immediate**: Fix VAD config to match production
2. **Short-term**: Add metrics and verify they work
3. **Long-term**: Extract shared pipeline builder to eliminate duplication
