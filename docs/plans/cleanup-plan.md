---
doc_type: plan
subsystem: general
status: active
freshness: current
last_reviewed: 2026-03-31
owners: Patrick MacLyman
version: 1.0.0
---

# ColdVox Cleanup Plan

## Goal

Strip the repo to what matters: a working speech-to-text pipeline on Windows 11.
Remove dead code, dead docs, dead references. Archive what has historical value.
Get from 86 docs to ~12 active ones with zero duplication.

## Guiding Principle

Every file that stays must answer: "Does this help an agent implement mic → VAD → Parakeet STT → text injection → Tauri GUI on Windows 11?" If not, it's archived or deleted.

---

## Phase 1: Root Directory Cleanup

### DELETE (zero value, session artifacts)

| File | Reason |
|------|--------|
| `NUL` | Windows NUL device artifact from broken command |
| `test_enigo_live.rs` | Loose test file; belongs in crates/ if anywhere |
| `pr_365_details.json` | GitHub API dump; can be re-fetched |
| `_projectsColdVoxtarget-http-remote-identity/` | Mangled build path artifact |
| `plugins.json` (root) | Duplicates `config/plugins.json`; ambiguous |
| `crates/app/plugins.json` | Hard-codes `"preferred_plugin": "whisper"` — dead backend |

### ARCHIVE (move to docs/archive/root/)

| File | Reason |
|------|--------|
| `FINAL_REPORT.md` | Historical Windows pipeline status |
| `VERIFICATION_REPORT.md` | Historical verification record |
| `WINDOWS_IMPLEMENTATION_SUMMARY.md` | Rubato migration history |
| `PYO3_DEPENDENCY_AUDIT_PLAN.md` | PyO3 debugging reference |
| `PYO3_DLL_TROUBLESHOOTING.md` | DLL troubleshooting — useful reference |
| `DEPENDENCY_AUDIT_ISSUE.md` | Audit action items, partially stale |

### DELETE agent instruction duplicates

| File | Reason |
|------|--------|
| `CLAUDE.md` | Byte-identical to `AGENTS.md` |
| `GEMINI.md` | Byte-identical to `AGENTS.md` |

> **Note:** Claude Code auto-reads `CLAUDE.md`, Gemini reads `GEMINI.md`.
> After deletion, `AGENTS.md` becomes the single source. Claude Code and
> Kilocode can be pointed at it via their config mechanisms if needed.

---

## Phase 2: Dead Backend Code Cleanup

### Rust code changes

| File | Change | Priority |
|------|--------|----------|
| `crates/coldvox-stt/src/types.rs:64` | Replace `WHISPER_MODEL_PATH` env var with `STT_MODEL_PATH` | HIGH |
| `crates/app/src/stt/tests/mod.rs:20,24` | Update test to use `STT_MODEL_PATH` | HIGH |
| `crates/app/tests/integration/text_injection_integration_test.rs:21` | Remove `set_var("COLDVOX_STT_PREFERRED", "whisper")` | HIGH |
| `crates/app/tests/integration/capture_integration_test.rs:25` | Remove `set_var("COLDVOX_STT_PREFERRED", "whisper")` | HIGH |
| `crates/coldvox-stt/src/plugin.rs:4,16` | Update doc comments: "Whisper, Cloud APIs" → "Moonshine, Parakeet" | MEDIUM |
| `crates/coldvox-stt/src/plugin_types.rs:12,14` | Update doc comments: "Whisper, Coqui" → "Moonshine, Parakeet" | MEDIUM |
| `crates/coldvox-stt/Cargo.toml:19` | Remove stale comment about faster-whisper | LOW |

### Delete dead vendor/scripts

| Path | Reason |
|------|--------|
| `vendor/vosk/` | Stubs pointing to dead Linux runner cache |
| `scripts/ci/setup-vosk-cache.sh` | Downloads whisper+vosk — both dead |
| `scripts/verify_vosk_model.sh` | Vosk model verification — dead backend |
| `scripts/ensure_venv.sh` | `pip install faster-whisper` — dead + violates uv-only |
| `scripts/start-headless.sh` | Xvfb+openbox — Linux-only, violates CI policy |

