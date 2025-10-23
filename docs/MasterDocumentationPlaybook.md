---
doc_type: playbook
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-22
---

# Master Documentation Playbook (Org‑wide)

This playbook is the single, canonical source of truth for documentation across all repositories. It prescribes the folder layout, file placement rules, metadata headers, lifecycle/retention policies, and CI enforcement. Project‑specific playbooks are allowed for domains/operations/etc.; however, do NOT create a project‑specific documentation playbook. All documentation rules are governed solely by this master file. Repo‑specific documentation nuances must be captured as clearly marked overrides via a short “Repository Overrides” section in that repo’s `README.md` or `docs/README.md` (not a separate documentation playbook).

This document is versioned. Changes to this playbook require a PR, review by the Documentation Working Group (DWG), and an entry in the playbook change log (Appendix A).

## 1) Scope & Goals

- Applies to: All repositories in the organization
- Goals:
  - Centralize documentation under `/docs` with minimal approved exceptions
  - Standardize structure, naming, and metadata for discoverability
  - Enforce quality via CI (headers, placement, retention)
  - Make task tracking and PR hygiene consistent and auditable

### Guiding Principles

- Centralized but flexible: `/docs` is the default home; exceptions are rare and documented.
- Domain-oriented: docs mirror functional domains and crate boundaries for predictability.
- Single source of truth: reduce duplication; use thin index pages to point to authoritative locations.
- Automation-backed: CI enforces headers, placement, and retention; humans focus on content.
- Ephemeral where appropriate: PR reports and investigation logs are short-lived unless promoted.

## 2) Canonical Docs Structure

All documentation MUST live under `/docs` unless covered by the explicit exceptions below.

Recommended top‑level layout:

```
docs/
  architecture.md                 # High-level overview & system vision
  architecture/roadmap.md         # Owned by Architecture, linked from architecture.md
  architecture/adr-0001.md        # Architecture Decision Records (ADR-XXXX naming)
  standards.md                    # Documentation standards & metadata schema
  dependencies.md                 # Project dependency documentation (Cargo/system/OS)
  repo/                           # Repo meta documentation
    gitignore.md                  # Rationale for .gitignore structure
    editor.md                     # Editor/workspace conventions (e.g., .vscode)
  agents.md                       # Assistant orientation (overview, workspace map, commands) mirrored in CLAUDE.md

  domains/                        # Technical docs per functional domain
    audio/
    vad/
    stt/
    text-injection/
    telemetry/
    gui/
    foundation/

  plans/                          # One-off plans; active working docs

  reference/
    crates/                       # Thin index pages that link to crate READMEs

  research/
    pr-reports/                   # Ephemeral PR/Change reports (see retention)
    checkpoints/                  # Versioned validation packs (limited retention)
    logs/                         # Investigation/conversation logs (ephemeral)

  playbooks/
    organizational/
      pr_playbook.md              # Organization-wide PR policy & templates
      ci_cd_playbook.md           # CI/CD conventions and enforcement hooks
      runner_setup.md             # Self-hosted runner setup & health checks
      github_governance.md        # Branch protection, merge modes, auto-merge

  tasks/                          # Reference task specs, breakdowns, links
  todo.md                         # The SINGLE source of truth for tasks
  revision_log.csv                # File watcher log (see §6)
```

### 2.1 Approved Exceptions (non-/docs)

- `README.md` (root): Entry point; may link prominently to `/docs`
- `CHANGELOG.md` (root): Release notes for the repo
- `.vscode/` project settings
- `.gitignore`

Any additional exception requires DWG approval and must be documented in `docs/standards.md` under “Exceptions”.

## 3) Required Metadata Header (All Markdown)

Every Markdown file under `/docs` MUST begin with this front matter:

```markdown
---
doc_type: [architecture|standard|playbook|reference|research|plan|troubleshooting|index]
subsystem: [domain-name|general]
version: [semver, e.g., 1.2.3]
status: [draft|review|approved|deprecated]
owners: [team or individual]
last_reviewed: [YYYY-MM-DD]
---
```

