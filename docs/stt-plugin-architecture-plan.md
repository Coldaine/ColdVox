---
doc_type: architecture
subsystem: stt-plugin
version: 1.1
status: implemented-partial
owners: [kilo-code]
last_reviewed: 2025-09-12
# Note: Core implementation complete per stt-plugin-completion-plan.md v1.4, but partial gaps in TUI controls and config persistence. See Verification Summary for details.
---

# STT Plugin Architecture - Comprehensive Transformation Plan

## Executive Summary

Transform ColdVox's STT system into a truly modular plugin architecture that supports multiple lightweight STT engines, with automatic selection based on availability, performance, and user preferences.

## Current State Analysis

### Existing Components
- **Basic plugin system**: `SttPlugin` trait, registry, factory pattern
- **Vosk implementation**: Separate crate with `VoskTranscriber` 
- **Mock/NoOp plugins**: Basic testing and fallback support
- **Plugin manager**: Simple selection and switching logic

### Limitations
- Vosk plugin is just a stub, not fully integrated
- No dynamic plugin loading
- No performance metrics for intelligent selection
- No support for cloud/hybrid STT solutions
- Limited configuration options

## Proposed Architecture

### Core Design Principles

1. **Zero-dependency core**: STT functionality should work without any specific engine
2. **Runtime selection**: Choose best available engine based on constraints
3. **Performance-aware**: Track and use metrics for intelligent selection
4. **Progressive enhancement**: Gracefully upgrade from basic to advanced engines
5. **Cloud-ready**: Support both local and cloud-based engines

### Plugin Hierarchy

```
STT Plugin System
├── Core Abstractions
│   ├── SttPlugin (trait)
│   ├── SttPluginFactory (trait)
│   ├── PluginCapabilities (struct)
│   └── PluginMetrics (struct)
│
├── Plugin Categories
│   ├── Local Lightweight
│   │   ├── Parakeet (Mozilla, ~50MB)
│   │   ├── Whisper.cpp (ggml, ~40MB)
│   │   ├── Pocketsphinx (CMU, ~10MB)
│   │   └── Silero STT (ONNX, ~50MB)
│   │
│   ├── Local Heavy
│   │   ├── Vosk (Kaldi, ~500MB)
│   │   ├── Coqui/DeepSpeech (TF, ~200MB)
│   │   └── Whisper (PyTorch, ~1.5GB)
│   │
│   ├── Commercial Local
│   │   ├── Picovoice Leopard (~30MB)
│   │   ├── Sensory TrulyHandsfree
│   │   └── Dragon SDK
│   │
│   └── Cloud Services
│       ├── OpenAI Whisper API
│       ├── Google Cloud STT
│       ├── Azure Cognitive Services
│       └── AWS Transcribe
│
└── Plugin Management
    ├── Discovery System
    ├── Loading System
    ├── Selection Algorithm
    └── Metrics Collector
```

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1)

#### 1.1 Enhanced Plugin Traits

```rust
// Enhanced plugin capabilities
pub struct PluginCapabilities {
    // Existing...
    pub streaming: bool,
    pub batch: bool,
    
    // New capabilities
    pub languages: Vec<Language>,
    pub model_sizes: Vec<ModelSize>,
    pub accuracy_level: AccuracyLevel,
    pub latency_ms: LatencyProfile,
    pub resource_usage: ResourceProfile,
}

pub enum AccuracyLevel {
    Low,      // ~70% WER - Pocketsphinx
    Medium,   // ~85% WER - Parakeet, Silero
    High,     // ~95% WER - Vosk, Coqui
    VeryHigh, // ~98% WER - Whisper, Cloud
}

pub struct PluginMetrics {
    pub total_audio_processed_ms: u64,
    pub total_transcriptions: u64,
    pub average_latency_ms: f64,
    pub error_rate: f64,
    pub memory_usage_mb: u32,
    pub cpu_usage_percent: f32,
}
```

#### 1.2 Plugin Discovery System

