---
doc_type: task-backlog
subsystem: stt-plugin
version: 1.0
status: maintenance
owners: [kilo-code]
last_reviewed: 2025-09-12
---

# STT Plugin Maintenance and Future Tasks

Post-completion backlog based on stt-plugin-completion-plan.md v1.4 verification. Core feature implemented with partial gaps; prioritize fixes then enhancements.

## Immediate Fixes (Address Verification Gaps)

- [ ] Implement TUI "Plugins" tab in tui_dashboard.rs: Add dedicated section in draw_ui for plugin status (current/active lists), integrate show_plugins toggle.
- [ ] Add TUI interactive controls: Implement [P] to toggle Plugins tab, [L] load plugin (mock call to plugin_manager.switch_plugin), [U] unload (call unload_plugin) in key event handling.
- [ ] Add configuration persistence: In plugin_manager.rs set_selection_config, implement serde_json serialization/deserialization for PluginSelectionConfig; add load from ./plugins.json on init, save on config changes; include config_path: Option<PathBuf> field in SttPluginManager.
- [ ] Integrate TUI in main.rs: Add --tui flag to CLI, spawn tui_dashboard task if enabled and feature "tui" active; forward vad_tx/stt_rx to TUI.
- [ ] Complete Step 8.6 test_unload_no_double_borrow: Add concurrent process_audio/GC test in plugin_manager.rs tests.
- [ ] Complete Step 8.7 end_to_end_stt_pipeline: Implement runtime.rs test spawning app, sending mock VAD, verifying stt_rx events.

## Validation and Documentation

- [ ] Run full Step 10 validation: cargo test --lib app (target >80% coverage for plugin_manager), cargo clippy --fix, cargo bench for decode latency; simulate CI matrix (default/vosk/tui/vosk+tui).
- [ ] Audit locks/deadlocks in plugin_manager.rs (RwLock usage in process_audio/GC); confirm no double-borrow panics.
- [ ] Cleanup legacy code: Search/grep for deprecated Transcriber traits in coldvox_stt; remove if unused (Step 1.4).
- [ ] Update README.md (Step 9.1): Add ## STT Plugins section with config flags/env vars, migration from VOSK_MODEL_PATH, example: cargo run --features vosk -- --stt-preferred=vosk.
- [ ] Update CHANGELOG.md (Step 9.3): Add v2.0.2 entry for STT Plugin Manager integration, telemetry/TUI/config (partial), tests added.
- [ ] Update stt-plugin-architecture-plan.md Timeline: Mark Week 4 cloud stubs as pending; add performance benchmarks from cargo bench.

## Future Enhancements (Phase 4+)

- [ ] Implement cloud plugins: OpenAI Whisper API, Google Cloud STT stubs per architecture plan Phase 4; add require_offline=false handling in selection.
- [ ] Add adaptive learning: Implement update_preferences in AdaptivePluginManager from architecture plan, using user feedback for selection weights.
- [ ] Expand plugin testing: Add PluginTestSuite from architecture plan for accuracy/latency/memory/robustness benchmarks.
- [ ] Performance optimization: Baseline Vosk decode latency <200ms; add criterion benches/decode_bench.rs (Step 7).
- [ ] Dynamic plugin loading: Implement PluginDiscovery trait for runtime load from path/URL (Phase 5).

## Monitoring

- [ ] Set up ongoing metrics: Monitor stt_failover_count, stt_total_errors in production; alert on >5% error rate.
- [ ] Quarterly review: Re-assess lightweight plugins (Parakeet, Whisper.cpp) availability; update if new models released.
