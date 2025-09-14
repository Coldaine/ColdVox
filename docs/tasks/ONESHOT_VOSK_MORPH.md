# One-Shot Prompt for Vosk STT Plugin Activation in ColdVox2

## Context: ColdVox2 Project and STT Plugin System

ColdVox2 is a Rust-based voice-to-text pipeline project using a modular crate structure. The STT (Speech-to-Text) subsystem is designed as a plugin architecture in `crates/coldvox-stt`, with the core trait `SttPlugin` defined in `crates/coldvox-stt/src/plugin.rs`. This trait requires implementations to provide `info()`, `capabilities()`, `process_audio()`, `finalize()`, `reset()`, and optional `load_model()`/`unload()` methods. Plugins are registered via factories in a registry (`SttPluginRegistry` in `plugin.rs`), which the app's `SttPluginManager` (in `crates/app/src/stt/plugin_manager.rs`) uses for discovery, selection, and switching.

The current codebase has a stub for Vosk integration: `VoskTranscriber` in `crates/coldvox-stt-vosk/src/vosk_transcriber.rs` implements `EventBasedTranscriber` (a lower-level trait) for direct use, but lacks a full `SttPlugin` wrapper and factory. The plugin manager has a TODO block for registration ([lines 502-507](crates/app/src/stt/plugin_manager.rs:502)). The `crates/coldvox-stt/src/plugins/mod.rs` declares `#[cfg(feature = "vosk")] pub mod vosk;` but lacks exports. Dependencies in `Cargo.toml` include `vosk-api` for the Vosk FFI bindings.

This one-shot task activates Vosk as a selectable plugin without altering existing telemetry (e.g., `PipelineMetrics` in `crates/coldvox-telemetry/src/pipeline_metrics.rs`), text-injection (uses `TranscriptionEvent` from `crates/coldvox-stt/src/types.rs`), or model handling (e.g., `model.rs` in `crates/coldvox-stt-vosk/src/model.rs` for path resolution). Align with Rust best practices: Use async_trait for SttPlugin, ensure thread-safety with Arc<Mutex> where needed, and maintain feature-gating (`vosk` feature).

### Relevant Existing Code Snippets

#### 1. Plugin Registration TODO in `crates/app/src/stt/plugin_manager.rs` (Lines 501-508)
```rust
// Register Vosk plugin if the vosk feature is enabled in the app
#[cfg(feature = "vosk")]
{
    // TODO: Implement Vosk plugin registration after Step 2 completion
    // This will use the actual VoskTranscriber from coldvox-stt-vosk crate
    // For now, Vosk is handled through the legacy processor
}
```
This is in the `register_builtin_plugins` function, where other plugins like NoOp ([lines 490-493](crates/app/src/stt/plugin_manager.rs:490)) and Whisper ([510-512](crates/app/src/stt/plugin_manager.rs:510)) are registered with `registry.register(Box::new(Factory::new()));`.

#### 2. Missing Exports in `crates/coldvox-stt/src/plugins/mod.rs` (Lines 1-31)
```rust
//! Built-in STT plugin implementations

pub mod mock;
pub mod noop;
pub mod whisper_plugin;

#[cfg(feature = "vosk")]
pub mod vosk;

#[cfg(feature = "parakeet")]
pub mod parakeet;

// Re-export commonly used plugins
pub use mock::MockPlugin;
pub use noop::NoOpPlugin;
pub use whisper_plugin::{WhisperPlugin, WhisperPluginFactory};

#[cfg(feature = "parakeet")]
pub use parakeet::ParakeetPluginFactory;
```
No `pub use vosk::{VoskPlugin, VoskPluginFactory};` under `vosk` mod, preventing import in plugin_manager.rs.

