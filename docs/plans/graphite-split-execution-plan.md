# Graphite Split Execution Plan: anchor/oct-06-2025 → 9 Stacked PRs

**Date:** 2024-10-07
**Status:** Ready for Execution
**Branch to Split:** `anchor/oct-06-2025` (93 files, 33 commits)
**Target:** 9 domain-based PRs using Graphite stacked workflow

---

## Quick Reference: Agent Cheat Sheet

```
Phase 1 - Split:
  Agent-1: Splitter (gt split --by-hunk, solo, 2-3h)

Phase 2 - Validation (parallel):
  Agent-2: Validator-Foundation (PR #01)
  Agent-3: Validator-Audio (PR #02)
  Agent-4: Validator-Processing (PR #03, #04)
  Agent-5: Validator-Integration (PR #05, #06, #07)
  Agent-6: Validator-Infra (PR #08, #09)

Phase 3 - PR Creation:
  Agent-7: PR-Creator (gh pr create ×9, solo, 30min)

Phase 4 - Review (parallel where safe):
  Agent-8:  Reviewer-Config (PR #01)
  Agent-9:  Reviewer-Audio (PR #02)
  Agent-10: Reviewer-VAD (PR #03) ← parallel with Agent-11
  Agent-11: Reviewer-STT (PR #04) ← parallel with Agent-10
  Agent-12: Reviewer-Runtime (PR #05)
  Agent-13: Reviewer-Injection (PR #06)
  Agent-14: Reviewer-Testing (PR #07)
  Agent-15: Reviewer-Observability (PR #08)
  Agent-16: Reviewer-Docs (PR #09)

Phase 5 - Merge:
  Agent-17: Merge-Coordinator (sequential, CI-gated, 1-2 weeks)

Total: 17 agents (max 5 concurrent in Phase 2, max 2 concurrent in Phase 4)
```

---

## Executive Summary

This plan splits the monolithic refactor branch into 9 reviewable PRs organized by crate/domain boundaries. Each PR maps to the multi-crate workspace structure, minimizing merge conflicts and enabling parallel development where possible.

**Key Decision:** Skip P0 extraction. The text injection improvements (AT-SPI/ydotool alternatives) will be addressed in a **post-refactor follow-up** after the stack merges.

---

## The Stack (9 PRs)

```
main
 └─ 01-config-settings
     └─ 02-audio-capture
         ├─ 03-vad (parallel with 04)
         └─ 04-stt (parallel with 03)
             └─ 05-app-runtime-wav (waits for BOTH 03 & 04)
                 └─ 06-text-injection
                     └─ 07-testing
                         └─ 08-logging-observability
                             └─ 09-docs-changelog
```

**Parallel-Safe:** PRs #03 and #04 can be reviewed simultaneously (both depend only on #02).

---

## PR Breakdown

### PR #01: config-settings
**Title:** `[01/09] config: centralize Settings + path-aware load`
**Base:** `main`
**Scope:** `crates/app/src/lib.rs`, `config/**`, `crates/app/tests/settings_test.rs`
**Size Estimate:** Medium (200-400 lines)

**Changes:**
- Centralize configuration loading with path-aware logic
- Add `COLDVOX_CONFIG_PATH` environment variable override
- Update Settings API for deterministic testing
- Add TOML config files (`config/default.toml`, `config/overrides.toml`)

**Why First:** Config is foundation; all other crates consume it.

**Validation:**
```bash
cargo test --test settings_test
cargo build -p coldvox-app
# Verify config loading: COLDVOX_CONFIG_PATH=config/test.toml cargo run
```

---

### PR #02: audio-capture
**Title:** `[02/09] audio: capture lifecycle fix + ALSA stderr suppression`
**Base:** `01-config-settings`
**Scope:** `crates/coldvox-audio/**`
**Size Estimate:** Medium (300-500 lines)

**Changes:**
- Audio capture thread lifecycle improvements
- Device monitor enhancements (PipeWire priority)
- ALSA stderr suppression (reduces log noise)
- Watchdog stability fixes

**Why Second:** Audio is the first processing layer after config.

**Validation:**
```bash
cargo test -p coldvox-audio
cargo run --bin mic_probe -- --duration 30
# Verify: PipeWire FPS stable, no ALSA stderr spam
```

---

### PR #03: vad
**Title:** `[03/09] vad: windowing/debounce consistency`
**Base:** `02-audio-capture`
**Scope:** `crates/coldvox-vad/**`, `crates/coldvox-vad-silero/**`
**Size Estimate:** Small-Medium (150-300 lines)
**Parallel-Safe:** ✅ Can review with PR #04

