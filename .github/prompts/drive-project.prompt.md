You are taking over as the lead systems engineer on ColdVox. Your job is to deeply understand this Rust voice pipeline project, create an extremely detailed implementation plan, and then use #runSubagent with git worktrees to execute 4 parallel workstreams simultaneously.

ColdVox is a Rust voice pipeline: microphone audio capture → VAD (voice activity detection) → STT (speech-to-text) → text injection into the focused application. Multi-crate Cargo workspace. The north star is reliable end-to-end flow with CUDA-first STT on high-end NVIDIA GPUs, with Moonshine as the current working fallback.

## PHASE 1: DEEP ORIENTATION (do NOT edit any files yet)

Read every single one of these files carefully. Do not skim:

1. `docs/northstar.md` — Product anchor: reliability first, CUDA-first STT, live overlay
2. `docs/anchor-2026-02-09.md` — Documentation triage anchor
3. `docs/architecture.md` — Architecture direction
4. `docs/plans/critical-action-plan.md` — CRITICAL: what's broken, what works, what's misleading
5. `AGENTS.md` — Agent conventions, working rules, commands, feature flags
6. `CHANGELOG.md` — Recent changes
7. `Cargo.toml` — Workspace structure, dependencies, feature flags
8. `crates/app/Cargo.toml` — App-level features and dependencies
9. `crates/app/src/main.rs` — Main entry point and orchestration
10. `crates/app/build.rs` — Build detection logic
11. `crates/coldvox-audio/src/capture.rs` — Audio capture pipeline
12. `crates/coldvox-vad/src/lib.rs` — VAD trait definition
13. `crates/coldvox-vad-silero/src/silero_wrapper.rs` — Silero VAD implementation
14. `crates/coldvox-stt/src/lib.rs` — STT plugin trait
15. `crates/coldvox-stt/src/plugins/` — All STT plugin implementations (scan every file)
16. `crates/coldvox-text-injection/src/manager.rs` — Text injection manager
17. `crates/coldvox-gui/src/` — Overlay GUI (scan all files)
18. `crates/coldvox-foundation/src/` — Shared types and utilities
19. `crates/coldvox-telemetry/src/` — Observability
20. `mise.toml` — Toolchain config (NOTE: Python version conflict documented in critical-action-plan)
21. `pyproject.toml` — Python dependencies for Moonshine
22. `requirements.txt` — Check if this is vestigial (likely is)
23. `scripts/local_ci.sh` — Local CI script
24. `docs/dev/CI/architecture.md` — CI source of truth
25. Scan all files under `docs/` and `docs/plans/`

After reading everything, write a 500+ word internal assessment answering:
- What works end-to-end right now? (mic → VAD → STT → injection)
- What is broken, misleading, or stub code?
- What are the P0 issues from critical-action-plan.md and their current status?
- Which crates need the most work?
- What is the gap between current state and "it works reliably"?
- What dead code, stub features, and misleading docs exist?
- What are the Python version/toolchain issues?

## PHASE 2: DETAILED IMPLEMENTATION PLAN

Create the file `PLAN.md` in the repo root. This plan must be extremely detailed — at minimum 2000 words. Structure it as follows:

