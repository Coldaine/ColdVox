# ColdVox STT Deferred Features – Implementation Proposal

> Status: Draft (Post-Unification)  
> Scope: Build on unified STT processor to add advanced partial policies, long-hold segmentation, batch-on-release buffering, runtime configuration, and multi-engine support. Prioritize based on user needs while maintaining low-latency core.

---

## 1. Executive Summary

The initial STT unification (completed per original proposal) established a single [`UnifiedSttProcessor`](crates/app/src/stt/processor.rs) handling activation (VAD/Hotkey) and basic partial emission (Emit policy). However, advanced features were deferred to avoid premature complexity: full partial policies (Suppress/Throttle), long-hold segmentation for extended narration, batch-on-release buffering for short PTT sessions, runtime CLI/TUI configuration, parallel engine layering, and enhanced metrics.

This proposal outlines implementation of these features in phases, focusing on extensibility. It preserves the unified architecture's maintainability while enabling richer UX (e.g., adaptive buffering, suppressed partials during batch mode). No breaking changes; all additions use feature flags/enums for optional enablement.

Key benefits: Improved handling of long sessions (segmentation prevents context bloat), flexible feedback (throttled partials reduce UI churn), and future-proofing (parallel engines for latency/accuracy trade-offs).

---

## 2. Current State (Gaps from Original Proposal)

### 2.1 Implemented Foundations
- Single processor with session events ([`SessionEvent`](crates/app/src/stt/session.rs)) for Start/End/Abort.
- Basic buffering during speech + incremental engine feeding (hybrid streaming).
- Default partial emission ([`partial_results: true`](crates/coldvox-stt/src/types.rs)); events flow in both modes.
- Startup config via [`AppRuntimeOptions`](crates/app/src/runtime.rs) and env vars (e.g., activation mode).
- Tests validate unification (e.g., [`end_to_end_wav.rs`](crates/app/src/stt/tests/end_to_end_wav.rs), mode switches in [`unified_processor_tests.rs`](crates/app/src/stt/unified_processor_tests.rs)).

### 2.2 Unresolved Gaps
- **Partial Policies**: Only Emit; no Suppress (drop partials in batch mode) or Throttle (rate-limit emissions, e.g., every 500ms).
- **Long-Hold Segmentation**: No detection of extended PTT (>60s); lacks internal VAD for silence splits (e.g., 6s silence → SegmentSplit event).
- **Buffering Behaviors**: Always incremental with buffering for finals; no pure BatchOnRelease (buffer entire hold, submit on release) or Hybrid (short holds batch, long holds stream).
- **Configuration**: No full loader (TOML/env/CLI precedence); runtime changes limited to activation mode switch ([`set_activation_mode`](crates/app/src/runtime.rs:182-259)). No TUI/GUI panels.
- **Advanced Strategies**: Single-engine only; no parallel fast/accurate engines with arbitration (final overrides partials).
- **Observability**: Basic metrics ([`SttMetrics`](crates/coldvox-stt/src/processor.rs:45-63)); no per-mode histograms (latency, partial cadence) or trace spans.

### 2.3 User Impact
- Long narrations risk large contexts/delays (no splits).
- Batch-like modes lack partial suppression, causing UI flicker.
- No runtime tuning without restart.
- Missed opportunities for hybrid accuracy/latency (e.g., fast partials + slow finals).

---

## 3. Clarified Domain Model (Extensions)

### 3.1 Enhanced Dimensions
| Dimension | Current Options | Proposed Additions |
|-----------|-----------------|---------------------|
| Partial Policy | Emit | Suppress, Throttle(Duration) |
| Hotkey Behavior | Incremental | BatchOnRelease, Hybrid{threshold: Duration} |
| Long-Hold Segmentation | Disabled | Enabled{min_hold_secs: u32, silence_split_secs: u32} |
| Engine Strategy | Single | Parallel{fast: PluginId, accurate: PluginId} |

### 3.2 Core Needs Now
- Adaptive PTT: Short holds (commands) batch for accuracy; long (narration) segment for manageability.
- Flexible Feedback: Suppress partials in batch; throttle in streaming to reduce noise.
- Runtime Config: TUI toggles without restart.
- Metrics: Track policy impacts for iteration.

---

## 4. Rationale for Priorities

### 4.1 Long-Hold Segmentation
- **Rationale**: Users may dictate minutes-long sessions; current buffering risks OOM/context limits in engines like Vosk. Delayed VAD (post-threshold) avoids overhead for short presses.
- **Benefits**: Splits on silence (e.g., 6s) → multiple finals; preserves pauses without giant utterances.
- **Trade-offs**: Added VAD instance (lightweight, ~5% CPU); fallback to full-buffer if init fails.

### 4.2 Advanced Partial Policies
- **Rationale**: Emit floods UI in batch; Throttle balances responsiveness/noise.
- **Benefits**: Configurable verbosity (e.g., Suppress in batch, Throttle(500ms) in streaming).
- **Implementation**: Early drop in event handler; timestamp checks for throttle.

