# Agent-1: Graphite Splitter Task

**Date:** 2025-10-08
**Assigned To:** Agent-1 (Splitter)
**Estimated Duration:** 2-3 hours
**Status:** Ready for Execution

---

## Task Overview

You are tasked with splitting the monolithic refactor branch `anchor/oct-06-2025` into 9 domain-based PRs using Graphite.

## Context
- **Branch:** `anchor/oct-06-2025` (93 files, 33 commits)
- **Target:** 9 stacked PRs organized by crate/domain boundaries
- **Reference Docs:** `docs/review/split-plan-comparison/` explains WHY this domain split was chosen over alternatives
- **Execution Plan:** `docs/plans/graphite-split-execution-plan.md` contains full workflow

---

## Pre-flight Checklist

1. Ensure you're on `anchor/oct-06-2025` branch
2. Verify branch is rebased on latest main: `git fetch origin && git rebase origin/main`
3. Create backup: `git branch backup/anchor-oct-06-2025-$(date +%Y%m%d-%H%M%S)`
4. Clean working tree: `git status` (should be clean)

---

## Execution Steps

### Step 1: Adopt branch into Graphite
```bash
gt track
```

### Step 2: Interactive Split by Hunk
```bash
gt split --by-hunk
```

During the interactive split, create these 9 branches IN ORDER and assign hunks according to this COMPLETE routing matrix:

### PRIMARY ROUTING MATRIX:
```
config/**                                → 01-config-settings
crates/app/src/lib.rs                   → 01-config-settings
crates/app/src/main.rs                  → 01-config-settings
crates/coldvox-foundation/**            → 01-config-settings
crates/app/tests/settings_test.rs       → 01-config-settings

crates/coldvox-audio/**                 → 02-audio-capture
crates/app/src/audio/mod.rs            → 02-audio-capture
crates/app/src/audio/vad_adapter.rs    → 02-audio-capture
crates/app/src/audio/vad_processor.rs  → 02-audio-capture

crates/coldvox-vad/**                   → 03-vad
crates/coldvox-vad-silero/**           → 03-vad
crates/app/src/vad.rs (if exists)      → 03-vad

crates/coldvox-stt/**                   → 04-stt
crates/coldvox-stt-vosk/**             → 04-stt
crates/app/src/stt/processor.rs        → 04-stt
crates/app/src/stt/vosk.rs            → 04-stt
crates/app/src/stt/persistence.rs      → 04-stt
crates/app/src/stt/plugin_manager.rs   → 04-stt
crates/app/src/stt/session.rs         → 04-stt
crates/app/src/stt/types.rs           → 04-stt

crates/app/src/runtime.rs              → 05-app-runtime-wav
crates/app/src/audio/wav_file_loader.rs → 05-app-runtime-wav
crates/app/src/stt/tests/end_to_end_wav.rs → 05-app-runtime-wav

crates/coldvox-text-injection/**       → 06-text-injection

**/tests/** (EXCEPT settings_test.rs and end_to_end_wav.rs) → 07-testing
crates/app/tests/** (EXCEPT settings_test.rs) → 07-testing
examples/**                             → 07-testing

crates/coldvox-telemetry/**            → 08-logging-observability
Logging changes in any file            → 08-logging-observability
crates/app/src/bin/*.rs                → 08-logging-observability

docs/**                                 → 09-docs-changelog
CHANGELOG.md                           → 09-docs-changelog
README.md                               → 09-docs-changelog
CLAUDE.md                               → 09-docs-changelog
agents.md                               → 09-docs-changelog
.github/**                              → 09-docs-changelog
Cargo.lock                              → 09-docs-changelog
```

### EDGE CASE HANDLING:
- **Mixed hunks:** If a hunk contains changes for multiple domains, prefer the "higher" domain (later in stack)
- **Cargo.toml files:** Route to the PR that owns that crate
- **Root Cargo.toml:** Route to 01-config-settings
- **Uncertain files:** Keep a scratch log for review notes

### Step 3: Verify Stack Order
```bash
gt log
```

Expected output should show:
```
09-docs-changelog
08-logging-observability
07-testing
06-text-injection
05-app-runtime-wav
04-stt
03-vad
02-audio-capture
01-config-settings
main
```

If order is wrong:
```bash
gt reorder
```

### Step 4: Push All Branches
```bash
git push --all origin
```

### Step 5: Create Validation Log
Create `/tmp/split-validation.log` with:
- List of branches created
- Any ambiguous hunks and decisions made
- Files that were difficult to categorize
- Total LOC per branch (rough estimate)

---

## Success Criteria
- [ ] 9 branches created with correct names
- [ ] All hunks assigned (no uncommitted changes)
- [ ] `gt log` shows correct stack order
- [ ] Each branch builds: `git checkout <branch> && cargo check`
- [ ] No crate is split across non-adjacent PRs
- [ ] `/tmp/split-validation.log` created

---

## Reference Domain Analysis

Before making routing decisions, review:
- `docs/review/split-plan-comparison/dependency-graph-comparison.md` - Shows why this split minimizes conflicts
- `docs/review/split-plan-comparison/refactor-split-strategy-comparison.md` - Explains domain boundaries
- `docs/plans/graphite-split-execution-plan.md` - Full execution workflow with all phases

---

## Notes

- This is a 2-3 hour task requiring careful attention
- Keep the execution plan (`docs/plans/graphite-split-execution-plan.md`) open for reference
- If you encounter merge conflicts during split, abort and ensure branch is cleanly rebased first
- Document any edge cases or difficult decisions in `/tmp/split-validation.log`
- The routing matrix is based on architectural analysis documented in `docs/review/split-plan-comparison/`

---

## Next Phase

After successful completion:
- **Phase 2:** Validation (Agents 2-6 run parallel validation)
- **Phase 3:** PR Creation (Agent-7 creates all PRs)
- **Phase 4:** Review (Agents 8-16 domain reviews)
- **Phase 5:** Merge Coordination (Agent-17 sequential merges)
- **Phase 6:** Cleanup (Async-Editor + Team)

---

**Signed:** Claude (Sonnet 4.5)
**Generated:** 2025-10-08T04:35:00Z
**Execution Plan:** `docs/plans/graphite-split-execution-plan.md`
