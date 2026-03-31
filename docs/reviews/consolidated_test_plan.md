# Consolidated Test Improvement Plan

**Status:** Approved compromise between original audit and rebuttal  
**Date:** 2026-03-31  
**Author:** Kimi (incorporating review feedback)

---

## Guiding Principle

> Tests should be **behaviorally resilient**, not **structurally fragile**.
>
> — From `test_addendum.md`

A test's value is determined by its **resilience to refactoring**:
- **Keep:** Only breaks when *outcome* is wrong (behavioral contract)
- **Remove:** Breaks when *shape* changes (structural fragility)

---

## Phase 1: Re-grading (Correcting Misclassifications)

### Promote to A-Grade (Keep & Protect)

| Test | Was | Now | Why |
|------|-----|-----|-----|
| `f32_to_i16_basic`, etc. | F | **A** | Silent failure protection for audio quality |
| `test_settings_validate_zero_timeout` | F | **A** | System constraint, not implementation detail |
| `test_noop_inject_success` | F | **A** | Liveness guarantee in hostile OS environments |
| `test_environment_info_new` | F | **F** | **REJECTED PROMOTION:** Testing no-logic constructors is testing the compiler. |
| WER metric tests | F | **B** | Infrastructure correctness matters |

### 🛑 Rebuttal 1: The "Schema Stability" Fallacy

The initial draft of this plan attempted to save `test_environment_info_new` and Telemetry creation tests by giving them a "B-Grade" to protect "Schema stability." **This justification is rejected.**

Testing "schema stability" by manually creating a struct and asserting its fields match the constructor arguments is testing the Rust compiler. If the struct schema changes (e.g., a field is added), the compiler automatically fails the build anywhere that struct is instantiated. We do not need a brittle unit test to act as a redundant compiler check. **If a test does not assert a behavioral invariant, it is an F-Grade tautology and must be deleted.**

### 🛑 Rebuttal 2: The "Sanity Canary" Fallacy

The plan proposed saving "Default value tests" and "Config canaries" by sweeping them into a `mod sanity` in each crate. **This is rejected.** Grouping tautologies into a single file just creates an organized trash bin. A test that asserts `config.timeout == 800` is a change-detector that provides negative value during refactoring. They must all be deleted (Grade F).

### Consolidate to B-Grade (Keep but Organize)

| Category | Action |
|----------|--------|
| Timeout wrapper tests | Keep 1-2 representative tests, remove duplicates |

*(Note: "Default value tests" and "Config canary tests" have been entirely rejected. Grouping tautologies into a single module does not make them useful.)*

---

## Phase 2: Elimination (True F-Grade Removal)

### Remove These (No Behavioral Value)

| Target | Count | Rationale |
|--------|-------|-----------|
| `test_word_info_creation` | ~15 | Testing that struct stores what you give it (language guarantee) |
| `test_transcription_config_default` | ~12 | Hardcoded constant mirrors implementation |
| Direct duplicates in `main.rs` | ~8 | Same tests exist in `tests/settings_test.rs` |
| Placeholder "Needs feature flag" tests | ~6 | No actual logic to test |
| Placeholder WER tests | ~10 | Empty or trivial assertions |

**Estimated removal:** ~70 tests (down from 118)

---

## Phase 3: Transformation (D-Grade Rework)

### Runtime Detection Pattern

Replace feature-gating with runtime detection:

```rust
// Before (compile-time gating)
#[cfg(feature = "live-hardware-tests")]
#[test]
fn test_real_microphone() { ... }

// After (runtime detection)
#[test]
#[ignore = "requires hardware microphone"]
fn test_real_microphone() {
    if !is_audio_available() {
        return; // Silent skip
    }
    // actual test
}
```

**Crates affected:**
- `coldvox-audio` — 18 tests
- `coldvox-text-injection` — 12 tests

### Strengthen Weak Assertions

| Test Type | Before | After |
|-----------|--------|-------|
| "No panic" tests | `let _ = func();` | Assert on actual return values |
| "Exists" tests | `assert!(path.exists());` | Assert on content checksum |
| "Returns Some" | `assert!(result.is_some());` | Assert on inner value properties |

---

## Phase 4: Integration Excellence (A-Grade Expansion)

