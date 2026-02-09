---
doc_type: standard
subsystem: general
status: active
freshness: current
preservation: reference
summary: Canonical frontmatter schema and documentation maintenance tooling
last_reviewed: 2026-02-09
owners: Documentation Working Group
version: 2.1.0
---

# Documentation Standards

This standard documents repository-specific enforcement notes derived from the Master Documentation Playbook v1.0.0.

## Metadata Schema

All Markdown files under `/docs` MUST include frontmatter with **required core fields**. Optional fields provide additional metadata for tracking and classification.

### Required Core Fields (CI Fails if Missing)

```yaml
doc_type: [architecture|standard|playbook|reference|research|plan|troubleshooting|index|history]
subsystem: [domain-name|general]
status: [draft|active|archived|superseded]
```

### Required Informational Fields (Required after 2026-03-11)

```yaml
freshness: [current|aging|stale|historical|dead]
preservation: [reference|preserve|summarize|delete]
```

### Optional Fields (Nice to Have)

```yaml
version: 1.0.0
owners: Your Name or Team
last_reviewed: YYYY-MM-DD
last_reviewer: Your Name
review_due: YYYY-MM-DD
retention: [keep|archive-90d|delete-merge|historical]
canonical: true
canonical_path: ./alternative.md  # Only if canonical: false
signals: [concept-tag-1, concept-tag-2]
summary: "One-line description of the document's content"
```

### Frontmatter Template

Copy-paste this template into new documentation files:

```yaml
---
doc_type: reference
subsystem: general
status: draft
freshness: current
preservation: preserve
summary: "Brief description of what this document covers"
---
```

### Field Descriptions

#### Core Fields (Required)

- **doc_type**: Type of document
  - `architecture` - System design, component interactions, ADRs
  - `standard` - Coding standards, conventions, policies
  - `playbook` - Step-by-step guides, procedures, operational runbooks
  - `reference` - API docs, crate documentation, technical references
  - `research` - Investigations, experiments, proof-of-concepts
  - `plan` - Proposals, roadmaps, future work
  - `troubleshooting` - Debugging guides, known issues, solutions
  - `index` - Directory or category landing pages
  - `history` - Historical logs, past investigations (auto-archived)

- **subsystem**: Which part of ColdVox this relates to
  - `general` - Cross-cutting or repository-wide concerns
  - `audio` - Audio capture, processing, VAD
  - `foundation` - Core utilities, telemetry, shared infrastructure
  - `gui` - User interface, TUI dashboard
  - `stt` - Speech-to-text engine and plugins
  - `text-injection` - Keyboard/clipboard automation
  - `vad` - Voice activity detection
  - `telemetry` - Metrics and observability

- **status**: Current state of the document
  - `draft` - Work in progress, under active development
  - `active` - Current, approved, and in use
  - `archived` - Historical, kept for reference
  - `superseded` - Replaced by newer documentation

#### Informational Fields (Required after 2026-03-11)

- **freshness**: Accuracy state of the content
  - `current` - Accurate as of today, actively maintained
  - `aging` - Mostly correct, minor updates may be needed
  - `stale` - Contains outdated specifics, read critically (valuable ideas remain)
  - `historical` - Intentionally old (checkpoints, logs, history)
  - `dead` - References removed code or deprecated features, should be archived/deleted

- **preservation**: Value classification for document lifecycle
  - `reference` - Actively maintained canonical documentation
  - `preserve` - Contains valuable ideas worth keeping; update details when touching
  - `summarize` - Should be folded into other docs, then archived
  - `delete` - No salvage value, safe to remove

#### Optional Fields

- **version**: Document version following semantic versioning
  - Start with `1.0.0` for new docs
  - Increment MAJOR for breaking reorganizations
  - Increment MINOR for significant additions
  - Increment PATCH for minor fixes/clarifications

- **owners**: Who maintains this document
  - Individual name: "Jane Developer"
  - Team: "Documentation Working Group"
  - Multiple: "Audio Team, Jane Developer"

- **last_reviewed**: Date of last review (YYYY-MM-DD format)
  - Update when you make changes or verify accuracy

- **last_reviewer**: Person/team that validated the document

- **review_due**: Next suggested review date (YYYY-MM-DD format)

- **retention**: Lifecycle policy for the document
  - `keep` - Permanent, part of core documentation
  - `archive-90d` - PR reports, temporary plans (archive after 90 days)
  - `delete-merge` - Delete after content merged elsewhere
  - `historical` - Keep in archive indefinitely

- **canonical**: Whether this is the authoritative version (default: true)
  - Set to `false` if another doc is the source of truth
  - Use with `canonical_path` to point to the authoritative doc

