# Dependency Graph Comparison: Plan 1 vs Plan 2

## Plan 1: Fix/Feature-Based Stack

```
                    ┌─────────────────────────┐
                    │ 10. docs/deployment     │ (Documentation)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 9. runtime-unification  │ (Refactor - Large)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 8. wav-loader-e2e       │ (Feature)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 7. vad-determinism      │ (Feature)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 6. audio-stability      │ (Feature)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 5. text-injection-strat │ (Refactor)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 4. config-system        │ (Refactor - Large)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 3. clipboard-restore-p1 │ (Fix - P1)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 2. clipboard-paste-p0   │ (Fix - P0 Bug)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 1. test-infrastructure  │ (Fix - Tests)
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │          main           │
                    └─────────────────────────┘
```

### Issues with Plan 1 Dependencies

❌ **Problem 1: Serial text-injection changes**
```
PR #2 (clipboard-paste)
  ↓ depends on
PR #3 (clipboard-restore)  
  ↓ depends on
PR #5 (text-injection-strategy)

SAME CRATE = 3 sequential rebases!
```

❌ **Problem 2: Delayed runtime changes**
```
PR #6 (audio) wants to integrate with runtime
PR #7 (vad) wants to integrate with runtime
PR #8 (wav-loader) wants to integrate with runtime
  BUT
PR #9 (runtime-unification) not merged yet!

Result: Complex merge conflicts or blocking dependencies
```

❌ **Problem 3: Test/Feature coupling**
```
PR #1: Test infrastructure changes
PR #8: E2E test additions
Other PRs: Implicit test updates

Result: Test changes interleaved with features
```

---

## Plan 2: Domain-Based Stack

```
                    ┌─────────────────────────┐
                    │ 09. docs/changelog      │
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 08. logging/observ.     │
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ 07. testing infra       │
                    └───────────┬─────────────┘
                                │
              ┌─────────────────▼─────────────────┐
              │                                    │
  ┌───────────▼─────────────┐       ┌─────────────▼───────────┐
  │ 06. text-injection      │       │ (glue if needed)        │
  └───────────┬─────────────┘       └─────────────────────────┘
              │
  ┌───────────▼─────────────┐
  │ 05. app-runtime-wav     │
  └───────────┬─────────────┘
              │
    ┌─────────┴─────────┐
    │                   │
┌───▼──────────┐   ┌────▼─────────┐
│ 03. vad      │   │ 04. stt      │  ← Parallel (both depend on config+audio)
└───┬──────────┘   └────┬─────────┘
    │                   │
    └─────────┬─────────┘
              │
  ┌───────────▼─────────────┐
  │ 02. audio-capture       │
  └───────────┬─────────────┘
              │
  ┌───────────▼─────────────┐
  │ 01. config-settings     │
  └───────────┬─────────────┘
              │
  ┌───────────▼─────────────┐
  │          main           │
  └─────────────────────────┘
```

### Benefits of Plan 2 Dependencies

✅ **Benefit 1: Natural crate boundaries**
```
PR #01 → crates/app/src/lib.rs + config/**
PR #02 → crates/coldvox-audio/**
PR #03 → crates/coldvox-vad/**
PR #04 → crates/coldvox-stt/**
PR #06 → crates/coldvox-text-injection/**  (single PR, all changes)

EACH CRATE = 1 PR = 1 rebase
```

✅ **Benefit 2: Parallel work possible**
```
PR #03 (vad) and PR #04 (stt) can be developed simultaneously
  ↓ both depend on
PR #02 (audio) and PR #01 (config)

Result: Faster development, independent reviews
```

✅ **Benefit 3: Testing isolation**
```
PR #07: ALL test infrastructure changes
PRs #01-06: Minimal test updates

Result: Easy to review test validity independently
```

---

## Merge Conflict Analysis

### Plan 1: High Conflict Risk

```
Merge Sequence:
1. PR #2 merges → clipboard_paste_injector.rs modified
2. PR #3 rebases → CONFLICT in clipboard_paste_injector.rs
3. PR #3 merges → clipboard_paste_injector.rs modified
4. PR #5 rebases → CONFLICT in clipboard_paste_injector.rs + manager.rs
5. PR #9 merges → runtime.rs modified
6. PRs #6,7,8 rebase → CONFLICT in runtime.rs (3 PRs!)

TOTAL EXPECTED CONFLICTS: 5-7 major rebases
```

