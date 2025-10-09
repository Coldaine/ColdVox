# EPIC-3: Comprehensive Testing Framework

## Description

This epic covers the development of the multi-layered, behavior-driven testing framework for the text injection system. The goal is to ensure the system is reliable, resilient, and performant through a combination of fast pre-commit checks, service integration tests, hardware-in-the-loop validation, and full end-to-end (E2E) tests.

The architecture and implementation details for this framework are specified in `docs/plans/InjectionTest1008.md` and `docs/plans/OpusTestInject2.md`.

## Acceptance Criteria

- A comprehensive test suite is in place, covering all layers of the testing pyramid (unit, integration, E2E).
- Fast, deterministic tests run in a pre-commit hook to provide immediate feedback to developers.
- Service integration tests validate complete injection flows with real dependencies (AT-SPI, clipboard).
- A failure resilience matrix is implemented, with tests for each conceivable failure scenario.
- A full end-to-end (WAV-to-injection) testing pipeline is established to validate the entire user journey.
- The testing framework includes behavioral fakes to simulate real-world behavior without relying on mocks.

## Sub-Tasks

- [ ] **FEAT-301:** Set up the basic test environment, including fixtures for managing test applications and services.
  - *Labels:* `feature`, `testing`
- [ ] **FEAT-302:** Develop behavioral fakes for key services (e.g., `ATSPIBusFake`) to enable fast, reliable tests.
  - *Labels:* `feature`, `testing`
- [ ] **TEST-303:** Implement the fast pre-commit test suite (`tests/injection/fast/`) as defined in `OpusTestInject2.md`.
  - *Labels:* `testing`, `ci-cd`
- [ ] **TEST-304:** Implement service integration tests that verify complete injection flows with fallbacks.
  - *Labels:* `testing`, `integration-test`
- [ ] **TEST-305:** Implement the failure resilience matrix, with a dedicated test for each failure scenario.
  - *Labels:* `testing`, `resilience`
- [ ] **TEST-306:** Develop the full end-to-end (WAV-to-injection) test pipeline.
  - *Labels:* `testing`, `e2e`, `hardware`
- [ ] **TEST-307:** Implement hardware-in-the-loop tests for continuous validation (non-blocking in CI).
  - *Labels:* `testing`, `hardware`
- [ ] **TEST-308:** Implement tests for race conditions and concurrent injection requests.
  - *Labels:* `testing`, `concurrency`
- [ ] **DOCS-309:** Document the testing architecture, including how to run different test suites (fast, hardware, E2E).
  - *Labels:* `documentation`, `testing`