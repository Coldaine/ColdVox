# ColdVox STT Unification Final Plan

> Document Type: Final Refactor & Architecture Alignment Plan  
> Audience: Internal (Alpha)  
> Status: Ready to Execute (Phase 1)  
> Repository: `ColdVox` (branch: `main`)  
> Authored By: GitHub Copilot AI Assistant (acting as architectural pair)  
> Date: 2025-09-15

---
## 0. Context & Purpose
This document consolidates two parallel efforts:
1. A pragmatic unification plan focusing on evolving the existing `PluginSttProcessor` and removing redundant streaming code.
2. A conceptual architecture model reframing concerns as **Activation** (how utterances start/end) and **Presentation** (how transcription events are surfaced) instead of legacy "batch vs streaming" dichotomy.

The goal is a single, lean STT processor that supports both **Hotkey (Push-to-Talk)** and **VAD (Ambient)** workflows while keeping future enhancements (delayed VAD for long holds, segmentation, hybrid buffering, multi-engine strategies) cleanly extensible.

---
## 1. Repository Status Snapshot
- Branch: `main`
- STT Engine: Vosk (feature gated; partially incremental behavior)
- Current Duplication: Separate streaming processor artifacts + branching in `runtime.rs`
- Activation Modes: Hotkey & VAD already conceptually present
- Missing Feature: True batch-on-release push-to-talk & long-hold segmentation
- Tests: End-to-end WAV test(s) present; partial gating not formalized

---
## 2. Core Conclusions
- Two processor implementations offer no material functional differentiation.
- Activation boundaries and partial emission policy are orthogonal; unify under one control surface.
- Future complexity (segmentation, hybrid, multi-engine) must not force another rewrite—design enum-driven extensibility now, implement later.

---
## 3. Design Pillars
| Pillar | Description | Outcome |
|--------|-------------|---------|
| Single Processor | One `UnifiedSttProcessor` evolves from existing plugin processor | Less code, simpler reasoning |
| Activation Abstraction | Session events normalize VAD & Hotkey | Decoupled boundary logic |
| Presentation Policy | Partial vs final treated as UI/event layer concern | Flexible emission control |
| Deferred Complexity | Long-hold segmentation & buffering stubbed, not built | Avoid premature bloat |
| Extensible Config | Settings file + env + CLI precedence | Future toggles without rewrites |

---
## 4. Session & State Model
### Session Events
```rust
enum SessionSource { Vad, Hotkey }

enum SessionEvent {
    Start(SessionSource, std::time::Instant),
    End(SessionSource, std::time::Instant),
    Abort(SessionSource, &'static str),
    // (future) SegmentSplit(SessionSource, std::time::Instant),
}
```
### Processor States
- Idle
- ActiveHotkey
- ActiveVad
- (Future) ActiveHotkeyLongHold

### Transitions
| From | Event | To | Side Effects |
|------|-------|----|--------------|
| Idle | Start(Hotkey) | ActiveHotkey | begin_utterance() |
| Idle | Start(Vad) | ActiveVad | begin_utterance() |
| ActiveHotkey | End | Idle | finalize_utterance() |
| ActiveVad | End | Idle | finalize_utterance() |
| Any Active | Abort | Idle | cancel + discard buffer |
| ActiveHotkey | LongHold threshold (future) | ActiveHotkeyLongHold | init secondary VAD |

---
## 5. Configuration (Phase 1 → Phase 2 Evolution)
### Phase 1 (Implement Now)
```rust
pub enum ActivationMode { Hotkey, Vad }
pub enum HotkeyBehavior { Incremental }
pub enum PartialPolicy { Emit }
#[derive(Clone, Debug)]
pub struct LongHoldStub { pub enabled: bool, pub min_hold_secs: u32, pub silence_split_secs: u32 }
#[derive(Clone, Debug)]
pub struct Settings { pub activation_mode: ActivationMode, pub hotkey_behavior: HotkeyBehavior, pub partial_policy: PartialPolicy, pub long_hold: LongHoldStub }
```
- Defaults: Hotkey + Incremental + Emit + long_hold disabled.
- Loader precedence: CLI → Env → `settings.toml` → defaults.

### Phase 2 (Planned Extensions)
- Add: `BatchOnRelease`, `Hybrid`, `PartialPolicy::Suppress | Throttle(Duration)`.
- Implement long-hold segmentation (lazy VAD activation) behind config.

---
## 6. Delayed VAD Justification (Hotkey Long-Hold)
| Benefit | Detail |
|---------|--------|
| Latency Avoidance | Skip model spin-up for short presses (<10s) |
| Resource Efficiency | No inference cycles for trivial commands |
| Clear Semantics | Key boundaries unambiguous without VAD race |
| Adaptive Complexity | Only escalate after threshold exceeded |
| Silence Gap Management | Internal segmentation for multi-minute narration |
| Jitter Reduction | Fewer real-time loops early in session |
| Thermal Headroom | Avoid constant inference on workstation |

