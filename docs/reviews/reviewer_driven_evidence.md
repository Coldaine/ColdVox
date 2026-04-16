---
doc_type: review
subsystem: ci
status: active
last_reviewed: 2026-03-31
---

# Reviewer-Driven Evidence: Workflow Strategy

## The Core Shift

Traditional CI workflows are **author-driven**: the PR author adds tests, the CI runs them, reviewers read the green checkmark. Under the Portable Agentic Evidence Standard, CI becomes **reviewer-driven**: an autonomous agent reads the PR from the *reviewer's* perspective, asking "what evidence would I need to believe this claim?" and then checking if that evidence is present.

## The Workflow

```
PR opened/updated
      │
      ▼
[Shadow Assessor Agent]
  1. Reads PR title + body
  2. Reads git diff
  3. Extracts material claims
  4. Checks diff + tests for evidence per claim
  5. Checks for semantic drift in docs
  6. Writes report → GitHub Step Summary
      │
      ▼
[Human Reviewer]
  Reads Step Summary before approving
  Uses report to guide focused review
  Does NOT need to audit every line for evidence
      │
      ▼
[Merge Decision]
  Phase 1: Human judgment (agent is advisory)
  Phase 2 (future): Block merge if EVIDENCE_MISSING + reviewer override required
```

## What Counts as Evidence

Evidence is ordered by strength (strongest first):

| Tier | Type | Example |
|------|------|---------|
| 1 | Runtime output attached to PR | Benchmark numbers, transcription output log |
| 2 | Integration test added or modified | `cargo test -p coldvox-stt --test integration` passes |
| 3 | Unit test that would fail if claim is false | A test that calls the actual code path, not a mock |
| 4 | Code change that structurally proves the claim | New struct field, new conditional branch |
| 5 | Documentation update aligned with code | Doc change matches code change |

Evidence tiers 1–3 are strong. Tier 4 is moderate. Tier 5 alone is weak (docs can be wrong).

**Example:**
> Claim: "This PR fixes the Moonshine STT backend crashing on Windows when the Python DLL is not found."
>
> - Strong evidence: A test that loads the DLL path, asserts on error type, and a CI log showing the test passed on Windows.
> - Weak evidence: A comment in the code saying "// fixed for Windows."
> - No evidence: Just changing the docs to say it works.

## Semantic Drift Detection

Semantic drift occurs when code and docs describe the same subsystem differently. The agent detects this by:

1. Identifying which subsystems the diff touches (e.g., `crates/coldvox-stt/` → STT subsystem).
2. Finding documentation about that subsystem (e.g., `docs/northstar.md` → STT goals).
3. Checking whether the PR's code changes contradict or confirm the documented behavior.

**ColdVox-specific drift signals to watch:**
- `docs/northstar.md` claims streaming partial transcription → does the STT code support it?
- `AGENTS.md` lists Parakeet as "planned not production-ready" → does any PR claim Parakeet works?
- `docs/plans/current-status.md` describes Moonshine as "fragile dependency" → does any PR claim it's stable?

## How Reviewers Use the Report

The Step Summary report is intended to reduce cognitive load for human reviewers:

1. **Skip** reading lines of code that implement well-evidenced claims — the agent verified them.
2. **Focus** on claims marked `EVIDENCE_WEAK` or `EVIDENCE_MISSING` — these are the review risk areas.
3. **Investigate** any semantic drift the agent flags — these are documentation debt that should be resolved.

The agent does NOT tell reviewers what to do. It gives them a structured starting point.

## Non-Goals

- The agent is not a code reviewer. It does not comment on code quality, style, or architecture.
- The agent does not replace domain expertise. It cannot judge whether an algorithm is correct — only whether the PR *claims* something and provides *some* evidence for it.
- The agent report is not authoritative. Reviewers can and should override it with domain knowledge.
