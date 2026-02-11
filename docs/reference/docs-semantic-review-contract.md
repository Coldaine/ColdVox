---
doc_type: reference
subsystem: general
status: draft
freshness: current
preservation: reference
summary: Contract for agentic docs semantic review and placement policy
last_reviewed: 2026-02-09
owners: Documentation Working Group
version: 1.0.0
---

# Docs Semantic Review Contract

This contract defines how the documentation review agent evaluates changed docs in pull requests.

## Purpose

- Keep documentation aligned with North Star and execution reality.
- Allow aspirational and research content without forcing implementation parity.
- Prevent low-value or misplaced docs from accumulating.

## Top Controls

1. Intent classification is mandatory (`northstar`, `spec`, `implementation`, `research`, `history`, `playbook`, `task`, `reference`).
2. Authority chain must be explicit (canonical vs subordinate docs).
3. Shipped claims require evidence links to code/tests/config paths.
4. Active docs should map to at least one North Star goal (or be marked research/history).
5. Every touched doc must get a lifecycle decision (`keep`, `revise`, `archive`, `delete`).
6. Cross-doc conflicts must be detected with a nominated source of truth.
7. Placement policy is enforced from `doc_type` to allowed directories.
8. Archive is preferred over delete unless there is no salvage value.

## Doc Type Aliases

Legacy labels are normalized before policy checks:

- `implementation-plan` -> `plan`
- `dev-guide` -> `reference`
- `runbook` -> `playbook`

## Placement Policy

Policy has two levels:

- `preferred`: ideal placement, no finding.
- `allowed`: accepted but flagged as placement drift (`minor`).

If a path matches neither preferred nor allowed for its `doc_type`, it is `placement-invalid` (`major`).

### `architecture`

- preferred: `docs/architecture.md`, `docs/northstar.md`, `docs/architecture/**`, `docs/domains/**`, `docs/dev/CI/**`
- allowed: `docs/plans/**`, `docs/archive/**`

### `standard`

- preferred: `docs/standards.md`, `docs/todo.md`, `docs/anchor-*.md`, `docs/repo/**`
- allowed: `docs/*.md`, `docs/dev/**`

### `playbook`

- preferred: `docs/playbooks/**`, `docs/observability-playbook.md`, `docs/MasterDocumentationPlaybook.md`
- allowed: `docs/*.md`, `docs/domains/**`, `docs/archive/**`

### `reference`

- preferred: `docs/reference/**`, `docs/domains/**`, `docs/repo/**`, `docs/dependencies.md`, `docs/logging.md`
- allowed: `docs/*.md`, `docs/playbooks/**`

### `research`

- preferred: `docs/research/**`, `docs/archive/research/**`
- allowed: `docs/plans/**`, `docs/history/**`, `docs/archive/**`

### `plan`

- preferred: `docs/plans/**`, `docs/tasks/**`, `docs/issues/**`
- allowed: `docs/research/**`, `docs/archive/plans/**`, `docs/domains/**`

### `troubleshooting`

- preferred: `docs/issues/**`, `docs/domains/**/troubleshooting/**`
- allowed: `docs/playbooks/**`, `docs/tasks/**`, `docs/domains/**`

### `index`

- preferred: `docs/index.md`, `docs/reference/**`, `docs/archive/reference/**`, `docs/**/index.md`, `docs/**/overview.md`, `docs/**/*-overview.md`
- allowed: `docs/domains/**`, `docs/archive/**`

### `history`

- preferred: `docs/history/**`, `docs/archive/**`
- allowed: `docs/research/logs/**`

## Output Contract

The agent must emit strict JSON with:

- global pass/fail and max severity
- per-document decision, intent type, severity, confidence, required actions
- cross-doc conflicts
- in-scope issues

## Aggressive Mode Defaults

- Block PR on `major` or `critical` findings when confidence is high.
- Emit concrete patch suggestions for flagged docs.
- Keep human override available for disputed findings.
