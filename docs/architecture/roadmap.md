---
doc_type: architecture
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# ColdVox Roadmap

> **Status**: Planning reference (subject to change)

This roadmap outlines the anticipated progression of major transformational initiatives, including the newly documented always-on intelligent listening vision. Dates are speculative and will be refined as research and implementation progress.

## Version Milestones

| Version | Target Window | Objectives | Key Artifacts |
|---------|----------------|------------|---------------|
| **v0.7 “Documentation Governance”** | Q4 2025 | Complete documentation restructure, publish standards, create organizational/project playbooks, instrument docs watcher logging. | `docs/proposal_documentation_restructure.md`, `docs/standards.md`, `docs/playbooks/organizational/*` |
| **v0.8 “Automation & Governance”** | Q1 2026 | Enforce documentation standards in CI, ship GitHub governance automation (auto-merge workflows, branch protections), finalize PR templates, launch changelog rubric. | `docs/playbooks/organizational/ci_cd_playbook.md`, `docs/playbooks/organizational/github_governance.md`, `.github/workflows/*` |
| **v0.9 “Domain Integration Readiness”** | Q2 2026 | Harden domain documentation, align crate READMEs with canonical docs, expand telemetry + text injection references, prepare architecture baselines for always-on work. | `docs/domains/*`, crate `README.md` stubs |
| **v1.0 “Always-On Foundation”** | Q4 2026 | Deliver Phase 1 of the future vision: decoupled listening thread, basic always-on capture, idle detection, tiered STT proof-of-concept. | `docs/architecture.md`, prototype always-on feature flag |
| **v1.1 “Intelligence Layer”** | Q2 2027 | Implement Phase 2 intelligence: advanced trigger detection, ML-driven pattern recognition, predictive engine loading, context-aware activation. | ML research notes, telemetry extensions |
| **v1.2 “Optimization & Integration”** | Q4 2027 | Finalize Phase 3: performance hardening, cross-platform integration, customizable UX, production-ready always-on experience. | Platform-specific integration guides, UX specs |

## Transformational Initiatives

1. **Documentation & Governance (v0.7–v0.8)**
   - Execute the restructure proposal, ensuring every domain has a dedicated docs home.
   - Stand up automated revision tracking, CI enforcement, and GitHub governance policies.
   - Maintain minimal crate-level README stubs that link to the canonical `/docs` materials.

2. **Always-On Intelligent Listening (v1.0+)**
   - Follow the architecture outlined in [`docs/architecture.md`](../architecture.md#coldvox-future-vision).
   - Deliver the decoupled threading model, tiered STT engines, and intelligent memory controller.
   - Stage delivery through experimental feature flags and opt-in user interfaces to address privacy expectations.

3. **Adaptive Telemetry & Observability (spanning v0.9–v1.2)**
   - Extend `coldvox-telemetry` to monitor resource usage, engine lifecycle events, and activation quality.
   - Integrate with the future always-on pipeline to expose actionable metrics.

4. **User Experience Alignment (v1.1–v1.2)**
   - Coordinate UX assets (TUI, GUI) with the always-on vision, adding clear state indicators and consent flows.
   - Document UI expectations in `docs/domains/gui/` and in forthcoming UX playbooks.

## Dependencies & Open Questions

- **Privacy & Compliance**: Always-on operation requires explicit consent flows, storage policies, and possibly legal review.
- **Model Footprint**: Tiered STT assumes availability of multiple engine profiles; research lightweight models and licensing.
- **Platform Variance**: Mobile/embedded support may require alternative capture pipelines and more aggressive power management.
- **CI Automation**: Auto-merge enablement via GitHub GraphQL/Actions needs prototyping to confirm feasibility and rate limits.

## Next Actions (Short-Term)

1. Finalize `docs/standards.md`, `docs/agents.md`, and playbook skeletons.
2. Land GitHub governance automation plans in `docs/playbooks/organizational/github_governance.md`.
3. Prototype docs watcher tooling and integrate logs into CI for visibility.
4. Draft the always-on listening proof-of-concept design notes and spike tickets for v1.0 foundation work.

> This roadmap will evolve as research findings, user feedback, and platform constraints emerge. Treat version numbers and windows as placeholders until validated by delivery proof points.