#### 3. VoskTranscriber Implementation in `crates/coldvox-stt-vosk/src/vosk_transcriber.rs` (Key Excerpts, Lines 1-301 total file)
```rust
use coldvox_stt::{
    next_utterance_id, EventBasedTranscriber, Transcriber, TranscriptionConfig, TranscriptionEvent,
    WordInfo,
};
use tracing::warn;
use vosk::{CompleteResult, DecodingState, Model, PartialResult, Recognizer};

pub struct VoskTranscriber {
    recognizer: Recognizer,
    config: TranscriptionConfig,
    current_utterance_id: u64,
}

impl VoskTranscriber {
    pub fn new(config: TranscriptionConfig, sample_rate: f32) -> Result<Self, String> {
        // ... model loading and recognizer creation (lines 16-51)
        let model = Model::new(&model_path)
            .ok_or_else(|| format!("Failed to load Vosk model from: {}", model_path))?;
        let mut recognizer = Recognizer::new(&model, sample_rate)
            .ok_or_else(|| format!("Failed to create Vosk recognizer with sample rate: {}", sample_rate))?;
        recognizer.set_max_alternatives(config.max_alternatives as u16);
        recognizer.set_words(config.include_words);
        recognizer.set_partial_words(config.partial_results && config.include_words);
        let mut final_config = config;
        final_config.model_path = model_path;
        Ok(Self {
            recognizer,
            config: final_config,
            current_utterance_id: next_utterance_id(),
        })
    }

    // EventBasedTranscriber impl
    fn accept_frame(&mut self, pcm: &[i16]) -> Result<Option<TranscriptionEvent>, String> {
        if !self.config.enabled {
            return Ok(None);
        }
        let state = self.recognizer.accept_waveform(pcm)
            .map_err(|e| format!("Vosk waveform acceptance failed: {:?}", e))?;
        match state {
            DecodingState::Finalized => {
                let result = self.recognizer.result();
                let event = Self::parse_complete_result_static(result, self.current_utterance_id, self.config.include_words);
                Ok(event)
            }
            DecodingState::Running => {
                if self.config.partial_results {
                    let partial = self.recognizer.partial_result();
                    let event = Self::parse_partial_result_static(partial, self.current_utterance_id);
                    Ok(event)
                } else {
                    Ok(None)
                }
            }
            DecodingState::Failed => Ok(Some(TranscriptionEvent::Error {
                code: "VOSK_DECODE_FAILED".to_string(),
                message: "Vosk recognition failed for current chunk".to_string(),
            })),
        }
    }

    fn finalize_utterance(&mut self) -> Result<Option<TranscriptionEvent>, String> {
        let final_result = self.recognizer.final_result();
        let event = Self::parse_complete_result_static(final_result, self.current_utterance_id, self.config.include_words);
        self.current_utterance_id = next_utterance_id();
        Ok(event)
    }

    fn reset(&mut self) -> Result<(), String> {
        let _ = self.recognizer.final_result();
        self.current_utterance_id = next_utterance_id();
        Ok(())
    }

    fn config(&self) -> &TranscriptionConfig {
        &self.config
    }
}

// Legacy Transcriber impl (backward compatibility)
impl Transcriber for VoskTranscriber {
    fn accept_pcm16(&mut self, pcm: &[i16]) -> Result<Option<String>, String> {
        match self.accept_frame(pcm)? {
            Some(TranscriptionEvent::Final { text, .. }) => Ok(Some(text)),
            Some(TranscriptionEvent::Partial { text, .. }) => Ok(Some(format!("[partial] {}", text))),
            Some(TranscriptionEvent::Error { message, .. }) => Err(message),
            None => Ok(None),
        }
    }

    fn finalize(&mut self) -> Result<Option<String>, String> {
        match self.finalize_utterance()? {
            Some(TranscriptionEvent::Final { text, .. }) => Ok(Some(text)),
            Some(TranscriptionEvent::Partial { text, .. }) => Ok(Some(text)),
            Some(TranscriptionEvent::Error { message, .. }) => Err(message),
            None => Ok(None),
        }
    }
}
```
VoskTranscriber handles PCM16 input via `accept_frame` and emits `TranscriptionEvent`s, but it's not wrapped in SttPlugin.

