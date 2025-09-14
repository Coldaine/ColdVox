---
doc_type: implementation-plan
subsystem: stt-plugin-vosk
version: 1.0
status: draft
owners: [kilo-code]
last_reviewed: 2025-09-14
# Phase 3 code plan: File-by-file changes to implement VoskPlugin integration per vosk-plugin-integration.md and stt-plugin-architecture-plan.md v1.2. Builds on partial completion (manager/runtime stubs); focuses on full Vosk wrap, registration, runtime/TUI integration. Atomic for Code mode.
---

# STT Vosk Plugin Code Implementation Plan

## Introduction and Scope

This plan details atomic code changes to realize the VoskPlugin design, eliminating stubs and enabling flagship integration. Builds on existing: VoskTranscriber (API/events/model), plugin manager (selection/failover/GC/metrics), runtime (VAD fanout). Key goals: Create full VoskPlugin in crates/coldvox-stt/src/plugins/vosk/, register factory, integrate pipeline (VAD→process), expose TUI (tab/controls). Assumptions: --features vosk enabled; no breaking traits. Non-goals: New deps (use existing vosk 0.3), cloud/other plugins.

**Files Involved**:
- New: crates/coldvox-stt/src/plugins/vosk/mod.rs (full impl ~200 lines)
- Mod: crates/coldvox-stt/src/plugin.rs (error classify variants)
- Mod: crates/app/src/stt/plugin_manager.rs (register VoskFactory)
- Mod: crates/app/src/runtime.rs (stt_vad_tx → manager.process_audio, shutdown unloads)
- Mod: crates/app/src/bin/tui_dashboard.rs (Plugins tab, keybinds P/L/U for switch/unload)
- Mod: crates/app/src/main.rs (CLI --stt-preferred=vosk mapping)

**Validation**: After each step: cargo check/test --lib stt --features vosk; no regressions.

## Detailed Implementation Steps

Steps ordered: Core plugin first (foundation), then registration/integration, finally TUI/CLI. Each atomic, testable.

### 1. Extend SttPluginError for Vosk Classification
- **Step 1.1**: In crates/coldvox-stt/src/plugin.rs, add variants to enum SttPluginError (non-exhaustive):
  ```rust
  pub enum SttPluginError {
      // Existing...
      Transient(AudioBufferEmpty, DecodingFailed),  // Retryable: buffer/decode issues
      Fatal(ModelLoadFail, RecognizerInitFail),  // Switch: model/recognizer errors
      Other(Box<dyn std::error::Error + Send + Sync>),
  }

  impl SttPluginError {
      pub fn is_transient(&self) -> bool { matches!(self, Self::Transient(..)) }
      pub fn is_fatal(&self) -> bool { matches!(self, Self::Fatal(..)) }
  }
  ```
- **From**: Branch patterns (classify for failover).
- **Validation**: Unit test classify_is_transient (mock Err → Transient true). Files: plugin.rs. Evidence: Manager uses err.is_transient() in process_audio for consecutive_errors.

### 2. Create Full VoskPlugin Implementation
- **Step 2.1**: Create new crates/coldvox-stt/src/plugins/vosk/mod.rs with full impl per design:
  - Imports: vosk, coldvox_stt::types::{TranscriptionEvent, WordInfo}, plugin::{SttPlugin, PluginInfo, PluginCapabilities, PluginMetrics}, model from coldvox-stt-vosk.
  - Struct: `pub struct VoskPlugin { transcriber: Option<VoskTranscriber>, config: TranscriptionConfig, metrics: PluginMetrics, state: PluginState }` (enum PluginState: Uninitialized/Initialized/Unloaded).
  - Impl new(): locate_model(None)?, VoskTranscriber::new(config, 16000.0)?, capabilities (streaming=true, accuracy=High, memory=600), info (id="vosk", name="Vosk Kaldi"), log_resolution.
  - SttPlugin impl:
    - process_audio: if state Unloaded { Err(AlreadyUnloaded) } else { start=now; event=transcriber.accept_frame classify err; metrics.update(start.elapsed(), samples.len()); activity insert (manager call); Ok(event) }
    - finalize: transcriber.finalize_utterance classify → Final event, metrics.transcriptions++.
    - reset: transcriber.reset, new utterance_id.
    - unload: transcriber.take().map(drop), state=Unloaded, Ok(()).
    - metrics: &mut self.metrics (manager read).
  - VoskFactory: impl SttPluginFactory { create_plugin() -> Result<Box<dyn SttPlugin>> { Box::new(VoskPlugin::new().await?) } }
  - #[cfg(feature="vosk")] pub use self::vosk_plugin::VoskFactory;
- **From**: Merge branch mod.rs patterns + Transcriber wrap, design details (classify, metrics, unload).
- **Validation**: cargo test --lib stt --features vosk (new test_vosk_plugin: new()?, process mock audio → Partial/Final, unload idempotent, classify transient/fatal). Files: New mod.rs. Evidence: Branch 508 lines → full ~200; no Transcriber changes.

