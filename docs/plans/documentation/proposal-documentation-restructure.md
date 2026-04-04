---
doc_type: plan
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# Proposal: Documentation Restructure and Standards (v3)

This document outlines a plan to restructure the project's documentation to improve clarity, maintainability, governance, and enforcement of standards. This version adds GitHub governance workflows (auto-merge, merge modes, branch protection), playbooks organization, explicit documentation exceptions, and an index strategy for agents.

## 1. Guiding Principles

- **Centralized but Flexible:** Most documentation will reside within the `/docs` directory, with clearly defined exceptions.
- **Domain-Oriented Structure:** Documentation is organized by functional domains that mirror the crate structure.
- **Playbook-Driven Processes:** Standard procedures are defined in dedicated playbooks.
- **Automated Enforcement:** Standards will be enforced via CI/CD automation.
- **Mandatory Revision Tracking:** All changes to documentation must be tracked.

## 2. Proposed Future-State File Tree

```
docs/
├── architecture.md         # High-level overview, linking to domain details
├── standards.md            # Comprehensive documentation standards guide
├── agents.md               # Guidelines for AI agents, including the documentation index
├── dependencies.md         # Project dependency documentation (e.g., Cargo, system)
├── repo/                   # Repo-level meta documentation
│   ├── gitignore.md        # Rationale and structure of .gitignore
│   └── editor.md           # Notes about .vscode settings and editor conventions
│
├── domains/                # Documentation for each functional crate/domain
│   ├── audio/
│   ├── foundation/
│   ├── gui/
│   ├── stt/
│   ├── telemetry/
│   ├── text-injection/
│   └── vad/
│
├── playbooks/
│   ├── organizational/     # Playbooks applicable across the organization
│   │   ├── documentation_playbook.md
│   │   ├── logging_playbook.md
│   │   ├── testing_playbook.md
│   │   ├── ci_cd_playbook.md
│   │   └── github_governance.md   # Branch protection, merge policies, auto-merge, permissions
│   │
│   └── project-specific/     # Playbooks scoped to ColdVox
│       └── coldvox_documentation_playbook.md
│
└── research/               # Reference materials, research, and implementation guides
    └── ...
```

> **Update**: `docs/architecture.md` now contains the canonical **ColdVox Future Vision** section (always-on intelligent listening, tiered STT, intelligent memory management). Keep speculative architecture material centralized there and reference it from other docs (README, CLAUDE, roadmap).

## 3. Documentation Migration Plan