- **signals**: List of concept tags for searchability
  - Examples: `[wayland-vkbd, race-condition, lock-ordering, portal-eis]`

- **summary**: One-line description for index display
  - Keep under 80 characters for readability

## Tooling

### Documentation Index Generator
`scripts/build_docs_index.py`
- Scans all files in `/docs`
- Validates frontmatter against `CORE_KEYS` and `INFO_KEYS`
- Generates `docs/index.md` with sections for:
  - **Action Required**: Invalid docs or dead docs outside archive
  - **Preserve These Ideas**: Docs with high preservation value
  - **Safe to Archive/Delete**: Docs flagged for removal
- Automates stats by freshness and preservation level

### Frontmatter Auto-fixer
`scripts/ensure_doc_frontmatter.py`
- Can be used as a pre-commit hook (warning only)
- Run with `--fix` to automatically insert missing frontmatter blocks
- Infers `doc_type` and `subsystem` based on file path
- Sets default `freshness: stale` and `preservation: preserve` for new docs

### Bulk Classifier
`scripts/classify_docs.py`
- One-time or batch tool to apply metadata patterns to large sets of files
- Uses regex patterns to map file paths to metadata values
- Merges new metadata with existing frontmatter without losing data

### CI Validator
`scripts/validate_docs.py`
- Invoked by `.github/workflows/docs-ci.yml`
- Fails PRs if required core keys are missing
- Warns on missing informational keys (grace period until 2026-03-11)
- Fails if `freshness: dead` docs are found outside `docs/archive/`

## Domain Documentation Cross-links

Canonical strategy and behavior for the text injection system are documented under:
- `docs/domains/text-injection/ti-overview.md` – injector approaches, strategy order, rationale
- `docs/domains/text-injection/ti-unified-clipboard.md` – implementation details of the clipboard-based injector
- `docs/domains/text-injection/ti-testing.md` – live testing requirements and procedures

Teams should update these documents when changing injection ordering, adding/removing backends, or altering confirmation/prewarm behavior.

### Expiry Policy

Transient docs should include an optional `expires_on: YYYY-MM-DD` key:

- `docs/research/pr-reports/`: default 14 days post-merge
- `docs/research/logs/`: default 30 days
- Checkpoints: keep latest 1-2 versions, archive older sets

When `expires_on` is reached, either:

1. Promote useful content into stable docs and delete the transient file, or
2. Move it to `docs/archive/` and mark status as `archived`.

### Grace Period

Until **2026-03-11**, the `freshness` and `preservation` fields are optional (warnings only). After this date, CI will fail if these fields are missing from docs under `/docs/`.

### Dead Docs Policy

Documents marked `freshness: dead` MUST reside under `docs/archive/`. CI will fail if a dead doc is found outside the archive directory.

### Master Documentation Index Execution

Generate the canonical index with freshness status:

```bash
python scripts/build_docs_index.py
```

### Auto-fix Missing Frontmatter Execution

Pre-commit hooks will warn about missing frontmatter. To auto-fix files:

```bash
# Fix specific files
python scripts/ensure_doc_frontmatter.py --fix docs/path/to/file.md

# Fix all docs in a directory
python scripts/ensure_doc_frontmatter.py --fix docs/plans/*.md
```

### Domain Filename Convention Enforcement

All domain documents under `docs/domains/<domain>/` must include the domain short code in the filename (prefix), e.g. `ti-overview.md`.

Pre-commit/CI enforcement:
- Run `python scripts/validate_domain_docs_naming.py` to validate naming.
- Old filenames may remain temporarily only if their frontmatter contains a `redirect:` key pointing to the new code-prefixed file.
- Each domain's overview MUST declare `domain_code: <code>` in its frontmatter.

## Documentation Placement

All Markdown documentation MUST reside under `/docs/` to maintain discoverability and consistency.

**Pre-commit enforcement**: The pre-commit hooks will warn about Markdown files outside `/docs/`.

### Approved Exceptions

The following files are explicitly allowed outside `/docs/`:

**Root-level files:**
- `README.md` - Repository overview and quick-start
- `CHANGELOG.md` - User-facing release notes
- `AGENTS.md` - AI assistant instructions (canonical)
- `CLAUDE.md` - Claude-specific context (mirrors AGENTS.md)

**GitHub templates:**
- `.github/pull_request_template.md` - PR template for GitHub

**Crate READMEs:**
- `crates/*/README.md` - Package READMEs required for crates.io publishing

Crate READMEs are allowed but should be kept minimal (overview, installation, quick example). Place detailed documentation in `docs/reference/crates/<crate-name>/` instead.

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