#### 4. Cargo.toml Dependencies (Excerpt from `crates/app/Cargo.toml`, Lines 91-93)
```toml
coldvox-stt = { path = "../coldvox-stt", features = ["parakeet", "whisper"] }
coldvox-stt-vosk = { path = "../coldvox-stt-vosk", optional = true, features = ["vosk"] }
```
Vos k is optional; enable with `--features vosk`. Vosk crate (vosk-api) is in coldvox-stt-vosk/Cargo.toml (not shown, but implied by feature).

#### 5. Manifests
- `crates/coldvox-stt/Cargo.toml` (Lines 17-24): Features include `vosk = []` (empty, no sub-deps; assumes vosk-api via vosk crate in dependency tree).
- `crates/coldvox-stt-vosk/Cargo.toml` (Lines 1-38 from prior): Depends on `vosk = "0.1"` (FFI), `coldvox-stt = { path = "../coldvox-stt" }`.

## Desired Outcome

Activate the Vosk STT plugin seamlessly in the ColdVox2 project. This means:
- The plugin manager in `crates/app/src/stt/plugin_manager.rs` registers Vosk as available when `--features vosk` is enabled.
- `VoskPlugin` (new) wraps `VoskTranscriber` to implement the `SttPlugin` trait, delegating `process_audio` to `transcriber.accept_frame` and handling events.
- `VoskPluginFactory` creates instances, checks model via `model.rs`.
- No breaks to telemetry (use `PipelineMetrics` for counts like `stt_transcription_requests` in `crates/coldvox-telemetry/src/pipeline_metrics.rs`), text-injection (emits standard `TranscriptionEvent` from `crates/coldvox-stt/src/types.rs`), or model handling (`VOSK_MODEL_PATH` honored in `VoskTranscriber::new`).
- Aligns with architecture: Modular (SttPlugin trait in `plugin.rs`), feature-gated, no changes to `Transcriber` trait (keep backward compat via adapter if needed).

This resolves activation gaps: Missing exports (add pub use), absent VoskPlugin (implement wrapper), incomplete registration (replace TODO). Validates no risks to STT core (registry in `plugin.rs` extensible), text-injection (events flow unchanged), modularity (Factory pattern per `plugin.rs` [174-183](crates/coldvox-stt/src/plugin.rs:174)).

## Step-by-Step Implementation Guidance

Implement the changes precisely as described. Use `apply_diff` for targeted edits, `write_to_file` for new code, and ensure all changes are under `#[cfg(feature = "vosk")]` where appropriate. After changes, verify with `cargo check --features vosk` and a basic test (e.g., create and register, call `process_audio` expecting events).

1. **Add Vosk Exports in `crates/coldvox-stt/src/plugins/mod.rs`**:
   - Under the vosk mod declaration (`#[cfg(feature = "vosk")] pub mod vosk;`), add:
     ```rust
     #[cfg(feature = "vosk")]
     pub use vosk::{VoskPlugin, VoskPluginFactory};
     ```
   - Ensure it matches existing re-exports (e.g., `pub use mock::MockPlugin;` at line 26).