### 4.3 Buffering Behaviors
- **Rationale**: Incremental suits real-time but loses batch accuracy for short holds; Hybrid combines both.
- **BatchOnRelease**: Buffer raw i16 until release → single engine call (simpler, no partials).
- **Hybrid**: <4s → BatchOnRelease; >4s → switch to Incremental + segmentation.
- **Trade-offs**: Batch delays feedback; memory linear with hold (cap at 5min).

### 4.4 Configuration & UI
- **Rationale**: Startup-only limits experimentation; TUI enables live tuning.
- **CLI Flags**: `--partial-policy suppress --hotkey-behavior batch`.
- **TUI**: 'P' toggle policy, 'B' toggle behavior; display current.
- **TOML**: Full loader in `settings.rs` (precedence: CLI > env > file > default).

### 4.5 Parallel Engines & Metrics (Future)
- **Rationale**: Layer fast (e.g., Vosk-small) for partials + accurate (Whisper) for finals.
- **Metrics**: Add spans (e.g., `stt.partial_latency`), histograms per policy/mode.

---

## 5. Proposed Architecture Extensions

### 5.1 Enhanced Settings Structure
**File**: `crates/app/src/stt/settings.rs` (expand existing)

```rust
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub activation_mode: ActivationMode,        // Existing: Hotkey/Vad
    pub partial_policy: PartialPolicy,          // New: Emit/Suppress/Throttle(Duration)
    pub hotkey_behavior: HotkeyBehavior,        // New: Incremental/BatchOnRelease/Hybrid
    pub long_hold: LongHoldConfig,              // New
}

#[derive(Debug, Clone, Default)]
pub enum PartialPolicy {
    #[default]
    Emit,
    Suppress,
    Throttle(Duration),  // e.g., Duration::from_millis(500)
}

#[derive(Debug, Clone, Default)]
pub enum HotkeyBehavior {
    #[default]
    Incremental,
    BatchOnRelease,
    Hybrid { threshold_secs: u32 },  // e.g., 4
}

#[derive(Debug, Clone, Default)]
pub struct LongHoldConfig {
    pub enabled: bool,             // default: false
    pub min_hold_secs: u32,        // default: 60
    pub silence_split_secs: u32,   // default: 6
    pub min_segment_secs: u32,     // default: 10 (safeguard)
}
```

### 5.2 Extended Event Model
**File**: `crates/app/src/stt/session.rs`

```rust
#[derive(Debug, Clone)]
pub enum SessionEvent {
    Start(SessionSource, Instant),
    End(SessionSource, Instant),
    Abort(SessionSource, &'static str),
    SegmentSplit(SessionSource, Instant),  // New: For long-hold splits
}
```

### 5.3 Processor Enhancements
**File**: `crates/app/src/stt/processor.rs` (extend UnifiedSttProcessor)

- **Partial Handling**:
  ```rust
  async fn handle_stt_event(&mut self, event: TranscriptionEvent) {
      match self.settings.partial_policy {
          PartialPolicy::Emit => self.send_event(event).await,
          PartialPolicy::Suppress => {
              if !matches!(event, TranscriptionEvent::Partial { .. }) {
                  self.send_event(event).await;
              }
          }
          PartialPolicy::Throttle(duration) => {
              let now = Instant::now();
              if now.duration_since(self.last_partial).as_millis() >= duration.as_millis() {
                  self.send_event(event).await;
                  self.last_partial = now;
              }
          }
      }
  }
  ```

- **Buffering Logic** (in `handle_audio_frame`):
  ```rust
  match self.settings.hotkey_behavior {
      HotkeyBehavior::Incremental => {
          // Existing: feed engine incrementally
          if let Some(event) = self.engine.on_speech_frame(&frame.data).await {
              self.handle_stt_event(event).await;
          }
      }
      HotkeyBehavior::BatchOnRelease => {
          // Buffer raw; process on End
          self.audio_buffer.extend_from_slice(&frame.data);
          if self.audio_buffer.len() > MAX_BUFFER { /* warn & flush */ }
      }
      HotkeyBehavior::Hybrid { threshold_secs } => {
          let elapsed = now.duration_since(self.utterance_start);
          if elapsed.as_secs() < threshold_secs as u64 {
              // Batch like above
          } else {
              // Switch to Incremental + enable long-hold if applicable
              self.enable_long_hold().await;
              // Incremental feed
          }
      }
  }
  ```

- **Long-Hold Segmentation** (new method):
  ```rust
  async fn enable_long_hold(&mut self) {
      if !self.settings.long_hold.enabled || self.long_hold_active { return; }
      // Lazy-init secondary VAD (tuned for silence: low threshold, short window)
      let secondary_vad = VadProcessor::spawn_long_hold_config(self.audio_rx.clone()).await?;
      self.long_hold_active = true;
      // On silence > silence_split_secs: emit SegmentSplit → finalize current → start new
      tokio::spawn(async move {
          while let Some(silence_ev) = secondary_vad.recv().await {
              if silence_ev.duration_ms > self.settings.long_hold.silence_split_secs as u64 * 1000 {
                  // Emit split, reset buffer
              }
          }
      });
  }
  ```

