# ColdVox STT Unification – Critique & Refactor Proposal

> Status: Draft (Internal Alpha)  
> Scope: Replace dual STT pipeline abstractions with a single activation-aware processor, introduce coherent configuration model, and defer long-hold segmentation until required.

---
## 1. Executive Summary
The current speech-to-text (STT) architecture carries unnecessary duplication ("plugin" vs "streaming" processors and parallel frame/event types) without delivering distinct functional value. The real differentiation needed by the product is *activation semantics* (Push-to-Talk vs Ambient VAD) and *output presentation policy* (emit incremental partials vs only final). These concerns do **not** require separate STT processor implementations.

This document critiques the existing unification plan, clarifies conceptual domains (activation vs processing vs presentation), and specifies a lean refactor path: a single `UnifiedSttProcessor` handling both activation modes with optional future extensions (delayed VAD for long holds, segmentation inside long push-to-talk sessions, multi-engine layering). The refactor prioritizes maintainability and incremental evolution while preserving future flexibility.

---
## 2. Current State (Observed Issues)
### 2.1 Structural Duplication
- Distinct processor variants (e.g., `PluginSttProcessor` vs `StreamingSttProcessor`) implement near-identical loops (receive audio → maybe gate via VAD → feed engine → propagate events).
- Redundant data representations: `AudioFrame` vs `StreamingAudioFrame`; `VadEvent` vs `StreamingVadEvent` (or similarly split types) increase conversion boilerplate and mental load.
- Environment branching (e.g., `COLDVOX_STT_ARCH`) introduces configuration complexity without measurable advantage *today*.

### 2.2 Conceptual Blurring
- "Batch" and "Streaming" are overloaded: operationally they map to *event filtering* (suppress partials) or *buffering*, but architecturally they spawned duplicated code paths.
- Activation mode (how utterance boundaries are defined) is orthogonal to engine feeding pattern, yet currently entangled with processor selection logic.

### 2.3 Latent Features Not Realized
- True "batch-on-release" push-to-talk (buffer entire hold, submit once) is not implemented; existing hotkey mode still streams frames.
- No dynamic long-hold segmentation (e.g., enabling VAD mid-press for multi-minute narration splitting).
- Partial emission control is not systematically formalized (ad-hoc based on chosen processor variant).

### 2.4 Maintenance & Evolution Costs
- Every additional STT engine integration would currently need to reconcile both pipelines.
- Debugging involves tracking which path was selected at runtime, slowing iteration.
- Harder to reason about state transitions (speech active, pending final flush, cancellation) across variants.

---
## 3. Clarified Domain Model
### 3.1 Core Dimensions (Orthogonal)
| Dimension | Options | Purpose |
|----------|---------|---------|
| Activation Source | `Vad` / `Hotkey` | Defines utterance boundaries. |
| Hotkey Behavior | `Incremental` / `BatchOnRelease` / (future) `Hybrid` | Controls when audio is fed to engine. |
| Partial Policy | `Emit` / `Suppress` / (future) `Throttle(d)` | Governs UI/event verbosity. |
| Long-Hold Segmentation | `Disabled` / (future) `Enabled{...}` | Splits very long push-to-talk sessions. |
| Engine Strategy (future) | `Single` / `Parallel (Fast+Accurate)` | Accuracy vs latency layering. |

### 3.2 What We Actually Need Now
- Activation: `Vad` and `Hotkey (Incremental)`.
- Partials: `Emit` in both modes (you confirmed partials are desired for future GUI feedback).
- No immediate need for: batch-on-release buffering, long-hold segmentation, or parallel engines.

---
## 4. Delayed VAD Activation Rationale (Hotkey Long-Hold Scenario)
Enabling VAD only after a prolonged hotkey press delivers:
1. **Latency Avoidance** – No model inference spin-up for short presses (<10s).  
2. **Resource Efficiency** – Skips ML compute for quick commands; preserves CPU/GPU cycles for core app tasks.  
3. **Simpler Semantics** – Short press boundaries unambiguously defined by key events (no premature VAD-end).  
4. **Adaptive Complexity** – Only escalate to VAD sophistication when user behavior (extended narration) justifies it.  
5. **Silence Gap Management** – Multi-minute monologues with long internal pauses can be segmented post-threshold to avoid giant contexts & delayed finals.  
6. **Stability & Jitter Reduction** – Fewer concurrent real-time loops early in a press lowers timing risk.  
7. **Thermal/Power Headroom** – Even on desktops, less always-on inference leaves headroom for future GPU/STT models.  