### Plan 2: Low Conflict Risk

```
Merge Sequence:
1. PR #01 merges → config/** + lib.rs modified
2. PRs #02,03,04 rebase → Minor conflicts in imports (3 PRs, same rebase)
3. PR #05 merges → runtime.rs modified
4. PR #06 rebases → Minor conflicts in runtime integration (1 PR)
5. PRs #07,08,09 rebase → Documentation-only conflicts (minimal)

TOTAL EXPECTED CONFLICTS: 2-3 minor rebases
```

---

## Review Complexity Analysis

### Plan 1: High Context Switching

**Reviewer must understand:**
- PR #1: Test infrastructure + Settings API
- PR #2: Clipboard internals + paste logic
- PR #3: Clipboard internals + restore logic (repeat context!)
- PR #4: Config system architecture (large context switch)
- PR #5: Strategy manager (back to text-injection, repeat context!)
- PR #6: Audio capture threading model (context switch)
- PR #7: VAD algorithms (context switch)
- PR #8: WAV file format + E2E testing (context switch)
- PR #9: Runtime lifecycle + VAD/STT integration (huge context)
- PR #10: Documentation (context switch)

**Context switches: 10 major switches**

### Plan 2: Domain Expertise

**Reviewer specialization:**
- PR #01: Config expert reviews (1 domain)
- PR #02: Audio expert reviews (1 domain)
- PR #03: VAD expert reviews (1 domain)
- PR #04: STT expert reviews (1 domain)
- PR #05: Runtime architect reviews (1 domain)
- PR #06: Text-injection expert reviews (1 domain, all changes at once)
- PR #07: Test engineer reviews (1 domain)
- PR #08: Observability engineer reviews (1 domain)
- PR #09: Technical writer reviews (1 domain)

**Context switches: 1 per PR (minimal)**

**Benefit:** Can assign PRs to domain owners; parallel reviews possible.

---

## Crate-Level Dependency Graph

### ColdVox Workspace Architecture (from CLAUDE.md)

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  coldvox-foundation (state, shutdown, health, error)       │
│                                                             │
└────────────────────┬────────────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        │                         │
┌───────▼──────┐         ┌────────▼────────┐
│              │         │                 │
│ coldvox-     │         │ coldvox-        │
│ audio        │         │ telemetry       │
│              │         │                 │
└───────┬──────┘         └─────────────────┘
        │
        ├─────────────────┬──────────────────┐
        │                 │                  │
┌───────▼──────┐  ┌───────▼─────┐   ┌───────▼──────┐
│              │  │             │   │              │
│ coldvox-vad  │  │ coldvox-    │   │ coldvox-text-│
│              │  │ stt         │   │ injection    │
│              │  │             │   │              │
└───────┬──────┘  └───────┬─────┘   └───────┬──────┘
        │                 │                 │
        └─────────────────┴─────────────────┘
                          │
                  ┌───────▼──────┐
                  │              │
                  │ app (main)   │
                  │              │
                  └──────────────┘
```

### Plan 2 Matches This Architecture

```
PR #01 (config)          → Foundation layer
PR #02 (audio)           → coldvox-audio
PR #03 (vad)             → coldvox-vad, coldvox-vad-silero
PR #04 (stt)             → coldvox-stt, coldvox-stt-vosk
PR #05 (app-runtime)     → app (integration)
PR #06 (text-injection)  → coldvox-text-injection
PR #07 (testing)         → Cross-cutting (infrastructure)
PR #08 (logging)         → coldvox-telemetry + app
PR #09 (docs)            → Documentation only
```

**Result:** Natural bottom-up merge order respecting dependency graph.

### Plan 1 Violates This Architecture

```
PR #1 (test-infra)       → app/tests (cross-cutting)
PR #2 (clipboard-p0)     → coldvox-text-injection (partial)
PR #3 (clipboard-p1)     → coldvox-text-injection (partial, same crate!)
PR #4 (config)           → app + foundation
PR #5 (text-injection)   → coldvox-text-injection (partial, same crate again!)
PR #6 (audio)            → coldvox-audio
PR #7 (vad)              → coldvox-vad
PR #8 (wav-loader)       → app (partial)
PR #9 (runtime)          → app (partial, same crate!)
```

**Result:** Multiple PRs modify same crate; no clear dependency order.

---

## Graphite Workflow Simulation

### Plan 1: Complex `gt split --by-hunk`

```
Interactive Split Session:

