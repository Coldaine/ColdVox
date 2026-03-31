# Test Review Critique & Iteration Plan

## Executive Summary
This report evaluates the "COMPREHENSIVE TEST REVIEW REPORT" provided earlier. While that report correctly identifies significant duplication and opportunities for better integration testing, it suffers from a strong **Integration Bias** that undervalues the "Sanity Check" and "Unit Logic" layers of the testing pyramid.

## Critique of Previous Report

### Where the Reviewer Anchored Too Hard (Weaknesses)
1. **The "Tautology" Trap:** The previous reviewer labeled basic unit tests (e.g., math conversions in `capture.rs`) as "F-grade". This is a mistake. 
   - *Why:* Integration tests with real audio hardware are non-deterministic and "noisy". If a rounding error is introduced in sample conversion, a unit test catches it in 1ms; an integration test might show a slightly higher noise floor that is hard to debug.
2. **Undervaluing Sanity Checks:** Tests like `test_transcription_config_default` were marked for removal. 
   - *Why:* These are "canaries". If a developer accidentally changes a global default that affects production behavior, these tests catch it instantly. They cost almost nothing to maintain.
3. **Infrastructure as Noise:** Marking test harness tests (WER metrics, timeout wrappers) as "F-grade" is risky. If the tools we use to measure success are broken, the "A-grade" results are meaningless.

### Where the Reviewer was Spot On (Strengths)
1. **Duplication:** The identification of duplicate tests in `coldvox-app` (settings) and `coldvox-audio` is accurate and should be acted upon immediately.
2. **Feature Gate Bitrot:** The reviewer correctly identified that gating tests behind `live-hardware-tests` causes them to rot in CI. Moving to runtime detection + `#[ignore]` is the correct path for modern Rust projects.
3. **Assertion Quality:** Many "D-grade" tests indeed have weak assertions (only checking "no panic"). These should be strengthened or consolidated.

---

## Proposed Action Plan

### 1. Retention & Re-classification (The "B-Grade" Recovery)
Instead of removing "F-grade" sanity checks, we will:
- **Consolidate:** Group "Canary" tests into a single `mod sanity` in each crate to reduce file-level noise.
- **Retain Math:** Keep all sample-conversion and math-logic unit tests. These are the foundation of audio reliability.

### 2. Elimination (The "True F-Grade" Cleanup)
We will remove the following:
- **Direct Duplicates:** `tests/settings_test.rs` in `coldvox-app` will be the single source of truth; duplicates in `main.rs` will be removed.
- **Redundant WER tests:** Consolidate WER testing into `coldvox-app/src/stt/tests/wer_utils.rs`.
- **Placeholders:** Remove the "Needs feature flag" placeholder tests in `coldvox-audio`.

### 3. Transformation (The "D-Grade" Rework)
We will transition from **Compile-time Gating** to **Runtime Detection**:
- **Pattern:** Use a helper like `is_hardware_available()` to skip tests at runtime.
- **Reporting:** Use `#[ignore = "Reason"]` so that `cargo test` shows a clear tally of skipped hardware tests without failing the CI.
- **Crates affected:** `coldvox-audio`, `coldvox-text-injection`.

### 4. Integration Excellence (The "A-Grade" Expansion)
- **Golden Masters:** Expand the use of `test_short_phrase_pipeline` to cover more edge cases (different sample rates, noisy backgrounds).
- **Environment Parity:** Improve the "Real Injection" tests by providing a mock X11/Wayland environment in CI (e.g., `xvfb-run`) so they can graduate from "D-grade (ignored)" to "A-grade (verified in CI)".

---

## Detailed Proposed Actions per Crate

| Crate | Action | Priority |
| :--- | :--- | :--- |
| **coldvox-app** | Remove duplicate settings tests; keep config sanity checks. | High |
| **coldvox-audio** | Move conversion tests to a submodule; implement `is_audio_available` skip. | Medium |
| **coldvox-text-injection** | Consolidate "Real Injection" tests; improve Wayland/X11 detection. | High |
| **coldvox-foundation** | Strengthen environment detection assertions. | Low |
| **coldvox-telemetry** | Keep creation tests (they verify schema stability). | Low |

## Next Steps
1. **Turn 1:** Consolidate settings tests in `coldvox-app`.
2. **Turn 2:** Implement the `RuntimeSkip` trait/macro in `coldvox-foundation`.
3. **Turn 3:** Batch update hardware-dependent tests to use the new skip logic.