---

## Phase 3: Fix Dead References

### `windows-multi-agent-recovery.md` → `current-status.md`

This file is referenced 20+ times across the repo but **does not exist**.
All references must point to `docs/plans/current-status.md` instead.

Files to update:
- `README.md` (3 references)
- `CHANGELOG.md` (2 references)
- `.github/copilot-instructions.md` (3 references)
- `.kilocode/rules/agents.md` (3 references)
- `docs/standards/agent-rules.md` (1 reference)
- `docs/todo.md` (2 references)
- `.github/prompts/drive-project.prompt.md` (3 references)
- `docs/domains/gui/gui-design-overview.md` (check for reference)

---

## Phase 4: Documentation Pruning

### DELETE (zero value)

| File | Reason |
|------|--------|
| `docs/revision_log.csv` | Empty — header only, no data |
| `docs/architecture/adr/index.md` | Empty placeholder: "No ADRs migrated yet" |
| `docs/playbooks/organizational/github_governance.md` | Empty stub |
| `docs/playbooks/organizational/pr_playbook.md` | Empty stub |
| `docs/tasks/ci-runner-readiness-proposal.md` | Self-marked SUPERSEDED and OUTDATED |
| `docs/archive/plans/gui/aspirational-gui-plan.md` | Dead Qt/KDE plan, marked `preservation: delete` |
| `docs/archive/plans/gui/comprehensive-gui-plan.md` | Dead Qt/KDE plan, marked `preservation: delete` |
| `docs/archive/plans/gui/raw-gui-plan.md` | Dead Qt/KDE plan, marked `preservation: delete` |
| `docs/archive/reference/crates/coldvox-stt.md` | Dead stub, marked `preservation: summarize` |
| `docs/archive/research/pr-reports/PR-temp-clipboard-test-timeout-fixes.md` | Past retention date |
| `docs/archive/research/pr-reports/PR-temp-comprehensive-testing-report.md` | Past retention date |
| `docs/archive/research/pr-reports/PR-temp-injection-path-alignment.md` | Past retention date |
| `docs/history/2025-11-06_04-16Z-branch-status-*.md` | Raw chat transcript, marked `preservation: delete` |
| `docs/history/2025-11-06_04-33Z-reviewing-implementation-*.md` | Raw chat transcript, duplicated in dedicated doc |
| `docs/history/2025-11-06_05-33Z-git-history-inquiry-*.md` | Raw chat transcript, marked `preservation: delete` |

### ARCHIVE (move to docs/archive/)

| File | → Destination | Reason |
|------|---------------|--------|
| `docs/logging.md` | `docs/archive/root/logging.md` | Duplicated by `docs/domains/telemetry/tele-logging.md` |
| `docs/observability-playbook.md` | `docs/archive/root/observability-playbook.md` | Org-wide scope; not ColdVox-specific |
| `docs/architecture/roadmap.md` | `docs/archive/plans/roadmap.md` | Completely stale — milestones from Q4 2025 |
| `docs/implementation-plans/phase1-audio-quality-monitoring.md` | `docs/archive/plans/` | Not near-term North Star work |
| `docs/issues/audio-quality-monitoring.md` | `docs/archive/research/` | Not near-term |
| `docs/research/dependency-audit-report-2025-02-09.md` | `docs/archive/research/` | Superseded by 2026-03-24 audit |
| `docs/domains/audio/aud-pipewire-design.md` | `docs/archive/domains/` | Linux-only (PipeWire/ALSA) |
| `docs/domains/telemetry/tele-observability-playbook.md` | `docs/archive/domains/` | Org-wide observability, not ColdVox |

### FIX (contradicts North Star)