Hunk 1: crates/app/tests/settings_test.rs (line 23-45)
→ Which branch? [1-10 or create new]
  - Could be PR #1 (test-infra) OR PR #4 (config)? 🤔

Hunk 2: crates/coldvox-text-injection/src/clipboard_paste_injector.rs (line 87-92)
→ Which branch? [1-10 or create new]
  - Could be PR #2 (p0 fix) OR PR #3 (p1 fix) OR PR #5 (refactor)? 🤔

Hunk 3: crates/app/src/runtime.rs (line 234-267)
→ Which branch? [1-10 or create new]
  - Could be PR #6 (audio) OR PR #7 (vad) OR PR #9 (runtime)? 🤔

COGNITIVE LOAD: HIGH
ERRORS: Likely to misassign hunks
```

### Plan 2: Natural `gt split --by-hunk`

```
Interactive Split Session:

Hunk 1: crates/app/src/lib.rs (line 23-45)
→ Branch: 01-config-settings ✓ (obvious)

Hunk 2: config/default.toml (line 1-50)
→ Branch: 01-config-settings ✓ (obvious)

Hunk 3: crates/coldvox-audio/src/capture.rs (line 87-92)
→ Branch: 02-audio-capture ✓ (obvious)

Hunk 4: crates/coldvox-vad/src/config.rs (line 234-267)
→ Branch: 03-vad ✓ (obvious)

Hunk 5: docs/deployment.md (line 45-78)
→ Branch: 09-docs-changelog ✓ (obvious)

COGNITIVE LOAD: LOW
ERRORS: Minimal (only glue code ambiguous)
```

---

## Rollback Scenario Analysis

### Scenario: PR #5 introduces a regression

**Plan 1:**
```
PR #5 = refactor/text-injection-strategy
Rollback impact:
  - PRs #6, #7, #8, #9 may depend on this (unclear)
  - Need to check runtime.rs for dependencies
  - Possible cascade rollback of 4+ PRs
```

**Plan 2:**
```
PR #05 = app-runtime-wav
Rollback impact:
  - Only PR #06 (text-injection) depends on this
  - Clear from dependency graph
  - Isolated rollback, 1 PR affected
```

---

## CI/CD Impact

### Plan 1: Frequent CI Failures

```
PR #1 merges → CI green ✓
PR #2 merges → CI green ✓
PR #3 merges → CI green ✓
PR #4 merges → CI may fail (test changes needed) ⚠️
PR #5 merges → CI may fail (runtime integration issues) ⚠️
PR #6 merges → CI may fail (audio+runtime interaction) ⚠️
PR #9 merges → CI likely fails (large refactor) ⚠️

EXPECTED CI FAILURES: 3-4 PRs need follow-up fixes
```

### Plan 2: CI Stability

```
PR #01 merges → CI green (config is foundation) ✓
PR #02 merges → CI green (audio isolated) ✓
PR #03 merges → CI green (vad isolated) ✓
PR #04 merges → CI green (stt isolated) ✓
PR #05 merges → CI green (runtime tested with all dependencies ready) ✓
PR #07 merges → CI green (test infra last, validates everything) ✓

EXPECTED CI FAILURES: 0-1 PRs (only if unforeseen integration issue)
```

---

## Conclusion

**Plan 2 is architecturally superior** because it:
1. Respects crate boundaries
2. Minimizes merge conflicts
3. Enables parallel development
4. Simplifies reviews
5. Matches natural dependencies
6. Works seamlessly with Graphite
7. Maintains CI stability

**Grade: A-** (minor deduction for P0 bug delay, easily fixed with PR #0)

**Plan 1 Grade: C+** (introduces unnecessary complexity and violates architectural principles)