```markdown
# ColdVox Implementation Plan
Generated: [today's date]
Strategy: 4 parallel workstreams via git worktrees

## Executive Summary
[3-4 sentences: current state, what we're fixing, what "done" looks like]

## End-to-End Flow Status
- Microphone capture (coldvox-audio): [WORKS/BROKEN/PARTIAL — details]
- VAD - Silero (coldvox-vad-silero): [WORKS/BROKEN/PARTIAL — details]
- STT - Moonshine (coldvox-stt): [WORKS/BROKEN/PARTIAL — details]
- STT - Parakeet (coldvox-stt): [BROKEN — compile errors, planned fix]
- Text injection (coldvox-text-injection): [WORKS/BROKEN/PARTIAL — details]
- Overlay GUI (coldvox-gui): [WORKS/BROKEN/PARTIAL — details]
- Streaming partial transcription: [WORKS/BROKEN/PARTIAL — details]

## Workstream 1: P0 Cleanup — Dead Code, Stub Features, Doc Accuracy
### Crates: app, stt
### Tasks:

1.1 Remove whisper dead code entirely
   - Delete: crates/coldvox-stt/src/plugins/whisper_plugin.rs
   - Delete: crates/coldvox-stt/src/plugins/whisper_cpp.rs
   - Remove: `whisper` feature from crates/app/Cargo.toml
   - Remove: all whisper references from AGENTS.md, README.md, CLAUDE.md
   - Acceptance: `grep -r "whisper" --include="*.rs" --include="*.toml" --include="*.md"` returns zero results (except CHANGELOG)

1.2 Remove all stub features from Cargo.toml
   - Remove features: coqui, leopard, silero-stt, no-stt
   - Remove from all documentation
   - Acceptance: No empty feature stubs remain in any Cargo.toml

1.3 Fix Python version chaos
   - Remove `python = "3.13"` from mise.toml
   - Keep .python-version = 3.12
   - Document UV as single Python source of truth
   - Acceptance: `uv sync && cargo build --features moonshine` works cleanly

1.4 Fix requirements.txt vs pyproject.toml
   - Delete requirements.txt (vestigial)
   - Document `uv sync` as the correct command everywhere
   - Acceptance: No references to `pip install -r requirements.txt` in docs or CI

1.5 Fix CI/code mismatches
   - Remove `pip install faster-whisper` from CI
   - Verify golden master tests test what they claim
   - Acceptance: CI green, no phantom dependencies

1.6 Update all docs to accurately reflect reality
   - AGENTS.md: remove whisper, mark parakeet as "planned not working"
   - README.md: update quick start to use moonshine only
   - CLAUDE.md: update STT section
   - Acceptance: Every doc claim is verifiable against code

### Branch: worktree/ws1-p0-cleanup

## Workstream 2: STT Pipeline Reliability (Moonshine + Parakeet Fix)
### Crates: coldvox-stt, coldvox-audio
### Tasks:

2.1 Audit Moonshine STT end-to-end reliability
   - Trace: audio capture → sample buffer → moonshine plugin → transcribed text
   - Identify: error handling gaps, retry logic, failure modes
   - File: crates/coldvox-stt/src/plugins/moonshine.rs (or equivalent)
   - Acceptance: Document every failure mode and add handling for each

2.2 Fix Parakeet plugin to compile against parakeet-rs 0.2
   - File: crates/coldvox-stt/src/plugins/parakeet.rs
   - Fix: transcribe_samples() signature mismatch
   - Fix: confidence field doesn't exist on TimedToken
   - Acceptance: `cargo build -p coldvox-stt --features parakeet` succeeds

2.3 Implement streaming partial transcription
   - Read northstar.md: "Support streaming partial transcription so users do not wait for end-of-utterance text"
   - Files: coldvox-stt plugin interface, moonshine plugin
   - Acceptance: Partial text available before utterance completes

2.4 Implement injection failure retry logic
   - Read northstar.md: "retry once, then notify in overlay"
   - File: crates/coldvox-text-injection/src/manager.rs
   - Acceptance: Failed injection retries once, then emits notification event

2.5 Add integration tests for the mic → STT → injection pipeline
   - Create: tests covering the happy path and failure modes
   - Acceptance: `cargo test -p coldvox-stt` covers core paths

### Branch: worktree/ws2-stt-reliability

## Workstream 3: Overlay GUI & User Experience
### Crates: coldvox-gui, coldvox-app
### Tasks:

3.1 Audit current overlay implementation
   - Read all files in crates/coldvox-gui/src/
   - Document: what works, what's placeholder, what's missing
   - Acceptance: Written assessment of GUI state

3.2 Implement live partial text display in overlay
   - Northstar: "transparent GUI overlay that shows recognized words while speaking"
   - Northstar: "Show words live in both PTT and VAD modes"
   - Files: coldvox-gui rendering, event subscription from STT
   - Acceptance: Overlay shows text updating in real-time during speech

3.3 Implement injection failure notification in overlay
   - Northstar: "retry once, then notify in overlay"
   - Coordinate with WS2 task 2.4 (they emit the event, we display it)
   - Acceptance: Failed injection shows user-visible notification

3.4 Overlay visibility management
   - Northstar: "visible while actively capturing"
   - Implement: show overlay when PTT held or VAD detects speech, hide otherwise
   - Acceptance: Overlay appears/disappears based on capture state

3.5 Visual polish pass
   - Transparency, positioning, font sizing, multi-monitor support
   - Acceptance: Overlay is readable, non-intrusive, positioned correctly

### Branch: worktree/ws3-overlay-gui

## Workstream 4: Build System, CI, and Testing Infrastructure
### Crates: workspace-level, CI, scripts
### Tasks:

4.1 Audit and fix local_ci.sh
   - Read scripts/local_ci.sh
   - Verify it tests what it claims
   - Remove references to dead features (whisper, faster-whisper)
   - Acceptance: `./scripts/local_ci.sh` runs clean on current codebase

4.2 Add CI job for moonshine feature flag
   - File: .github/workflows/ (find or create)
   - Acceptance: CI builds with `--features moonshine` on every PR

4.3 Add CI job that verifies parakeet compiles (after WS2 fixes it)
   - Acceptance: CI catches future parakeet breakage

4.4 Add workspace-level integration test
   - Test the full pipeline: mock audio → VAD → STT → injection
   - Acceptance: `cargo test --workspace` includes pipeline integration test

4.5 Verify deny.toml and dependency audit
   - Read deny.toml, run `cargo deny check`
   - Acceptance: No denied dependencies, no security advisories

4.6 Update CHANGELOG.md with all changes across workstreams
   - Acceptance: CHANGELOG reflects everything that changed

### Branch: worktree/ws4-build-ci

## Dependency Map
- WS1 should run first or in parallel with others (it removes dead code others might reference)
- WS2 task 2.4 (retry logic) coordinates with WS3 task 3.3 (notification display)
- WS4 task 4.3 depends on WS2 task 2.2 (parakeet fix)
- All other tasks are independent across workstreams

## Risk Register
- Moonshine Python bridge (PyO3) is version-sensitive — WS1 Python fix must not break it
- Parakeet 0.2 API changes may be larger than expected — if fix exceeds 2 hours, defer
- Overlay GUI may have platform-specific issues (Windows vs Linux) — test on available platform only
- Feature flag removal may break downstream CI — check all workflow files first
```

