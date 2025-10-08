# Refactor Split Strategy Comparison

**Date:** 2025-10-08  
**Status:** Complete  
**Recommendation:** Adopt Plan 2 (Domain-Based) with grade **A-**

---

## Quick Summary

This directory contains a comprehensive analysis comparing two strategies for splitting the `anchor/oct-06-2025` refactor branch into reviewable stacked PRs:

- **Plan 1 (Fix/Feature-Based):** Grade **C+**
- **Plan 2 (Domain-Based):** Grade **A-** ✅ **RECOMMENDED**

---

## Documents in This Directory

### 1. [refactor-split-strategy-comparison.md](./refactor-split-strategy-comparison.md)
**Main analysis document** with detailed comparison matrix, strengths/weaknesses analysis, and final verdict.

**Key Findings:**
- Plan 2 respects crate boundaries (multi-crate workspace structure)
- Plan 2 minimizes merge conflicts (domain isolation)
- Plan 2 enables parallel development (independent layers)
- Plan 2 simplifies reviews (domain experts per PR)
- Plan 2 works seamlessly with Graphite (`gt split --by-hunk`)

**Grade Breakdown:**
- Plan 1: C+ (well-intentioned but architecturally unsound)
- Plan 2: A- (minor deduction for P0 bug delay, easily fixed with PR #0)

### 2. [dependency-graph-comparison.md](./dependency-graph-comparison.md)
**Visual dependency analysis** showing how each plan structures the PR stack.

**Key Visualizations:**
- ASCII dependency graphs for both plans
- Merge conflict analysis (Plan 1: 5-7 conflicts, Plan 2: 2-3 conflicts)
- Review complexity comparison (Plan 1: 10 context switches, Plan 2: 1 per domain)
- Crate-level architecture matching

### 3. [execution-guide.md](./execution-guide.md)
**Step-by-step implementation guide** for executing Plan 2.

**Includes:**
- Pre-flight checklist
- Graphite CLI workflow (`gt track` → `gt split` → `gt reorder`)
- Path-based hunk assignment rules
- Per-branch validation scripts
- PR creation templates (manual + automated)
- Troubleshooting common issues
- Timeline estimate (3.5-4 hours first-time)

---

## Executive Recommendation

### Adopt Plan 2 with Modifications

**Recommended 10-branch stack:**
```
00. hotfix-clipboard-p0          ← Extract critical P0 bug fix (NEW)
01. config-settings              ← Foundation
02. audio-capture                ← Layer 1
03. vad                          ← Layer 2 (parallel with 04)
04. stt                          ← Layer 2 (parallel with 03)
05. app-runtime-wav              ← Integration
06. text-injection               ← Output
07. testing                      ← Infrastructure
08. logging-observability        ← Infrastructure
09. docs-changelog               ← Documentation
```

**Key Change from Original Plan 2:** Add PR #0 to address P0 clipboard bug immediately.

---

## Why Plan 2 Wins

### 1. Architectural Coherence
Matches ColdVox's multi-crate workspace structure:
```
Foundation → Audio → VAD/STT → App → Injection
```

### 2. Conflict Minimization
- Plan 1: 5-7 major rebases (text-injection modified 3× sequentially)
- Plan 2: 2-3 minor rebases (each crate modified once)

### 3. Review Efficiency
- Plan 1: 10 context switches (reviewers jump between domains)
- Plan 2: 1 context per PR (domain experts assigned)

### 4. Parallel Development
- Plan 1: Strict serial order (blocks parallelism)
- Plan 2: VAD + STT can develop in parallel (both depend on audio only)

### 5. Graphite Workflow Fit
- Plan 1: Complex hunk assignment (ambiguous decisions)
- Plan 2: Natural path-based clustering (`crates/coldvox-audio/**` → PR #2)

---

## Quick Start

To execute Plan 2 immediately:

```bash
# 1. Install Graphite
npm install -g @withgraphite/graphite-cli@latest

# 2. Backup current branch
git checkout anchor/oct-06-2025
git branch backup/anchor-oct-06-2025-$(date +%Y%m%d-%H%M%S)

# 3. Track and split
gt track
gt split --by-hunk  # Follow path-based rules in execution-guide.md

# 4. Validate stack
gt log  # Visual check
cargo test --workspace  # Per-branch validation

# 5. Push and create PRs
git push --all
gt submit  # Or manual gh pr create
```

**Full details:** See [execution-guide.md](./execution-guide.md)

---

## Comparison Matrix (At a Glance)

| Criteria | Plan 1 | Plan 2 | Winner |
|----------|--------|--------|--------|
| Architectural Coherence | Mixed | Clean | Plan 2 |
| Review Complexity | High | Low | Plan 2 |
| Merge Conflicts | 5-7 | 2-3 | Plan 2 |
| Parallel Work | No | Yes | Plan 2 |
| CI Stability | 3-4 failures | 0-1 failures | Plan 2 |
| Graphite Fit | Poor | Excellent | Plan 2 |

---

## Key Insights from Repository Analysis

### Workspace Structure (from `CLAUDE.md`)
```
crates/
├── coldvox-foundation/       → Foundation layer
├── coldvox-audio/            → Audio layer
├── coldvox-vad(-silero)/     → Processing layer
├── coldvox-stt(-vosk)/       → Processing layer
├── coldvox-text-injection/   → Output layer
├── coldvox-telemetry/        → Infrastructure
└── app/                      → Integration layer
```

**Plan 2 matches this structure perfectly.** Each PR maps to 1-2 crates, respecting natural boundaries.

### Development Commands (from `CLAUDE.md`)
```bash
# Build & test per crate (Plan 2 friendly)
cargo build -p coldvox-audio
cargo test -p coldvox-vad

# Workspace build (validates integration)
cargo build --workspace
cargo test --workspace
```

**Plan 2 enables per-crate validation**, making CI failures easier to diagnose and fix.

---

## Supporting Context

### Referenced in Problem Statement
- **Graphite documentation:** Context about `gt split`, `gt track`, `gt reorder` commands
- **Repository structure:** Multi-crate workspace with clear architectural layers
- **Current refactor:** 93 files, 33 commits on `anchor/oct-06-2025`
- **Testing requirements:** Deterministic E2E tests with WAV files

### Additional Resources
- [CLAUDE.md](../../../CLAUDE.md): Workspace structure and development commands
- [docs/review_plan.md](../../review_plan.md): Review objectives for refactor branch
- [docs/refactoring_and_integration_plan.md](../../refactoring_and_integration_plan.md): Strategic refactoring history

---

## Feedback Loop

After executing Plan 2, document lessons learned:
- **What worked well?** (e.g., path-based hunk assignment)
- **What was challenging?** (e.g., glue code classification)
- **How long did it take?** (actual vs. estimated 3.5-4 hours)
- **Merge conflict count?** (actual vs. predicted 2-3)

**Location for feedback:** `docs/review/split-plan-comparison/execution-feedback.md` (create after execution)

---

## Conclusion

**Adopt Plan 2 (Domain-Based) for the `anchor/oct-06-2025` refactor split.**

This strategy:
- Respects repository architecture
- Minimizes reviewer cognitive load
- Reduces merge conflict risk
- Enables parallel development
- Works seamlessly with Graphite tooling

**Grade: A-** (excellent strategy with minor room for improvement)

---

**Author:** GitHub Copilot Coding Agent  
**Review Status:** Ready for stakeholder sign-off  
**Next Action:** Execute Plan 2 using [execution-guide.md](./execution-guide.md)
