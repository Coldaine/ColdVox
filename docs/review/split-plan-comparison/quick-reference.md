# Quick Reference: Plan Comparison

## One-Page Summary

### Plan 1: Fix/Feature-Based (Grade: C+)

```
10. docs/deployment-config
 9. runtime-unification          ← Large refactor, late in stack
 8. wav-loader-e2e
 7. vad-determinism
 6. audio-stability
 5. text-injection-strategy      ← 3rd change to text-injection crate
 4. config-system
 3. clipboard-restore-p1         ← 2nd change to text-injection crate
 2. clipboard-paste-p0           ← 1st change to text-injection crate
 1. test-infrastructure
    └─ main
```

**Problems:**
- ❌ Text-injection crate modified 3 times sequentially (PRs #2, #3, #5)
- ❌ Runtime refactor delayed until PR #9 (blocks earlier PRs)
- ❌ Cross-cutting changes increase merge conflicts
- ❌ Mixed concerns (tests in PR #1, features in PR #8)
- ❌ 10 context switches for reviewers

**Predicted Metrics:**
- Merge conflicts: 5-7
- CI failures: 3-4 PRs
- Review time: 3-4 weeks (serial reviews)

---

### Plan 2: Domain-Based (Grade: A-)

```
09. docs-changelog
08. logging-observability
07. testing                      ← All test changes consolidated
06. text-injection              ← Single PR, all changes
05. app-runtime-wav             ← Integration layer
 ├─ 04. stt                     ← Parallel-safe
 └─ 03. vad                     ← Parallel-safe
02. audio-capture
01. config-settings             ← Foundation
    └─ main
```

**Benefits:**
- ✅ Each crate modified once (domain isolation)
- ✅ Natural dependency graph (config → audio → vad/stt → app → injection)
- ✅ Parallel development (vad + stt can work simultaneously)
- ✅ Clean reviews (1 domain expert per PR)
- ✅ Graphite-friendly (path-based hunk clustering)

**Predicted Metrics:**
- Merge conflicts: 2-3
- CI failures: 0-1 PRs
- Review time: 1-2 weeks (parallel reviews possible)

---

## Side-by-Side Comparison

| Aspect | Plan 1 | Plan 2 | Winner |
|--------|--------|--------|--------|
| **Stack Size** | 10 PRs | 9 PRs (+ optional PR #0) | Tie |
| **Crate Isolation** | ❌ Mixed | ✅ Clean | **Plan 2** |
| **text-injection edits** | 3 PRs | 1 PR | **Plan 2** |
| **runtime edits** | PR #9 (late) | PR #5 (mid-stack) | **Plan 2** |
| **Parallel work** | ❌ Serial | ✅ VAD+STT parallel | **Plan 2** |
| **Review context** | 10 switches | 9 switches (1/PR) | **Plan 2** |
| **Merge conflicts** | 5-7 predicted | 2-3 predicted | **Plan 2** |
| **CI stability** | 3-4 failures | 0-1 failures | **Plan 2** |
| **Graphite fit** | Poor | Excellent | **Plan 2** |
| **P0 bug timing** | Early (PR #2) | Late (PR #6) | **Plan 1** |
| **Test organization** | Scattered | Consolidated (PR #7) | **Plan 2** |
| **Documentation** | Scattered | Consolidated (PR #9) | **Plan 2** |

**Score: Plan 2 wins 9-1** (only loses on P0 bug timing, which is easily fixed)

---

## Modified Plan 2 (Recommended)

Add PR #0 to extract P0 bug fix:

```
00. hotfix-clipboard-p0         ← NEW: Extract critical bug
01. config-settings
02. audio-capture
 ├─ 03. vad                     ← Parallel-safe
 └─ 04. stt                     ← Parallel-safe
05. app-runtime-wav
06. text-injection
07. testing
08. logging-observability
09. docs-changelog
    └─ main
```

**Now Plan 2 wins 10-0!**

---

## Graphite Commands Cheat Sheet

```bash
# Setup
gt track                         # Adopt existing branch into stack
gt split --by-hunk              # Interactive split by hunks
gt split --by-commit            # Split by commit (if pre-clustered)

# Navigation
gt log                          # Visualize stack
gt checkout <branch>            # Switch to branch
gt up / gt down                 # Navigate relatives

# Modification
gt reorder                      # Interactive reorder
gt move --onto <base>           # Re-parent current branch
gt create --insert              # Insert branch mid-stack
gt fold                         # Merge child into parent

# Maintenance
gt sync                         # Pull trunk + auto-restack
gt restack                      # Explicit restack (after conflicts)
gt continue                     # Continue after conflict resolution

# Publishing
git push --all                  # Push all branches
gt submit                       # Create PRs (Graphite Cloud)
```

---

## Path-Based Hunk Assignment Rules (Plan 2)

Use these during `gt split --by-hunk`:

| File Path | Branch | Priority |
|-----------|--------|----------|
| `config/**` | 01-config-settings | 1 |
| `crates/app/src/lib.rs` (Settings) | 01-config-settings | 1 |
| `crates/coldvox-audio/**` | 02-audio-capture | 2 |
| `crates/coldvox-vad*/**` | 03-vad | 3 |
| `crates/coldvox-stt*/**` | 04-stt | 4 |
| `crates/app/src/runtime.rs` | 05-app-runtime-wav | 5 |
| `crates/app/src/audio/wav_file_loader.rs` | 05-app-runtime-wav | 5 |
| `crates/coldvox-text-injection/**` | 06-injection | 6 |
| `**/tests/**` | 07-testing | 7 |
| `crates/coldvox-telemetry/**` | 08-logging-observability | 8 |
| Logging changes (scattered) | 08-logging-observability | 8 |
| `docs/**`, `CHANGELOG*` | 09-docs-changelog | 9 |

**Rule:** If path matches multiple patterns, choose by priority (lower number = earlier in stack).

---

## Time Estimates

| Phase | Plan 1 | Plan 2 | Notes |
|-------|--------|--------|-------|
| Interactive split | 90-120 min | 60-90 min | Plan 2: clearer path rules |
| Validation | 100 min (10 PRs) | 90 min (9 PRs) | 10 min per branch |
| Conflict resolution | 60-90 min | 20-30 min | Plan 2: fewer conflicts |
| Review time (team) | 3-4 weeks | 1-2 weeks | Plan 2: parallel reviews |
| **Total (solo)** | **4.5-5.5 hours** | **3-4 hours** | First-time execution |
| **Total (team)** | **3-4 weeks** | **1-2 weeks** | Including review cycles |

---

## Decision Matrix

Use this to choose between plans:

| Your Priority | Choose Plan 1 if... | Choose Plan 2 if... | Recommended |
|---------------|---------------------|---------------------|-------------|
| **Speed** | You need P0 bug ASAP | You value overall velocity | **Plan 2** + PR #0 |
| **Quality** | - | ✓ (better reviews) | **Plan 2** |
| **CI Stability** | - | ✓ (fewer failures) | **Plan 2** |
| **Team Size** | Solo developer | 2+ developers | **Plan 2** |
| **Merge Conflicts** | - | ✓ (fewer conflicts) | **Plan 2** |
| **Architectural Clarity** | - | ✓ (crate boundaries) | **Plan 2** |
| **Learning Graphite** | - | ✓ (easier workflow) | **Plan 2** |
| **Risk Tolerance** | Low (fixes first) | High (refactor first) | **Plan 2** + PR #0 |

**Verdict: Plan 2 wins in 7/8 categories.**

---

## Action Items

- [ ] Review this comparison with team
- [ ] Get stakeholder sign-off on Plan 2
- [ ] Install Graphite CLI: `npm install -g @withgraphite/graphite-cli@latest`
- [ ] Schedule 4-hour block for split execution
- [ ] Follow [execution-guide.md](./execution-guide.md)
- [ ] Create feedback document after execution
- [ ] Update project workflow docs with lessons learned

---

## Quick Links

- 📊 [Full Comparison](./refactor-split-strategy-comparison.md)
- 🔀 [Dependency Graphs](./dependency-graph-comparison.md)
- 📖 [Execution Guide](./execution-guide.md)
- 📋 [Overview README](./README.md)

---

**Last Updated:** 2025-10-08  
**Author:** GitHub Copilot Coding Agent