Fill in every task with the same level of detail shown above. Include specific file paths, crate names, and acceptance criteria. Read the actual code to make the plan concrete, not speculative.

## PHASE 3: PARALLEL EXECUTION VIA GIT WORKTREES + SUBAGENTS

After the plan is written and saved, execute it using 4 parallel subagents, each in an isolated git worktree.

### Step 1: Create worktree branches

Run these commands in the terminal:
```bash
git worktree add -b worktree/ws1-p0-cleanup .worktrees/ws1-p0-cleanup HEAD
git worktree add -b worktree/ws2-stt-reliability .worktrees/ws2-stt-reliability HEAD
git worktree add -b worktree/ws3-overlay-gui .worktrees/ws3-overlay-gui HEAD
git worktree add -b worktree/ws4-build-ci .worktrees/ws4-build-ci HEAD
```

### Step 2: Spawn 4 parallel subagents

Use #runSubagent to spawn 4 subagents simultaneously. Each subagent works in its own worktree directory with its own crate scope.

**Subagent 1 — P0 Cleanup:**
```
You are working in the git worktree at .worktrees/ws1-p0-cleanup on ColdVox, a Rust voice pipeline.
Read PLAN.md for full context, then execute Workstream 1: P0 Cleanup.

Your scope:
- Remove all whisper dead code (plugins, features, docs)
- Remove all stub features (coqui, leopard, silero-stt, no-stt)
- Fix Python version chaos (mise.toml, .python-version)
- Delete vestigial requirements.txt
- Fix CI mismatches
- Update all docs to match reality

Constraints:
- Run `cargo check --workspace` after removing features
- Run `cargo fmt --all -- --check`
- Run `cargo clippy --workspace --all-targets`
- Do NOT touch STT plugin logic (that's WS2's job)
- Do NOT modify the GUI (that's WS3's job)
- Commit with message "fix: P0 cleanup — remove dead code, fix docs, fix toolchain (WS1)"
```

