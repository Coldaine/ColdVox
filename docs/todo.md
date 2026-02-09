---
doc_type: standard
subsystem: general
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# Documentation Todo Backlog

## Epic: Documentation Migration

- [x] Establish canonical directory skeleton and enforcement tooling (Phase 1).
- [ ] Migrate existing Markdown into canonical structure (Phase 2–3).
  - [ ] Align GUI documentation with canonical domains ([spec](./tasks/gui-integration.md)).
  - [ ] Document CI runner readiness requirements ([spec](./tasks/ci-runner-readiness-proposal.md)).
- [ ] Normalize content metadata and retention banners (Phase 3 follow-up).
- [ ] Cross-link tasks and specs per playbook policy (Phase 4).
- [ ] Finalize documentation governance updates in root README and standards.

## Epic: Testing Strategy
- [x] Develop playbook for debugging test failures with LLMs ([playbook](./playbooks/testing/llm-test-debugging-playbook.md)).

## Epic: Agentic Documentation Governance

- [ ] Replace deterministic frontmatter CI gate with scoped LLM docs reviewer in CI.
- [ ] Define strict prompt contract for docs review: intent type, status, freshness, conflicts, evidence links.
- [ ] Implement non-blocking advisory mode first; collect precision/recall notes on 20+ PRs.
- [ ] Add human override label/process for contested LLM findings.
- [ ] Decide blocking threshold after advisory trial and document the promotion criteria.
- [ ] Track rollout issue and scope decisions in [docs issue](./issues/docs-agentic-semantic-review-rollout.md).
- [ ] Keep the review rules synced with [semantic review contract](./reference/docs-semantic-review-contract.md).
