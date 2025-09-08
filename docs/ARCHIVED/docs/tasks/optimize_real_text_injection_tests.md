# Task: Optimize and Consolidate Real Text Injection Tests

Status: In Progress
Owner: (assign)
Created: 2025-09-07
Last Updated: 2025-09-07
Target Milestone: Short-term (1–2 iterations)

## 1. Purpose & Goals
Improve performance, reliability, and developer usability of the `real-injection-tests` suite in `coldvox-text-injection` so it becomes:
- Fast enough for an optional pre-commit "smoke" run (< ~2s warm path)
- Stable (low flake rate) across local desktops and CI (Xvfb)
- Scalable for future backends
- Observable (timing + outcomes easily inspected)

## 2. Current State (Summary)
File: `crates/coldvox-text-injection/src/tests/real_injection.rs`
Characteristics:
- ~16 independent async tests (AT-SPI, clipboard, ydotool, Enigo variants)
- Each launches its own GTK test app process + fixed sleeps (200–500 ms)
- Verification by polling a /tmp file up to 500 ms (50 ms intervals)
- Conservative per-method timeouts (250 ms general, 200 ms paste)
- High cumulative wall time (≈9–11 s serial) dominated by fixed sleeps & process spawning
- Feature-gated (`real-injection-tests` + backend features)
- Skips if no display / Wayland / X11

## 3. Pain Points
| Issue | Impact |
|-------|--------|
| Fixed 500 ms sleeps | Wastes time; dominates runtime even when backend ready fast |
| Process spawn per test | Startup overhead (GTK map + AT-SPI bus) repeated |
| Test multiplicity (simple/unicode/long/special) | Redundant setup cost |
| Lack of adaptive timeouts | Warm path not exploited |
| Potential focus/clipboard race under parallel test harness | Flakiness risk |
| Real tests unusable in pre-commit due to cost | Reduced developer feedback |
| Silent skips (no display) reduce perceived coverage | Coverage ambiguity |

## 4. High-Level Strategy
1. Consolidate into a single **parameterized smoke test** reusing one GTK app.
2. Introduce adaptive (cold vs warm) backend timeouts.
3. Replace fixed sleeps with readiness polling (window map, file touch, AT-SPI object).
4. Shorten verification window (max 200 ms, exponential interval schedule).
5. Introduce a lightweight pre-commit optional run (non-blocking, env-gated).
6. Instrument phases with `tracing` spans and summary metrics.
7. Split "smoke" vs "full" (long text & stress) via feature or env flag.

## 5. Detailed Changes
### 5.1 Test Consolidation
- New file: `real_injection_smoke.rs` (or refactor existing) with a single `#[tokio::test]`.
- Table-driven cases: `(backend_kind, text_variant, expected_mode)`.
- Fail early with contextual assertion messages.
- Retain long-text stress case only under `INJECTION_STRESS=1` or feature `real-injection-full`.

### 5.2 Backend Invocation Strategy (Initial Implementation)
Phase 1 (this iteration): spawn a fresh lightweight GTK test app per backend in the smoke test (keeps logic simple; still fast with adaptive waits).
Phase 2 (optional later): introduce persistent `GtkAppSession` once stability proven.

### 5.3 Readiness Polling
Replace fixed sleeps with a bounded polling loop (interval schedule: 15,30,45,60,90,120,160 ms; cap ≤ 180 ms) for app readiness + file verification.

### 5.4 Adaptive Timeouts
Use simple per-backend first-use tracking in the smoke test (cold: 400 ms inject / 400 ms verify; warm: 120 ms inject / 200 ms verify). No production change yet.

### 5.5 Verification Optimization
- `verify_injection_fast(path, expected)`:
  - Poll schedule: 30ms, 60ms, 100ms, 160ms (stop on success) — total cap ≤ 200ms.
  - Immediate read attempt first (optimistic path).
  - On failure include last observed content snippet in error.

### 5.6 Instrumentation
- Add span wrappers: `spawn_app`, `backend=ATSPI inject`, `verify`, `total_case`.
- End-of-test summary log (INFO): JSON line with per-backend timings & pass/fail counts.

### 5.7 Pre-Commit Optional Integration
- Update `.git-hooks/pre-commit-injection-tests` (or new step) to:
  - Always run mock tests (blocking).
  - If `DISPLAY || WAYLAND_DISPLAY` and `RUN_REAL_INJECTION_SMOKE=1`: run smoke test via:
    `cargo test -p coldvox-text-injection --features real-injection-tests -- real_injection_smoke --test-threads=1 --quiet`
  - Non-zero exit only if mock suite fails; real smoke failures print warning unless `ENFORCE_REAL_SMOKE=1`.