### 3. Register Vosk in Plugin Manager
- **Step 3.1**: In crates/app/src/stt/plugin_manager.rs, in register_builtin_plugins: Add #[cfg(feature="vosk")] registry.register(Box::new(VoskFactory));
- **Step 3.2**: Update create_fallback_plugin: Prioritize "vosk" if available in fallback_plugins (default add if !preferred).
- **From**: Manager gaps (TODO 502-507: GC unload – already impl, but ensure Vosk unload calls drop).
- **Validation**: Unit test register_vosk: initialize preferred="vosk" → current=="vosk", fallback to noop if !feature. Files: plugin_manager.rs. Evidence: Existing NoOp/Mock/Whisper register; Vosk optional.

### 4. Integrate Runtime Pipeline
- **Step 4.1**: In crates/app/src/runtime.rs, in start(): Instantiate manager = SttPluginManager::new().with_metrics_sink(metrics); if opts.stt_preferred=="vosk" { manager.set_selection_config(PluginSelectionConfig { preferred_plugin: Some("vosk".to) , .. }); }
- **Step 4.2**: Replace legacy STT TODO (lines ~285-296): Create stt_vad_tx = mpsc::channel(10); fanout task forward VadEvent::Speech to manager.process_audio(&samples).await? → stt_tx.send(event);
- **Step 4.3**: In AppHandle::shutdown: manager.unload_all_plugins().await?; manager.stop_gc_task().await?; manager.stop_metrics_task().await?;
- **From**: Completion plan Step 1 (partial: TODO stt_vad_tx_opt=None → full manager.process).
- **Validation**: Integration test end_to_end_pipeline: Spawn runtime, send mock VAD speech → receive TranscriptionEvent via stt_rx. cargo test --lib app --features vosk. Files: runtime.rs. Evidence: Existing fanout for vad_bcast_tx; add stt integration.

### 5. Add TUI Exposure for Plugins
- **Step 5.1**: In crates/app/src/bin/tui_dashboard.rs, add Plugins tab enum (e.g., Tab::Plugins), draw_ui: if tab==Plugins { draw_plugins(f, &state) – grid: Current [id], Failovers {count}, Errors {total}, Active Plugins {len}, key help [P]cycle [L]load [U]unload }
- **Step 5.2**: In draw_plugins: Poll manager.list_plugins_sync() → table id/name/available/memory; highlight current; metrics from app.metrics (stt_failover_count etc.).
- **Step 5.3**: In key handlers: 'p'/'P' → if running { app.switch_plugin(next_id).await? } (cycle available); 'l'/'L' → app.load_plugin(specified_id).await?; 'u'/'U' → app.unload_plugin(current_id).await?;
- **Step 5.4**: Add AppHandle methods: async switch_plugin(id: &str) { self.plugin_manager.write().await.switch_plugin(id).await? }, load_plugin (initialize if None), unload_plugin.
- **From**: Completion plan Step 6 gap (no tab/controls; existing metrics snapshot but separate bin).
- **Validation**: Run bin/tui_dashboard --features vosk+tui: Verify Plugins tab draws, P cycles vosk→noop, U unloads (metrics update), no panic. Files: tui_dashboard.rs, AppHandle in runtime.rs. Evidence: Existing draw_status uses metrics; add tab like audio/pipeline.

### 6. Update CLI for Vosk Default
- **Step 6.1**: In crates/app/src/main.rs, add clap arg --stt-preferred [PLUGIN] default "vosk" if --features vosk (detect via cfg!); map VOSK_MODEL_PATH to config.model_path if preferred=="vosk".
- **Step 6.2**: In opts to manager.set_selection_config: preferred = Some(cli.stt_preferred.unwrap_or("vosk".to_string())) if cfg!(feature="vosk") else "noop".
- **From**: Migration (VOSK_MODEL_PATH → preferred=vosk).
- **Validation**: cargo run --features vosk -- --stt-preferred=vosk → log "selected: vosk"; without → "noop". Files: main.rs. Evidence: Existing --stt-* flags for constraints.

## Workflow Diagram

```mermaid
graph TD
    A[VAD Speech Event] --> B[Runtime: manager.process_audio(samples)]
    B --> C[VoskPlugin: transcriber.accept_frame]
    C --> D{State?}
    D -->|Running| E[Partial Event + Metrics Update]
    D -->|Finalized| F[Final Event + Words/Conf]
    D -->|Failed| G{Classify Error}
    G -->|Transient| H[Consecutive++ < Threshold → Err]
    G -->|Fatal| I[attempt_failover → Switch + Cooldown]
    E --> J[Manager: Activity Insert, Propagate Metrics]
    F --> J
    I --> K[New Plugin: Retry Process]
    L[Shutdown] --> M[unload_all_plugins + Stop Tasks]
```

## Risks and Tradeoffs

- **Risk: Transcriber Borrow Races**: Manager RwLock scoped; test concurrent process/GC no panic.
- **Tradeoff: Feature-Gate**: Vosk optional (no bloat), but CLI default needs cfg! check.
- **Risk: TUI Overhead**: Ratatui redraw <50ms; gate behind --tui.
- **Validation Criteria**: All tests pass (--features vosk+tui), no regressions (cargo check/test/bench), memory stable (GC unloads Vosk ~600MB→0).

This plan ensures clean Vosk integration without diverging from architecture. Ready for Code mode execution.