2. **Implement VoskPlugin and VoskPluginFactory in `crates/coldvox-stt-vosk/src/lib.rs`**:
   - Add `VoskPlugin` struct wrapping `VoskTranscriber` (add field `transcriber: Option<VoskTranscriber>` for lazy init).
   - Impl `SttPlugin` for VoskPlugin:
     - `info()`: Return PluginInfo with id="vosk", name="Vosk STT", description="Offline speech recognition using Vosk toolkit", is_local=true, supported_languages=vec!["en"], memory_usage_mb=Some(500).
     - `capabilities()`: streaming=true, batch=true, word_timestamps=true, confidence_scores=true, others=false.
     - `is_available()`: Check libvosk via `pkg-config` or simple FFI call; return true if feature enabled.
     - `initialize(config)`: Create VoskTranscriber with config.model_path (use default from `model.rs` if empty), store in self.transcriber.
     - `process_audio(samples)`: If transcriber loaded, call transcriber.accept_frame(&samples); map to TranscriptionEvent (Partial/Final/Error); return Ok(Some(event)) or Ok(None) if no result.
     - `finalize()`: If transcriber, call transcriber.finalize_utterance(); return event.
     - `reset()`: Call transcriber.reset() if Some.
     - `load_model(path)`: If path Some, set config.model_path=path; call initialize.
     - `unload()`: Set transcriber=None; log success.
   - Impl `SttPluginFactory` for VoskPluginFactory:
     - `create()`: Create VoskPlugin with default config (model_path from env or default_model_path() in model.rs).
     - `plugin_info()`: Return info from default VoskPlugin::info().
     - `check_requirements()`: Call locate_model(None) from model.rs; return Ok(()) if found, else Err(NotAvailable with reason).
   - Use async_trait; ensure Send+Sync. Delegate events without altering Transcriber.

3. **Register Vosk in `crates/app/src/stt/plugin_manager.rs`**:
   - In `register_builtin_plugins` ([lines 489-521](crates/app/src/stt/plugin_manager.rs:489)), replace the vosk TODO block ([502-507](crates/app/src/stt/plugin_manager.rs:502)) with:
     ```rust
     #[cfg(feature = "vosk")]
     {
         use coldvox_stt::plugins::vosk::VoskPluginFactory;
         registry.register(Box::new(VoskPluginFactory::new()));
     }
     ```
   - Import at top: `use coldvox_stt::plugins::vosk::VoskPluginFactory;`.

4. **Update Dependencies if Needed**:
   - In `crates/coldvox-stt-vosk/Cargo.toml`, ensure `vosk-api` version >=0.1.0 (current compatible). Add to `coldvox-stt` if cross-crate use requires (but since separate crate, no).
   - In workspace Cargo.toml, no change needed (vosk feature exists).

5. **Add Basic Tests**:
   - In `crates/coldvox-stt-vosk/src/lib.rs` or tests/, add #[tokio::test] for VoskPluginFactory::create() succeeding, info() matching, process_audio() delegating to VoskTranscriber (mock if needed), and integration with registry (create_plugin("vosk") returns Ok).
   - Ensure feature "vosk" for tests.

Apply changes in order, using prior context for exact lines. Verify: Plugin listed in registry.available_plugins(), create_plugin("vosk") works, no panics in process_audio with sample data. Aligns with modularity (no Transcriber changes); risks low (feature-gated, errors via SttPluginError).

## How to Use Morph Apply for Minimal-Step Execution

1. **Load the One-Shot Prompt**: Copy the above prompt into Morph's context window (or upload the MD file directly if supported).

2. **Invoke Morph Apply**: Use Morph Apply command with the prompt, specifying the workspace path (`/home/coldaine/Projects/Worktrees/ColdVox2`) and target files (e.g., `crates/app/src/stt/plugin_manager.rs`, `crates/coldvox-stt/src/plugins/mod.rs`, `crates/coldvox-stt-vosk/src/lib.rs`, `crates/coldvox-stt-vosk/Cargo.toml`). Morph will generate diffs for review.

3. **Review and Apply Diffs**: Inspect the diffs in Morph (check for correct feature guards, imports, and no unrelated changes). Apply in one batch via Morph's apply function.

4. **Verify**: Run `cargo build --features vosk` to ensure compilation, then `cargo test` (add if needed). Test runtime: `cargo run --features vosk -- --log-level info,stt=debug` and confirm "Registered plugin: vosk" logs and transcription flow.

If issues (e.g., vosk-api version mismatch), use Morph's iterative mode: Apply subset, fix, re-run. Limit to 3-5 steps; target <200ms decode latency.