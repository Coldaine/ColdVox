---
doc_type: plan
subsystem: general
status: draft
freshness: current
preservation: preserve
summary: Rollout issue for agentic semantic documentation review in CI
last_reviewed: 2026-02-09
owners: Documentation Working Group
version: 1.0.0
---

# Issue: Agentic Semantic Docs Review Rollout

## Goal

Replace the retired deterministic frontmatter gate with aggressive agentic docs review in CI, while keeping placement rules flexible and practical.

## Implementation Track

- [x] Land `scripts/docs_semantic_review.py` as the packet + policy engine.
- [x] Wire CI advisory step to generate packet/prompt artifacts for changed docs.
- [x] Add LLM execution step (provider-backed) that returns strict JSON.
- [ ] Add parser step that fails on `major+` findings at configured confidence.
- [ ] Enable auto-patch output for docs with findings.
- [ ] Add `docs-override` label/process for disputed findings.
- [ ] Run advisory for at least 20 PRs and record precision/recall.
- [ ] Promote to blocking mode after advisory threshold is met.

## Policy Decisions Locked

- Placement policy uses `preferred` and `allowed` paths by `doc_type`.
- `allowed` path matches are warnings (`minor`), not hard failures.
- Missing or unknown `doc_type`, or invalid placement, is `major`.
- Legacy `doc_type` aliases are normalized (`implementation-plan`, `dev-guide`, `runbook`).
- Archive is preferred over delete unless no salvage value exists.

## Additional In-Scope Issues

- [ ] Resolve policy drift between `docs/standards.md` and `docs/MasterDocumentationPlaybook.md` around CI frontmatter hard-fail language.
- [ ] Update `docs/observability-playbook.md` to remove stale references to frontmatter CI schema gating.
- [ ] Normalize legacy `doc_type` values in live docs (for example `implementation-plan` -> `plan`).
- [ ] Add a clear provider integration section for running semantic review in CI (`OPENAI_API_KEY` or equivalent secret contract).
- [ ] Define where CI stores semantic review artifacts for auditability.

## Success Criteria

- CI runs semantic docs review on doc-changing PRs.
- Placement and garbage-doc checks are enforced with low manual overhead.
- Review output includes actionable patch suggestions.
- Human intervention is exception-only, not default workflow.
