---
doc_type: research
subsystem: general
status: final
freshness: current
preservation: reference
summary: Active triage plan for today's work
last_reviewed: 2026-02-09
owners: Maintainers
version: 0.1.0
---

# ColdVox Issue & PR Triage — 2026-02-09

Audit of all 64 open issues and 7 open PRs. Verified by two independent review passes
with cross-referencing of actual issue bodies via `gh api`.

**Summary**: Close 32 issues + 1 PR | Keep 32 issues | Merge 6 PRs

---

## CLOSE — 32 Issues + 1 PR

### Dead STT Backends (7 issues)

Whisper is removed, Parakeet is broken. Only Moonshine works.
These issues reference deleted files, non-existent migration plans, or dead backends.

| # | Title | Reason |
|---|-------|--------|
| #221 | Implement WhisperEngine API per stt-candle-whisper-migration plan | Whisper backend removed; entire migration plan is dead |
| #222 | Build benchmarking harness comparing legacy vs new Rust pipeline | No legacy Whisper pipeline exists to compare against |
| #223 | Research word-level timestamps using token-level heuristics | Body references `candle/timestamps.rs` (deleted), `whisper_plugin.rs` (deleted), and Candle examples. 100% Candle-specific despite generic title. Open a fresh backend-agnostic issue if needed |
| #224 | Update README.md removing Faster-Whisper references | Duplicate of #321 which has the same scope |
| #290 | Parakeet plugin missing Drop implementation for GPU cleanup | Parakeet backend broken (API drift with parakeet-rs 0.2); not fixable without rewrite |
| #323 | Research: word-level timestamps for STT output (DTW) | Duplicate of #223; same dead Candle/Whisper scope |
| #47 | Implement async processing for non-blocking STT operations | Vague single-paragraph stub from Sept 2025; superseded by #322 which has concrete scope |

### Duplicates (10 issues)

For each pair, the "Keep" column shows the better-scoped surviving issue or implementing PR.

| Close | Keep | Reason |
|-------|------|--------|
| #326 | #264 | "Convert disabled real-injection tests to mock-based" — #264 is older, same scope, tracked in #325 framework |
| #327 | #271 | "Input simulation test harness" — identical scope, #271 has more detail |
| #272 | #325 | "Cross-platform integration test suite" — #325 is the comprehensive framework issue that encompasses this |
| #265 | PR #311 | "Add unit tests for ydotool" — PR #311 implements this; merge the PR, close the issue |
| #266 | PR #312 | "Add unit tests for enigo/kdotool" — PR #312 implements this; merge the PR, close the issue |
| #40 | #325 | "Platform-specific text injection backend testing" — Sept 2025 stub superseded by #325's comprehensive plan |
| #317 | #320 | "Add code coverage job" — #320 has identical scope with more detail |
| #211 | #320 | "Add code coverage job to CI" — third duplicate of the same coverage request |
| #329 | PR #330 | "Implement CI bifurcation" — PR #330 implements this directly |
| #171 | #316 | "Complete AT-SPI focus backend" — #171 is a single sentence ("keeping pending focused review"). #316 has ~100 lines of working Rust code, crate imports, feature-gating strategy, and fallback behavior. #299 (on KEEP list) references #316's approach. Keeping the empty stub and closing the one with code would be backwards |

### Bot-Generated / Jules (4 issues)

| # | Title | Reason |
|---|-------|--------|
| #253 | Implement claudeZ Automated Debugging System | Jules bot meta-tooling with no actionable code or plan |
| #254 | URGENT: claudeZ In-Place Automated Debugging | Same as #253; duplicate jules bot output |
| #45 | Optimize format conversions throughout the audio pipeline | Jules stub with one paragraph; no specific conversions identified, no benchmarks, no acceptance criteria |
| #335 | Dependency Audit: Pending Updates and UV Compliance Review | Thorough audit content (credit to Jules here), but its actionable items (rubato update, dep freshness) are now superseded by Dependabot PR #343. Close as "superseded" not "junk" |

### Too Vague / Low Priority (9 issues)

