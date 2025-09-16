---
doc_type: implementation-plan
subsystem: stt-plugin-vosk
version: 1.0
status: draft
owners: [kilo-code]
last_reviewed: 2025-09-14
# Phase 3 testing plan: Comprehensive strategy for Vosk integration per stt-vosk-code-plan.md and vosk-plugin-integration.md. Covers unit (traits/classify/metrics), integration (manager/runtime), benchmarks (latency/memory), validation (no regressions, >90% coverage). Uses tokio-test/criterion; mocks for Transcriber/audio.
---

# STT Vosk Plugin Testing Strategy

## Overview

This plan outlines testing for VoskPlugin integration, ensuring robustness (failover/GC/errors), performance (<200ms decode), correctness (events/confidence). Builds on existing: plugin_manager.rs tests (failover/GC/metrics), runtime.rs e2e stub. Goals: >90% coverage stt/vosk, no leaks, validate design (resource/error/metrics). Non-goals: Hardware audio (mock WAV), cloud (local only).

**Tools**: #[tokio::test] for async, criterion for benches, mockall for Transcriber mock, tracing-test for logs. Run: cargo test --lib stt --features vosk, cargo bench --features vosk.

**Success Criteria**: All tests pass, coverage >90% (cargo tarpaulin), benchmarks stable (latency <200ms, memory drop on unload), no panics in concurrent (process/GC).

## Unit Tests (Plugin Level)

Focus: SttPlugin trait compliance, Vosk-specific (model load/classify/metrics/unload).

### VoskPlugin Creation and Info
- **test_vosk_new_success**: new() → Ok, assert transcriber Some, capabilities (streaming=true, accuracy=High, memory=600), info id="vosk".
- **test_vosk_new_model_missing**: Mock locate_model Err → Err(ModelLoadFail fatal).
- **test_vosk_info_capabilities**: info() == expected, capabilities match profiles.
- **Files**: crates/coldvox-stt/src/plugins/vosk/mod.rs tests mod.

### Process Audio and Events
- **test_process_partial**: Mock accept_frame Running → Partial event (text), metrics audio_ms++.
- **test_process_final**: Mock Finalized → Final (text + words/conf/timings), transcriptions++.
- **test_process_failed_transient**: Mock Failed buffer → Err Transient, is_transient true.
- **test_process_failed_fatal**: Mock model fail → Err Fatal, is_fatal true.
- **test_process_unloaded**: State Unloaded → Err AlreadyUnloaded.
- **Validation**: Assert event types/conf >0.5, latency stored, no panic on classify.

### Finalize and Reset
- **test_finalize_flush**: Pending audio → Final event, utterance_id new.
- **test_finalize_error**: Mock finalize Err → classify propagate.
- **test_reset_state**: reset() → new utterance_id, state cleared.
- **Validation**: Multiple finalize → distinct ids, no stale events.

### Unload and Metrics
- **test_unload_drop**: unload() → transcriber None, state Unloaded, memory mock drop.
- **test_unload_idempotent**: unload twice → second AlreadyUnloaded, no panic.
- **test_metrics_update**: process → latency/audio/transcriptions ++, error_rate = failures/requests.
- **Files**: Same mod.rs tests. Mock: impl MockVoskTranscriber via mockall.

## Integration Tests (Manager/Runtime Level)

Focus: Selection/failover/GC with Vosk, pipeline VAD→event.

### Plugin Manager with Vosk
- **test_manager_select_vosk**: initialize preferred="vosk" → current=="vosk", active_plugins=1.
- **test_failover_from_vosk**: 3 transients → attempt_failover "noop", cooldown insert, consecutive reset.
- **test_gc_unload_vosk**: Activity insert, sleep TTL, gc_inactive → unload called, active=0, memory drop.
- **test_switch_vosk_mock**: switch "mock" → unload Vosk, current=="mock", no double-borrow (concurrent process/gc).
- **test_config_persistence_vosk**: set preferred="vosk" → save json, reload → same config.
- **Files**: crates/app/src/stt/plugin_manager.rs tests. Mock: VoskFactory create → MockVoskPlugin.

### Runtime Pipeline Integration
- **test_runtime_vad_to_vosk**: Spawn runtime, send VadEvent::Speech mock samples → stt_rx recv Final/Partial.
- **test_shutdown_unload_vosk**: runtime shutdown → unload_all + stop_gc/metrics, no leaks.
- **test_hot_reload_vosk**: set_selection_config preferred="vosk" → gc start, process uses new.
- **Validation**: End-to-end: Mock audio frames → events match expected text/conf, metrics ++ in sink.

## Benchmarks

Focus: Performance targets (Vosk decode <200ms, memory stable).

### Decode Latency
- **bench_vosk_process**: criterion black_box on process_audio 1s WAV (16000 samples), assert mean <150ms.
- **bench_vosk_finalize**: Flush after process, assert <50ms.
- **Files**: crates/coldvox-stt/benches/vosk_decode.rs [[bench]] dependencies = ["criterion"].

### Memory and GC
- **bench_load_unload_cycle**: Load Vosk, process 10x, GC unload, assert RSS peak <700MB, drop to baseline.
- **Validation**: Run cargo bench --features vosk, compare pre/post (regression <10%).

## Validation and Coverage

- **Coverage**: cargo tarpaulin --lib stt --features vosk >90% (plugin/manager/runtime).
- **Concurrent Safety**: test_concurrent_process_gc: 5 tasks process + 3 GC, no panic (RwLock scoped).
- **Logs**: tracing-test assert structured (plugin_id="vosk", event="process", latency=xx).
- **CI**: .github/workflows add vosk matrix: test/bench/tarpaulin, skip if !libvosk (setup-vosk-cache.sh).
- **Edge**: test_vosk_no_model → NotAvailable logged, fallback noop; test_high_load 100 concurrent process <1s total.

This strategy validates Vosk integration: Correct (events), resilient (errors/failover), performant (benchmarks), observable (metrics/logs). Execute post-code changes.