---
## 7. Memory & Performance Notes
- i16 mono @16kHz = 32 KB/s → Even 10 min raw buffer < 20 MB.
- Incremental decode avoids single large decode spike and preserves responsiveness.
- Buffer ceiling (safety): 30 s soft limit (log + early finalize if ever reached unexpectedly).

---
## 8. Refactor Phases
### Phase 1 (Execution)
1. Introduce `session_event.rs` (or inline) & wire hotkey/VAD triggers.
2. Refactor `PluginSttProcessor` → `UnifiedSttProcessor` (in-place rename optional; minimize churn in public exports).
3. Remove: `streaming_processor.rs`, `streaming_adapter.rs`, related re-exports.
4. Remove `COLDVOX_STT_ARCH` branching & unused streaming config flags.
5. Add `Settings` + minimal loader (defaults only if loader too large for first patch).
6. Ensure partial events flow in both activation modes (policy = Emit).
7. Implement buffer ceiling guard (warn & defensive finalize if exceeded).
8. Update tests (simulate hotkey session events; reuse VAD path test). 

### Phase 2 (Enhancement)
- CLI + env overrides for activation & partial policy.
- Optional partial suppression / future throttling stub.
- Long-hold segmentation design scaffolding (unused until enabled).
- TUI display of active activation mode + partial policy.

### Phase 3 (Deferred)
- BatchOnRelease & Hybrid behaviors.
- Long-hold segmentation operational (secondary VAD injection).
- Partial throttling logic.

### Phase 4 (Future)
- Parallel engine arbitration (Fast + Accurate).
- GUI settings editor.
- Advanced metrics & adaptive policies.

---
## 9. Risks & Mitigations
| Risk | Impact | Mitigation |
|------|--------|------------|
| Transcript regression | Medium | Run existing end-to-end WAV test before/after; diff final outputs. |
| Missed partials after merge | Low | Default policy = Emit; add debug log on emission. |
| Race conditions in session transitions | Medium | Serialize through single session event channel; no direct multi-source state mutation. |
| Over-segmentation later | Low | Defer segmentation feature; explicit config gate. |
| Reintroduction of duplication | Medium | Architecture doc (this file) as acceptance guard for future PRs. |

---
## 10. Acceptance Criteria (Phase 1)
- Only one STT processing implementation in repo.
- No references to deprecated streaming processor artifacts.
- Hotkey + VAD both produce partial + final events via common path.
- Safety buffer guard reachable & logged when artificially forced in dev test.
- CI / tests pass unchanged aside from updated imports.

---
## 11. Implementation Sketch
```rust
// session_event.rs
enum SessionSource { Vad, Hotkey }
enum SessionEvent { Start(SessionSource, Instant), End(SessionSource, Instant), Abort(SessionSource, &'static str) }

// unified_stt_processor.rs (formerly processor.rs snippet)
match session_event { SessionEvent::Start(..) => begin_utterance(); SessionEvent::End(..) => finalize(); SessionEvent::Abort(..) => cancel(); }
if active { engine.process_frame(samples); if let Some(evt) = maybe_partial { emit_if_policy_allows(evt); } }
```

---
## 12. Telemetry Hooks (Optional in Phase 1)
| Metric | Purpose |
|--------|---------|
| stt_sessions_started | sanity trend |
| stt_partials_emitted | UI load visibility |
| stt_frames_per_session | performance baseline |
| stt_session_duration_ms | distribution analysis |

---
## 13. Out-of-Scope (Confirmed)
- Parallel engines
- GUI settings
- Diarization / multi-speaker segmentation
- Dynamic runtime mode switching (Phase 2+ only if needed)

---
## 14. Action Checklist (Phase 1 PR)
- [ ] Remove streaming processor files & references
- [ ] Add session event abstraction
- [ ] Refactor processor to unified path
- [ ] Insert buffer safety guard
- [ ] Keep partial emission default
- [ ] Add settings struct (loader minimal or TODO)
- [ ] Update tests; add hotkey simulation
- [ ] Validate end-to-end transcript parity

---
## 15. Rollback Plan
If unforeseen regression: revert single commit (unification PR) — no cascading dependency changes introduced. Streaming artifacts are non-referenced and easily restored from history.

---
## 16. Rationale Recap
This plan merges tactical reuse (no rewrite) with a future-proof abstraction boundary. It avoids speculative complexity while preventing another architectural reset when long-hold segmentation or hybrid buffering is requested.

---
## 17. Next Steps
On approval: execute Phase 1 steps in a single focused PR; measure allocation + log output pre/post; record diff summary in PR description referencing this plan.

---
*Prepared by GitHub Copilot AI Assistant as a consolidation of prior discussion and architectural intent.*