| File | Issue |
|------|-------|
| `docs/domains/stt/stt-overview.md` | Lists "Whisper: Legacy/removed path" as Supported Backend |
| `docs/domains/audio/aud-user-config-design.md` | Claims "Moonshine: Pure Rust implementation, CPU-efficient" (it's Python/PyO3) |
| `docs/domains/foundation/fdn-testing-guide.md` | Claims `--features parakeet` and `--features mock` work (they don't) |
| `docs/architecture.md` | 90% is speculative "Future Vision" — should describe actual system |

---

## Phase 5: Restructure Agent Instruction Chain

### Current (broken):
```
AGENTS.md (17-line pointer)
  → docs/northstar.md
  → docs/plans/windows-multi-agent-recovery.md  ← DOES NOT EXIST
  → docs/architecture.md                        ← 90% speculation
  → docs/dev/CI/policy.md
```

### Target:
```
AGENTS.md (full onboarding — currently in .github/copilot-instructions.md)
  → docs/northstar.md         (product vision)
  → docs/plans/current-status.md  (what works/broken)
  → docs/dev/commands.md      (build commands)
  → docs/standards/agent-rules.md  (working rules)
```

### Actions:
1. Move content from `.github/copilot-instructions.md` into `AGENTS.md` (make AGENTS.md the canonical full version)
2. Point `.github/copilot-instructions.md` and `.kilocode/rules/agents.md` at AGENTS.md (or sync them)
3. Delete `CLAUDE.md` and `GEMINI.md`
4. Fix all `windows-multi-agent-recovery.md` references → `current-status.md`
5. Rewrite `docs/architecture.md` to describe actual system (pipeline, crates, threading)

---

## Phase 6: Future Ideas Archive

The following represent valuable future ideas that should be preserved in archive but removed from the active doc tree:

- `docs/future/portable-agentic-evidence-standard.md` (if exists)
- `docs/prompts/review-and-implement-evidence-assessor.md`
- The "Future Vision" content in `docs/architecture.md` (extract to `docs/archive/future-vision.md`)

---

## Target: Active Doc Tree (~12 files)

```
AGENTS.md                                    ← Full agent onboarding
docs/
  northstar.md                               ← Product vision
  architecture.md                            ← ACTUAL architecture (rewritten)
  plans/
    current-status.md                        ← What works, what's broken
    parakeet-http-remote-integration-spec.md ← Active execution spec
    cleanup-plan.md                          ← This document
    northstar-drift-guard.md                 ← Drift guard system
  dev/
    commands.md                              ← Build commands
    CI/architecture.md                       ← CI split rationale
  reference/
    stt-docker-containers.md                 ← STT benchmarks
  domains/
    stt/stt-overview.md                      ← STT plugin overview (fixed)
    stt/stt-parakeet-integration-plan.md     ← Parakeet API contract
    gui/gui-design-overview.md               ← GUI design reference
    text-injection/ti-overview.md            ← Injection backends
    foundation/fdn-voice-pipeline-core-design.md ← Pipeline data flow
  standards/
    agent-rules.md                           ← Working rules
  issues/
    pyo3_instability.md                      ← Active PyO3 risk
  archive/                                   ← Everything else
```

---

## Execution Order

1. **Phase 1** — Root cleanup (delete junk, archive reports)
2. **Phase 2** — Dead backend code (Rust changes, delete scripts/vendor)
3. **Phase 3** — Fix dead references (windows-multi-agent-recovery.md)
4. **Phase 4** — Doc pruning (delete, archive, fix contradictions)
5. **Phase 5** — Agent instruction restructure
6. **Phase 6** — Future ideas archive

Each phase is one commit. Phases 1-3 are safe mechanical changes.
Phases 4-5 require judgment calls. Phase 6 is cosmetic.

---

## What This Does NOT Touch

- Any Rust implementation code beyond dead-reference cleanup
- The Parakeet plugin (upgrading to v0.3.4 is separate work)
- GUI implementation
- CI workflows (except removing dead backend scripts)
- The archive/ directory (already archived content stays)