| # | Title | Reason |
|---|-------|--------|
| #215 | Enhance Docs Cross-Reference Analyzer | Tooling automation; low ROI, no clear deliverable |
| #304 | Tooling: review .envrc auto-uv behavior | Real DX annoyance but low priority; `uv sync` on every `cd` is not causing harm. Close as "low priority" not "vague" |
| #162 | Testing infrastructure | Single-paragraph stub from Oct 2025; superseded by specific testing issues (#325, #306, #320) |
| #212 | Explore Test Parallelization | Tests already pass in ~10s; premature optimization |
| #213 | Use GitHub-Hosted Runners for certain jobs | Superseded by #318 / PR #330 which implement this concretely |
| #208 | Refactor existing tests to align with new testing paradigm | Jules stub referencing `docs/dev/testing.md`; no concrete plan |
| #229 | Dependency Audit and Update | Handled by Dependabot (#342, #343); no additional scope |
| #228 | CI/CD Workflow Enhancements | Jules stub; vague "enhancements" with no specifics |
| #252 | Speed Demon Build Optimization + Candle PR Integration Strategy | Candle integration is dead; build optimization is low priority (compiles in ~30s) |

### Vague Jules Stubs — Consistency Closures (2 issues)

These are structurally identical to #208/#212/#228 above (jules-created stubs referencing
`docs/dev/testing.md` or issue #136) but were originally on the KEEP list. Closing for
consistency — keeping these while closing their siblings would be arbitrary.

| # | Title | Reason |
|---|-------|--------|
| #209 | Improve the logging to be more verifiable and context-aware | Jules stub; same single-paragraph format as #208/#228 which are being closed |
| #230 | Developer Onboarding and Documentation | Jules stub; same template as #229/#228. Re-open with concrete scope if onboarding docs become a priority |

### PR to Close (1)

| PR | Title | Reason |
|----|-------|--------|
| #336 | Fix: Update rubato to v1.0.0 and Address Dependency Audit | Draft PR from Jules bot. Superseded by Dependabot PR #343 which includes rubato among 19 dependency updates |

---

## KEEP — 32 Issues

### P0 — Fix This Week (7 issues)

These block correct operation or actively mislead developers.

| # | Title | Priority Justification |
|---|-------|----------------------|
| #287 | CLAUDE.md references deleted whisper_plugin.rs | CLAUDE.md is loaded into every Claude Code session; wrong paths waste AI context and human time |
| #286 | AGENTS.md contains incorrect feature flag descriptions | AGENTS.md claims whisper/parakeet are working features; actively misleading |
| #285 | Dead code references non-existent WhisperPluginFactory | Compile-time dead code behind `#[cfg(feature = "whisper")]`; blocks understanding the STT pipeline |
| #321 | docs: remove faster-whisper references from README | README is the first thing contributors see; claims whisper works |
| #281 | PyO3 0.24 Instability on Python 3.13 (Moonshine Backend) | Blocks the ONLY working STT backend. If Moonshine breaks, ColdVox has zero speech-to-text |
| #284 | Plugin manager GC can unload active plugins | Concurrency bug — the garbage collector can destroy plugins mid-transcription |
| #283 | Blocking locks in audio callback violate real-time constraints | Real-time safety violation; can cause audio glitches and dropped frames |

### P1 — Fix This Month (5 issues)

Real bugs and critical test gaps.

| # | Title | Priority Justification |
|---|-------|----------------------|
| #292 | Concurrency hazards in text injection (4 of 6 documented still present) | Documented race conditions with repro steps; 4 of 6 original hazards remain |
| #293 | No explicit plugin unload on replacement (memory leak) | Plugin manager leaks memory when switching plugins; confirmed in code review |
| #291 | Failover cooldown too short (1s) enables oscillation | Live config bug: `config/plugins.json` overrides the 30s default to 1s. Plugin manager actively reads this value. Will cause rapid oscillation on any backend failure |
| #289 | No cancellation tokens for background tasks | Shutdown/cleanup bug; background tasks can outlive their parent context |
| #288 | Plugin manager has zero tests | The plugin manager has 3 confirmed bugs (#284, #293, #291) and zero test coverage. Testing it would catch the bugs above and prevent regressions |

### P2 — STT Enhancement (4 issues)

Valid for Moonshine or any future backend.

| # | Title | Notes |
|---|-------|-------|
| #322 | Research: async STT refactor for parallel audio processing | Backend-agnostic; applies to Moonshine |
| #42 | Implement support for long utterance processing | Chunking/buffering strategy needed for any real-world use |
| #46 | Harden STT model loading and validation | Security hardening; applies to Moonshine's ONNX model loading |
| #316 | feat: AT-SPI focus backend implementation | Has ~100 lines of working code for focus detection. Referenced by #299 |

### P2 — Testing Infrastructure (8 issues)