---
## 5. Partial vs Final Transcription (Definitions)
| Type | Emission Timing | Mutability | Use Case |
|------|-----------------|-----------|----------|
| Partial | While audio still flowing | Overwritten by later partials or final | Real-time UI feedback, responsiveness |
| Final | After utterance boundary (SpeechEnd / Hotkey release / forced flush) | Stable, authoritative | Logging, downstream actions, persistence |

Design Implication: Partial emission belongs to *presentation policy*, not engine architecture.

---
## 6. Proposed Unified Architecture
### 6.1 Core Component: `UnifiedSttProcessor`
Responsibilities:
- Consume continuous audio frames.
- Accept session boundary events from multiple *sources* mapped into a single `SessionEvent` channel.
- Manage utterance lifecycle (`begin_utterance`, `process_frame`, `finalize_utterance`).
- Apply partial policy filtering.
- (Future) Manage buffering modes & long-hold segmentation.

### 6.2 Event Model
```rust
enum SessionSource { Vad, Hotkey }

enum SessionEvent {
    Start(SessionSource, Instant),
    End(SessionSource, Instant),
    Abort(SessionSource, &'static str),
    // (future) SegmentSplit(SessionSource, Instant)
}
```

### 6.3 State Machine
| State | Entered By | Exited By | Notes |
|-------|------------|-----------|-------|
| Idle | Init / after finalize | Start | No active accumulation. |
| ActiveHotkey | Hotkey Start | End / Abort | Incremental feed (current scope). |
| ActiveVad | VAD SpeechStart | SpeechEnd / Abort | Standard streaming path. |
| (Future) ActiveHotkeyLongHold | Time threshold reached | SegmentSplit / End | Enables internal VAD splits. |

### 6.4 Audio Handling Paths (Current Scope)
- Hotkey Incremental: Frame arrives → if state ActiveHotkey → feed engine → handle partial → maybe emit.  
- VAD Mode: Frame arrives → if ActiveVad → feed similarly.  
- No buffering layer needed now (keep a minimal Vec reserved for future end-of-utterance smoothing or engine finalization).

### 6.5 Partial Policy (Phase 1)
```rust
enum PartialPolicy { Emit, Suppress /*, Throttle(Duration) future */ }
```
Logic: Drop partial events early if `Suppress`. Throttle logic can wrap emission timestamp check later.

---
## 7. Configuration Model (Initial Implementation)
### 7.1 Settings Structure
```rust
pub enum ActivationMode { Hotkey, Vad }

pub enum HotkeyBehavior { Incremental /*, BatchOnRelease, Hybrid */ }

pub enum PartialPolicy { Emit /*, Suppress, Throttle(Duration) */ }

#[derive(Debug, Clone)]
pub struct LongHoldStub { // future extension
    pub enabled: bool,        // default false
    pub min_hold_secs: u32,   // default 60
    pub silence_split_secs: u32, // default 6
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub activation_mode: ActivationMode,      // default: Hotkey
    pub hotkey_behavior: HotkeyBehavior,      // default: Incremental
    pub partial_policy: PartialPolicy,        // default: Emit
    pub long_hold: LongHoldStub,              // default: disabled
}
```

### 7.2 Resolution Precedence
1. CLI flags (if provided)  
2. Environment variables (`COLDVOX_ACTIVATION_MODE`, `COLDVOX_PARTIAL_POLICY`, etc.)  
3. Config file (`settings.toml`: search CLI path → XDG config → local project)  
4. Hardcoded defaults  

### 7.3 Example `settings.toml`
```toml
activation_mode = "hotkey"
hotkey_behavior = "incremental"
partial_policy = "emit"
[long_hold]
enabled = false
min_hold_secs = 60
silence_split_secs = 6
```

---
## 8. Deferred Features (Design Stubs)
### 8.1 Long-Hold Segmentation
Trigger: Hotkey hold duration > `min_hold_secs`.  
Actions: Lazy-init secondary VAD with tuned config (e.g., faster silence detection).  
Splitting Rule: `silence_split_secs` of continuous silence ⇒ emit `SegmentSplit` → finalize current utterance → auto-start next.  
Safeguards: Minimum segment length (e.g., 10s) & failure fallback (disable if VAD init fails).  

### 8.2 Batch-On-Release Mode
- Buffer raw i16 samples (32 KB/s).  
- On release: feed buffer sequentially (or single aggregate call) → request final.  
- Pros: Simplicity, minimal engine churn.  
- Cons: Lost real-time partials, delayed feedback.  

### 8.3 Hybrid Hotkey Behavior
- Below threshold (e.g., 4–6 s): treat as BatchOnRelease.  
- Above threshold: switch to Incremental (start emitting partials) & optionally enable long-hold segmentation.  

### 8.4 Parallel Engine Strategy (Future)
- Fast engine (low accuracy) feeds partials for UI.  
- Accurate engine (higher latency) produces corrected final.  
- Arbitration merges events (final supersedes prior).  
Complexity intentionally deferred until a second engine is available.

