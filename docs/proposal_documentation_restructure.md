# Proposal: Documentation Restructure and Standards (v2)

This document outlines a plan to restructure the project's documentation to improve clarity, maintainability, and enforcement of standards. This version incorporates feedback regarding playbooks, documentation location exceptions, and the need for a central documentation index.

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
│   │   └── ci_cd_playbook.md
│   │
│   └── project-specific/     # Playbooks scoped to ColdVox
│       └── coldvox_documentation_playbook.md
│
└── research/               # Reference materials, research, and implementation guides
    └── ...
```

## 3. Documentation Migration Plan

| Current Path                                                | Proposed Action | New Path / Target                                      | Notes                                                              |
| ----------------------------------------------------------- | --------------- | ------------------------------------------------------ | ------------------------------------------------------------------ |
| **Root**                                                    |                 |                                                        |                                                                    |
| `README.md`                                                 | Edit            | `README.md`                                            | Simplify and link to `/docs`.                                      |
| `CHANGELOG.md`                                              | Keep            | `CHANGELOG.md`                                         | To be managed by new changelog policy in the PR playbook.          |
| `CLAUDE.md`                                                 | Merge & Move    | `docs/agents.md`                                       | Consolidate all agent instructions.                                |
| **.github**                                                 |                 |                                                        |                                                                    |
| `.github/copilot-instructions.md`                           | Merge & Move    | `docs/agents.md`                                       | Consolidate all agent instructions.                                |
| `.github/SETUP_RELEASE_TOKEN.md`                            | Move            | `docs/playbooks/organizational/ci_cd_playbook.md`      | Becomes part of the CI/CD playbook.                                |
| **Crates**                                                  |                 |                                                        |                                                                    |
| `crates/app/docs/updated_architecture_diagram.md`           | Merge           | `docs/architecture.md`                                 |                                                                    |
| `crates/coldvox-audio/README.md` & `docs/*.md`              | Move            | `docs/domains/audio/`                                  | Consolidate all crate-specific docs.                               |
| `crates/coldvox-foundation/README.md`                       | Move            | `docs/domains/foundation/README.md`                    |                                                                    |
| `crates/coldvox-gui/README.md` & `docs/*.md`                | Move            | `docs/domains/gui/`                                    |                                                                    |
| `crates/coldvox-stt/README.md`                              | Move            | `docs/domains/stt/README.md`                           |                                                                    |
| `crates/coldvox-stt-vosk/README.md`                         | Merge           | `docs/domains/stt/vosk.md`                             |                                                                    |
| `crates/coldvox-telemetry/README.md`                        | Move            | `docs/domains/telemetry/README.md`                     |                                                                    |
| `crates/coldvox-text-injection/README.md`                   | Move            | `docs/domains/text-injection/README.md`                |                                                                    |
| `crates/coldvox-text-injection/TESTING.md`                  | Merge & Move    | `docs/playbooks/organizational/testing_playbook.md`    | Consolidate into the main testing playbook.                        |
| `crates/voice-activity-detector/MODIFICATIONS.md`           | Move            | `docs/domains/vad/vendor_modifications.md`             |                                                                    |
| **Docs (Old Structure)**                                    |                 |                                                        |                                                                    |
| `docs/TextInjectionArchitecture.md`                         | Merge           | `docs/architecture.md`                                 |                                                                    |
| `docs/adr/0001-vosk-model-distribution.md`                  | Move            | `docs/architecture/adr-0001.md`                        | Create a new `architecture` sub-folder for ADRs.                   |
| `docs/dev/logging.md`                                       | Merge & Move    | `docs/playbooks/organizational/logging_playbook.md`    |                                                                    |
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
- **`.gitignore` Documentation:** A section explaining the structure and reasoning of the project's `.gitignore` file.

### b. Documentation Index (in `docs/agents.md`)

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
