# EPIC-4: Telemetry and Diagnostics

## Description

This epic covers the implementation of a comprehensive telemetry and diagnostics system for the text injection feature. The goal is to produce structured, actionable data for every injection attempt, enabling developers to quickly diagnose issues, monitor performance, and understand system behavior in production.

The requirements for structured logging and OpenTelemetry integration are derived from the specifications in `docs/plans/MasterPlan.md` and `docs/plans/InjectionTest1008.md`.

## Acceptance Criteria

- All injection attempts generate a structured log event (e.g., JSON) containing detailed information about the attempt.
- The system is integrated with OpenTelemetry to produce distributed traces for each injection, telling the complete story of the operation.
- Key performance indicators (e.g., injection duration, method success rates, fallback triggers) are tracked as metrics.
- The telemetry system is designed to be lightweight and have minimal impact on injection performance.
- User-facing failure messages are clear, actionable, and derived from the diagnostic data.

## Sub-Tasks

- [ ] **FEAT-401:** Implement a structured JSON logger for the injection system.
  - *Labels:* `feature`, `telemetry`, `logging`
- [ ] **FEAT-402:** Define and implement the schema for the injection log event, including all relevant fields (e.g., `env`, `utterance_id`, `method`, `stage_ms`, `result`).
  - *Labels:* `feature`, `telemetry`, `logging`
- [ ] **FEAT-403:** Integrate with OpenTelemetry to create a tracer for the injection service.
  - *Labels:* `feature`, `telemetry`, `tracing`
- [ ] **FEAT-404:** Implement the `injection_span` context manager to create detailed traces for each injection attempt.
  - *Labels:* `feature`, `telemetry`, `tracing`
- [ ] **FEAT-405:** Define and track key metrics using the OpenTelemetry Metrics API (e.g., duration, attempts, fallbacks).
  - *Labels:* `feature`, `telemetry`, `metrics`
- [ ] **REFACTOR-406:** Refactor the error handling logic to produce structured diagnostic information for failed injections.
  - *Labels:* `refactor`, `telemetry`, `error-handling`
- [ ] **DOCS-407:** Document the telemetry schema, including the meaning of each field and how to interpret the data.
  - *Labels:* `documentation`, `telemetry`
- [ ] **TEST-408:** Write tests to verify that telemetry events are correctly generated for various injection scenarios (success, failure, fallback).
  - *Labels:* `testing`, `telemetry`