---
doc_type: review
subsystem: ci
status: active
last_reviewed: 2026-03-31
---

# Critique: Portable Agentic Evidence Standard

## The Problem with Tautological Unit Tests

Traditional unit tests in a project like ColdVox suffer from a fundamental epistemic flaw: **they only prove what the author chose to test**. A developer who implements Moonshine STT integration can write tests that mock the Python subprocess, mock the audio buffer, and mock the output — passing 100% while the actual end-to-end path is completely broken (as has repeatedly happened in this repo).

This is not a criticism of unit tests per se. It is a criticism of treating unit test pass/fail as a proxy for "the claim made in this PR is true."

### Concrete Failure Modes Observed in ColdVox

1. **Stub features passing CI**: The `whisper`, `coqui`, `leopard`, and `silero-stt` feature flags compiled cleanly and passed unit tests for months while being completely non-functional. CI said "green." The feature was dead.

2. **Semantic Drift in documentation**: `docs/northstar.md` describes streaming partial transcription as a requirement. The actual code has no streaming path. The doc claim is unverified by any test. CI is blind to this gap.

3. **PyO3 DLL link failures**: The Moonshine backend would compile and "pass" on CI runners that lacked the correct Python environment, then fail at runtime. No test caught this because the test mocked the subprocess.

4. **Architecture docs contradicting code**: Multiple docs referenced `docs/plans/windows-multi-agent-recovery.md` as the execution anchor, while the actual code had diverged without a doc update. Nobody noticed because there is no automated check for semantic consistency.

## The Core Thesis: Evidence, Not Coverage

The **Portable Agentic Evidence Standard** rests on one insight: **a PR should be required to demonstrate, not merely claim**.

> "Claim" = text in a PR description or commit message asserting behavior.
> "Evidence" = code change, test output, benchmark, or runtime log that would fail if the claim were false.

A traditional test suite checks the second but cannot audit the first. An agentic reviewer can read both.

## What "Portable" Means

The standard is "portable" because it does not depend on this specific repo's test suite, language, or toolchain. The agent's evaluation criteria are:

1. **Material Claim Extraction**: Parse the PR title and body for factual assertions about behavior, performance, or correctness.
2. **Evidence Search**: Look at code changes, added tests, runtime artifacts attached to the PR.
3. **Semantic Drift Detection**: Compare what docs say versus what code does for the subsystems touched by the PR.
4. **Verdict**: `EVIDENCE_PRESENT` / `EVIDENCE_WEAK` / `EVIDENCE_MISSING`.

## What This Standard Does NOT Do

- It does not replace unit tests, integration tests, or CI linting.
- It does not block merges (shadow mode only — Phase 1).
- It does not make architectural decisions.
- It does not evaluate code style or Rust best practices (other tools do that).

## Known Limitations

- The LLM may hallucinate evidence it did not see in the diff. The prompt must explicitly instruct the agent to only cite evidence present in the provided context.
- The binary verdict (`EVIDENCE_PRESENT` etc.) is coarse. A future iteration should include a confidence score.
- Token limits mean large diffs must be truncated; the agent may miss evidence buried in large refactors.

## Recommendation

Adopt the shadow mode assessor as Phase 1. Run it on all PRs. Require humans to read the Step Summary report before approving. Do not gate merges on its verdict until the false-positive rate is measured over at least 20 PRs.