Notes:
- `doc_type=index` is used for thin index pages (e.g., crate indexes in `reference/crates/`).
- `subsystem` MUST match a domain folder when applicable (e.g., `stt`, `text-injection`).

## 4) Placement, Naming, and Linking Rules

- All documentation lives in `/docs` (see approved exceptions).
- Use kebab-case filenames: `vosk-model-discovery-flow.md`.
- Reference crate indexes: place at `docs/reference/crates/<crate>.md` and link to the crate’s README:
  - Example contents: “This is the index for `<crate>` — authoritative docs live in `../../../crates/<crate>/README.md`.”
  - Do not duplicate README contents; add only navigation/context.
- Domain troubleshooting (e.g., Vosk model discovery) must live inside the relevant `docs/domains/<domain>/troubleshooting/`.
- Roadmap lives at `docs/architecture/roadmap.md` and is linked from `docs/architecture.md`.
- ADRs live under `docs/architecture/adr-XXXX.md` with incrementing numeric IDs and MUST be linked from `docs/architecture.md`.
- `docs/agents.md` and `CLAUDE.md` MUST both contain the full assistant orientation (overview, workspace map, key commands, feature flags). Keep the contents synchronized, include links to this playbook and `docs/standards.md`, and retain these files at the documented paths.

## 5) Lifecycle & Retention Policies

Define the default lifecycle for transient documentation and where it should live.

### 5.1 PR & Change Reports
- Location: `docs/research/pr-reports/PR-<number>-<short-slug>.md`
- Retention: Delete within 14 days after merge unless explicitly promoted.
- Promotion rules:
  - Process insight → update `playbooks/organizational/pr_playbook.md` or `ci_cd_playbook.md`.
  - Technical insight → update the relevant `docs/domains/<domain>/...` or `docs/standards.md`.
  - If retention needed, move to `docs/archive/pr-reports/` (90‑day max) and add a deprecation banner.

### 5.2 One-off Plans
- Location: `docs/plans/<topic>.md`
- After completion:
  - Fold outcomes into `architecture.md` and/or domain docs, then delete; OR
  - Move to `docs/research/plans/` with a summary link in the affected docs.

### 5.3 Checkpoint Validation Packs
- Location: `docs/research/checkpoints/<version>/`
- Keep latest 1–2 versions; move older to `docs/archive/checkpoints/` or delete per policy.

### 5.4 Conversation/Investigation Logs
- Location: `docs/research/logs/<yyyy-mm-dd>-<topic>.md`
- Retention: 30 days unless promoted to standards/playbooks/architecture.

### 5.5 User Policies / Cheatsheets
- Co‑locate with the domain: e.g., `docs/domains/text-injection/policies.md`.

## 6) File Watcher & CI Enforcement

### 6.1 File Watcher
- Monitor `**/*.md` changes.
- Append CSV entries to `docs/revision_log.csv` with fields:
  - `timestamp, actor, path, action, summary`
- Minimal spec is documented here; implementation details live alongside CI (see PR playbook / CI/CD playbook).

### 6.2 CI Validation
- Reject PRs if any changed Markdown under `/docs` is missing the required header.
- Reject PRs if Markdown is added outside `/docs` (unless approved exception).
- Recommend check: if files added under `docs/tasks/`, ensure related items exist/updated in `docs/todo.md` (best‑effort heuristic; may not be hard‑fail initially).
- Lint Mermaid diagrams (syntax only) where present.
- Optionally, enforce that `docs/reference/crates/*.md` link to crate READMEs.
 - CI/CD playbook MUST include two repository documentation structure visualizations:
   - A text-first outline version, and
   - A Mermaid diagram version with legend/labels; CI should validate both sections exist (syntax lint for Mermaid is sufficient).

## 7) Tasks & Backlog (Critical)