### 5.8 Full Suite Retention
- Keep original comprehensive scenarios behind env flag / separate feature:
  - `cargo test -p coldvox-text-injection --features real-injection-tests,real-injection-full`
- Document difference in `TESTING.md`.

### 5.9 Configuration / Env Flags
| Variable | Purpose | Default |
|----------|---------|---------|
| RUN_REAL_INJECTION_SMOKE | Enable smoke test in hooks | unset (off) |
| ENFORCE_REAL_SMOKE | Fail commit on smoke failure | unset (warn only) |
| INJECTION_STRESS | Include long-text heavy case | unset (skip) |

### 5.10 Guardrails
- Test forced serial: `--test-threads=1` to avoid focus races.
- ydotool absence: skip that case with a logged notice (not failure).
- Missing feature/backends: case auto-skips.

## 6. Implementation Tasks (Revised)
Legend Status: NS = Not Started, IP = In Progress, BL = Blocked, DN = Done

| # | Task | Status | Est | Notes |
|---|------|--------|-----|-------|
| 0 | Capture baseline timing of current real suite (single warm run) | DN | 0.1d | Manual observation (~9–11s) listed above |
| 1 | Draft smoke test skeleton file (`real_injection_smoke.rs`) (compiles; cases maybe skipped) | NS | 0.25d | Ensure feature gating & serial execution |
| 2 | Introduce readiness polling helper (window/app, file verification) | NS | 0.25d | Replace fixed sleeps (Section 5.3) |
| 3 | Implement adaptive timeout bookkeeping (cold vs warm) | NS | 0.25d | Local HashMap/backend flag |
| 4 | Add fast verification function (`verify_injection_fast`) | NS | 0.1d | Poll schedule 30/60/100/160 ms |
| 5 | Instrument with tracing spans + per-backend timing summary log | NS | 0.25d | JSON line at INFO end-of-test |
| 6 | Fix injector naming / import mismatches (`AtSpiInjector` vs `AtspiInjector`) & missing trait imports | NS | 0.1d | Observed during attempted run 2025-09-07 |
| 7 | Add optional pre-commit hook step (env gated) | NS | 0.25d | `RUN_REAL_INJECTION_SMOKE` / warning-only mode |
| 8 | Update docs (`TESTING.md`, this file) clarifying smoke vs full | NS | 0.2d | Add usage matrix |
| 9 | Introduce `GtkAppSession` for backend reuse (Phase 2 optimization) | NS | 0.5d | After baseline smoke stabilized |
| 10 | Gate / prune legacy per-variant tests under `real-injection-full` | NS | 0.2d | After confidence (≥1–2 weeks) |
| 11 | Record post-Phase 1 metrics & compare delta | NS | 0.1d | Include warm + cold durations |
| 12 | (Stretch) Evaluate moving adaptive timeouts into library proper | NS | 0.25d | Decide based on data |

## 7. Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Flaky readiness detection on slow systems | Keep one generous cold attempt (250ms); log explicit readiness failure cause |
| Single shared GTK app crashes mid-run | Fallback spawn new instance once; mark retry in metrics |
| CI variance with Xvfb | Add explicit window manager readiness polling (wmctrl check) |
| Hidden regression in original granular tests | Temporarily retain old tests under `real-injection-full` until confidence builds |
| Increased complexity | Contain logic inside `tests/support/` module to isolate complexity |

## 8. Success Metrics (Targets)
| Metric | Baseline (2025-09-07) | Target |
|--------|------------------------|--------|
| Smoke test warm runtime | ~9–11s (legacy multi-test) | <3s Phase 1 (fresh app/ adaptive waits); <2s Phase 2 (session reuse) |
| First backend cold latency (inject+verify) | ~500–700ms (due to sleeps) | ≤400ms cold / ≤200ms warm |
| Flake rate (10 consecutive runs) | Unmeasured | 0 failures |
| Pre-commit added wall time (opt-in) | N/A | <2.5s |
| Backend coverage in smoke | 4 backends planned | All available; skip w/ explicit log if absent |
| Logging observability | Minimal timings | JSON summary + per-span timings present |

## 9. Acceptance Criteria
- `cargo test -p coldvox-text-injection --features real-injection-tests -- real_injection_smoke` warm run <3s Phase 1; <2s after session reuse (Phase 2).
- Cold first backend path ≤400ms (inject+verify) and subsequent backend paths ≤200ms each under typical dev environment.
- Optional pre-commit path documented and functional (warning-only unless `ENFORCE_REAL_SMOKE=1`).
- Skips produce clear, single-line INFO/WARN logs naming backend & reason.
- Documentation updated (TESTING.md + this task file) distinguishing smoke vs full.
- Legacy granular tests gated (not removed) until 14 days of stable smoke runs.
- No regressions in existing mock test suite.
- PR includes baseline vs post-change timing evidence (raw numbers + % delta).