| Current Path                                                | Proposed Action | New Path / Target                                      | Notes                                                              |
| ----------------------------------------------------------- | --------------- | ------------------------------------------------------ | ------------------------------------------------------------------ |
| **Root**                                                    |                 |                                                        |                                                                    |
| `README.md`                                                 | Edit            | `README.md`                                            | Simplify and link to `/docs`.                                      |
| `CHANGELOG.md`                                              | Keep            | `CHANGELOG.md`                                         | To be managed by new changelog policy in the PR playbook.          |
| `CLAUDE.md`                                                 | Retain (Pointer)| `CLAUDE.md`                                            | Keep a thin file that links to `docs/agents.md` and includes the index. |
| **.github**                                                 |                 |                                                        |                                                                    |
| `.github/copilot-instructions.md`                           | Merge & Move    | `docs/agents.md`                                       | Consolidate all agent instructions.                                |
| `.github/SETUP_RELEASE_TOKEN.md`                            | Move            | `docs/playbooks/organizational/ci_cd_playbook.md`      | Becomes part of the CI/CD playbook.                                |
| **Crates**                                                  |                 |                                                        |                                                                    |
| `crates/app/docs/updated_architecture_diagram.md`           | Merge           | `docs/architecture.md`                                 |                                                                    |
| `crates/coldvox-audio/README.md`                            | Keep (crate stub)| `crates/coldvox-audio/README.md`                      | Keep minimal README in crate that links to `docs/domains/audio/`.
| `crates/coldvox-audio/docs/*.md`                            | Move            | `docs/domains/audio/`                                  | Move crate-specific docs into domain folder.                       |
| `crates/coldvox-foundation/README.md`                       | Keep (crate stub)| `crates/coldvox-foundation/README.md`                 | Keep minimal README in crate that links to `docs/domains/foundation/`.
| `crates/coldvox-foundation/docs/*.md`                       | Move            | `docs/domains/foundation/`                             | Move crate-specific docs into domain folder.                       |
| `crates/coldvox-gui/README.md`                              | Keep (crate stub)| `crates/coldvox-gui/README.md`                        | Keep minimal README in crate that links to `docs/domains/gui/`.
| `crates/coldvox-gui/docs/*.md`                              | Move            | `docs/domains/gui/`                                    | Move GUI docs into the GUI domain folder.                          |
| `crates/coldvox-stt/README.md`                              | Keep (crate stub)| `crates/coldvox-stt/README.md`                        | Keep minimal README in crate that links to `docs/domains/stt/`.
| `crates/coldvox-stt-vosk/README.md`                         | Keep (crate stub)| `crates/coldvox-stt-vosk/README.md`                   | Keep crate README; merge implementation notes into `docs/domains/stt/vosk.md`.
| `crates/coldvox-stt-vosk/docs/*.md`                         | Move            | `docs/domains/stt/`                                    | Move STT implementation docs into STT domain folder.               |
| `crates/coldvox-telemetry/README.md`                        | Keep (crate stub)| `crates/coldvox-telemetry/README.md`                  | Keep minimal README in crate that links to `docs/domains/telemetry/`.
| `crates/coldvox-telemetry/docs/*.md`                        | Move            | `docs/domains/telemetry/`                              | Move crate-specific docs into telemetry domain.                    |
| `crates/coldvox-text-injection/README.md`                   | Keep (crate stub)| `crates/coldvox-text-injection/README.md`             | Keep minimal README in crate that links to `docs/domains/text-injection/`.
| `crates/coldvox-text-injection/TESTING.md`                  | Merge & Move    | `docs/playbooks/organizational/testing_playbook.md`    | Consolidate into the main testing playbook.                        |
| `crates/coldvox-text-injection/docs/*.md`                   | Move            | `docs/domains/text-injection/`                         | Move other text-injection docs into domain folder.                 |
| `crates/voice-activity-detector/MODIFICATIONS.md`           | Move            | `docs/domains/vad/vendor_modifications.md`             |                                                                    |
| **Docs (Old Structure)**                                    |                 |                                                        |                                                                    |
| `docs/TextInjectionArchitecture.md`                         | Merge           | `docs/architecture.md`                                 |                                                                    |
| `docs/adr/0001-vosk-model-distribution.md`                  | Move            | `docs/architecture/adr-0001.md`                        | Create a new `architecture` sub-folder for ADRs.                   |
| `docs/dev/logging.md`                                       | Merge & Move    | `docs/playbooks/organizational/logging_playbook.md`    |                                                                    |
| `.github/*` workflow docs (if any)                           | Document Only   | `docs/playbooks/organizational/github_governance.md`   | Centralize repo settings and governance policies.                  |
| `docs/plans/*.md`, `docs/research/*.md`, `docs/review/*.md` | Move            | `docs/research/`                                       | Consolidate historical plans and research.                         |

## 4. Items for Discussion

- `docs/reports/injection-stackpre-10-8.md`: Is this historical report still relevant?
- `docs/dev/pr152-testing-summary.md`: Can this be archived or merged into a general testing lessons-learned document?
- `docs/dev/COMMIT_HISTORY_REWRITE_PLAN.md`: Was this plan executed? If so, this can be deleted.
- `.kiro/specs/voice-pipeline-core/requirements.md`: Is this an active specification? If so, move to `docs/domains/audio/requirements.md`.