- `docs/todo.md` is the single source of truth for tasks across the repo.
- `docs/tasks/` stores supporting materials (breakdowns, specs, links).
- Every actionable task MUST be recorded in `docs/todo.md`.
- Any supporting plan/spec MUST be linked from both `docs/tasks/` and the corresponding entry in `docs/todo.md`.
- This rule MUST be reiterated in `docs/standards.md`, `README.md` (Contributing), and `agents.md`.

## 8) Organization‑wide PR Playbook (Summary)

Authoritative details live in `docs/playbooks/organizational/pr_playbook.md`. Summary:

- PRs required for all changes to `main` (branch protection).
- Allowed merge method: “Rebase and merge” (squash optional if org policy allows). No merge commits.
- Auto‑merge may be enabled via workflow once checks pass (see governance).
- PR template must include:
  - Links to updated docs (if applicable) and confirmation of metadata headers
  - Changelog rubric decision (user‑visible? If yes, update `CHANGELOG.md`). The rubric lives in `docs/standards.md` and MUST be followed.
  - Confirmation that tasks were added/updated in `docs/todo.md` (if applicable)
- CI checks: formatting, lint, typecheck/build, docs header/placement, optional Mermaid lint, and watcher log update when significant docs are changed.

## 9) Governance & Exceptions

- This playbook is owned by the DWG. Changes require review and a version bump.
- Exceptions to structure/retention must be approved by DWG and documented in `docs/standards.md` → “Exceptions”.

## 10) Onboarding & Migration

### 10.1 New Repos
1. Create `/docs` with the canonical layout (empty files allowed initially).
2. Add `docs/todo.md`, `docs/standards.md`, and the file watcher hook.
3. Add PR template referencing this playbook and the standards.
4. Add `docs/reference/crates/` indexes for each crate, linking to crate READMEs.

### 10.2 Existing Repos (Migration)
1. Move stray docs under `/docs` respecting structure and exceptions.
2. Convert crate reference pages into thin indexes that link to crate READMEs.
3. Relocate troubleshooting to domain folders.
4. Triage legacy “reports/logs” into `research/` and apply retention policy.
5. Create or update `docs/todo.md` and link any supporting materials in `docs/tasks/`.

## 11) Examples

### 11.1 Required Header (example)

```markdown
---
doc_type: troubleshooting
subsystem: stt
version: 1.0.0
status: approved
owners: STT Team
last_reviewed: 2025-10-22
---
```

### 11.2 Crate Reference Index (example)

```markdown
---
doc_type: index
subsystem: general
version: 1.0.0
status: approved
owners: Maintainers
last_reviewed: 2025-10-22
---

# crate: coldvox-audio (Index)

Authoritative docs live in: `../../../crates/coldvox-audio/README.md`

Key entry points:
- Capture: `src/capture.rs`
- Chunking: `src/chunker.rs`
```

### 11.3 PR Report (ephemeral) example

```markdown
---
doc_type: research
subsystem: general
version: 0.1.0
status: draft
owners: Platform Team
last_reviewed: 2025-10-22
---

# PR-152: Clipboard Timeout Fixes – Testing Summary

Outcome: Merged. Insights promoted to `playbooks/organizational/ci_cd_playbook.md`.
Retention: Delete 2025-11-02 if no further action.
```

## 12) FAQ (selected)

**Q: Can we keep PR reports indefinitely?**
A: No. Default is delete within 14 days post‑merge unless promoted or explicitly archived with a time‑boxed retention.

**Q: Where does the roadmap live?**
A: `docs/architecture/roadmap.md`, linked from `docs/architecture.md`.

**Q: Do we allow project‑specific playbooks?**
A: Yes—for domains, operations, or other areas. Exception: do NOT create project‑specific documentation playbooks. Documentation is governed solely by this master file; any repo‑specific documentation nuances should be listed as a short “Repository Overrides” section in the repo’s `README.md` or `docs/README.md` and must link back to this playbook.

---

## Appendix A — Playbook Change Log

- 1.0.0 (2025‑10‑19)
  - Initial canonical version: structure, headers, placement rules, retention, PR hygiene, tasks backlog policy, file watcher + CI enforcement.