### Golden Master Expansion

Extend `test_short_phrase_pipeline` to cover:
- Different sample rates (8kHz, 16kHz, 44.1kHz)
- Noisy backgrounds (SNR: 20dB, 10dB, 5dB)
- Different accents/phrases

### CI Environment Parity

Improve "Real Injection" tests:
- Provide mock X11/Wayland environment in CI (`xvfb-run`)
- Graduate from "D-grade (ignored)" to "A-grade (verified in CI)"

---

## Execution Order

| Phase | Tasks | Est. Time |
|-------|-------|-----------|
| 1 | Re-grade tests, update documentation | 30 min |
| 2 | Remove true F-grade tests (~70) | 1 hour |
| 3a | Implement `RuntimeSkip` trait in `coldvox-foundation` | 45 min |
| 3b | Transform D-grade tests (runtime detection) | 1.5 hours |
| 3c | Strengthen weak assertions | 1 hour |
| 4 | Expand golden masters, CI improvements | 2 hours |

**Total:** ~6.5 hours of focused work

---

## Success Metrics

| Metric | Before | Target |
|--------|--------|--------|
| Total tests | 389 | 320 |
| Test execution time | ~45s | ~30s |
| Tests requiring hardware | 30 (gated) | 30 (ignored by default) |
| Behavioral coverage | Baseline | Maintain + 10% |
| CI pass rate | 97% | 99%+ |

---

## File Changes Summary

### New Files
- `crates/coldvox-foundation/src/testing/runtime_skip.rs` — Runtime detection trait

### Modified Files
- `crates/app/src/lib.rs` — Remove duplicate settings tests
- `crates/coldvox-audio/src/capture.rs` — Re-grade math tests
- `crates/coldvox-audio/src/resampler.rs` — Re-grade conversion tests
- `crates/coldvox-text-injection/src/lib.rs` — Runtime detection for injection tests
- `.github/workflows/ci.yml` — Add X11 mock environment

### Deleted Files
- ~70 F-grade tests (identified in Phase 2)

---

## Appendix: Re-grading Checklist

### ColdVox-Audio (89 tests → ~75 tests)
- [ ] Keep: `f32_to_i16_basic`, `i16_to_f32_basic`, `f32_to_i16_clamp` → A
- [ ] Keep: `test_resampler_quality` → A  
- [x] Remove: `test_audio_config_default*` → F (Rejected Sanity Canary fallacy)
- [ ] Remove: `test_audio_buffer_creation` → F (language guarantee)

### ColdVox-App (112 tests → ~85 tests)
- [ ] Keep: `test_settings_validate_zero_timeout` → A
- [ ] Remove: Duplicates in `main.rs` that mirror `tests/settings_test.rs` → F
- [x] Remove: Config canaries → F (Rejected Sanity Canary fallacy)
- [ ] Transform: Feature-gated tests → runtime detection → D→B

### ColdVox-Text-Injection (45 tests → ~38 tests)
- [ ] Keep: `test_noop_inject_success` → A
- [ ] Transform: `test_real_injection_*` → runtime detection → D→B
- [ ] Strengthen: Weak assertions on injection results

### ColdVox-Foundation (67 tests → ~55 tests)
- [x] Remove: `test_environment_info_new` → F (Rejected "schema stability" fallacy - ALREADY DELETED)
- [ ] Remove: Placeholder WER tests → F
- [ ] Consolidate: Timeout wrapper tests → 2 representative tests → B

### ColdVox-Telemetry (34 tests → ~30 tests)
- [x] Remove: Creation/schema tests → F (Testing struct creation is a compiler test. *Serialization* tests to verify JSON wire formats are A-grade, but pure struct creation is F-grade).
- [ ] Remove: `test_telemetry_event_creation` → F (language guarantee)

### ColdVox-STT (42 tests → ~37 tests)
- [ ] Keep: Pipeline integration tests → A
- [ ] Transform: Hardware-dependent tests → runtime detection → D→B

---

## References

- Original audit: Previous conversation
- Rebuttal: `docs/reviews/test_critique_report.md`
- Framework: `docs/reviews/test_addendum.md`

---

*This plan represents consensus between the original audit's rigor and the rebuttal's emphasis on behavioral resilience.*