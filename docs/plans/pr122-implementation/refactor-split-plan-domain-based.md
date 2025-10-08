# Domain-Based Refactor Split Plan (Recommended)

**Plan Name:** Domain-Based Stack
**Grade:** A+ (with PR #0 modification)
**Status:** Recommended for Execution
**Date:** 2024-10-07

---

## Overview

This plan splits the `anchor/oct-06-2025` refactor branch (93 files, 33 commits) into **10 stacked PRs** organized by domain boundaries. Each PR maps to one or two crates in the multi-crate workspace, respecting the natural architectural layers.

**Why This Plan:** 
- Respects repository architecture (multi-crate workspace)
- Minimizes merge conflicts (2-3 vs 5-7 for alternative approaches)
- Enables parallel development (VAD + STT can work simultaneously)
- Simplifies reviews (domain experts per PR)
- Natural fit for Graphite's `gt split --by-hunk` workflow

---

## The Stack (Bottom → Top)

```
┌─────────────────────────────────────────────────────────────┐
│ 09. docs-changelog                                          │
│     Scope: docs/**, CHANGELOG*, README*                     │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│ 08. logging-observability                                   │
│     Scope: crates/coldvox-telemetry/**, logging changes     │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│ 07. testing                                                 │
│     Scope: **/tests/**, E2E WAV tests                       │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│ 06. text-injection                                          │
│     Scope: crates/coldvox-text-injection/**                 │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│ 05. app-runtime-wav                                         │
│     Scope: crates/app/src/runtime.rs,                       │
│            crates/app/src/audio/wav_file_loader.rs          │
└─────────────────────────────────────────────────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
┌───────────────────▼─────┐   ┌─────────▼───────────────────┐
│ 03. vad                 │   │ 04. stt                     │
│ Scope: crates/coldvox-  │   │ Scope: crates/coldvox-stt/**│
│        vad*/**          │   │        crates/coldvox-stt-  │
└───────────────────┬─────┘   └─────────┬───────────────────┘
                    │                   │
                    └─────────┬─────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│ 02. audio-capture                                           │
│     Scope: crates/coldvox-audio/**                          │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│ 01. config-settings                                         │
│     Scope: crates/app/src/lib.rs, config/**, tests         │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│ 00. hotfix-clipboard-p0                                     │
│     Scope: clipboard P0 bug fix (~10 lines)                 │
└─────────────────────────────────────────────────────────────┘
                              │
                          ┌───▼───┐
                          │ main  │
                          └───────┘
```

---

## Branch Details

### PR #00: hotfix-clipboard-p0
**Title:** `[00] hotfix(injection): clipboard paste actually issues Ctrl+V`  
**Scope:** Critical P0 bug fix extracted from text-injection changes  
**Dependencies:** None (merges directly to main)  
**Estimated Size:** ~10 lines  
**Why First:** Unblocks users immediately, addresses urgent production issue

**Key Changes:**
- Fix clipboard paste injector to actually issue Ctrl+V command
- No other changes (pure bug fix)

**Validation:**
```bash
cargo test -p coldvox-text-injection
# Manual test: clipboard paste functionality
```

---

### PR #01: config-settings
**Title:** `[01] config: centralize Settings + path-aware load`  
**Scope:** `crates/app/src/lib.rs`, `config/**`, `crates/app/tests/settings_test.rs`  
**Dependencies:** PR #00  
**Estimated Size:** Medium (foundation changes)

**Key Changes:**
- Centralize configuration loading with path-aware logic
- Add environment variable overrides (`COLDVOX_CONFIG_PATH`)
- Update Settings API for deterministic testing
- Add TOML config files (`config/default.toml`, `config/overrides.toml`)

**Why This Order:** Config is foundation; all other crates consume it

**Validation:**
```bash
cargo test --test settings_test
cargo build -p coldvox-app
# Verify config file loading with different paths
```

---

### PR #02: audio-capture
**Title:** `[02] audio: capture lifecycle fix + ALSA stderr suppression`  
**Scope:** `crates/coldvox-audio/**`  
**Dependencies:** PR #01  
**Estimated Size:** Medium

**Key Changes:**
- Audio capture thread lifecycle improvements
- Device monitor enhancements
- ALSA stderr suppression (reduces noise in logs)
- Watchdog and stability fixes

**Why This Order:** Audio is the first processing layer after config

**Validation:**
```bash
cargo test -p coldvox-audio
cargo run --bin mic_probe -- --duration 30
# Check PipeWire FPS and capture stability
```

---

### PR #03: vad
**Title:** `[03] vad: windowing/debounce consistency`  
**Scope:** `crates/coldvox-vad/**`, `crates/coldvox-vad-silero/**`  
**Dependencies:** PR #02  
**Estimated Size:** Small-Medium  
**Parallel-Safe:** Can develop in parallel with PR #04 (both depend on #02 only)

**Key Changes:**
- Frame-based VAD debouncing for deterministic testing
- Timestamp-ms candidates for reproducibility
- Windowing consistency improvements

**Why This Order:** VAD processes audio frames; can work in parallel with STT

**Validation:**
```bash
cargo test -p coldvox-vad
cargo test -p coldvox-vad-silero
cargo run --example test_silero_wav --features examples
```

---

### PR #04: stt
**Title:** `[04] stt: finalize handling + helpers`  
**Scope:** `crates/coldvox-stt/**`, `crates/coldvox-stt-vosk/**`  
**Dependencies:** PR #02  
**Estimated Size:** Small-Medium  
**Parallel-Safe:** Can develop in parallel with PR #03 (both depend on #02 only)

**Key Changes:**
- STT finalization behavior improvements
- Helper utilities for transcription processing
- Session event handling refinements

**Why This Order:** STT processes audio frames; can work in parallel with VAD

**Validation:**
```bash
cargo test -p coldvox-stt
cargo test -p coldvox-stt-vosk
cargo run --features vosk --example vosk_test
```

---

### PR #05: app-runtime-wav
**Title:** `[05] app: unify VAD↔STT runtime + real WAV loader`  
**Scope:** `crates/app/src/runtime.rs`, `crates/app/src/audio/wav_file_loader.rs`, E2E glue  
**Dependencies:** PR #03, PR #04  
**Estimated Size:** Large (integration layer)

**Key Changes:**
- Unified VAD/STT pipeline in runtime
- Deterministic WAV file streaming for E2E tests
- Real WAV loader with trailing silence support
- Integration hooks for deterministic testing

**Why This Order:** Integrates VAD and STT; requires both to be complete

**Validation:**
```bash
cargo test -p coldvox-app test_end_to_end_wav --nocapture
cargo test -p coldvox-app --features vosk
# Run full integration test suite
```

---

### PR #06: text-injection
**Title:** `[06] injection: clipboard-preserve + Wayland-first strategy`  
**Scope:** `crates/coldvox-text-injection/**` (all remaining changes)  
**Dependencies:** PR #05  
**Estimated Size:** Medium-Large

**Key Changes:**
- Clipboard preservation (save → inject → restore)
- Wayland-first strategy ordering (AT-SPI → Clipboard → ydotool)
- Strategy manager refactor with per-app success caching
- Combined clipboard+paste injector improvements (beyond P0 fix)
- Timing improvements for clipboard restoration (P1 fix)

**Why This Order:** Text injection is the output layer; depends on runtime

**Validation:**
```bash
cargo test -p coldvox-text-injection
cargo run --features text-injection --example inject_demo
# Integration tests with strategy manager
```

---

### PR #07: testing
**Title:** `[07] tests: deterministic E2E + integration suites`  
**Scope:** `**/tests/**`, E2E WAV tests, integration test setup  
**Dependencies:** PR #06  
**Estimated Size:** Medium

**Key Changes:**
- Deterministic E2E test infrastructure
- Settings test fixtures with path-aware loading
- Integration test suite improvements
- WAV file-based testing validation

**Why This Order:** Consolidates all test infrastructure after features are complete

**Validation:**
```bash
cargo test --workspace
cargo test --workspace --features vosk
# Verify all tests pass with new infrastructure
```

---

### PR #08: logging-observability
**Title:** `[08] logs: prune noisy hot paths; telemetry tweaks`  
**Scope:** `crates/coldvox-telemetry/**`, scattered logging changes  
**Dependencies:** PR #07  
**Estimated Size:** Small-Medium

**Key Changes:**
- Reduce hot-path logging noise
- Telemetry metric improvements
- Observability enhancements for debugging
- Log level adjustments

**Why This Order:** Logging touches many files; best done after features stabilize

**Validation:**
```bash
cargo run --bin tui_dashboard -- --log-level debug
cargo run --features vosk,text-injection
# Check log output for reduced noise
```

---

### PR #09: docs-changelog
**Title:** `[09] docs: changelog + guides + fixes`  
**Scope:** `docs/**`, `CHANGELOG.md`, `README.md`, deployment guides  
**Dependencies:** PR #08  
**Estimated Size:** Medium

**Key Changes:**
- Update CHANGELOG.md with all changes from stack
- Fix false documentation claims (e.g., XDG paths)
- Add deployment guides
- Update configuration documentation
- Add runflags reference

**Why This Order:** Documentation comes last when all changes are known

**Validation:**
```bash
# Link validation
find docs -name "*.md" -exec markdown-link-check {} \;
# Manual review of accuracy
```

---

## Expected Metrics

| Metric | Value | Rationale |
|--------|-------|-----------|
| **Total PRs** | 10 | Clean domain boundaries |
| **Merge Conflicts** | 2-3 | Each crate modified once |
| **CI Failures** | 0-1 | Domain isolation prevents integration issues |
| **Review Time** | 1-2 weeks | Parallel reviews possible (PRs #3 & #4) |
| **Context Switches** | 1 per PR | Domain experts assigned per PR |
| **Parallel Work** | 2 PRs | VAD + STT can develop simultaneously |

---

## Execution Steps (Graphite)

### 1. Preparation
```bash
# Backup current branch
git checkout anchor/oct-06-2025
git branch backup/anchor-oct-06-2025-$(date +%Y%m%d-%H%M%S)

# Ensure clean working tree
git status
```

### 2. Track and Split
```bash
# Adopt branch into Graphite stack
gt track

# Interactive split by hunk
gt split --by-hunk

# During interactive split, assign hunks by path:
# - config/** → 01-config-settings
# - crates/coldvox-audio/** → 02-audio-capture
# - crates/coldvox-vad*/** → 03-vad
# - crates/coldvox-stt*/** → 04-stt
# - crates/app/src/runtime.rs, wav_file_loader.rs → 05-app-runtime-wav
# - crates/coldvox-text-injection/** → 06-text-injection
#   (BUT extract ~10 lines for P0 bug to 00-hotfix-clipboard-p0)
# - **/tests/** → 07-testing
# - logging/tracing changes → 08-logging-observability
# - docs/**, CHANGELOG* → 09-docs-changelog
```

### 3. Extract PR #0 (Critical)
```bash
# After split, extract P0 bug fix manually if needed
gt checkout 06-text-injection
gt create --insert 00-hotfix-clipboard-p0
# Cherry-pick only the P0 bug fix lines
# Move 00-hotfix-clipboard-p0 to base of stack
gt move --onto main
```

### 4. Reorder Stack
```bash
# Verify order
gt log

# Reorder if needed
gt reorder
# Ensure: main → 00 → 01 → 02 → 03/04 → 05 → 06 → 07 → 08 → 09
```

### 5. Validate Each Branch
```bash
# For each branch (00 through 09):
gt checkout <branch-name>
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt -- --check
gt up  # move to next branch
```

### 6. Push and Create PRs
```bash
# Push all branches
git push --all

# Create PRs (Graphite Cloud or manual)
gt submit
# OR
# Use gh pr create for each branch with proper --base
```

### 7. Post-Merge Maintenance
```bash
# After each PR merges:
gt sync

# If conflicts:
gt checkout <conflicted-branch>
gt restack
# resolve conflicts
git add -A
gt continue
```

---

## Why This Plan Gets A+

1. **Immediate P0 Fix** (PR #0) - No delay for critical bugs
2. **Natural Dependencies** - Follows repository architecture (Foundation → Audio → VAD/STT → App → Injection)
3. **Parallel Development** - VAD and STT can work simultaneously (both depend on audio only)
4. **Domain Isolation** - Each crate modified once (text-injection: 1 PR instead of 3)
5. **Review Efficiency** - Domain experts assigned per PR (no context switching)
6. **CI Stability** - Domain boundaries prevent integration failures
7. **Merge Safety** - 2-3 conflicts vs 5-7 for alternative approaches
8. **Graphite-Friendly** - Natural path-based hunk clustering
9. **Testing Coherence** - All test infrastructure in single PR (easy to validate)
10. **Documentation Sync** - All docs in final PR (single source of truth for changelog)

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| PR #0 too small | Small is OK for hotfixes; establishes precedent for urgent fixes |
| PR #1 too large | Can split config into `config-core` + `config-integration` if needed |
| PR #5 integration fails | Has all dependencies ready; validated with E2E tests |
| Conflict during restack | `gt restack` + manual resolution + `gt continue` |
| Branch order wrong | `gt reorder` in interactive editor |

---

## Success Criteria

- [ ] All 10 branches created and pushed
- [ ] Each branch passes `cargo test --workspace`
- [ ] Each branch passes `cargo clippy --workspace -- -D warnings`
- [ ] Dependency graph matches plan (visualized with `gt log`)
- [ ] No cross-cutting changes (each PR modifies 1-2 crates max)
- [ ] PR descriptions document dependencies clearly
- [ ] CI passes for each PR before merge
- [ ] Merge order: 00 → 01 → 02 → 03/04 → 05 → 06 → 07 → 08 → 09

---

## Timeline Estimate

| Phase | Time | Notes |
|-------|------|-------|
| Pre-flight + backup | 10 min | Safety first |
| Interactive split | 60-90 min | Path-based hunk assignment |
| Extract PR #0 | 15 min | Manual cherry-pick if needed |
| Stack reordering | 10 min | `gt reorder` |
| Per-branch validation | 90 min | 10 branches × 9 min each |
| Push + PR creation | 30 min | 10 PRs with descriptions |
| **Total (execution)** | **3.5-4 hours** | First-time with this flow |
| Review + merge | 1-2 weeks | Team-dependent, parallel reviews |

---

## Comparison to Alternatives

This plan was compared against a "Fix/Feature-Based" alternative that prioritized P0/P1 fixes first, then features. The alternative scored **C+** due to:
- Text-injection crate modified 3× sequentially (PRs #2, #3, #5)
- Runtime refactor delayed until PR #9 (blocking earlier work)
- High merge conflict risk (5-7 predicted)
- Mixed concerns (tests in PR #1, features in PR #8)

**This domain-based plan addresses all those issues** and achieves **A+ grade** by including PR #0 from the start.

---

## References

- Full comparison: `docs/review/split-plan-comparison/refactor-split-strategy-comparison.md`
- Execution guide: `docs/review/split-plan-comparison/execution-guide.md`
- Repository structure: `CLAUDE.md`
- Graphite documentation: https://graphite.dev/docs

---

**Status:** Ready for Execution  
**Next Action:** Get stakeholder sign-off and schedule 4-hour execution block  
**Owner:** Development Team  
**Priority:** High