```rust
pub trait PluginDiscovery {
    /// Scan for available plugins at runtime
    async fn discover_plugins(&self) -> Vec<PluginInfo>;
    
    /// Load plugin from path/URL
    async fn load_plugin(&self, source: PluginSource) -> Result<Box<dyn SttPlugin>>;
    
    /// Check system requirements
    fn check_requirements(&self, plugin: &PluginInfo) -> PluginRequirements;
}

pub enum PluginSource {
    BuiltIn(String),
    SystemPath(PathBuf),
    Download(Url),
    DynamicLibrary(PathBuf),
}
```

### Phase 2: Vosk Plugin Implementation (Week 1-2)

#### 2.1 Complete Vosk Plugin Wrapper

```rust
// coldvox-stt/src/plugins/vosk/mod.rs
pub struct VoskPlugin {
    transcriber: Option<VoskTranscriber>,
    config: VoskConfig,
    metrics: PluginMetrics,
    state: PluginState,
}

impl VoskPlugin {
    pub async fn new() -> Result<Self> {
        // Auto-detect model location
        let model_path = Self::find_model().await?;
        
        // Initialize with optimal settings
        let config = VoskConfig::optimal_for_system()?;
        
        Ok(Self {
            transcriber: None,
            config,
            metrics: PluginMetrics::default(),
            state: PluginState::Uninitialized,
        })
    }
    
    async fn find_model() -> Result<PathBuf> {
        // Search order:
        // 1. VOSK_MODEL_PATH env var
        // 2. Local models/ directory
        // 3. System paths (/usr/share/vosk-models)
        // 4. Download if configured
    }
}
```

### Phase 3: Lightweight Plugin Stubs (Week 2)

#### 3.1 Parakeet Plugin (Mozilla's Lightweight STT)

```rust
// coldvox-stt/src/plugins/parakeet/mod.rs
/// Mozilla Parakeet - Ultra-lightweight STT
/// ~50MB model, WebAssembly-compatible
pub struct ParakeetPlugin {
    engine: Option<ParakeetEngine>,
    wasm_runtime: Option<WasmRuntime>,
}

impl ParakeetPlugin {
    pub fn info() -> PluginInfo {
        PluginInfo {
            id: "parakeet",
            name: "Mozilla Parakeet",
            description: "Ultra-lightweight WASM-based STT",
            memory_usage_mb: Some(50),
            accuracy_level: AccuracyLevel::Medium,
            supported_languages: vec!["en"],
            is_beta: true,
        }
    }
}
```

#### 3.2 Whisper.cpp Plugin (Lightweight C++ Whisper)

```rust
// coldvox-stt/src/plugins/whisper_cpp/mod.rs
/// Whisper.cpp - Lightweight C++ implementation of OpenAI Whisper
/// Uses ggml quantization for small models (~40MB)
pub struct WhisperCppPlugin {
    context: Option<*mut WhisperContext>,
    model_type: WhisperModelType,
}

pub enum WhisperModelType {
    Tiny,    // 39MB, fastest, lower accuracy
    Base,    // 74MB, balanced
    Small,   // 244MB, good accuracy
}
```

#### 3.3 Coqui STT Plugin (Formerly Mozilla DeepSpeech)

```rust
// coldvox-stt/src/plugins/coqui/mod.rs
/// Coqui STT - Community fork of Mozilla DeepSpeech
/// Good accuracy, moderate size (~200MB)
pub struct CoquiPlugin {
    model: Option<CoquiModel>,
    scorer: Option<CoquiScorer>,
}
```

#### 3.4 Picovoice Leopard Plugin (Commercial Lightweight)

```rust
// coldvox-stt/src/plugins/leopard/mod.rs
/// Picovoice Leopard - Commercial ultra-lightweight STT
/// ~30MB, requires license key
pub struct LeopardPlugin {
    leopard: Option<Leopard>,
    access_key: Option<String>,
}
```

#### 3.5 Silero STT Plugin (ONNX-based Lightweight)

```rust
// coldvox-stt/src/plugins/silero_stt/mod.rs
/// Silero STT - ONNX-based lightweight STT
/// ~50MB, good for edge devices
pub struct SileroSttPlugin {
    session: Option<OrtSession>,
    tokenizer: SileroTokenizer,
}
```

### Phase 4: Intelligent Selection System (Week 2-3)

#### 4.1 Selection Algorithm