### 5.4 Configuration Loader
**File**: `crates/app/src/stt/settings.rs`

- Precedence: CLI flags > Env (e.g., `COLDVOX_PARTIAL_POLICY=suppress`) > TOML (`settings.toml` in XDG/CLI path) > defaults.
- Example TOML:
  ```
  [stt]
  partial_policy = "throttle"
  throttle_ms = 500

  [hotkey]
  behavior = "hybrid"
  threshold_secs = 4

  [long_hold]
  enabled = true
  min_hold_secs = 60
  silence_split_secs = 6
  ```

### 5.5 Parallel Engines (Future)
- Add `ParallelSttLayer` wrapper: Fast engine for partials, accurate for finals.
- Arbitration: Final from accurate overrides; fallback if one fails.
- Config: `engine_strategy: Parallel { fast: "vosk-small", accurate: "whisper-base" }`.

---

## 6. Implementation Plan (Phases)

### Phase 1 (Near-Term: Policies & Buffering)
- Add PartialPolicy enum + handler logic (drop/throttle).
- Implement HotkeyBehavior: BatchOnRelease (buffer → single call), Hybrid (threshold check).
- CLI flags: `--partial-policy <emit|suppress|throttle:<ms>> --hotkey-behavior <inc|batch|hybrid:<s>>`.
- Update tests: Verify suppression (no partial events in batch), throttling (events spaced), hybrid switch.
- TUI: Add 'P' (policy toggle), 'B' (behavior toggle); display current.

### Phase 2 (Mid-Term: Long-Hold & Config)
- LongHoldConfig + enable_long_hold() with secondary VAD (reuse Silero, tuned params).
- Full settings loader: TOML parser, precedence resolution.
- Runtime apply: Expose `set_settings()` (unload/reload processor if needed; guard during speech).
- E2E tests: Simulate long WAV (>60s with pauses) → expect SegmentSplit + multiple finals.
- Metrics: Add `partial_throttle_drops`, `segment_splits`, `buffer_peak_size`.

### Phase 3 (Deferred: Parallel & Advanced)
- ParallelSttLayer: Dual-plugin orchestration, event merging.
- Enhanced metrics: `tracing::Span` for `stt.utterance` with policy/mode tags; histograms via `coldvox-telemetry`.
- GUI: Settings panel for live changes (Qt integration).
- Benchmarks: Compare latency/accuracy across configs.

---

## 7. Risk & Mitigation Table

| Risk | Impact | Mitigation |
|------|--------|------------|
| Secondary VAD overhead in long-hold | Low-Medium | Lazy-init post-threshold; lightweight config; fallback to full-buffer. |
| Throttling drops useful partials | Low | User-configurable duration; default conservative (500ms). |
| BatchOnRelease latency spike | Medium | Cap buffer (5min); warn on large holds; Hybrid as default for PTT. |
| Runtime config races | Medium | Guard switches (only idle); snapshot settings on utterance start. |
| Parallel engine complexity | High (deferred) | Stub interface now; implement with real dual models; fallback to single. |
| TOML parsing errors | Low | Graceful fallback to defaults; validate on load. |

---

## 8. Acceptance Criteria
- PartialPolicy: Tests show no partials in Suppress, spaced events in Throttle.
- HotkeyBehavior: BatchOnRelease → single final (no partials); Hybrid → switch after threshold.
- Long-Hold: >min_hold → secondary VAD active; silence >split_secs → SegmentSplit + new utterance.
- Config: TOML loads correctly; CLI/env overrides; runtime apply without crash (idle only).
- Metrics: New counters logged; no regressions in core latency.
- Existing E2E: Unchanged behavior in default config.

---

## 9. Minimal Implementation Sketch
```rust
// settings.rs (loader)
pub async fn load_settings(cli: &CliArgs) -> Settings {
    // CLI > env > TOML > default
    let mut settings = Settings::default();
    if let Some(policy) = &cli.partial_policy { settings.partial_policy = policy.parse()?; }
    // ... similar for others
    settings
}

// processor.rs (handle_audio_frame extension)
match behavior {
    HotkeyBehavior::BatchOnRelease => self.buffer_raw(&frame),
    // ...
}

// session.rs (long-hold spawn)
if hold_duration > min_hold && !self.long_vad_active {
    self.long_vad = Some(spawn_tuned_vad(self.audio_rx.clone()));
    // On silence: self.session_tx.send(SessionEvent::SegmentSplit(source, now)).await;
}
```

---

## 10. Out-of-Scope
- Diarization/multi-speaker.
- Offline/online model switching.
- Custom VAD models per policy.

---

## 11. Next Steps Checklist
- [ ] Implement PartialPolicy + tests.
- [ ] Add HotkeyBehavior enum/logic.
- [ ] Build settings.toml loader.
- [ ] Long-hold VAD integration + E2E for splits.
- [ ] TUI runtime toggles.
- [ ] Defer parallel until dual-engine need.

*End of Proposal*