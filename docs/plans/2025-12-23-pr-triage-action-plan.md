# PR Triage Action Plan

# PR Triage Action Plan

---
doc_type: plan
subsystem: general
version: 1.0.0
status: draft
owners: Coldaine
last_reviewed: 2025-12-23
---

**Date**: 2025-12-23
**Status**: Active
**Last Updated**: 2025-12-23 (session corrections applied)
**Scope**: Analysis of 50 PRs (open, draft, and closed)

---

## Recent Updates (Session Corrections)

| Item | Status | Notes |
|------|--------|-------|
| PR #297 (actions bump) | ✅ Merged | Already in main |
| PR #298 (rust deps) | ✅ Closed | Superseded by #300 |
| PR #294 (PyO3 0.27) | ✅ Superseded | Merged as part of PR #296 |

---

## Executive Summary

Comprehensive analysis of the ColdVox PR backlog identified:
- **2 open feature PRs** requiring attention (#295, #300)
- **2 cleanup PRs** (one supersedes the other: #282 over #262)
- **12 Jules draft PRs** with varying value
- **5 closed PRs** correctly abandoned

**Quick wins**: PR #295 is merge-ready. PR #282 replaces #262 and cleans up 2,825 lines.

---

## Priority Matrix

### Tier 1: Immediate Action

| PR | Title | Status | Action | Effort |
|----|-------|--------|--------|--------|
| #295 | Fix zbus API changes | Ready | **MERGE** | 5 min |
| #282 | Remove stub STT plugins | CI failing | **FIX & MERGE** | 1 hr |
| #262 | Cleanup deprecated code | Superseded | **CLOSE** | 5 min |

### Tier 2: Needs Work

| PR | Title | Status | Action | Effort |
|----|-------|--------|--------|--------|
| #300 | Dependabot: 9 Rust deps | Broken | **FIX CPAL 0.17** | 2 hr |
| ~~#294~~ | ~~PyO3 0.27 upgrade~~ | ✅ Superseded | ~~INVESTIGATE~~ | - |

> **Note**: PR #294 was merged as part of PR #296. No action needed.

### Tier 3: Jules Drafts - Merge Candidates

| PR | Title | Value | Action |
|----|-------|-------|--------|
| #276 | Xvfb for X11 tests | HIGH | Test & merge |
| #273 | ydotool unit tests | HIGH | Review & merge |
| #268 | Enigo/kdotool tests | MED-HIGH | Review refactoring |

### Tier 4: Jules Drafts - Cherry-Pick

| PR | Title | Value | Notes |
|----|-------|-------|-------|
| #232 | AT-SPI focus backend | HIGH | Real implementation |
| #234 | Code coverage CI | MED | cargo-llvm-cov |
| #267 | Mock injection harness | MED | Reusable patterns |
| #240 | Model checksum validation | MED | Security hardening |

### Tier 5: Close/Defer

| PR | Title | Reason |
|----|-------|--------|
| #277 | Headless Wayland (WIP) | Author stuck, incomplete |
| #274 | Orchestrator tests | Fundamentally broken |
| #239 | Word timestamps research | Research only, incomplete |
| #236 | Async STT perf | Too risky, major architecture |
| #255, #256 | claudeZ debugging | Never completed |
| #258, #261 | Moonshine tweaks | Superseded by merged #259 |

---

## Detailed Analysis

### PR #295: Fix zbus API Changes

**Branch**: `claude/fix-zbus-api-changes-01EgwLaBdotQodf24puCui6T`
**Files**: 2 files, +7/-5 lines
**Created**: 2025-12-12

**What it does**: Adapts to atspi-common 0.13.0 API changes where `ObjectRef` changed from struct fields to accessor methods.

**Changes**:
```rust
// Before
obj_ref.name → obj_ref.name_as_str()
obj_ref.path → obj_ref.path_as_str()
```

**Reviews**: Two bot reviews passed (kiloconnect, copilot)

**Recommendation**: **MERGE IMMEDIATELY** - Trivial, safe, reviewed.

---

### PR #282: Remove Stub STT Plugin Files

**Branch**: `cleanup/remove-stub-plugins-v2`
**Files**: 30 files, +201/-2,825 lines
**Created**: 2025-12-11

**What it removes**:
1. **5 stub STT plugins** (2,091 lines):
   - `coqui.rs` (223 lines)
   - `leopard.rs` (233 lines)
   - `silero_stt.rs` (303 lines)
   - `whisper_cpp.rs` (374 lines)
   - `whisper_plugin.rs` (943 lines)

2. **Vosk infrastructure** (734 lines):
   - `.github/workflows/vosk-integration.yml`
   - Vosk ADR and troubleshooting docs
   - Self-hosted runner setup docs

**CI Status**: 4/9 checks failing
- Documentation validation
- Security audit (likely benign rustup warning)
- Build & Test
- Text Injection Tests

**Replaces**: PR #262 (explicitly stated in description)

**Why #282 over #262**:
- #282 correctly preserves `moonshine.rs`
- #262 accidentally deleted `moonshine.rs`
- #282 is more comprehensive (includes Vosk cleanup)
- #282 has better CHANGELOG documentation

**Test Plan** (from PR):
- [ ] `cargo check -p coldvox-stt` passes
- [ ] `cargo build --features parakeet` works
- [ ] `cargo build --features moonshine` works

**Recommendation**: **FIX CI FAILURES, THEN MERGE**

---

### PR #262: Remove Deprecated and Unimplemented Code

**Branch**: `chore/cleanup-deprecated-code`
**Files**: 38 files, +30/-4,922 lines
**Created**: 2025-12-03

**Status**: **SUPERSEDED BY #282**

**Issues**:
- Accidentally deleted `moonshine.rs` (critical bug)
- 8/17 CI checks failing (more than #282)
- Older and less comprehensive

**Unique valuable deletions** (consider adding to #282):
- `crates/coldvox-text-injection/src/compat.rs` (535 lines)
- `crates/coldvox-stt/src/processor.rs` (234 lines)
- `.tests_temp/` directory (1,552 lines orphaned tests)
- Duplicate `plugins.json` files

**Recommendation**: **CLOSE** with comment "Superseded by #282"

---

### PR #300: Dependabot Rust Dependencies

**Branch**: `dependabot/cargo/rust-dependencies-dabf92191a`
**Created**: 2025-12-23 (today)
**Updates**: 9 packages

**Dependency Updates**:
| Package | From | To | Impact |
|---------|------|-----|--------|
| **cpal** | 0.16.0 | 0.17.0 | **BREAKING** |
| tracing | 0.1.43 | 0.1.44 | Bug fix |
| serde_json | 1.0.145 | 1.0.146 | RISC-V optimization |
| toml | 0.9.8 | 0.9.10 | TOML 1.1 support |
| wl-clipboard-rs | 0.9.2 | 0.9.3 | Improvements |
| cc | 1.2.49 | 1.2.50 | Bug fixes |
| parakeet-rs | 0.2.5 | 0.2.6 | Token joining fixes |
| cxx | 1.0.190 | 1.0.192 | Minor |
| cxx-qt-build | 0.7.3 | 0.8.0 | Minor version |

**CPAL 0.17.0 Breaking Changes**:

1. **`SampleRate` type change**: Now a `u32` alias, not a tuple struct
   ```rust
   // Before
   cpal::SampleRate(16000)
   // After
   16000_u32
   ```
   - **Affected**: `device.rs:318`

2. **Field access removed**: `SampleRate.0` no longer valid
   ```rust
   // Before
   sample_rate.0
   // After
   sample_rate  // Already u32
   ```
   - **Affected**: `device.rs:317`, `capture.rs:392`

3. **`device.name()` deprecated**: 8 occurrences
   ```rust
   // Before
   device.name()
   // After
   device.description()  // For human-readable
   device.id()           // For stable identifiers
   ```
   - **Affected**: `device.rs` (7 locations), `capture.rs` (1 location)

**Recommendation**: **CREATE FIX COMMIT** for CPAL compatibility, then merge

---

### ~~PR #294: PyO3 0.27 Upgrade~~ (SUPERSEDED)

> **Status**: ✅ **SUPERSEDED** - Merged as part of PR #296

**Branch**: `feat/pyo3-0.27-upgrade`
**Files**: 10 files, +366/-220 lines
**Created**: 2025-12-12

**Key Changes** (now in main via #296):
1. PyO3 0.24.1 → 0.27 with `auto-initialize`
2. API migration: `Python::with_gil()` → `Python::attach()`
3. Development tooling: `.envrc`, justfile recipes
4. Documentation: `docs/issues/pyo3_instability.md`

**Recommendation**: **CLOSE** - Already merged via #296

---

## Jules Draft PRs Analysis

### Ready to Merge

#### PR #276: Enable Xvfb for X11 Text Injection Tests

**Branch**: `feat/xvfb-text-injection-tests`
**Value**: HIGH - Unblocks CI text injection testing

**What it does**:
- Sets up Xvfb virtual display in CI
- Re-enables `real_injection.rs` tests
- Allows headless X11 testing

**Merge conflicts**: Minimal (CI config only)

#### PR #273: ydotool Unit Tests

**Branch**: `feat/ydotool-unit-tests`
**Value**: HIGH - Comprehensive test coverage

**What it does**:
- 200+ lines of proper mock infrastructure
- `UINPUT_PATH_OVERRIDE` env var for testability
- Isolated unit tests for ydotool injector

#### PR #268: Enigo and kdotool Unit Tests

**Branch**: `feat/add-injector-unit-tests`
**Value**: MEDIUM-HIGH

**What it does**:
- Refactors enigo injector for dependency injection
- `MockEnigo` implementing `Keyboard` trait
- Comprehensive test coverage

---

### Useful Code to Cherry-Pick

#### PR #232: AT-SPI Focus Backend

**Branch**: `feature/at-spi-focus-backend`
**Issue**: #171
**Value**: HIGH - Real implementation

**What it does**:
- Replaces stub `SystemFocusAdapter` with real AT-SPI
- Proper async implementation using atspi crate
- Critical for smart text injection

**Action**: Test on real system, merge separately

#### PR #234: Code Coverage CI Job

**Branch**: `feat/add-code-coverage-job`
**Issue**: #211
**Value**: MEDIUM

**What it does**:
- Adds `cargo-llvm-cov` CI job
- Codecov upload integration

**Action**: Merge after other tests stabilize

#### PR #240: Harden STT Model Loading

**Branch**: `harden-stt-model-loading`
**Issue**: #46
**Value**: MEDIUM

**What it does**:
- SHA256 checksum validation for models
- `validation.rs` with `Checksums` struct
- Security hardening

**Note**: Issue #46 marked "will review after Candle Whisper" - may need update

---

### Close/Defer

| PR | Branch | Reason |
|----|--------|--------|
| #277 | `feature/wayland-headless-wip` | WIP, author stuck on approach |
| #274 | `feature/orchestrator-integration-tests` | Can't mock concrete types |
| #239 | `feat/word-level-timestamps-research` | Research only, incomplete DTW |
| #236 | `perf/async-stt` | Major architecture change, risky |
| #255 | `feature/claudeZ-automated-debugging` | Never completed |
| #256 | `feat/claudez-automated-debugging` | Duplicate, never completed |
| #258 | `add-moonshine-stt` | Superseded by merged #259 |
| #261 | `feat/moonshine-stt-plugin-tweaks` | Superseded by merged #278 |

---

## Closed PRs - Status

| PR | Title | Why Closed | Salvageable |
|----|-------|------------|-------------|
| #298 | Dependabot rust deps | Superseded by #300 | No |
| #294 | PyO3 0.27 upgrade | Merged as part of PR #296 | No (already merged) |
| #242 | Candle Whisper STT | Broken implementation (empty mel_filters), superseded by Moonshine | No |
| #241 | Comprehensive docs | Docs for broken #242 | No |
| #225 | CI docs/fmt fix | 22 commits behind, fixes applied elsewhere | Maybe buffer prealloc |
| #216 | Compare PR 204-205 | Analysis-only, served purpose | No |

---

## Execution Checklist

### Phase 1: Quick Wins (Today)

- [ ] Close PR #294 with comment "Superseded - merged as part of #296"
- [ ] Merge PR #295 (zbus fix)
- [ ] Close PR #262 with comment "Superseded by #282"
- [ ] Close stale Jules drafts: #277, #274, #255, #256, #258, #261

### Phase 2: Cleanup PR (This Week)

- [ ] Fix PR #282 CI failures
  - [ ] Documentation validation
  - [ ] Build & test
  - [ ] Text injection tests
- [ ] Merge PR #282
- [ ] Optionally incorporate valuable #262 deletions:
  - [ ] `compat.rs` removal
  - [ ] `processor.rs` removal
  - [ ] `.tests_temp/` cleanup

### Phase 3: Dependency Update (This Week)

- [ ] Create branch from #300
- [ ] Fix CPAL 0.17.0 breaking changes:
  - [ ] Update `SampleRate` usage in `device.rs`
  - [ ] Remove `.0` field access in `device.rs`, `capture.rs`
  - [ ] Replace `device.name()` with `device.description()`
- [ ] Test audio device detection
- [ ] Merge updated PR #300

### Phase 4: Test Infrastructure (Next Week)

- [ ] Review and merge PR #276 (Xvfb CI)
- [ ] Review and merge PR #273 (ydotool tests)
- [ ] Review and merge PR #268 (enigo/kdotool tests)
- [ ] Review PR #232 (AT-SPI focus) - test separately

### ~~Phase 5: PyO3 Upgrade~~ (COMPLETE)

> ✅ **Already done** - PR #294 was merged as part of PR #296

### Phase 5: Optional Enhancements

- [ ] Evaluate PR #234 (code coverage)
- [ ] Evaluate PR #240 (model checksums)
- [ ] Close remaining research PRs: #239, #236

---

## Appendix: PR Statistics

**Total PRs Analyzed**: 50

| Category | Count | Notes |
|----------|-------|-------|
| Open (non-draft) | 4 | #295, #300, #282, #262 |
| Open (draft) | 15 | Jules drafts |
| Merged | 22 | Includes #296, #297 |
| Closed (not merged) | 9 | Includes #294, #298 |

**By Author**:
| Author | Open | Draft | Merged |
|--------|------|-------|--------|
| Coldaine | 3 | 0 | 16 |
| dependabot | 1 | 0 | 5 |
| google-labs-jules | 0 | 14 | 2 |
| copilot-swe-agent | 0 | 1 | 0 |

**Lines of Code Impact** (if all cleanup merged):
- Removed: ~7,747 lines (stub code, obsolete docs)
- Added: ~400 lines (tests, tooling)
- Net: **-7,347 lines**
