# Issue Triage - 2025-11-10

This document records the quick triage pass over the currently open issues (34 total) and provides an actionable classification: Close / Keep / Split / Consolidate. The goal is confirmation and classification only — no deep debugging.

Summary of actions taken:
- Ran repository presence checks (grep/ls) for keywords referenced in the issue set (faster-whisper, x11, atspi, docs/domains, changelog workflows, hooks).
- Used evidence from code, docs, and workflow files to mark issues as superseded, done, or still-relevant.
- Marked ambiguous items as `status:confirming` for a follow-up manual check.

Totals (follow-up pass):
- Closed / Superseded candidates: 16 _(includes the confirmed clipboard/AT-SPI fixes plus GUI duplicates #58-#63/#227)_
- Kept / Roll-forward: 16 _(now includes the renewed #38 focus issue and the split follow-ups #228-#230)_
- Consolidated / Split-tracked: 7 _(Testing Infra Phase 2 funnel for #208-#212, the #136 split, and #171 collapsing into #38)_
- Needs confirmation (follow-up): 0 _(all prior confirm-and-close items resolved or reassigned)_

---

## Table: Issue → Action → Rationale

| Issue | Action | Rationale / Evidence |
|------:|:------:|:---------------------|
| 179 | Close (superseded) | Faster-Whisper migration items. Repo references to `faster-whisper` exist in CI and docs but there is an explicit Candle migration plan (`docs/plans/stt-candle-whisper-migration.md`) and PR #219 referenced in planning. Close and link migration plan. |
| 41 | Close (superseded) | Faster-Whisper STT plugin references present, but roadmap and PR #219 move to Candle/pure-Rust. Mark superseded. |
| 174 | Close (done) | Documentation domain structure exists under `docs/domains/` — appears implemented. Keep note to link specific commits if needed. |
| 138 | Close (done) | Changelog/workflow checks found `.github/workflows/` references; treat as implemented. |
| 160 | Close (done) | Strategy manager always appends `InjectionMethod::ClipboardPasteFallback` at the end of the attempt order (`crates/coldvox-text-injection/src/manager.rs:596-744`), and the unified clipboard path seeds/restores clipboard data while cascading through Enigo and ydotool paste attempts (`crates/coldvox-text-injection/src/injectors/unified_clipboard.rs:446-735`). |
| 159 | Close (done) | All AT-SPI entry points bind to `org.a11y.atspi.Registry` with `/org/a11y/atspi/accessible/root` (see `crates/coldvox-text-injection/src/injectors/atspi.rs:83-150`, `prewarm.rs:188`, `confirm.rs:192`), matching the fixed D-Bus path spec. |
| 172 | Close (test added) | Added `crates/coldvox-text-injection/src/tests/xclip_roundtrip_test.rs` to cover the X11 roundtrip (ignored unless DISPLAY + xclip exist), so the regression is backed by a concrete test. |
| 171 | Consolidate (→ #38) | `SystemFocusAdapter` still returns `FocusStatus::Unknown` pending the AT-SPI focus backend (`crates/coldvox-text-injection/src/focus.rs:74-119`); merge this issue into the renewed #38 tracking item. |
| 38 | Keep / roll-forward | AT-SPI Application Identification Enhancement now inherits the focus/backend scope from #171. Keep it open with `status:roll-forward`, wire it to the `focus.rs` work, and make it the canonical tracker for AT-SPI focus coverage. |
| 133 | Close (replace) | Large tracking/meta issue — many coordination tasks are complete. Close and replace with smaller epics if needed. |
| 136 | Split-needed | "Crazy enormous issue" — split into focused actionable tasks. |
| 100 | Close (done) | CI & pre-commit work: scripts like `install-githooks.sh` and `ensure_venv.sh` exist. Mark done and close. |
| 34 | Keep | Plugin architecture & STT extensibility still relevant — align with Candle migration plan. |
| 37 | Keep | Plugin architecture — keep and label blocked by Candle migration where applicable. |
| 42 | Keep | Telemetry/performance related — keep and map to benchmarking epic. |
| 44 | Keep | Performance task — schedule under benchmarking infrastructure. |
| 45 | Keep | Telemetry/security — keep. |
| 46 | Keep | STT loading validation/security — keep and align with Candle migration. |
| 47 | Keep | Performance/telemetry — same as above. |
| 58 | Close (→ #226) | Comment that scope lives in #226 (GUI Integration Roadmap) and close once the new issue is confirmed. |
| 59 | Close (→ #226) | Same as #58 — all tasks tracked in the roadmap milestones (docs/tasks/issue-gui-integration-roadmap.md). |
| 60 | Close (→ #226) | " |
| 61 | Close (→ #226) | " |
| 62 | Close (→ #226) | " |
| 63 | Close (→ #226) | " |
| 226 | Keep (canonical) | Use #226 “GUI Integration Roadmap” as the consolidated tracker; mirrors `docs/tasks/issue-gui-integration-roadmap.md` milestones (M1–M4). Add `status:roll-forward` + GUI labels. |
| 227 | Close (duplicate) | Created in parallel with #226; merge any unique comments into #226, comment with “tracking via #226” and close as duplicate. |
| 173 | Keep | VM-based compositor matrix — keep for multi-environment testing goals (links into Testing Infra Phase 2). |
| 212 | Consolidate (→ Testing Infra Phase 2) | File the consolidated epic using docs/tasks/issue-testing-infra-phase2.md, link #208-#212 plus perf+VM issues. |
| 211 | Consolidate (→ Testing Infra Phase 2) | " |
| 210 | Consolidate (→ Testing Infra Phase 2) | " |
| 209 | Consolidate (→ Testing Infra Phase 2) | " |
| 208 | Consolidate (→ Testing Infra Phase 2) | " |
| 221 | Keep / roll-forward | Whisper/VAD golden master reliability owner; align with Testing Infra Phase 2 initiative #1. |
| 222 | Keep / roll-forward | Canonical benchmarking harness tracker — update body per docs/tasks/issue-222-benchmarking.md and link #42/#44/#45/#47. |
| 224 | Keep / roll-forward | Faster-Whisper doc/reference cleanup aligned with Candle migration; ensure README/CI references updated before closing. |
| 228 | Keep / split-from-136 | CI/CD workflow enhancements split from #136 (`docs/tasks/issue-136-split.md`). Labels: `ci`, `enhancement`, `status:roll-forward`. |
| 228 | Keep / split-from-136 | CI/CD workflow enhancements split from #136 (`docs/tasks/issue-136-split.md`). Labels: `ci`, `enhancement`, `status:roll-forward`. |
| 229 | Keep / split-from-136 | Dependency audit + `cargo-deny` cleanup. Add `dependencies`, `tech-debt`, and link back to #136 closure comment. |
| 230 | Keep / split-from-136 | Developer onboarding/documentation improvements. Requires README + CONTRIBUTING refresh as described in `issue-136-split.md`. |

> Notes: Several issues reference broader plans that were superseded by PR #218 and the Candle migration plan (#219). Those should be closed with the "superseded" template and linked to the migration docs.

---

## Follow-up new issues (split or consolidation suggestions)
1. **GUI Integration Roadmap (#226 canonical, #227 duplicate):** Consolidated from #58-63. See [issue-gui-integration-roadmap.md](./issue-gui-integration-roadmap.md).
2. **Testing Infra Phase 2 (new GH issue required):** Consolidated from #208-#212 plus #173/#42/#44/#45/#47/#221/#222. See [issue-testing-infra-phase2.md](./issue-testing-infra-phase2.md) for ready-to-paste body.
3. **Deconstruct "Crazy Enormous Issue" #136:** Split into #228 (CI/CD), #229 (dependency audit), and #230 (onboarding). See [issue-136-split.md](./issue-136-split.md).
4. **Benchmarking Harness Alignment (#222):** Use [issue-222-benchmarking.md](./issue-222-benchmarking.md) when refreshing the GitHub issue body and linking #42/#44/#45/#47.

### Ready-to-file issue text: Testing Infrastructure Phase 2

```
Title: Testing Infrastructure Phase 2 (Golden masters, model paths, benchmarking)

This consolidates #208, #209, #210, #211, #212, #173, #42, #44, #45, #47, #221, and #222 into a single follow-up epic.

Goals:
- Stabilize VAD/stt golden-master tests using the timeout + isolation patterns from PR #152.
- Fix model path discovery (Vosk + Whisper) so CI can locate artifacts deterministically.
- Stand up the VM-based compositor testing matrix (Wayland, X11, KDE, GNOME) described in #173.
- Build/own a benchmarking harness that records VAD/STT/text-injection metrics per PR (#42/#44/#45/#47).
- Wire these checks into CI and document how to run them locally.

Acceptance:
- CI job `testing-infra-phase2` runs the golden master + benchmarking suites.
- Docs updated to describe how to refresh golden data and run compositors locally.
- Linked issues above are referenced/closed with status updates.
```

### Outstanding manual GitHub actions
- [ ] File the Testing Infrastructure Phase 2 issue using the snippet above and immediately cross-link #208–#212, #221, #222, #173, #42, #44, #45, and #47.
- [ ] Comment + close GUI legacy issues #58–#63 referencing #226, and close #227 as a duplicate once any unique notes are copied over.
- [ ] Edit #222’s description to name the benchmarking harness explicitly and to reference #42/#44/#45/#47; apply the `benchmarking` label to all performance issues.
- [ ] Ensure each kept issue (#34, #37, #42, #44, #45, #46, #47, #173, #221, #222, #224, #228–#230) carries a `status:roll-forward` label (plus `benchmarking` where relevant).
- [ ] Backfill closure comments for #174, #138, #100, #41, #179, #133, and #136 using the templates below so the audit trail is consistent.
- [ ] Use #224 to scrub Faster-Whisper references from README/CI/docs; once complete, confirm that only #221 remains for Whisper backend work before closing other legacy STT tickets.

---

## Closure / Update Templates
- Superseded:
  > Closing as superseded by PR #218 and the Candle migration plan (#219). Functionality direction changed to Candle/pure-Rust path; this issue's scope no longer applies.

- Completed:
  > Closing: functionality implemented in commit <hash> / PR <number>. Please reopen if gaps remain.

- Replaced:
  > Closing in favor of consolidated roadmap issue #<new>. Original goals preserved; tracking continues there.

- Needs decomposition:
  > Updating: This issue is too broad. Will split into focused issues for workload sizing (list forthcoming). Keeping open until splits are created.

---

## Next steps (manual actions requiring GH access)
- For each issue marked Close or Close (superseded): apply `status:done-needs-close` or `status:obsolete`, add a comment with the template and close the issue.
- For Keep / roll-forward items: add `status:roll-forward` and assign owners and milestones (Q1 2026 where appropriate).
- For Consolidate / Split-needed: create new meta issues and link originals. Tag originals with `status:split-needed` and reference the newly-created meta issue.

---

## Closing notes
This is a conservative first pass. Where code or docs clearly indicate the work is implemented or superseded, I've marked for closure. Items touching the STT migration and architecture were evaluated relative to `docs/plans/stt-candle-whisper-migration.md` and references to `faster-whisper` in CI and README were treated as historical or transitional references.

If you want me to apply the GH updates now (labels, comments, closing, and creating consolidated issues), say `proceed` and I'll continue with the GH-backed updates (requires repository GH token / permissions). Otherwise I can make the triage file more detailed (include explicit issue lines for all 34 IDs) — tell me which you'd prefer.
