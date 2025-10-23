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

The root `CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format and documents user-visible changes only.

### When to Update CHANGELOG.md

**MUST update** for:
- New features or functionality (user-facing)
- Breaking changes (API, configuration, behavior)
- Deprecations (features being phased out)
- Removed features or APIs
- Bug fixes affecting user experience
- Security fixes
- Major performance improvements
- Major dependency updates (version bumps, security patches)
- Major documentation overhauls (like documentation restructures)

**SHOULD update** for:
- Minor bug fixes
- Minor performance improvements
- Quality-of-life improvements
- Notable dependency updates

**SKIP changelog** for:
- Internal refactoring (no user impact)
- Test additions/changes
- CI/CD configuration
- Development tooling
- Documentation typos or minor clarifications
- Code formatting or linting

### Version Numbering (Semantic Versioning)

- **MAJOR (x.0.0)**: Breaking changes, incompatible API changes
- **MINOR (0.x.0)**: New features, backwards-compatible additions
- **PATCH (0.0.x)**: Bug fixes, backwards-compatible fixes

### Changelog Entry Format

```markdown
## [Unreleased]

### Added
- New feature description (#PR-number)

### Changed
- Changed behavior description (#PR-number)

### Deprecated
- Deprecated feature (#PR-number)

### Removed
- Removed feature (#PR-number)

### Fixed
- Bug fix description (#PR-number)

### Security
- Security fix description (#PR-number)

### Documentation
- Major documentation changes only (#PR-number)

### Dependencies
- Dependency update with rationale (#PR-number)
```

### Changelog Review Process

1. **PR Author**: Add changelog entry in PR if change is user-visible per rubric
2. **Reviewer**: Verify changelog entry exists and is appropriate
3. **Release Manager**: Move `[Unreleased]` entries to versioned release section

### Documentation Updates

All substantive documentation updates must:

1. Update the relevant section in this file if they introduce new exceptions or schema adjustments.
2. Append an entry to `docs/revision_log.csv` via the automated watcher.

## Watcher Specification

The docs CI workflow invokes the revision logger to append entries to `docs/revision_log.csv` for any Markdown file updates. Contributors MUST NOT edit the CSV manually; rely on the automation to ensure consistent auditing.