**Subagent 2 — STT Reliability:**
```
You are working in the git worktree at .worktrees/ws2-stt-reliability on ColdVox, a Rust voice pipeline.
Read PLAN.md for full context, then execute Workstream 2: STT Pipeline Reliability.

Your scope (crates ONLY):
- coldvox-stt (moonshine audit, parakeet fix, streaming partial transcription)
- coldvox-text-injection (injection retry logic)
- coldvox-audio (if needed for pipeline tracing)

Constraints:
- Run `cargo check -p coldvox-stt` and `cargo check -p coldvox-text-injection` after changes
- Run `cargo test -p coldvox-stt`
- Do NOT claim Whisper is working
- Do NOT modify the GUI crate
- Do NOT modify CI workflows
- Commit with message "feat: STT reliability — moonshine audit, parakeet fix, retry logic (WS2)"
```

**Subagent 3 — Overlay GUI:**
```
You are working in the git worktree at .worktrees/ws3-overlay-gui on ColdVox, a Rust voice pipeline.
Read PLAN.md for full context, then execute Workstream 3: Overlay GUI & UX.

Your scope (crates ONLY):
- coldvox-gui (all overlay work)
- coldvox-app (only for wiring GUI events)

Constraints:
- Run `cargo check -p coldvox-gui` and `cargo check -p coldvox-app` after changes
- Do NOT modify STT plugins or text injection logic
- Do NOT modify CI or build scripts
- Coordinate with WS2 on the injection failure event interface
- Commit with message "feat: overlay GUI — live text, notifications, visibility (WS3)"
```

**Subagent 4 — Build & CI:**
```
You are working in the git worktree at .worktrees/ws4-build-ci on ColdVox, a Rust voice pipeline.
Read PLAN.md for full context, then execute Workstream 4: Build System, CI, Testing.

Your scope:
- scripts/local_ci.sh
- .github/workflows/ (CI files)
- deny.toml
- CHANGELOG.md
- Workspace-level test files

Constraints:
- Run `./scripts/local_ci.sh` after changes
- Do NOT modify any crate source code
- Do NOT modify docs (that's WS1's job)
- Commit with message "chore: CI fixes, testing infrastructure, changelog (WS4)"
```

### Step 3: Integration

After all 4 subagents complete:
1. Review each worktree branch for conflicts
2. Merge in order: WS1 (cleanup first), WS2 (STT), WS3 (GUI), WS4 (CI)
3. Resolve merge conflicts
4. Run full workspace verification:
   ```bash
   cargo build --workspace --locked
   cargo clippy --workspace --all-targets --locked
   cargo test --workspace --locked
   cargo fmt --all -- --check
   ```
5. Verify the end-to-end pipeline if hardware is available
6. Update CHANGELOG.md with combined summary

## CONSTRAINTS

- Do NOT claim Whisper is a working backend
- Do NOT claim Parakeet is currently production-ready (it will be after WS2 fixes it)
- Do NOT use `unwrap()` in production code paths — use proper error handling
- Do NOT add dependencies without stating why
- Do NOT use `apt-get` on the Fedora self-hosted CI runner
- Do NOT modify crates outside your assigned workstream
- Always run crate-scoped checks first (`cargo check -p <crate>`) for fast feedback
- Run `cargo fmt --all -- --check` before finishing
- Follow the precedence: northstar.md > anchor doc > CI architecture doc > other docs
