# Refactor Split Strategy Comparison & Grade

**Date:** 2025-10-08  
**Context:** Evaluating two strategies for splitting the `anchor/oct-06-2025` refactor branch (93 files, 33 commits) into reviewable stacked PRs.

---

## Executive Summary

**Grade for Domain-Based Plan (Plan 2): A-**

The domain-based plan is **significantly superior** to the fix/feature-based plan and represents best practices for large-scale refactoring. It follows proper architectural boundaries, minimizes cross-cutting changes, and creates a natural dependency graph that matches the codebase structure.

**Grade for Fix/Feature-Based Plan (Plan 1): C+**

While the fix/feature-based plan attempts to prioritize critical fixes first, it creates artificial ordering constraints and mixes architectural concerns in ways that complicate reviews and increase merge conflict risk.

---

## Plan Comparison Matrix

| Criteria | Plan 1 (Fix/Feature) | Plan 2 (Domain) | Winner |
|----------|---------------------|-----------------|---------|
| **Architectural Coherence** | Mixed domains per PR | Clean domain boundaries | **Plan 2** |
| **Review Complexity** | Low (small fixes first) → High (runtime changes) | Consistent (domain experts per PR) | **Plan 2** |
| **Dependency Clarity** | Artificial (fixes before features) | Natural (config → consumers) | **Plan 2** |
| **Merge Conflict Risk** | High (cross-cutting changes delayed) | Low (isolated domains) | **Plan 2** |
| **Bisectability** | Poor (mixed concerns) | Excellent (domain-isolated) | **Plan 2** |
| **Graphite Workflow Fit** | Requires complex hunk splitting | Natural domain clustering | **Plan 2** |
| **Testing Isolation** | Tests mixed with features | Tests in dedicated PR | **Plan 2** |
| **Documentation Sync** | Docs scattered across PRs | Docs in final PR | **Plan 2** |

---

## Detailed Analysis

### Plan 1 (Fix/Feature-Based): 10-Step Stack

```
1. fix/test-infrastructure
2. fix/clipboard-paste-p0
3. fix/clipboard-restore-p1
4. refactor/config-system
5. refactor/text-injection-strategy
6. feat/audio-stability
7. feat/vad-determinism
8. feat/wav-loader-e2e
9. refactor/runtime-unification
10. docs/deployment-config
```

#### Strengths ✅
- **Prioritizes critical fixes**: P0/P1 bugs land first
- **Incremental risk**: Small, safe changes before large refactors
- **Clear urgency**: Reviewers know what's critical vs. nice-to-have

#### Weaknesses ❌
- **Artificial ordering**: Config refactor depends on test fixes (unnatural dependency)
- **Cross-cutting delays**: Runtime unification delayed until PR #9, but many changes touch runtime
- **Mixed concerns**: Text injection split across PRs #2, #3, and #5
- **Dependency confusion**: Does audio stability depend on config? Does VAD depend on audio? Unclear from order.
- **Review fatigue**: Reviewers must context-switch between domains
- **Merge conflicts**: Delayed runtime changes create rebase hell for audio/VAD/STT PRs

#### Repository Structure Mismatch
Looking at the codebase structure in `CLAUDE.md`:
```
crates/
├── coldvox-foundation/
├── coldvox-audio/
├── coldvox-vad/
├── coldvox-vad-silero/
├── coldvox-stt/
├── coldvox-stt-vosk/
├── coldvox-text-injection/
├── coldvox-telemetry/
└── app/
```

Plan 1 **cuts across** these natural boundaries:
- `fix/clipboard-paste-p0` + `fix/clipboard-restore-p1` + `refactor/text-injection-strategy` all touch `coldvox-text-injection/`
- This creates **3 sequential PRs** modifying the same crate, increasing conflict risk

---

### Plan 2 (Domain-Based): 9-Step Stack

```
01. config: centralize Settings + path-aware load
02. audio: capture lifecycle fix + ALSA stderr suppression
03. vad: windowing/debounce consistency
04. stt: finalize handling + helpers
05. app: unify VAD↔STT runtime + real WAV loader
06. injection: clipboard-preserve + Wayland-first strategy
07. tests: deterministic E2E + integration suites
08. logs: prune noisy hot paths; telemetry tweaks
09. docs: changelog + guides + fixes
```

