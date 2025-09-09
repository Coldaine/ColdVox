# Documentation Inventory and Audit

This document provides an inventory of the project's documentation, maps each document to its corresponding code area, and prioritizes the documentation for updates.

**Prioritization Criteria:**

*   **High:** Core architecture, public APIs, and configuration that other components depend on. These are critical to get right first.
*   **Medium:** Detailed design documents for specific features and testing strategies. Important for understanding implementation details.
*   **Low:** Process documents, status reports, and crate-level `README`s that are less likely to cause confusion if slightly outdated.

| Priority | Documentation File | Corresponding Code / Area | Notes |
| :--- | :--- | :--- | :--- |
| **High** | `docs/architecture_diagram.md` | Entire Project | **Action:** Consolidate with `improved_architecture_diagram.md`. This is the most critical document for understanding the project. |
| **High** | `docs/improved_architecture_diagram.md` | Entire Project | **Action:** Consolidate with `architecture_diagram.md`. |
| **High** | `docs/configuration-architecture.md` | `crates/coldvox-foundation/src/config/` | Core to how the application is configured and run. |
| **High** | `docs/stt-plugin-architecture.md` | `crates/coldvox-stt/` | Defines a key extension point of the application. |
| **High** | `docs/text_injection_failover.md` | `crates/coldvox-text-injection/` | Describes the complex logic of the text injection system. |
| **Medium** | `docs/3_audio_processing/*` | `crates/coldvox-audio/` | Detailed implementation notes for the audio pipeline. |
| **Medium** | `docs/text_injection_testing.md` | `crates/coldvox-text-injection/` | Important for understanding how to test a complex part of the system. |
| **Medium** | `docs/ci_pipeline.md` | `.github/workflows/ci.yml` | Describes the CI process, important for contributors. |
| **Medium** | `docs/cxx-qt-bridge-resolution.md` | `crates/coldvox-gui/` | Important for understanding the GUI implementation. |
| **Medium** | `docs/gui-improvements-roadmap.md` | `crates/coldvox-gui/` | Outlines the future direction of the GUI. |
| **Medium** | `docs/parakeet-stt-integration-plan.md` | `crates/coldvox-stt/` | A plan for a new feature, should be reviewed for current relevance. |
| **Low** | `README.md` | Entire Project | High-level overview, should be updated after core docs. |
| **Low** | `crates/*/README.md` | Individual Crates | Crate-specific overviews. |
| **Low** | `docs/PROJECT_STATUS.md` | Entire Project | A snapshot in time, less critical to keep perfectly up-to-date. |
| **Low** | `docs/pr-reviews.md` | Project Processes | A log of past PRs, not critical for understanding the current codebase. |
| **Low** | `github_issues/*.md` | Project Processes | Historical issue descriptions. |