| # | Title | Notes |
|---|-------|-------|
| #325 | feat: text-injection integration testing framework | Comprehensive framework issue; supersedes 5+ duplicates |
| #264 | Convert disabled real-injection tests to proper unit tests | Concrete task; some backends still lack unit tests |
| #271 | Create input simulation test harness for keyboard injection | Unit-level simulation distinct from #325's integration scope |
| #269 | Set up Xvfb environment for X11 text injection tests | Part of #325 framework; needed for CI |
| #270 | Implement Wayland headless compositor for Wayland tests | Part of #325 framework; needed for CI |
| #306 | Tests: real-injection suite should not be 'green' when all backends are skipped | Test correctness: currently reports "pass" when nothing actually runs |
| #308 | Code Quality: Fix unused code warnings in text-injection tests | Pairs with #303 warnings policy |
| #302 | Tests: make GTK test app readiness signaling robust | Flaky test infrastructure; stale `/tmp` files cause false-ready |

### P2 — CI & Tooling (4 issues)

| # | Title | Notes |
|---|-------|-------|
| #320 | ci: code coverage with cargo-llvm-cov and Codecov | Best of 3 duplicates; clear implementation plan |
| #303 | CI: define warnings policy after removing -D warnings | Need to decide: deny warnings or allow them |
| #318 | CI Refactor: Bifurcation and Runner Optimization (Proposals A & B) | PR #330 only covers bifurcation. #318 also tracks: sccache 10GB, mold linker, CARGO_INCREMENTAL=1, persistent rust-cache keys, workflow consolidation. Keep open until those items are addressed or spun off |
| #305 | Tooling: fix mise fmt:rust task quoting/behavior | Concrete bug: `mise run fmt:rust` fails on fresh machines due to heredoc quoting in `mise.toml`. Clear acceptance criteria |

### P2 — Enhancement Backlog (4 issues)

| # | Title | Notes |
|---|-------|-------|
| #299 | AT-SPI injector: Replace Collection.GetMatches with event-driven approach | Performance/compatibility improvement; references #316's code |
| #173 | Implement VM-based compositor testing matrix | Platform-specific validation strategy |
| #226 | GUI Integration Roadmap | Tracks the `coldvox-gui` crate's future; placeholder but intentional |
| #301 | Docs: CI/CD playbook references removed ci-failure-analysis workflow | Doc cleanup; lower priority than P0 doc fixes |

---

## MERGE — 6 PRs

| PR | Title | Action | Notes |
|----|-------|--------|-------|
| #343 | chore(deps): bump rust-dependencies group (19 updates) | Merge if CI passes | Includes rubato 0.16→1.0 and ratatui 0.29→0.30 (major bumps — review changelogs) |
| #342 | chore(deps): bump actions/checkout 6.0.1→6.0.2 | Merge if CI passes | Routine dependency bump |
| #330 | feat(ci): bifurcate CI into hosted and self-hosted jobs | Merge | Your work; closes #329 |
| #313 | docs: mark Phases 2-4 complete in PR triage action plan | Merge | Your doc update |
| #312 | feat(text-injection): add unit tests for enigo and kdotool | Merge | Your work; closes #266 |
| #311 | feat(ydotool): add comprehensive unit tests for ydotool | Merge | Your work; closes #265 |

---

## Stale Branches to Delete

These remote branches correspond to closed/merged work or superseded bot PRs:

| Branch | Reason |
|--------|--------|
| `origin/fix/dependency-audit-rubato-update-13523515918554210530` | Jules bot branch for PR #336 (closing) |
| `origin/integrate/268` | No corresponding open issue/PR |
| `origin/integrate/273` | No corresponding open issue/PR |
| `origin/docs/update-action-plan-phases` | Corresponds to PR #313 (merge then delete) |
| `origin/feat/ci-bifurcation` | Corresponds to PR #330 (merge then delete) |
| `origin/dependabot/cargo/rust-dependencies-00915d846b` | Corresponds to PR #343 (merge then delete) |
| `origin/dependabot/github_actions/actions-e6ee9d7de3` | Corresponds to PR #342 (merge then delete) |

---

## Methodology Notes

- All issue bodies were read via `gh api` to verify scope and detect false duplicates
- A second-opinion review pass caught 5 corrections to the initial triage:
  1. #291 was miscategorized as "dead STT" — it is a live config bug
  2. #316/#171 were swapped — #171 is the empty stub, #316 has the code
  3. #305 is a concrete reproducible bug, not "vague"
  4. #318 has scope beyond PR #330 (sccache, mold, caching optimizations)
  5. #223's body is 100% dead Candle/Whisper despite the generic title
- #209 and #230 were moved from KEEP to CLOSE for consistency with identical jules stubs being closed
