# EPIC-1: Core Injection System

## Description

This epic covers the development of the foundational components for the high-performance text injection system. The goal is to build a robust, reliable, and extensible core that can be built upon with platform-specific implementations. This includes the main `TextInjector` struct, a pre-warming strategy to minimize latency, and strict clipboard hygiene to ensure data integrity.

This work is central to the project and is derived from the technical specifications outlined in `docs/plans/MasterPlan.md` and `docs/plans/InjectionMaster.md`.

## Acceptance Criteria

- A `TextInjector` struct is implemented and serves as the central entry point for all injection operations.
- A pre-warming mechanism is in place to proactively establish connections (e.g., to AT-SPI, portals) and back up the clipboard before an injection is requested.
- A robust clipboard hygiene system is implemented to ensure that the user's clipboard is always restored to its original state after a paste operation.
- The core system is designed with clear, fast-fail stages (≤ 50ms per stage) to meet the overall end-to-end injection target of ≤ 200ms.
- All core components are covered by unit and integration tests.
- Structured logging is integrated into the core components to provide detailed diagnostics for each injection attempt.

## Sub-Tasks

- [ ] **FEAT-101:** Implement the main `TextInjector` struct and its primary `inject` method.
  - *Labels:* `feature`, `core-system`
- [ ] **FEAT-102:** Develop the pre-warming strategy to prepare for injection.
  - *Labels:* `feature`, `performance`, `core-system`
- [ ] **FEAT-103:** Implement robust clipboard hygiene with backup and restore functionality.
  - *Labels:* `feature`, `clipboard`, `core-system`
- [ ] **FEAT-104:** Add optional Klipper clipboard history cleaning for KDE environments.
  - *Labels:* `feature`, `platform:kde`, `clipboard`
- [ ] **REFACTOR-105:** Design and implement the fast-fail staging logic within the injection pipeline.
  - *Labels:* `refactor`, `performance`, `core-system`
- [ ] **TEST-106:** Write unit tests for the `TextInjector` struct and its core logic.
  - *Labels:* `testing`, `core-system`
- [ ] **TEST-107:** Write integration tests for the clipboard hygiene helpers.
  - *Labels:* `testing`, `clipboard`
- [ ] **DOCS-108:** Add documentation for the core injection system architecture and its components.
  - *Labels:* `documentation`, `core-system`