## 10. Rollout Plan
1. Add new smoke test (no removal yet).
2. Validate locally + CI (collect timings).
3. Enable optional hook flag docs.
4. After 1–2 weeks stable: decide whether to remove or gate legacy per-variant tests.
5. Consider migrating adaptive timeout logic into production config if beneficial.

## 11. Follow-Up / Stretch
- Replace file-based verification with AT-SPI text read (stronger semantic check) where available.
- Unix domain socket IPC for richer app control (clear buffer, multi-field states).
- Metrics export (JSON) consumed by a lightweight dashboard.
- Add Wayland-native injection backend (future) and extend smoke matrix.

## 12. Open Questions
- Should adaptive timeouts migrate into library runtime (beyond tests)?
- Is ydotool coverage essential in smoke if absent on many dev machines?
- Keep long text stress? Gate behind nightly-only? (e.g., `INJECTION_STRESS` in scheduled CI).
- Accept single JSON summary line vs structured artifact file? (Scope creep risk)

## 13. Recent Findings (2025-09-07)
- Attempted running nascent smoke test revealed naming mismatch: code expects `AtSpiInjector` (capital S) vs earlier assumption `AtspiInjector`.
- Missing trait import surfaced for `YdotoolInjector::is_available`; ensure all availability checks are unified in a helper (planned Task 6).
- Baseline timing captured informally; formal measurement task (Task 11) will record cold/warm splits after instrumentation.
- No smoke test file committed yet; Task 1 remains Not Started despite earlier draft assumption (document corrected).
- Need to confirm feature flag combinations for minimal smoke (likely: `real-injection-tests,atspi,wl_clipboard,enigo` plus optional `ydotool`).

## 14. Failure Cause Taxonomy Integration
Source insight (external matrix) maps common failure causes → which injection strategies remain viable. We will use this to (a) prioritize smoke test assertions, (b) drive instrumentation categories, and (c) justify retaining AT-SPI direct insertion until empirical data suggests otherwise.

| Failure Cause | AT-SPI Insert | AT-SPI Action Paste | Clipboard+Ctrl+V | Clipboard+ydotool | Per-Key Typing | Test/Instrumentation Implication |
|---------------|---------------|---------------------|------------------|-------------------|----------------|-------------------------------|
| No accessibility bus | Affected | Affected | Viable | Viable | Viable | Detect early (availability probe) → classify as `no_bus`; ensure fallback attempts happen quickly |
| EditableText iface absent | Affected | Affected | Viable | Viable | Viable | Log `no_iface`; measure frequency to decide future removal of direct insert |
| Focus transient / not stabilized | Viable (after retry) | Viable | Affected (paste may target wrong) | Affected | Affected | Add one fast retry & record `focus_transient` when first lookup empty, second succeeds |
| Clipboard write denied / sandboxed | Viable | Viable | Affected | Affected | Viable | Classify clipboard failure (`clipboard_denied`) and confirm keystroke fallback path exercised |
| IME active composition interfering | Viable | Viable | Affected | Affected | Affected | Hard to simulate headless; flag as `ime_interference` only if specific error surfaces (low priority) |
| uinput access denied | Viable | Viable | Viable | Affected | Affected (if keys blocked) | Distinguish ydotool availability vs permission error (`uinput_denied`) |
| Key synthesis blocked (Wayland security) | Viable | Viable | Viable | Viable (if via uinput) | Affected | Capture failed keystroke injection with Wayland session detection → `keys_blocked` |
| Password / secure entry protections | Viable? (depends) | Viable? | Affected | Viable | Viable | Out-of-scope for smoke; potential future secure-field classification |
| Selection replaced unexpectedly | Viable | Viable | Affected | Affected | Viable | Track mismatched final content length vs expected (`selection_race`) |

Rationale:
- Empirical retention rule: keep AT-SPI Insert while `no_iface` + `no_bus` combined < (successful AT-SPI insert + unique rescue cases *threshold). Initial threshold: ≥3–5% unique rescues vs next fallback.
- Instrumentation must emit counters per failure reason to enable a 2‑week sampling experiment.

### 14.1 New Instrumentation Requirements
- Per attempt JSON (TRACE or DEBUG gated by feature flag) with fields: `{backend, method_variant, chars, result, failure_reason(optional), latency_ms}`.
- Aggregated summary at end of smoke test: `{attempts, successes, failures_by_reason{...}, rescued_by_fallback{method->count}}`.
- Environment annotation: `{session: wayland|x11|unknown}` to correlate key synthesis blocks.

### 14.2 Test Plan Adjustments
- Add synthetic negative tests (under `real-injection-full` or mocked command layer) to force: clipboard denial (mock), ydotool not present, AT-SPI unavailable (feature off), and ensure classification does not panic.
- Smoke test remains positive-path only; classification exercised opportunistically (e.g., missing ydotool).