## 5. Recommendations and New Standards

### a. Documentation Standards (`docs/standards.md`)

This new file will define:
- **Revision Tracking:** A mandatory header for all `.md` files.
- **Changelog Policy:** A rubric for versioning (Patch, Minor, Major).
- **File Watcher Requirement:** Specification for a script to log changes to `/docs`.
- **Documentation Location Exceptions:** A clear policy stating that while most `.md` files must be in `/docs`, exceptions are allowed for root-level configuration and tooling files. Initial exceptions include:
  - `README.md` (root)
  - `CHANGELOG.md` (root)
  - `.vscode/settings.json` (for editor configuration)
  - `.gitignore` (for repository ignores)
- `.github/` workflows, issue/PR templates (policy documents live in `/docs` but YAML config remains in `.github/`)
- **`.gitignore` Documentation:** Covered in `/docs/repo/gitignore.md` and referenced from standards.

### b. Documentation Index (in `docs/agents.md` and mirrored in `CLAUDE.md`)

`docs/agents.md` will contain a well-formatted index to help agents (and humans) locate all documentation.

```markdown
### Documentation Index

#### Core Concepts
- **Architecture:** [`/docs/architecture.md`](./architecture.md)
- **Standards:** [`/docs/standards.md`](./standards.md)
- **Dependencies:** [`/docs/dependencies.md`](./dependencies.md)

#### Domains
- **Audio:** [`/docs/domains/audio/`](./domains/audio/)
- **VAD:** [`/docs/domains/vad/`](./domains/vad/)
- **STT:** [`/docs/domains/stt/`](./domains/stt/)
- **Text Injection:** [`/docs/domains/text-injection/`](./domains/text-injection/)
- **GUI:** [`/docs/domains/gui/`](./domains/gui/)
- **Telemetry:** [`/docs/domains/telemetry/`](./domains/telemetry/)
- **Foundation:** [`/docs/domains/foundation/`](./domains/foundation/)

#### Playbooks
- **Organizational:** [`/docs/playbooks/organizational/`](./playbooks/organizational/)
- **Project-Specific:** [`/docs/playbooks/project-specific/`](./playbooks/project-specific/)

#### Research & History
- **Research Archive:** [`/docs/research/`](./research/)
```

### c. CI/CD Enforcement

The plan for a CI agent using "Gemini in the CLI" to enforce these standards will be detailed in the new `docs/playbooks/organizational/ci_cd_playbook.md`.

### d. New Playbooks

- `docs/playbooks/organizational/documentation_playbook.md`: Will define the process for creating and maintaining documentation.
- `docs/playbooks/project-specific/coldvox_documentation_playbook.md`: Will contain specifics for this project, linking to the organizational playbook.
- The `logging`, `testing`, and `ci_cd` playbooks will be created in the `organizational` folder.

## 6. GitHub Governance and Workflow Policies

This section captures the proposed repo governance and the feasibility of enforcing it. The authoritative, maintained source of truth will be `docs/playbooks/organizational/github_governance.md`, with developer-facing summaries in `docs/playbooks/organizational/ci_cd_playbook.md` and `docs/playbooks/project-specific/coldvox_documentation_playbook.md`.

### 6.1 Policies to Establish

- Branch protection on `main`:
  - Require pull request before merging (no direct pushes).
  - Require status checks to pass (CI, lint, docs enforcement).
  - Require linear history (disallow merge commits).
  - Restrict who can push to `main` (empty allow-list; use PRs only).
  - Require signed commits (optional but recommended).
- Merge methods:
  - Allow only “Rebase and merge” (preferred), optionally allow “Squash and merge”.
  - Disable “Create a merge commit”.
- Auto-merge:
  - Enable repository-level Auto-merge feature.
  - Optionally auto-enable auto-merge via a workflow for eligible PRs once checks pass.
