# Graphite Domain-Based Execution Plan (Merged)

**Date:** 2024-10-08  
**Status:** Ready for Execution  
**Branch to Split:** `anchor/oct-06-2025` (93 files, 33 commits)  
**Stack Shape:** 9 domain-focused PRs (`01`–`09`) stacked with Graphite  
**Hotfix Assumption:** The clipboard P0 fix (#00) already landed on `main`; this stack contains the refactor payload only.  
**Parallelism Support:** Agents may run up to 5 tool calls concurrently. Async edit agents are available for doc rewrites, restacks, and PR template updates when noted.

---

## Agent Launch Board (Start Here)

1. **Pre-flight (5 min):** `git fetch origin` and confirm `anchor/oct-06-2025` is rebased onto the latest `main`. Create a backup branch `backup/anchor-oct-06-2025-$(date +%Y%m%d-%H%M%S)` before touching Graphite state.
2. **Phase 1 – Split (Agent-1 Splitter):** Adopt the branch (`gt track`) and run `gt split --by-hunk`, assigning hunks according to the routing rules in *Execution Workflow*. Keep a scratch log of tricky hunks for reviewer notes.
3. **Phase 2 – Validation (Agents 2-6, parallel):** Once the nine branches exist, dispatch validator agents concurrently (≤5 shells). Each validator runs the command block tied to its PRs and appends results to `/tmp/validation.log`.
4. **Phase 3 – PR Creation (Agent-7 PR-Creator):** After validation passes, generate stacked PRs via Graphite Cloud or `gh pr create`, respecting the base relationships in the stack diagram.
5. **Phase 4 – Review (Agents 8-16):** Assign domain reviewers. PR #03 and PR #04 can be reviewed in parallel; the rest proceed in stack order to minimize context switching.
6. **Phase 5 – Merge Coordination (Agent-17):** Merge sequentially, restacking (`gt sync`) after each merge. Hold PR #05 until both PR #03 and PR #04 are merged; pause if CI fails.
7. **Phase 6 – Cleanup (Async-Editor + Team):** Trigger the documentation, diagram, and CI sweep once the stack lands. File retro notes and archive validation artifacts.

> Tip: If a phase stalls, delegate documentation or automation tasks to the Async-Editor agent so the main pipeline keeps moving.

---

## Quick Stack Reference

```
main
 └─ 01-config-settings
     └─ 02-audio-capture
         ├─ 03-vad (parallel with 04)
         └─ 04-stt (parallel with 03)
             └─ 05-app-runtime-wav (waits for 03 & 04)
                 └─ 06-text-injection
                     └─ 07-testing
                         └─ 08-logging-observability
                             └─ 09-docs-changelog
```

---

## Agent Cheat Sheet

```
Phase 0 – Prep
  Agent-0: Async-Editor (optional) → update PR templates, seed CI scripts, prep diagrams.

Phase 1 – Split
  Agent-1: Splitter (solo, 2-3h) → gt track; gt split --by-hunk; enforce routing matrix.

Phase 2 – Validation (≤5 concurrent shells)
  Agent-2: Validator-Foundation → PR #01
  Agent-3: Validator-Audio → PR #02
  Agent-4: Validator-Processing → PR #03 then PR #04
  Agent-5: Validator-Integration → PR #05, #06, #07
  Agent-6: Validator-Infra → PR #08, #09

Phase 3 – PR Creation
  Agent-7: PR-Creator (solo, 30m) → gh pr create / gt submit

Phase 4 – Review (assign domain experts)
  Agent-8: Reviewer-Config → PR #01
  Agent-9: Reviewer-Audio → PR #02
  Agent-10: Reviewer-VAD → PR #03
  Agent-11: Reviewer-STT → PR #04
  Agent-12: Reviewer-Runtime → PR #05
  Agent-13: Reviewer-Injection → PR #06
  Agent-14: Reviewer-Testing → PR #07
  Agent-15: Reviewer-Observability → PR #08
  Agent-16: Reviewer-Docs → PR #09

Phase 5 – Merge
  Agent-17: Merge-Coordinator (CI-gated, 1-2 weeks) → merge order enforcement, gt sync, conflict alerts.

Phase 6 – Cleanup
  Agent-0 + Team: Docs sweep, diagram exports, CI validation, retro capture.
```

---

## PR Breakdown

### PR #01: config-settings
- **Title:** `[01/09] config: centralize Settings + path-aware load`
- **Base:** `main`
- **Depends On:** None
- **Blocks:** PR #02 → PR #09
- **Scope:** `crates/app/src/lib.rs`, `config/**`, `crates/app/tests/settings_test.rs`
- **Size Guardrail:** Target 200-400 LOC · Max 800 LOC · Split into `config-core` + `config-integration` if exceeded.
- **Key Changes:**
  - Centralize configuration loading with path-aware logic.
  - Add `COLDVOX_CONFIG_PATH` environment override.
  - Update Settings API for deterministic tests.
  - Introduce default and overrides TOML fixtures.
- **Validation:**
```bash
cargo check --workspace
cargo test --test settings_test
cargo clippy --workspace -- -D warnings
```
- **Notes:** Async-Editor can update config docs or templates if reviewers ask for more context.

### PR #02: audio-capture
- **Title:** `[02/09] audio: capture lifecycle fix + ALSA stderr suppression`
- **Base:** `01-config-settings`
- **Depends On:** PR #01
- **Blocks:** PR #03, PR #04, PR #05+
- **Scope:** `crates/coldvox-audio/**`
- **Size Guardrail:** Target 300-500 LOC · Max 1000 LOC · Split by subsystem (device monitor vs capture core) if needed.
- **Key Changes:**
  - Stabilize audio capture thread lifecycle.
  - Prioritize PipeWire devices via monitor enhancements.
  - Suppress ALSA stderr noise.
  - Harden watchdog and error recovery paths.
- **Validation:**
```bash
cargo test -p coldvox-audio
cargo run --bin mic_probe -- --duration 30
```
- **Notes:** Capture sample logs for reviewers; noisy logs belong in PR #08.

### PR #03: vad
- **Title:** `[03/09] vad: windowing/debounce consistency`
- **Base:** `02-audio-capture`
- **Depends On:** PR #02
- **Blocks:** PR #05
- **Parallel-Safe:** ✅ Parallel review with PR #04.
- **Scope:** `crates/coldvox-vad/**`, `crates/coldvox-vad-silero/**`
- **Size Guardrail:** Target 150-300 LOC · Max 600 LOC.
- **Key Changes:**
  - Frame-based debounce for deterministic outcomes.
  - Timestamp-ms utilities for reproducibility.
  - Windowing consistency across CPU/GPU paths.
- **Validation:**
```bash
cargo test -p coldvox-vad
cargo test -p coldvox-vad-silero
cargo run --example test_silero_wav --features examples
```

### PR #04: stt
- **Title:** `[04/09] stt: finalize handling + helpers`
- **Base:** `02-audio-capture`
- **Depends On:** PR #02
- **Blocks:** PR #05
- **Parallel-Safe:** ✅ Parallel review with PR #03.
- **Scope:** `crates/coldvox-stt/**`, `crates/coldvox-stt-vosk/**`
- **Size Guardrail:** Target 150-300 LOC · Max 600 LOC.
- **Key Changes:**
  - Improve STT finalization behavior.
  - Add transcription helper utilities.
  - Refine session event handling and telemetry hooks.
- **Validation:**
```bash
cargo test -p coldvox-stt
cargo test -p coldvox-stt-vosk --features vosk
cargo run --features vosk --example vosk_test
```

### PR #05: app-runtime-wav
- **Title:** `[05/09] app: unify VAD↔STT runtime + real WAV loader`
- **Base:** `02-audio-capture` (rebased after PR #03 and PR #04 merge)
- **Depends On:** PR #03 and PR #04
- **Blocks:** PR #06, PR #07
- **Scope:** `crates/app/src/runtime.rs`, `crates/app/src/audio/wav_file_loader.rs`, integration glue
- **Size Guardrail:** Target 400-600 LOC · Max 1200 LOC · Split into runtime core vs WAV loader if breached.
- **Key Changes:**
  - Unify the runtime pipeline across VAD and STT.
  - Provide deterministic WAV streaming for E2E tests.
  - Implement real WAV loader with trailing silence support.
  - Wire integration hooks for reproducible testing.
- **Validation:**
```bash
cargo test -p coldvox-app test_end_to_end_wav --features vosk --nocapture
cargo test -p coldvox-app --features vosk
```
- **Notes:** Hold merge until both PR #03 and PR #04 are green. Consider handing heavy edits to Async-Editor for formatting if rebases get messy.

### PR #06: text-injection
- **Title:** `[06/09] injection: clipboard-preserve + Wayland-first strategy`
- **Base:** `05-app-runtime-wav`
- **Depends On:** PR #05
- **Blocks:** PR #07, PR #08
- **Scope:** `crates/coldvox-text-injection/**`
- **Size Guardrail:** Target 300-500 LOC · Max 1000 LOC.
- **Key Changes:**
  - Clipboard preservation (save → inject → restore) refinements.
  - Wayland-first strategy ordering (AT-SPI → clipboard → ydotool fallback).
  - Strategy manager refactor with per-app success caching.
  - Combined clipboard + paste injector improvements and timing tweaks.
- **Validation:**
```bash
cargo test -p coldvox-text-injection
cargo run --features text-injection --example inject_demo
# Manual: verify clipboard preservation across Firefox, VS Code, native apps
```
- **Known Limitation:** AT-SPI works only where accessibility is enabled; ydotool remains fallback.

### PR #07: testing
- **Title:** `[07/09] tests: deterministic E2E + integration suites`
- **Base:** `06-text-injection`
- **Depends On:** PR #06
- **Blocks:** PR #08, PR #09
- **Scope:** `**/tests/**`, integration harnesses, E2E WAV fixtures
- **Size Guardrail:** Target 200-400 LOC · Max 800 LOC.
- **Key Changes:**
  - Deterministic E2E infrastructure and fixtures.
  - Path-aware settings test harness.
  - Integration test suite coverage improvements.
  - WAV-based validation for runtime correctness.
- **Validation:**
```bash
cargo test --workspace
cargo test --workspace --features vosk,text-injection
```

### PR #08: logging-observability
- **Title:** `[08/09] logs: prune noisy hot paths; telemetry tweaks`
- **Base:** `07-testing`
- **Depends On:** PR #07
- **Blocks:** PR #09
- **Scope:** `crates/coldvox-telemetry/**`, logging adjustments across crates
- **Size Guardrail:** Target 100-200 LOC · Max 400 LOC.
- **Key Changes:**
  - Reduce hot-path logging noise in audio pipelines.
  - Tune telemetry metrics and sampling.
  - Improve observability for debugging (structured fields).
  - Normalize log levels (trace → debug where appropriate).
- **Validation:**
```bash
cargo run --bin tui_dashboard -- --log-level debug
cargo run --features vosk,text-injection
# Confirm log volume and telemetry dashboards
```

### PR #09: docs-changelog
- **Title:** `[09/09] docs: changelog + guides + fixes`
- **Base:** `08-logging-observability`
- **Depends On:** PR #08
- **Blocks:** None
- **Scope:** `docs/**`, `CHANGELOG.md`, `README.md`, deployment guides
- **Size Guardrail:** Target 200-400 LOC · Max 800 LOC.
- **Key Changes:**
  - Update changelog and public docs to match refactor.
  - Fix stale documentation (XDG paths, deployment instructions).
  - Add run flags and configuration references.
  - Summarize integration and testing matrix.
- **Validation:**
```bash
find docs -name "*.md" -exec markdown-link-check {} \;
# Manual accuracy review
```
- **Notes:** Async-Editor can help with doc polish and diagram exports.

---

## Sizing & Quality Guardrails

| PR | Target LOC | Max LOC | If Exceeds Max |
|----|-----------:|--------:|----------------|
| #01 | 200-400 | 800 | Split into `config-core` + `config-integration` |
| #02 | 300-500 | 1000 | Split by audio subsystem (capture vs monitor) |
| #03 | 150-300 | 600 | Acceptable as-is; flag reviewers if near max |
| #04 | 150-300 | 600 | Same as PR #03 |
| #05 | 400-600 | 1200 | Split into runtime core vs WAV loader |
| #06 | 300-500 | 1000 | Already isolated from P0; split only if absolutely needed |
| #07 | 200-400 | 800 | Separate E2E framework vs fixtures if necessary |
| #08 | 100-200 | 400 | Keep telemetry tweaks focused |
| #09 | 200-400 | 800 | Consider separate doc-only PR if scope expands |

**Monitoring:** Validator agents log LOC counts; Async-Editor can spin up a quick `scripts/check_pr_size.sh` if automated checks fail.

---

## Validation & Automation Templates

### GitHub Workflow Snippet (`.github/workflows/pr-validation.yml`)
```yaml
name: PR Validation

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Check PR size
        run: |
          LINES=$(git diff --shortstat origin/main | awk '{print $4+$6}')
          if [ "${LINES:-0}" -gt 1000 ]; then
            echo "::warning::PR touches ${LINES:-0} lines. Consider splitting."
          fi

      - name: Check crate isolation
        run: |
          CRATES=$(git diff --name-only origin/main | grep "^crates/" | cut -d/ -f2 | sort -u | wc -l)
          if [ "${CRATES:-0}" -gt 2 ]; then
            echo "::error::PR spans ${CRATES:-0} crates (limit 2)."
            exit 1
          fi
```
*Assignment:* Async-Editor creates this workflow after PR #05 merges to enforce guardrails.

### Dependency Block Template (paste into each PR)

```markdown
## Stack Position
- **Position:** #NN of 09
- **Depends On:** PR #XX (<domain>)
- **Blocks:** PR #YY (<domain>)
- **Parallel With:** <PRs> ✅/❌

## Merge Checklist
- [ ] Upstream merged and synced (`gt sync`)
- [ ] Validation commands re-run
- [ ] Downstream owners notified of interface changes

## Rollback Plan
1. `git revert <commit>`
2. Affects: <crates>
3. Downstream impact: <PRs that must rebase>
```

### Integration Testing Matrix (documented in PR #09)

| Config Profile | Audio | VAD | STT | Runtime | Injection | Outcome |
|----------------|-------|-----|-----|---------|-----------|---------|
| Full stack (default features) | ✅ | ✅ | ✅ | ✅ | ✅ | CI baseline |
| Audio + VAD only | ✅ | ✅ | ❌ | ✅ | ❌ | Wake-word only mode |
| Audio + STT only | ✅ | ❌ | ✅ | ✅ | ❌ | Transcription without VAD |
| No injection | ✅ | ✅ | ✅ | ✅ | ❌ | Headless mode |

### Merge Conflict Prevention Checklist

- [ ] PR touches ≤2 crates and expected docs.
- [ ] Interfaces documented in PR description.
- [ ] `gt log` shows correct dependency chain.
- [ ] `cargo test --workspace --features vosk,text-injection` green.
- [ ] Manual domain smoke test complete.
- [ ] Downstream PR owners acknowledged changes.

---

## Execution Workflow (Detailed Commands)

### Phase 0 – Pre-flight
```bash
git checkout anchor/oct-06-2025
git fetch origin
git rebase origin/main
git branch backup/anchor-oct-06-2025-$(date +%Y%m%d-%H%M%S)
git status
```

### Phase 1 – Graphite Split (Agent-1)
```bash
gt track
gt split --by-hunk
```
**Routing Matrix (enforce during split):**
```
config/**                          → 01-config-settings
crates/app/src/lib.rs              → 01-config-settings
crates/coldvox-audio/**            → 02-audio-capture
crates/coldvox-vad*/**             → 03-vad
crates/coldvox-stt*/**             → 04-stt
crates/app/src/runtime.rs          → 05-app-runtime-wav
crates/app/src/audio/wav_file_loader.rs → 05-app-runtime-wav
crates/coldvox-text-injection/**   → 06-text-injection
**/tests/** (except settings test) → 07-testing
crates/coldvox-telemetry/**        → 08-logging-observability
logging tweaks across crates       → 08-logging-observability
docs/**, CHANGELOG*                → 09-docs-changelog
```

### Phase 1.5 – Order Verification
```bash
gt log
gt reorder  # if needed
```

### Phase 2 – Validation (Agents 2-6)
Each validator:
```bash
git checkout <branch>
cargo fmt -- --check
# Run the branch-specific validation block (see PR breakdown)
echo "✅ <branch>" >> /tmp/validation.log
```

### Phase 3 – PR Creation (Agent-7)
For each branch:
```bash
gt checkout <branch>
gh pr create \
  --base <parent-branch> \
  --title "[NN/09] <domain>: <summary>" \
  --body-file docs/review/split-plan-comparison/execution-guide.md
```
*Tip:* Pre-fill dependency blocks from template above.

### Phase 4 – Reviews (Agents 8-16)
- Reviewers follow domain checklists.
- Encourage parallel review for PR #03 and PR #04.
- Track findings in shared checklist (Async-Editor can maintain).

### Phase 5 – Merge Coordination (Agent-17)
```python
order = [
    "01-config-settings",
    "02-audio-capture",
    "03-vad",
    "04-stt",
    "05-app-runtime-wav",
    "06-text-injection",
    "07-testing",
    "08-logging-observability",
    "09-docs-changelog",
]

for branch in order:
    wait_for_ci(branch)
    if branch == "05-app-runtime-wav":
        assert merged("03-vad") and merged("04-stt")
    merge(branch)
    run("gt sync")
```

### Phase 6 – Cleanup (Async-Editor + Team)
- Update `crates/coldvox-text-injection/README.md` to clarify ydotool fallback.
- Sync `lib.rs` docs with new strategy manager behavior.
- Refresh `docs/architecture.md` and related diagrams (`text_injection_strategy_manager.mmd`, `text_injection_flow.mmd`).
- Remove stale references to standalone ydotool strategy.
- Re-run full Linux build: `cargo test --workspace --features vosk,text-injection`.
- Validate feature combos: AT-SPI only, ydotool only, no injection.
- Normalize logging strings (ClipboardPasteFallback naming).
- Confirm Cargo feature docs describe fallback semantics.

---

## Metrics & ROI

| Metric | Target | Rationale |
|--------|--------|-----------|
| Merge Conflicts | ≤3 total | Each crate touched once; stack sequencing limits churn. |
| CI Failures | ≤1 PR | Domain isolation; quick to pinpoint. |
| Review Duration | 1-2 weeks | Parallel reviews for PR #03/#04 reduce calendar time. |
| Context Switches | 1 per reviewer | Domain mapping keeps focus. |
| Stack Restacks | ≤2 | Expected after #03/#04 merge and final cleanup. |
| Manual QA Hours | ≤6 | Validation scripts cover most scenarios. |

**ROI Snapshot:** ≈4 hours to split/validate + 1-2 weeks review yields 50% fewer conflicts versus feature-based approach and keeps production hotfixes decoupled.

---

## Success Criteria

- [ ] All 9 branches created, validated, and pushed.
- [ ] Every PR description includes dependency block and sizing notes.
- [ ] `cargo test --workspace --features vosk,text-injection` passes for each branch.
- [ ] `gt log` matches stack diagram after every restack.
- [ ] Merge order follows plan (01 → 02 → 03+04 → 05 → 06 → 07 → 08 → 09).
- [ ] `git diff main anchor/oct-06-2025` is empty after final merge.
- [ ] Post-merge cleanup checklist fully resolved.

---

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PR #05 integration breaks | Medium | High | Run E2E WAV tests pre-merge; hold until #03/#04 merged. |
| Restack conflicts late in stack | Medium | Medium | Immediate `gt sync` after each merge; Async-Editor can assist with conflict resolution. |
| Validator resource contention | Low | Medium | Limit to 5 concurrent shells; stagger long-running tests. |
| Text injection regressions | Medium | High | Manual multi-app verification + follow-up fallback work item. |
| Oversized PR slips through | Low | Medium | Guardrail table + workflow warnings. |
| Documentation drift | Low | Medium | PR #09 update + cleanup phase enforcement. |

---

## Troubleshooting

- **Unexpected branch allocation during `gt split`:**
  ```bash
  gt fold
  gt split --by-commit  # fallback to commit split, then re-split by hunk
  ```
- **Validation failure on a branch:**
  ```bash
  git checkout <branch>
  # fix issue
  cargo test ...
  git commit --amend
  ```
- **Restack conflicts:**
  ```bash
  gt restack
  # resolve conflicts manually
  git add -A
  gt continue
  ```
- **Need to insert a corrective branch mid-stack:**
  ```bash
  gt checkout <parent>
  gt create --insert
  ```

---

## Reference Material (Keep in Repository)

- `docs/review/split-plan-comparison/refactor-split-strategy-comparison.md`
- `docs/review/split-plan-comparison/dependency-graph-comparison.md`
- `docs/review/split-plan-comparison/execution-guide.md`
- `docs/review/split-plan-comparison/quick-reference.md`
These explain *why* the domain-based plan was chosen and should remain alongside this execution plan.

---

## Next Actions & Ownership

- [ ] Assign Agent roles and time slots (Team Lead).
- [ ] Kick off Phase 1 split (Agent-1).
- [ ] Schedule validator execution windows (Project Ops).
- [ ] Prep Async-Editor backlog (docs, CI workflow, diagrams).
- [ ] Track review progress in shared checklist (Reviewer Coordinator).
- [ ] After final merge, run cleanup tasks and log retro findings (Team).

**Status:** Ready for execution. Ping the Async-Editor agent as soon as PR #05 starts if additional doc or automation support is needed.