### 14.3 Decision Milestone
After instrumentation live for ≥ 2 weeks (or N≥500 real attempts locally + CI), re-evaluate removing or gating AT-SPI direct insertion.

## 15. Task Additions (Failure Taxonomy)
| # | Task | Status | Est | Notes |
|---|------|--------|-----|-------|
| 13 | Add per-attempt failure reason enum + logging | NS | 0.25d | Extend existing metrics or new lightweight struct |
| 14 | Aggregated summary including failure buckets | NS | 0.1d | Printed at INFO end-of-smoke |
| 15 | Mockable layers / test hooks to simulate failure causes | NS | 0.5d | Enables deterministic classification tests |
| 16 | Documentation section: failure reasons & interpretation | NS | 0.15d | Update `TESTING.md` |
| 17 | 2-week data collection review checkpoint | NS | 0.05d | Calendar reminder / issue ticket |

Success Metrics additions:
- Failure classification coverage: ≥90% of failures tagged with non-generic reason.
- Unique rescues (earlier fail, later success) tracked per backend.
- Decision readiness metric: enough samples (≥500) or early convergence if AT-SPI rescue count ≪ threshold.

## 16. Research Synthesis & Strategy Ordering (2025-09-07)
External research (113 sources) indicates AT-SPI direct insertion offers unique rescue value (~15–20% scenarios where synthetic & clipboard paths fail) and future-proofs Wayland security changes. We incorporate these conclusions as provisional hypotheses to be validated via our telemetry.

### 16.1 Provisional Injection Ordering (Runtime Logic Hypothesis)
1. atspi_insert (EditableText) – skip if `no_bus` OR `no_iface` previously recorded for app class
2. clipboard_native_paste (AT-SPI Action or platform paste) – skip if `clipboard_denied`
3. clipboard_ydotool (uinput path) – skip if `uinput_denied`
4. per_key_typing (synthetic keystrokes) – final fallback; avoid for >500 chars unless all others failed

### 16.2 Dynamic Adaptation Rules
- Maintain per (app_class, method) ring buffer of last N outcomes (default N=50) to calculate rolling success rate.
- Demote a method for that app_class if success rate <10% and another method succeeds >80% within same window.
- Promote previously demoted method after cooldown if system environment changes (session switch or version bump hint).

### 16.3 Telemetry Fields (Refined)
Extend Section 14 schema with:
`rescue_chain_index` (u8) – position at which success occurred; `unique_rescue=true` if index>0 and all prior failed.
`app_class` derived via heuristic regex mapping (gtk|qt|electron|terminal|browser|sandbox|other).
`session` (wayland|x11|unknown).

### 16.4 Decision Thresholds (Confirm / Adjust)
- Retain AT-SPI direct if unique rescue rate ≥5% overall OR ≥10% for any high-usage app_class.
- Consider gating (disabled by default) if overall unique rescue <2% for 4 consecutive weeks and no security/Wayland policy regression flagged.

### 16.5 Smoke Test Scope Alignment
Smoke test remains minimalist: only verifies that each enabled backend can reach success path; it will not artificially simulate all failure classes—those are covered in classification tests & runtime telemetry.

### 16.6 Risk Notes
- Reported success percentages are currently qualitative; we treat them as priors, not authoritative metrics.
- Potential bias: community reports skew toward failure cases; actual unique rescue rate could be lower.

### 16.7 Immediate Adjustments
- Add placeholder adaptive ordering scaffold behind feature flag `adaptive-ordering`.
- Provide a no-op metrics sink when feature disabled (minimal overhead).

## 17. Additional Tasks (Adaptive Strategy & Telemetry)
| # | Task | Status | Est | Notes |
|---|------|--------|-----|-------|
| 18 | Implement rolling success cache (per app_class/method) | NS | 0.4d | Ring buffer or fixed-size vec |
| 19 | Add adaptive ordering feature flag (`adaptive-ordering`) | NS | 0.15d | Compile-time gate for experimental logic |
| 20 | Emit `unique_rescue` counters & chain index | NS | 0.15d | Extend summary output |
| 21 | Heuristic app_class classifier (WM_CLASS regex) | NS | 0.25d | Reusable util module |
| 22 | Weekly telemetry evaluation script stub (JSON → summary) | NS | 0.3d | Optional Python or Rust bin |

Success Metric additions:
- Adaptive ordering reduces median rescue chain length by ≥1 step after stabilization.
- Overhead of telemetry instrumentation <1% added wall time in smoke test (measured).

---
**Next Action:** Start Task 1 (draft compiling smoke test skeleton) then Task 2 (readiness polling helper). Assign owner and schedule.