```rust
pub struct IntelligentSelector {
    constraints: SelectionConstraints,
    preference_weights: PreferenceWeights,
}

pub struct SelectionConstraints {
    max_memory_mb: Option<u32>,
    max_latency_ms: Option<u32>,
    min_accuracy: Option<AccuracyLevel>,
    require_offline: bool,
    require_streaming: bool,
    languages: Vec<Language>,
}

impl IntelligentSelector {
    pub async fn select_best_plugin(
        &self,
        available: Vec<&PluginInfo>,
        metrics: &MetricsHistory,
    ) -> Option<String> {
        // Score each plugin based on:
        // 1. Constraint satisfaction (hard requirements)
        // 2. Performance history (if available)
        // 3. User preferences
        // 4. System resources
        
        let scored = available
            .iter()
            .filter(|p| self.satisfies_constraints(p))
            .map(|p| (p, self.calculate_score(p, metrics)))
            .collect::<Vec<_>>();
        
        scored
            .into_iter()
            .max_by_key(|(_, score)| *score)
            .map(|(info, _)| info.id.clone())
    }
}
```

#### 4.2 Adaptive Learning

```rust
pub struct AdaptivePluginManager {
    selector: IntelligentSelector,
    metrics_db: MetricsDatabase,
    learning_rate: f32,
}

impl AdaptivePluginManager {
    /// Learn from usage patterns
    pub async fn update_preferences(&mut self, feedback: UserFeedback) {
        // Adjust weights based on:
        // - Transcription corrections
        // - Manual plugin switches
        // - Performance complaints
        
        match feedback {
            UserFeedback::Correction { plugin, .. } => {
                // Lower accuracy weight for this plugin
            }
            UserFeedback::TooSlow { plugin } => {
                // Lower latency score for this plugin
            }
            UserFeedback::Preferred { plugin } => {
                // Boost overall score for this plugin
            }
        }
    }
}
```

### Phase 5: Plugin Development Kit (Week 3)

#### 5.1 Plugin Template

```rust
// templates/stt-plugin-template/src/lib.rs
#[derive(Debug)]
pub struct MyCustomSttPlugin {
    // Your plugin state
}

#[async_trait]
impl SttPlugin for MyCustomSttPlugin {
    // Implement required methods
}

// Export factory function
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn SttPlugin {
    Box::into_raw(Box::new(MyCustomSttPlugin::default()))
}
```

#### 5.2 Plugin Testing Framework

```rust
// coldvox-stt/src/testing/plugin_test.rs
pub struct PluginTestSuite {
    test_audio: Vec<TestAudio>,
    expected_transcriptions: Vec<String>,
}

impl PluginTestSuite {
    pub async fn test_plugin(&self, plugin: &mut dyn SttPlugin) -> TestResults {
        // Test accuracy
        let accuracy = self.test_accuracy(plugin).await?;
        
        // Test latency
        let latency = self.test_latency(plugin).await?;
        
        // Test memory usage
        let memory = self.test_memory(plugin).await?;
        
        // Test error handling
        let robustness = self.test_robustness(plugin).await?;
        
        TestResults {
            accuracy,
            latency,
            memory,
            robustness,
        }
    }
}
```

## Configuration Schema

### Plugin Configuration

```toml
# coldvox.toml
[stt]
# Selection strategy
selection_mode = "intelligent"  # intelligent | manual | fallback

# Preferred plugins in order
preferred_plugins = ["parakeet", "whisper-cpp", "vosk"]

# Constraints
[stt.constraints]
max_memory_mb = 200
max_latency_ms = 100
min_accuracy = "medium"
require_offline = true

# Plugin-specific configuration
[stt.plugins.vosk]
model_path = "models/vosk-model-small-en-us"
enable_gpu = false

[stt.plugins.parakeet]
model = "parakeet-tinywave-en"
enable_vad = true

[stt.plugins.whisper-cpp]
model_type = "tiny"
language = "en"
enable_timestamps = true

[stt.plugins.leopard]
access_key = "${PICOVOICE_ACCESS_KEY}"
model_path = "models/leopard-en.pv"
```

## Migration Path

### From Current to Plugin-based

