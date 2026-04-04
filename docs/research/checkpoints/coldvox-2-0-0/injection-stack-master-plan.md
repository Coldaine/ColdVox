---
doc_type: research
subsystem: text-injection
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# Injection Stack Master Plan

## Purpose
Create a living roadmap for the text injection subsystem so future iterations stay aligned with the current architecture, prioritize high-value improvements, and mitigate known risks.

## Current Architecture Snapshot
- **Pipeline entry**: `AsyncInjectionProcessor` consumes `TranscriptionEvent`s, buffers text via `InjectionSession`, and hands completed buffers to `StrategyManager` for injection.
- **Strategy fallback**: Prioritized order of AT-SPI → Clipboard+Paste → ydotool/kdotool → Enigo → NoOp with clipboard restoration and telemetry on every attempt.
- **Configuration surface**: `InjectionConfig` toggles method availability, timings, safety policies, and chunking modes (auto/paste/keystroke).
- **Observability**: `InjectionMetrics` and `ProcessorMetrics` track buffer behavior, per-method success, and latency budgets.

## Strategic Objectives
1. **Strengthen fallback reliability**
   - Expand clipboard restoration coverage with mocks so tests run across CI targets.
   - Add automated validation for ydotool/kdotool availability checks and error messaging.
   - Verify Enigo integration across supported desktop environments and formalize opt-in guidance.
2. **Tighten configuration & policy controls**
   - Audit `InjectionConfig` defaults for realistic latency, buffer limits, and privacy settings.
   - Surface allow/block list behavior in user docs and provide sample policies.
   - Create regression tests ensuring CLI/environment overrides respect config precedence.
3. **Elevate telemetry & debugging**
   - Enrich metrics with per-method latency histograms and focus-state counters.
   - Automate `StrategyManager::print_stats()` snapshots in debug builds or log dumps.
   - Define alerting thresholds for sustained failure rates or excessive cooldowns.
4. **Clarify documentation & developer onboarding**
   - Produce an architecture walkthrough that links each injector to code hotspots.
   - Maintain an updated Mermaid flowchart and ensure docs avoid aspirational language.
   - Capture troubleshooting playbooks for common failure scenarios (AT-SPI bus down, clipboard daemon missing, etc.).

## Workstreams & Milestones
| Workstream | Milestone | Owner (TBD) | Target |
|------------|-----------|-------------|--------|
| Fallback Resilience | Clipboard + ydotool integration tests in CI | Platform | Sprint 2 |
| Config Governance | Config precedence and policy doc update | Docs + Platform | Sprint 2 |
| Telemetry Expansion | Metrics enrichment & alerting rules | Observability | Sprint 3 |
| Doc Refresh | Publish revised architecture guide & playbooks | Docs | Sprint 1 |

## Risk & Mitigation Log
- **AT-SPI Unavailability**: Provide fast failover guidance; add health checks to warn users before injection attempts.
- **Clipboard Side Effects**: Guarantee clipboard restoration via transactional API; document user-facing settings.
- **Security/Privacy**: Enforce sanitized logging and confirm allow/block lists cover sensitive applications.
- **Performance Regressions**: Monitor latency budget adherence; set up performance benchmarks before and after changes.

## Metrics & Success Criteria
- Focus detection error rate <5% over 7-day rolling window.
- Clipboard restoration failures <1% across CI and manual tests.
- Mean injection latency ≤500 ms with 95th percentile ≤800 ms.
- Documentation onboarding checklist completed by new contributors in ≤1 day.

## Next Steps
1. Align workstream owners and confirm sprint targets.
2. File tickets for each milestone with acceptance criteria tied to the metrics above.
3. Schedule a design review for focus detection fixes before implementation.
4. Stand up CI jobs for injector availability checks and metrics regression tests.
5. Keep this plan updated after each sprint review.</content>
<parameter name="filePath">/home/coldaine/Desktop/ColdVoxRefactorTwo/ColdVox/docs/plans/InjectionStackMasterPlan.md