---
## 9. Memory & Performance Considerations
| Aspect | Incremental (Chosen) | Batch-On-Release (Deferred) |
|--------|----------------------|-----------------------------|
| Latency | Low, continuous | Single spike at end |
| Memory | Constant (frame buffers) | Linear with duration (32 KB/s) |
| CPU per second | Evenly distributed | Idle then burst decode |
| UX Feedback | Real-time partials | None until final |

Large holds (5–10 min) remain RAM-cheap (<20 MB) but risk decode stall; incremental sidesteps this.

---
## 10. Refactor Plan (Phases)
### Phase 1 (Implement Now)
- Add `settings.rs` loader (TOML + env + CLI skeleton).
- Introduce `UnifiedSttProcessor` (reuse existing plugin processor internals; remove alternate streaming processor files & redundant frame/event types).
- Wire activation events from hotkey and VAD into unified `SessionEvent` channel.
- Implement partial emission (policy = Emit).  
- Remove `COLDVOX_STT_ARCH` branching.

### Phase 2 (Optional / Near-Term)
- Add PartialPolicy variants: `Suppress`, placeholder for `Throttle` (no logic yet).  
- Provide CLI flags for changing activation & partial policy at launch.
- TUI: display current activation mode & partial policy.

### Phase 3 (Deferred Until Need)
- Long-Hold Segmentation (delayed VAD injection).  
- Batch-On-Release & Hybrid behaviors.  
- Partial throttling implementation.  

### Phase 4 (Future Evolutions)
- Multi-engine orchestration.
- GUI settings panel (persisting changes live).  
- Metrics: per-mode latency & partial cadence instrumentation.

---
## 11. Risk & Mitigation Table
| Risk | Impact | Mitigation |
|------|--------|------------|
| Regression in final transcript handling | Medium | Keep end-to-end wav test; compare outputs pre/post refactor. |
| Unintended suppression of partials | Low | Default policy = Emit; add trace logs on drop path. |
| State race (start/end overlap) | Medium | Central session event channel with serialized handling. |
| Future feature creep reintroduces branching | Medium | Enforce extensibility via enum configs, not new processors. |
| Long-hold segmentation complexity | Low (deferred) | Stub interface now; implement only with real use case. |

---
## 12. Acceptance Criteria (Phase 1)
- Only one STT processor file in codebase (`UnifiedSttProcessor`).
- No references to `StreamingSttProcessor`, `StreamingAudioFrame`, or legacy STT arch env var.
- Hotkey incremental mode: partial + final events observed.
- VAD mode: partial + final events observed with same engine path.
- Configuration object loaded successfully (default path) and applied to processor initialization.
- Existing tests pass; new minimal test validates partial emission in both activation modes.

---
## 13. Minimal Implementation Sketch
```rust
// settings.rs
pub fn load_settings() -> Settings { /* parse precedence; fall back to defaults */ }

// unified_stt_processor.rs
pub struct UnifiedSttProcessor { /* channels + state */ }
impl UnifiedSttProcessor {
    async fn run(mut self) { /* select over audio + session events */ }
    async fn handle_session(&mut self, evt: SessionEvent) { /* start/end */ }
    async fn handle_audio(&mut self, frame: AudioFrame) { /* feed engine if active */ }
}
```

---
## 14. Tooling & Observability Enhancements (Optional)
- Add trace span: `stt.session` with attributes: source=hotkey/vad, id, start_ts.
- Metrics hooks: frames_per_session, avg_partial_interval, utterance_duration_histogram.
- Log gating decisions (e.g., future partial suppression) at `debug` level.

---
## 15. Out-of-Scope (Explicitly)
- GUI configuration panels.  
- Multi-device / multi-channel diarization.  
- Parallel STT engine arbitration.  
- Offline persistence of user-modified runtime settings (beyond static file + env).  

---
## 16. Summary
The proposed refactor eliminates structural duplication, clarifies conceptual axes (activation vs presentation), and sets a stable foundation for future sophistication (segmentation, hybrid buffering, parallel engines) without premature complexity. Phase 1 is intentionally modest: unify, configure, and preserve partials. All additional behaviors layer cleanly atop this base.

---
## 17. Next Steps Checklist (Actionable)
- [ ] Implement `settings.rs` and defaults.
- [ ] Replace dual processors with `UnifiedSttProcessor`.
- [ ] Excise legacy streaming types & env branches.
- [ ] Add partial verification test for Hotkey + VAD.
- [ ] Validate with existing WAV end-to-end test.
- [ ] Draft stubs / TODOs for long-hold segmentation in code comments.

---
*End of Document*
