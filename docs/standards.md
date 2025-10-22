---
doc_type: standard
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# Documentation Standards

This standard documents repository-specific enforcement notes derived from the Master Documentation Playbook v1.0.0.

## Metadata Schema

All Markdown files under `/docs` MUST include the canonical frontmatter:

```yaml
doc_type: [architecture|standard|playbook|reference|research|plan|troubleshooting|index]
subsystem: [domain-name|general]
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: YYYY-MM-DD
```

## Approved Exceptions

- Root `README.md`
- Root `CHANGELOG.md`
- Workspace configuration directories such as `.vscode/`
- Crate README files required for package publishing (see `docs/reference/crates/`)

Any additional exception requires approval from the Documentation Working Group and must be recorded here.

## Changelog Rubric

All substantive documentation updates must:

1. Update the relevant section in this file if they introduce new exceptions or schema adjustments.
2. Append an entry to `docs/revision_log.csv` via the automated watcher.

## Watcher Specification

The docs CI workflow invokes the revision logger to append entries to `docs/revision_log.csv` for any Markdown file updates. Contributors MUST NOT edit the CSV manually; rely on the automation to ensure consistent auditing.