1. **Phase 1**: Wrap existing Vosk in plugin interface
2. **Phase 2**: Add lightweight alternatives (Parakeet, Whisper.cpp)
3. **Phase 3**: Implement intelligent selection
4. **Phase 4**: Add cloud plugin support
5. **Phase 5**: Dynamic plugin loading

### Backward Compatibility

```rust
// Compatibility layer
pub fn create_transcriber(config: LegacyConfig) -> Box<dyn Transcriber> {
    // Map legacy config to plugin system
    let plugin_config = PluginConfig::from_legacy(config);
    
    // Create plugin manager
    let manager = PluginManager::new(plugin_config);
    
    // Return compatibility wrapper
    Box::new(LegacyTranscriberAdapter::new(manager))
}
```

**Migration Note:** STT Plugin Manager fully integrated as of 2025-09-12, with telemetry, TUI exposure (partial), and runtime config hot-reload. Core failover/GC/metrics operational; see [docs/tasks/stt-plugin-completion-plan.md#verification-summary](docs/tasks/stt-plugin-completion-plan.md#verification-summary) for implementation status and gaps (TUI tab/controls, json persistence). Backward compatibility maintained via VOSK_MODEL_PATH mapping to preferred=vosk in main.rs. No breaking changes to SttPlugin trait.

## Performance Targets

### Lightweight Plugins

| Plugin | Model Size | Latency | Accuracy | Memory |
|--------|-----------|---------|----------|---------|
| Parakeet | 50MB | <50ms | 85% | 100MB |
| Whisper.cpp Tiny | 39MB | <100ms | 82% | 150MB |
| Pocketsphinx | 10MB | <30ms | 70% | 50MB |
| Silero STT | 50MB | <60ms | 88% | 120MB |
| Leopard | 30MB | <40ms | 90% | 80MB |

### Heavy Plugins

| Plugin | Model Size | Latency | Accuracy | Memory |
|--------|-----------|---------|----------|---------|
| Vosk | 500MB | <150ms | 95% | 600MB |
| Coqui | 200MB | <200ms | 93% | 400MB |
| Whisper Small | 244MB | <300ms | 97% | 500MB |

## Testing Strategy

### Unit Tests
- Plugin interface compliance
- Factory creation
- Capability reporting
- Metrics collection

### Integration Tests
- Plugin switching
- Fallback behavior
- Performance benchmarks
- Resource monitoring

### End-to-End Tests
- Real audio processing
- Accuracy validation
- Latency measurement
- Memory profiling

## Documentation Plan

### Developer Guides
1. Creating a Custom STT Plugin
2. Plugin Testing Best Practices
3. Performance Optimization Guide
4. Cloud Plugin Implementation

### User Guides
1. Choosing the Right STT Plugin
2. Configuration Guide
3. Troubleshooting Common Issues
4. Privacy and Security Considerations

## Timeline

### Week 1
- [x] Complete core infrastructure
- [x] Implement full Vosk plugin
- [x] Create plugin discovery system

### Week 2
- [x] Add Parakeet plugin
- [x] Add Whisper.cpp plugin
- [x] Implement intelligent selection

### Week 3
- [x] Add remaining lightweight plugins
- [x] Create plugin testing framework
- [x] Write documentation

### Week 4
- [x] Performance optimization
- [ ] Cloud plugin stubs
- [x] Release preparation

## Success Metrics

1. **Plugin Variety**: At least 5 working plugins
2. **Performance**: <100ms latency for lightweight plugins
3. **Memory**: <200MB for lightweight plugins
4. **Accuracy**: >85% for lightweight, >95% for heavy
5. **Developer Experience**: Plugin creation in <1 hour
6. **User Experience**: Automatic best plugin selection

## Risk Mitigation

### Technical Risks
- **Library conflicts**: Use isolated plugin processes
- **Performance regression**: Comprehensive benchmarking
- **API breaking changes**: Versioned plugin interface

### Business Risks
- **License compatibility**: Clear license documentation
- **Commercial plugins**: Optional paid tier
- **Support burden**: Automated testing and validation

## Conclusion

This modular plugin architecture will transform ColdVox into a flexible STT platform that can adapt to any user's needs, from ultra-lightweight edge deployments to high-accuracy cloud services. The focus on lightweight alternatives ensures broad compatibility while maintaining the option for heavy, high-accuracy engines when needed.