- PR requirements:
  - PR template must reference: standards, changelog rubric, doc revision header, and playbooks.
  - Enforce changelog updates per rubric when applicable.
  - Enforce documentation standards on changed `.md` files (header present, placement, links).
- Maintainer “backdoor” (emergency-only):
  - Admin override is permitted but must be logged in an “Emergency Merge Record” with rationale.
  - Alternative: use a dedicated `maintainer-emergency` branch requiring follow-up PR into `main`.
  - Personal tokens with admin rights should be treated as sensitive; prefer GitHub App tokens for automation.

### 6.2 Feasibility Answers (GitHub capabilities)

- Can we enable Auto-merge by default for all PRs?
  - GitHub does not auto-enable Auto-merge per PR by default via settings. However, you can enable the repo-level feature and use a GitHub Action (GraphQL API) to auto-enable Auto-merge once required checks pass. Feasible via automation.
- Can we mandate “rebase” as the merge mode for all PRs?
  - You can restrict allowed merge methods at the repository level (disable merge commits and squash, allow only rebase). With Auto-merge enabled, the only available method will be rebase. Feasible via repo settings.
- Can we forbid direct commits to `main` except via PRs?
  - Yes. Branch protection can require PRs and restrict who can push to `main`. Admins can still bypass, but this should be reserved for emergencies and documented in the governance playbook. Feasible via branch protection.
- Can we maintain a personal/admin “backdoor” while minimizing risk?
  - Yes, but prefer using admin override with mandatory logging and follow-up PR, or a dedicated emergency branch. Avoid widespread distribution of personal tokens; prefer a GitHub App with scoped permissions.

### 6.3 Where These Live (Document Placement)

- Auto-merge, merge modes, branch protection, emergency procedures:
  - Primary: `docs/playbooks/organizational/github_governance.md`
  - Summaries: `docs/playbooks/organizational/ci_cd_playbook.md`, PR template
- CI/CD enforcement logic and workflows (including docs standards checks and auto-enable auto-merge):
  - `docs/playbooks/organizational/ci_cd_playbook.md`
- Project-specific nuances (e.g., which checks are required for ColdVox):
  - `docs/playbooks/project-specific/coldvox_documentation_playbook.md`
- Agent awareness and quick-links:
  - `docs/agents.md` (index) and `CLAUDE.md` (pointer + index)

## 7. Automated File Watcher and Logging (Docs Changes)

- A lightweight file watcher will monitor changes to `**/*.md` and log events (create/update/move/delete) to `docs/revision_log.csv` with fields: timestamp, actor (if available), path, action, summary.
- Specification lives in `docs/playbooks/organizational/documentation_playbook.md` and `docs/standards.md`.
- CI will validate that modified docs include the revision header and that watcher logs are updated for non-trivial changes.

## 8. Changelog Policy and Location

- Location: keep `CHANGELOG.md` at repository root for ecosystem discoverability.
- Policy: defined in `docs/standards.md` and enforced/process-documented in `docs/playbooks/organizational/ci_cd_playbook.md` and PR templates.
- PR Playbook will include a rubric and examples for when changelog entries are required and how to version (avoid bumping on every PR; focus on user-visible changes).

## 9. Index Locations for Agents and Claude

- `docs/agents.md`: canonical, structured documentation index with headings and links.
- `CLAUDE.md`: retained as a thin file that mirrors the index and links to `docs/agents.md` for details.

## 10. CI Playbook: Repository Structure Visualizations (Requirements + Options)

The CI playbook must include two versions of the repository documentation structure visualization:

- A readable “Markdown-style flow” diagram (text-first, minimal syntax) for quick scanning.
- A detailed, richly styled Mermaid diagram that is visually encoded by domain and documentation type, with a legend and annotations for enforcement rules.

The following are proposed strategies to make the rich diagram visually clear and information-dense without sacrificing accessibility. These are conceptual options, not code examples.