#### Strengths ✅
- **Natural dependencies**: Config → Audio → VAD → STT → App → Injection (follows data flow)
- **Crate isolation**: Each PR maps to 1-2 crates (clean ownership)
- **Domain expertise**: Can assign PRs to crate maintainers
- **Parallel work possible**: Audio and VAD PRs can be developed simultaneously (both depend on config only)
- **Testing coherence**: All deterministic testing in PR #7 (no test changes scattered across PRs)
- **Documentation sync**: All doc updates in final PR (single source of truth for changelog)
- **Graphite-friendly**: `gt split --by-hunk` naturally clusters by file path

#### Weaknesses ❌
- **P0 bug delay**: Critical clipboard paste fix doesn't land until PR #6
- **Test failures early**: Tests might fail in PRs #1-5 if they depend on fixes in PR #7
- **Initial review overhead**: PR #1 (config) may be large/complex
- **Integration risk**: Runtime unification (PR #5) is mid-stack, potential merge conflicts

#### Mitigation Strategies
Plan 2 can address weaknesses:
1. **Hot-fix branch**: Extract clipboard P0 from PR #6, land as PR #0 before config
2. **Test fixture updates**: Include minimal test fixes in PR #1 (config) to keep CI green
3. **Config PR split**: If PR #1 is too large, split into `config-core` + `config-integration`

---

## Repository Context Analysis

### Workspace Structure Alignment

From `CLAUDE.md`, the repository is a **multi-crate workspace** with clear architectural layers:

```
Foundation → Audio → VAD/STT → App → Injection
```

**Plan 2 follows this structure perfectly:**
- PR #1 (config) → Foundation layer
- PR #2 (audio) → Audio layer
- PR #3 (vad) + PR #4 (stt) → Processing layers (parallel-safe)
- PR #5 (app runtime) → Integration layer
- PR #6 (injection) → Output layer

**Plan 1 violates this structure:**
- Mixes foundation (test infra) with output (injection fixes)
- Delays app runtime changes until PR #9 (should be earlier per architecture)

### File Path Analysis

Based on problem statement paths:

**Plan 1 path grouping:**
```
PR #1: crates/app/tests/settings_test.rs (scattered)
PR #2: crates/coldvox-text-injection/src/clipboard_paste_injector.rs
PR #3: crates/coldvox-text-injection/src/clipboard_paste_injector.rs (duplicate!)
PR #4: crates/app/src/lib.rs, config/**
PR #5: crates/coldvox-text-injection/src/manager.rs (duplicate crate!)
```

**Plan 2 path grouping:**
```
PR #1: crates/app/src/lib.rs, config/**, tests (cohesive)
PR #2: crates/coldvox-audio/** (single crate)
PR #3: crates/coldvox-vad/** (single crate)
PR #6: crates/coldvox-text-injection/** (single crate, all changes)
```

**Winner:** Plan 2 avoids duplicate crate modifications.

---

## Graphite Workflow Evaluation

### `gt split --by-hunk` Efficiency

**Plan 1:** Requires **manual hunk selection**
- Reviewer must decide: "Is this test change part of test-infrastructure or config-system?"
- Clipboard changes split across 3 PRs requires careful hunk assignment
- High cognitive load during interactive split

**Plan 2:** **Natural path-based clustering**
- Hunks in `crates/coldvox-audio/` → PR #2 (obvious)
- Hunks in `config/` → PR #1 (obvious)
- Minimal ambiguity (only glue code in PR #10 if needed)

### `gt reorder` and `gt sync` Impact

**Plan 1:**
- After PR #2 (clipboard P0) merges, PR #3 (clipboard P1) has conflicts → rebase
- After PR #3 merges, PR #5 (text-injection refactor) has conflicts → rebase
- **3 sequential rebases** for one crate

**Plan 2:**
- After PR #1 (config) merges, PRs #2-4 rebase once (they all depend on config)
- After PR #5 (runtime) merges, PR #6 rebases once
- **2 rebase waves** for entire stack

---

## Testing Strategy Comparison

### Plan 1: Distributed Testing
- PR #1: Settings tests
- PR #8: E2E WAV tests
- Other PRs: Implicit test updates

**Risk:** Test changes interleaved with feature changes complicate rollback

### Plan 2: Consolidated Testing
- PR #7: ALL deterministic testing infrastructure
- Other PRs: Minimal test adjustments

**Benefit:** Can review test changes independently; easier to validate test suite reliability

---

## Documentation Strategy Comparison

### Plan 1: Scattered Docs
- PR #4: Config docs
- PR #6: Audio docs
- PR #10: Deployment docs

**Risk:** Changelog spans multiple PRs; hard to generate release notes

### Plan 2: Unified Docs
- PR #9: All docs, changelog, guides

**Benefit:** Single PR for release note generation; easier to audit documentation completeness

---

## Recommendations

### Short-Term (Current Refactor)

**Adopt Plan 2 with modifications:**

```bash
# Recommended 10-branch stack:
00-hotfix-clipboard-p0          # Extract critical bug fix
01-config-settings              # Foundation
02-audio-capture                # Layer 1
03-vad                          # Layer 2 (parallel with 04)
04-stt                          # Layer 2 (parallel with 03)
05-app-runtime-wav              # Integration
06-text-injection               # Output
07-testing                      # Infrastructure
08-logging-observability        # Infrastructure
09-docs-changelog               # Documentation
```

**Changes from Plan 2:**
1. **Add PR #0**: Extract clipboard P0 fix (10 lines) as hot-fix
2. **Keep rest of Plan 2 intact**: Natural domain boundaries

### Execution Steps

```bash
# 1. Backup
git checkout anchor/oct-06-2025
git branch backup/anchor-oct-06-2025-$(date +%Y%m%d-%H%M%S)

# 2. Track and split
gt track
gt split --by-hunk

# During interactive split, use path-based heuristics:
# - config/** → 01-config-settings
# - crates/coldvox-audio/** → 02-audio-capture
# - crates/coldvox-vad*/** → 03-vad
# - crates/coldvox-stt*/** → 04-stt
# - crates/app/src/runtime.rs, wav_file_loader.rs → 05-app-runtime-wav
# - crates/coldvox-text-injection/** → 06-text-injection
# - **/tests/** → 07-testing
# - logging/tracing calls → 08-logging-observability
# - docs/**, CHANGELOG* → 09-docs-changelog

# 3. Reorder to match dependency graph
gt reorder

# 4. Validate each branch
gt checkout 01-config-settings
cargo build && cargo test
gt up  # repeat for each branch

# 5. Push stack
git push --all

# 6. Create PRs
gt submit  # or manual gh pr create with proper --base
```

---

## Long-Term Recommendations

### For Future Refactors

1. **Use proactive stacking**: Build with `gt create` as you code (not retroactive split)
2. **Follow workspace structure**: One PR per crate when possible
3. **Config changes first**: Always land foundation changes before consumers
4. **Parallel-safe layers**: Group independent crates (VAD + STT) at same level
5. **Testing last**: Consolidate test infrastructure in final PR before merge

### Workflow Improvements

Add to `docs/dev/graphite-workflow.md`:
- Domain-based split checklist
- Path glob templates for common splits
- Dependency graph visualization

Add to `.github/PULL_REQUEST_TEMPLATE.md`:
- "Which crate(s) does this PR modify?"
- "What is this PR's position in the stack?"

---

## Verdict

**Plan 2 wins decisively** because it:
1. **Matches repository structure** (multi-crate workspace)
2. **Minimizes merge conflicts** (domain isolation)
3. **Simplifies reviews** (domain experts per PR)
4. **Enables parallel work** (independent layers)
5. **Works with Graphite** (path-based clustering)

**Grade: A-** (deducted 0.5 for P0 bug delay, which is easily fixed with PR #0)

**Plan 1 Grade: C+** (well-intentioned but architecturally unsound)

---

## Appendix: PR Title Templates (Plan 2)

Ready to paste into Graphite or GitHub:

```
[00] hotfix(injection): clipboard paste actually issues Ctrl+V
[01] config: centralize Settings + path-aware load
[02] audio: capture lifecycle fix + ALSA stderr suppression
[03] vad: windowing/debounce consistency
[04] stt: finalize handling + helpers
[05] app: unify VAD↔STT runtime + real WAV loader
[06] injection: clipboard-preserve + Wayland-first strategy
[07] tests: deterministic E2E + integration suites
[08] logs: prune noisy hot paths; telemetry tweaks
[09] docs: changelog + guides + fixes
```

---

**Author:** GitHub Copilot Coding Agent  
**Review Status:** Ready for stakeholder sign-off