**Changes:**
- Frame-based VAD debouncing for deterministic testing
- Timestamp-ms candidates for reproducibility
- Windowing consistency improvements

**Why This Order:** VAD processes audio frames; independent of STT.

**Validation:**
```bash
cargo test -p coldvox-vad
cargo test -p coldvox-vad-silero
cargo run --example test_silero_wav --features examples
```

---

### PR #04: stt
**Title:** `[04/09] stt: finalize handling + helpers`
**Base:** `02-audio-capture`
**Scope:** `crates/coldvox-stt/**`, `crates/coldvox-stt-vosk/**`
**Size Estimate:** Small-Medium (150-300 lines)
**Parallel-Safe:** ✅ Can review with PR #03

**Changes:**
- STT finalization behavior improvements
- Helper utilities for transcription processing
- Session event handling refinements

**Why This Order:** STT processes audio frames; independent of VAD.

**Validation:**
```bash
cargo test -p coldvox-stt
cargo test -p coldvox-stt-vosk --features vosk
cargo run --features vosk --example vosk_test
```

---

### PR #05: app-runtime-wav
**Title:** `[05/09] app: unify VAD↔STT runtime + real WAV loader`
**Base:** `02-audio-capture` (rebase after #03 & #04 merge)
**Scope:** `crates/app/src/runtime.rs`, `crates/app/src/audio/wav_file_loader.rs`, E2E glue
**Size Estimate:** Large (400-600 lines)
**Dependencies:** **REQUIRES both #03 AND #04 merged first**

**Changes:**
- Unified VAD/STT pipeline in runtime
- Deterministic WAV file streaming for E2E tests
- Real WAV loader with trailing silence support
- Integration hooks for deterministic testing

**Why This Order:** Integrates VAD and STT; requires both to be complete.

**Special Handling:**
```bash
# Option A: Wait for both #03 and #04 to merge, then create PR #05
# Option B: Create PR early based on #02, rebase twice after #03 & #04 merge
```

**Validation:**
```bash
cargo test -p coldvox-app test_end_to_end_wav --features vosk --nocapture
cargo test -p coldvox-app --features vosk
# Verify: WAV files stream correctly, E2E tests deterministic
```

---

### PR #06: text-injection
**Title:** `[06/09] injection: clipboard-preserve + Wayland-first strategy`
**Base:** `05-app-runtime-wav`
**Scope:** `crates/coldvox-text-injection/**`
**Size Estimate:** Medium-Large (300-500 lines)

**Changes:**
- Clipboard preservation (save → inject → restore)
- Wayland-first strategy ordering (AT-SPI → Clipboard → ydotool)
- Strategy manager refactor with per-app success caching
- Combined clipboard+paste injector improvements
- Timing improvements for clipboard restoration (500ms default)

**Known Limitation:**
AT-SPI paste only works with accessibility-enabled apps (Firefox, VS Code). Most apps lack AT-SPI support, requiring ydotool or manual setup.

**Post-Refactor TODO:**
Research and implement fallback methods (xdotool, wtype, evdev) - see separate knowledge agent research task.

**Validation:**
```bash
cargo test -p coldvox-text-injection
cargo run --features text-injection --example inject_demo
# Manual: Test clipboard preservation across apps
```

---

### PR #07: testing
**Title:** `[07/09] tests: deterministic E2E + integration suites`
**Base:** `06-text-injection`
**Scope:** `**/tests/**`, E2E WAV tests, integration test setup
**Size Estimate:** Medium (200-400 lines)

**Changes:**
- Deterministic E2E test infrastructure
- Settings test fixtures with path-aware loading
- Integration test suite improvements
- WAV file-based testing validation

**Why This Order:** Consolidates all test infrastructure after features are complete.

**Validation:**
```bash
cargo test --workspace
cargo test --workspace --features vosk
# Verify: All tests pass, no flaky tests
```

---

### PR #08: logging-observability
**Title:** `[08/09] logs: prune noisy hot paths; telemetry tweaks`
**Base:** `07-testing`
**Scope:** `crates/coldvox-telemetry/**`, scattered logging changes
**Size Estimate:** Small-Medium (100-200 lines)

**Changes:**
- Reduce hot-path logging noise (audio frame processing)
- Telemetry metric improvements
- Observability enhancements for debugging
- Log level adjustments (trace → debug for performance-critical paths)

**Why This Order:** Logging touches many files; best done after features stabilize.

**Validation:**
```bash
cargo run --bin tui_dashboard -- --log-level debug
cargo run --features vosk,text-injection
# Verify: Log output clean, no spam in hot paths
```

---

### PR #09: docs-changelog
**Title:** `[09/09] docs: changelog + guides + fixes`
**Base:** `08-logging-observability`
**Scope:** `docs/**`, `CHANGELOG.md`, `README.md`, deployment guides
**Size Estimate:** Medium (200-400 lines)

**Changes:**
- Update CHANGELOG.md with all changes from stack
- Fix false documentation claims (XDG paths, deployment)
- Add deployment guides
- Update configuration documentation
- Add runflags reference

**Why Last:** Documentation comes last when all changes are known.

**Validation:**
```bash
# Link validation (if markdown-link-check installed)
find docs -name "*.md" -exec markdown-link-check {} \;
# Manual review of accuracy
```

---

## Execution Timeline

| Phase | Duration | Owner |
|-------|----------|-------|
| **Phase 1: Graphite Split** | 2-3 hours | Agent: Splitter |
| **Phase 2: Validation** | 90 min | Agents: Validators (parallel) |
| **Phase 3: PR Creation** | 30 min | Agent: PR-Creator |
| **Phase 4: Review** | 3-6 hours | Agents: Reviewers (parallel where safe) |
| **Phase 5: Sequential Merge** | 1-2 weeks | Agent: Merge-Coordinator (CI-gated) |
| **Phase 6: Post-Merge Cleanup** | 1-2 days | Team (docs/diagrams/CI) |
| **Total Active Work** | ~6 hours + 1-2 days | Agents + Team |
| **Total Calendar Time** | 2-3 weeks | Team + CI |

---

## Agent Assignments

### Phase 1: Split (Solo Agent)

**Agent: "Splitter"**
```bash
# On branch: anchor/oct-06-2025
gt track
gt split --by-hunk

# Follow path-based rules:
# - config/** → 01-config-settings
# - crates/coldvox-audio/** → 02-audio-capture
# - crates/coldvox-vad*/** → 03-vad
# - crates/coldvox-stt*/** → 04-stt
# - crates/app/src/runtime.rs, wav_file_loader.rs → 05-app-runtime-wav
# - crates/coldvox-text-injection/** → 06-text-injection
# - **/tests/** → 07-testing (except settings_test.rs → 01)
# - crates/coldvox-telemetry/**, logging changes → 08-logging-observability
# - docs/**, CHANGELOG* → 09-docs-changelog

# Verify order
gt log

# Reorder if needed
gt reorder

# Push all branches
git push --all
```

---

### Phase 2: Validation (Parallel Agents)

**Agent: "Validator-Foundation"** (Branches: 01)
```bash
git checkout 01-config-settings
cargo check --workspace
cargo test --test settings_test
cargo clippy --workspace -- -D warnings
echo "✅ 01 validated" >> /tmp/validation.log
```

**Agent: "Validator-Audio"** (Branches: 02)
```bash
git checkout 02-audio-capture
cargo test -p coldvox-audio
cargo run --bin mic_probe -- --duration 10
echo "✅ 02 validated" >> /tmp/validation.log
```

**Agent: "Validator-Processing"** (Branches: 03, 04 - parallel)
```bash
git checkout 03-vad
cargo test -p coldvox-vad -p coldvox-vad-silero
echo "✅ 03 validated" >> /tmp/validation.log

git checkout 04-stt
cargo test -p coldvox-stt -p coldvox-stt-vosk --features vosk
echo "✅ 04 validated" >> /tmp/validation.log
```

**Agent: "Validator-Integration"** (Branches: 05, 06, 07)
```bash
for branch in 05-app-runtime-wav 06-text-injection 07-testing; do
  git checkout $branch
  cargo test --workspace --features vosk,text-injection
  echo "✅ $branch validated" >> /tmp/validation.log
done
```

**Agent: "Validator-Infra"** (Branches: 08, 09)
```bash
git checkout 08-logging-observability
cargo check --workspace
echo "✅ 08 validated" >> /tmp/validation.log

git checkout 09-docs-changelog
# Manual doc review
echo "✅ 09 validated" >> /tmp/validation.log
```

---

### Phase 3: PR Creation (Solo Agent)

**Agent: "PR-Creator"**

For each branch (01-09), run:
```bash
gh pr create \
  --base <parent-branch> \
  --head <current-branch> \
  --title "[XX/09] <domain>: <description>" \
  --body "<PR template from execution guide>"
```

**Critical:**
- PR #01 bases on `main`
- PR #02 bases on `01-config-settings`
- PR #03 bases on `02-audio-capture`
- PR #04 bases on `02-audio-capture` (same as #03!)
- PR #05 bases on `02-audio-capture` (will need rebase after #03 & #04)
- Remaining PRs stack linearly

---

### Phase 4: Review (Parallel Agents)

**9 Reviewer Agents** (one per PR):

| Agent | PR | Domain | Review Time |
|-------|-----|--------|-------------|
| Reviewer-Config | #01 | Config/Settings | 30 min |
| Reviewer-Audio | #02 | Audio capture | 30 min |
| Reviewer-VAD | #03 | VAD/Silero | 30 min (parallel with #04) |
| Reviewer-STT | #04 | STT/Vosk | 30 min (parallel with #03) |
| Reviewer-Runtime | #05 | App runtime | 45 min |
| Reviewer-Injection | #06 | Text injection | 45 min |
| Reviewer-Testing | #07 | Test infrastructure | 30 min |
| Reviewer-Observability | #08 | Logging/telemetry | 20 min |
| Reviewer-Docs | #09 | Documentation | 20 min |

**Review Checklist:**
- [ ] Code quality and Rust best practices
- [ ] Architectural alignment with CLAUDE.md
- [ ] Test coverage adequate
- [ ] Documentation accurate
- [ ] No scope creep (only touches expected crates)
- [ ] CI passes (build + tests + clippy)

---

### Phase 5: Merge Coordination (Automated Agent)

**Agent: "Merge-Coordinator"**

```python
merge_order = [
    "01-config-settings",
    "02-audio-capture",
    # Parallel merge when both ready:
    "03-vad",
    "04-stt",
    # Wait for BOTH above before proceeding:
    "05-app-runtime-wav",
    "06-text-injection",
    "07-testing",
    "08-logging-observability",
    "09-docs-changelog"
]

for pr in merge_order:
    # Wait for CI and approvals
    while not (ci_passing(pr) and approvals_met(pr)):
        sleep(1 hour)

    # Special case: PR #05 needs both #03 and #04
    if pr == "05-app-runtime-wav":
        assert is_merged("03-vad") and is_merged("04-stt")

    # Merge
    merge_pr(pr)

    # Restack remaining PRs
    run_command("gt sync")

    # Let CI stabilize
    sleep(30 min)
```

**Conflict Resolution:** If `gt sync` finds conflicts, pause and alert human.

---

## Expected Metrics

| Metric | Target | Rationale |
|--------|--------|-----------|
| **Merge Conflicts** | 2-3 total | Each crate modified once |
| **CI Failures** | 0-1 PRs | Domain isolation prevents integration issues |
| **Review Time** | 1-2 weeks | Parallel reviews (PRs #3 & #4) |
| **Context Switches** | 1 per PR | Domain experts assigned per PR |
| **Parallel Work** | 2 PRs | VAD + STT can review simultaneously |

---

## Success Criteria

- [ ] All 9 branches created and pushed
- [ ] Each branch passes `cargo test --workspace`
- [ ] Each branch passes `cargo clippy --workspace -- -D warnings`
- [ ] Dependency graph matches plan (visualized with `gt log`)
- [ ] No cross-cutting changes (each PR modifies 1-2 crates max)
- [ ] PR descriptions document dependencies clearly
- [ ] CI passes for each PR before merge
- [ ] Merge order: 01 → 02 → 03/04 → 05 → 06 → 07 → 08 → 09
- [ ] All PRs merged to main within 2 weeks
- [ ] `git diff main anchor/oct-06-2025` is empty after final merge

---

## Post-Merge Cleanup Tasks (Phase 6)

**Status:** Code refactor is effectively done; what's left is cleanup and docs/diagrams to reflect the "single paste path with ydotool fallback" decision plus a CI build pass.

**Timeline:** After all 9 PRs merge to main (1-2 days work)

### Documentation Sweep

- [ ] **Crate README** (`crates/coldvox-text-injection/README.md`)
  - State "ydotool is fallback-only inside ClipboardPaste; no standalone ydotool strategy registered"
  - Update system requirements: "ydotool (optional fallback for paste)"

- [ ] **Library docs** (`lib.rs`)
  - Align with crate README on ydotool fallback positioning
  - Remove references to "separate ydotool strategy"

- [ ] **Architecture docs** (`docs/architecture.md`)
  - Replace "clipboard-only" and "standalone ydotool paste" with unified ClipboardPaste path
  - Clarify AT-SPI → ydotool fallback chain

- [ ] **Feature documentation**
  - Confirm `text-injection-ydotool` feature described as "enables fallback capability, not a standalone strategy"

### Diagrams Refresh

- [ ] **`diagrams/text_injection_strategy_manager.mmd`**
  - Remove `YdotoolStrategy` class
  - Rename `ClipboardStrategy` → `ClipboardPasteStrategy`
  - Re-export PNG/SVG

- [ ] **`diagrams/text_injection_flow.mmd`**
  - Show ydotool only as fallback within ClipboardPaste lane
  - Remove obsolete `allow_ydotool` config labels
  - Re-export PNG/SVG

### Residual References Cleanup

- [ ] **Docs referencing "ydotool injector as first-class strategy"**
  - `docs/updated_architecture_diagram.md`
  - `docs/architecture.md`
  - Reword to "ydotool fallback in ClipboardPaste"

- [ ] **Examples/tests using YdotoolInjector directly**
  - `examples/real_injection_smoke.rs`
  - Add comment: "Backend probe only; manager doesn't register as standalone strategy"

### CI/Build Validation

- [ ] **Linux build + tests**
  - Ensure no lingering references to removed enum variants
  - Run `cargo test --workspace --features vosk,text-injection`

- [ ] **Feature combo testing**
  - Test with AT-SPI present + ydotool present
  - Test with AT-SPI absent + ydotool present (fallback path)
  - Test with both absent (graceful failure)

### Minor Polish

- [ ] **Logging consistency**
  - Ensure all logs use `ClipboardPasteFallback` variant name
  - Check method name strings in telemetry

- [ ] **Cargo.toml feature clarity**
  - If keeping `text-injection-ydotool` feature, document "enables fallback compilation only"

**Owner:** Development team or cleanup agents
**Priority:** High (polish before announcing release)

---

## Post-Refactor Work

### Text Injection Additional Fallbacks (Separate PR after stack merges)

**Problem:** AT-SPI paste only works with accessibility-enabled apps. Most apps lack support, requiring ydotool manual setup.

**Research Task:** Investigate additional fallback methods for triggering paste:
- X11: xdotool, xte, dotool
- Wayland: wtype, wshowkeys alternatives
- Kernel: evdev, /dev/uinput direct access
- DBus: KDE KGlobalAccel for shortcuts

**Deliverable:** New PR with extended fallback chain:
```
AT-SPI → ydotool → xdotool/wtype → evdev → fail
```

**Priority:** P1 (high) - improves usability for users without ydotool setup
**Timeline:** 1 week after stack merges

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PR #05 integration fails | Medium | High | E2E tests validate before merge |
| Merge conflicts during restack | Medium | Medium | `gt restack` + manual resolution |
| CI flakes block merge | Low | Medium | Retry failed tests, fix if persistent |
| Branch order wrong after split | Low | Low | `gt reorder` in interactive editor |
| Text injection breaks in production | Medium | High | Post-refactor PR for fallback methods |

---

## Troubleshooting

### Issue: `gt split` creates unexpected branches
**Solution:**
```bash
gt fold  # Merge child back into parent
gt split --by-commit  # Try commit-based split first
```

### Issue: Validation fails on a branch
**Solution:**
```bash
git checkout <branch>
# Fix issues
git commit -am "fix: resolve validation issues"
cargo test  # Re-validate
```

### Issue: Merge conflict during `gt restack`
**Solution:**
```bash
git status  # Shows conflicted files
# Resolve manually
git add -A
gt continue  # Resume restack
```

### Issue: Need to insert a new branch mid-stack
**Solution:**
```bash
gt checkout <parent-branch>
gt create --insert --message "<new branch description>"
# Result: main → parent → NEW → child → ...
```

---

## Next Actions

### Immediate (Start Execution)
1. ✅ Review this plan
2. ⏳ Merge PR #122 (strategy docs)
3. ⏳ Kick off Agent: Splitter (Phase 1)
4. ⏳ Launch Validator agents (Phase 2, parallel)
5. ⏳ Launch PR-Creator agent (Phase 3)
6. ⏳ Launch Reviewer agents (Phase 4, parallel where safe)
7. ⏳ Hand off to Merge-Coordinator agent (Phase 5)
8. ⏳ Verify `git diff main anchor/oct-06-2025` is empty

### Post-Merge Cleanup (Phase 6)
9. ⏳ Complete docs sweep (README, architecture, lib.rs)
10. ⏳ Refresh diagrams (text_injection_*.mmd → PNG/SVG)
11. ⏳ Run full CI validation on main
12. ⏳ Verify all checkboxes in "Post-Merge Cleanup Tasks" section

### Future Enhancements
13. ⏳ Create PR for additional text injection fallbacks (xdotool/wtype/evdev)

---

**Status:** Ready for Execution
**Owner:** Development Team
**Priority:** High
**Estimated Completion:** 2024-10-21 (2 weeks from start)