### 10.1 Visual encoding strategies (choose one or combine)

1) Domain-colored nodes; shape by document type
- Color: Assign each domain (audio, vad, stt, text-injection, gui, telemetry, foundation) a distinct hue.
- Shape: Use distinct node shapes for types (architecture, standard, playbook, reference, research, ADR, index).

## Long-term vision

As part of our long-term goals, the system's interaction model should make clear trade-offs between privacy and convenience. The aspirational modes we expect to document and discuss in architecture/UX materials are:

- Default: Push-to-Talk (manual hotkey activation) — recommended for privacy and predictability.
- Aspirational: Always listening — system monitors audio continuously but requires an explicit trigger before performing any injection. This mode must be opt-in and surfaced with clear UI affordances and consent.
- Optional UI: Single-button "Speak-to-Talk" toggle — a lightweight, user-visible toggle that provides a momentary activation without a hotkey (behaves like push-to-talk but presented as a persistent toggle).

These modes should be described in domain architecture documents (audio, text-injection) and UX playbooks, and any enabling of always-listening must be gated by explicit user consent and clear indicators in the UI.
- Accents: Border style to indicate enforcement level (e.g., solid=mandatory, dashed=optional).
- Pros: Easy mapping from color to domain; redundant encoding via shape increases accessibility.

2) Swimlanes by document type; color by domain
- Layout: Place each doc type in a horizontal lane (architecture, standards, playbooks, domains, research, repo-meta).
- Color: Within lanes, color by domain to show cross-cutting ownership.
- Links: Draw edges to indicate required references (e.g., architecture -> domain docs -> playbooks).
- Pros: Highlights responsibilities and relationships between types.

3) Status/badges overlays; monochrome domain groups
- Color: Use a neutral palette per domain; emphasize status via icon/badges (e.g., Draft, In Review, Approved, Archived).
- Overlays: Add small corner badges for “Enforced by CI”, “Required in PRs”, “Optional”.
- Pros: Focuses attention on maturity and enforcement rather than color differentiation.

4) Lifecycle gradient + enforcement border
- Color: Apply gradients or tints to indicate lifecycle (proposal -> active -> deprecated), consistent across domains.
- Border: Encode enforcement (mandatory vs optional) via border thickness/style.
- Pros: Communicates time/status evolution visually.

### 10.2 Dense-information conventions

- Subgraphs for domains: group related files under collapsible domain clusters; label clusters with a one-line purpose.
- Legend and key: include a compact legend describing color, shape, border, and badge semantics.
- Tag chips: append short tags to nodes (e.g., [REQ], [OPT], [CI], [PR]) to indicate requirements quickly.
- Tooltips/links: configure nodes with tooltips and links to the actual files for quick navigation.
- Cross-links: show edges for “must reference” relationships (e.g., standards -> playbooks, playbooks -> templates).
- Accessibility: pair color with shape and labels; use colorblind-friendly palettes.

### 10.3 Markdown-style version (text-first)

- Hierarchical list or ASCII tree with inline tags (e.g., [domain], [type], [REQ/OPT], [CI]).
- Group by document type or by domain (whichever matches the Mermaid strategy) with consistent label ordering.
- Keep line lengths short; rely on headings and indentation for scannability.

### 10.4 Mandated elements in both diagrams

- Title, version, and “last updated” date.
- Legend/key mapping colors, shapes, borders, and badges to meanings.
- Explicit sections for: Architecture, Standards, Playbooks (organizational, project-specific), Domains, Repo meta, Research, Agents/Index, Changelog.
- Clear markers for files that are exceptions to the “/docs” placement rule.

### 10.5 CI enforcement hooks

- Validate that both diagram sections exist and include a legend and last-updated date.
- Lint Mermaid syntax statically (no rendering in CI required) and ensure required labels/tags are present.
- Fail PRs that remove mandated sections or mislabel enforcement badges.
