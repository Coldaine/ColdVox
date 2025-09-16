---
doc_type: design
subsystem: stt-plugin-vosk
version: 1.0
status: draft
owners: [kilo-code]
last_reviewed: 2025-09-14
# Supporting design for stt-plugin-architecture-plan.md v1.2: Details VoskPlugin realization of SttPlugin trait, focusing on resource management, error classification, and metrics/telemetry from branch analysis (patterns: structured logs, atomic counters, cooldowns).
---

# Vosk Plugin Integration Design

## Overview

This design realizes the Vosk flagship backend in ColdVox's STT plugin system, wrapping the existing VoskTranscriber in a production-ready SttPlugin implementation. It combines main branch Transcriber (robust event-based API, model resolution) with branch patterns (plugin architecture, metrics, resources, errors). Goals: Eliminate stub via full integration, enable runtime selection/failover/GC, track performance for intelligent choice. Non-goals: Dynamic model download (env/config only), GPU accel (CPU baseline).

## Requirements Realization

### Core Trait Implementation (SttPlugin)
- **process_audio(&[i16]) -> Result<Option<TranscriptionEvent>, SttPluginError>**: Map samples to Transcriber.accept_frame (PCM16 direct, 16kHz optimal). Handle DecodingState: Running → Partial (text only), Finalized → Final (text + words/conf/timings), Failed → Error (classify). Track metrics (latency, audio bytes). Idempotent if unloaded.
- **finalize() -> Result<Option<TranscriptionEvent>, SttPluginError>**: Call Transcriber.finalize_utterance → Final event (flush pending). Reset utterance_id for next.
- **reset() -> Result<(), SttPluginError>**: Transcriber.reset (final_result clears state) + new utterance_id.
- **unload() -> Result<(), SttPluginError>** (default trait): Take/drop Transcriber (releases Kaldi model), set state Unloaded. Idempotent (AlreadyUnloaded error if called twice).
- **info() -> PluginInfo**: id="vosk", name="Vosk Kaldi", desc="High-accuracy offline STT", available=true (model exists), version="0.3".
- **capabilities() -> PluginCapabilities**: streaming=true, batch=false, languages=["en"], model_sizes=[500MB], accuracy=High (~95% WER), latency=LatencyProfile::Medium (<150ms), resource=ResourceProfile { memory_mb=600, cpu_cores=2 }.
- **metrics() -> PluginMetrics**: Mutable ref for manager access (total_audio_ms, transcriptions, avg_latency, error_rate, memory_mb, cpu_percent).

### Resource Management Strategies
- **Model Loading**: On new(): locate_model (priority: env VOSK_MODEL_PATH > config > models/vosk-model-small-en-us-0.15 > root legacy). Log resolution (path/source). Load Model::new(path), create Recognizer (sample_rate=16000.0, max_alternatives=1, words=true). Memory: ~500-600MB (small-en-us model); profile via ResourceProfile.
- **Lifecycle**: Initialized (transcriber Some), Unloaded (take/drop). Manager GC: Activity insert on process_audio (last_activity[ "vosk" ] = now), unload on TTL (300s default) via current_plugin.write → if id=="vosk" { plugin.unload() } → None.
- **Tradeoffs**: Static load (no hot-reload model), but fast init (<1s). No shared models (per-plugin instance avoids races). Risk: Memory leak if !dropped → Mitigate: Explicit manager unloads, Drop impl aborts tasks.
- **Validation**: Unit: Load/unload cycle, assert memory drop (RSS check mock). Integration: GC task unloads after inactivity, no double-borrow (RwLock scoped).

### Error Handling Approaches
- **Classification**: Extend SttPluginError variants: Transient (AudioBufferEmpty, DecodingFailed – retryable), Fatal (ModelLoadFail, RecognizerInitFail – failover). Classify in process/finalize: e.g., accept_waveform Err → Transient if buffer-related, Fatal if model corrupt.
- **Branch Patterns**: Wrap vosk::Error in Other(Box<dyn Error + Send + Sync>), propagate. Manager: Consecutive transients >= threshold (3) → attempt_failover (cooldown 30s HashMap, fallback chain). Logs: Structured warn/error (plugin_id="vosk", event="failover", errors_consecutive=3).
- **Tradeoffs**: Granular classify adds ~5% overhead (if/else), but enables smart failover (transient skip vs fatal switch). Risk: Misclassification → Over-failover; Mitigate: Default Transient, log for tuning.
- **Validation**: Unit: Simulate errors (mock Transcriber Err), assert classify/transient skip/fatal switch. Integration: 3 transients → failover to noop, cooldown skips retry <30s.

### Metrics and Telemetry Implementation
- **Tracking**: PluginMetrics updated in process: latency=now.elapsed(), audio_processed_ms += len/16kHz, transcriptions++ on Final, error_rate = failures / requests (atomic). Manager propagates: stt_transcription_requests++/success/failures, last_latency_ms store.
- **Branch Patterns**: Periodic metrics_task (30s interval): Log summary (active_plugins, load/unload counts, failovers, requests/success/failures, gc_runs). Structured: target="coldvox::stt::metrics", fields atomic loads. Sink to PipelineMetrics (Arc<AtomicU64>).
- **Adaptive**: Basic learning stub: update_preferences(feedback) adjusts selection weights (e.g., Correction lowers accuracy score for "vosk"). Future: MetricsHistory for score calc.
- **Tradeoffs**: Atomics low-overhead (Relaxed), but no persistence (in-memory). Risk: Contention on high-load → Fine-grained (per-call increment).
- **Validation**: Unit: Assert metrics ++ on process/success, propagate to sink. Integration: Run pipeline 10s audio, verify logs/counters. Benchmark: Latency overhead <1ms.

## Tradeoffs and Risks

- **Performance**: Vosk ~150ms/decode (balanced accuracy/speed); light alternatives faster but lower WER. Tradeoff: Vosk default for quality, fallback on constraints (max_memory<600MB → parakeet).
- **Complexity**: Wrapper adds layer (error map, metrics wrap), but enables modularity. Risk: Transcriber API mismatch → Unit tests cover events/conf.
- **Dependencies**: vosk 0.3 (Kaldi bindings, Apache-2.0); optional via app feature. No cycles (stt-vosk → stt). Risk: Native libvosk missing → NotAvailable error, log guidance.
- **Validation**: End-to-end: Mock VAD speech → Vosk events → injection (if enabled). Success: No leaks (valgrind), failover works, metrics accurate (>95% requests success).

## Conclusion

This design delivers production VoskPlugin: Resource-safe (locate/drop/GC), resilient (classify/failover), observable (metrics/logs). Integrates seamlessly with manager (selection/switch/unload), eliminates stub via Transcriber wrap. Ready for Phase 3 impl: Merge branch, add classify/metrics, test cycles.
