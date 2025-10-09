# ColdVox Planning Documents

This directory contains strategic planning and design documents for the ColdVox project.

## Master Plan

📋 **[MASTER_PLAN.md](MASTER_PLAN.md)** - Comprehensive synthesis of all planning documents

The master plan consolidates:
- Text injection architecture and implementation strategy
- Comprehensive testing framework (unit → integration → E2E)
- Hardware test matrix and non-blocking validation approach
- 6-phase implementation roadmap (12 weeks)
- Success criteria, performance budgets, and risk management

**Start here** for a complete overview of the vNext text injection initiative.

---

## Source Documents

The master plan synthesizes the following detailed design documents:

### Text Injection Implementation

1. **[InjectionMaster.md](InjectionMaster.md)** - Core injection design
   - Fast-fail stages (≤50ms each, ≤200ms total)
   - Pre-warm strategy (triggered on buffer entry)
   - Event-based success detection
   - Strict clipboard hygiene
   - Method rankings by environment (KDE, Hyprland, Windows)

2. **[OpusCodeInject.md](OpusCodeInject.md)** - Complete implementations
   - Wayland Virtual Keyboard (wlroots/Hyprland)
   - Portal/EIS (xdg-desktop-portal + libei)
   - KWin Fake Input (KDE Plasma)
   - Includes working code with keymap handling, D-Bus flows, error handling

### Testing Strategy

3. **[InjectionTest1008.md](InjectionTest1008.md)** - Test architecture
   - Pragmatic Test Architect philosophy
   - Behavioral fakes instead of mocks
   - Complete user journey tests
   - Test distribution: 70% integration, 15% trace, 10% contract, 5% logic
   - Logging and telemetry validation

4. **[OpusTestInject2.md](OpusTestInject2.md)** - Hardware test framework
   - Pre-commit hooks (<3s fast tests)
   - Non-blocking hardware tests
   - Flakiness detection and quarantine
   - Performance regression tracking
   - Makefile targets for different test tiers

5. **[QwenTestMerge.md](QwenTestMerge.md)** - vNext test plan
   - Unit/integration/E2E test matrix
   - Complete WAV→injection validation
   - Environment requirements (Nobara, KDE, Hyprland)
   - Success criteria (95%+ AT-SPI, 80%+ non-AT-SPI)
   - Observability and telemetry requirements

---

## Document Status

| Document | Status | Last Updated | Owner |
|----------|--------|--------------|-------|
| MASTER_PLAN.md | ✅ Active | 2025-10-08 | ColdVox Team |
| InjectionMaster.md | 📋 Reference | 2025-10-08 | Design |
| InjectionTest1008.md | 📋 Reference | 2025-10-08 | Testing |
| OpusCodeInject.md | 📋 Reference | 2025-10-08 | Implementation |
| OpusTestInject2.md | 📋 Reference | 2025-10-08 | Testing |
| QwenTestMerge.md | 📋 Reference | 2025-10-08 | Testing |

---

## Quick Navigation

### By Topic

**Architecture & Design:**
- [Master Plan - Section 2: Text Injection Architecture](MASTER_PLAN.md#2-text-injection-architecture)
- [InjectionMaster.md](InjectionMaster.md) - Detailed design

**Testing:**
- [Master Plan - Section 3: Testing Strategy](MASTER_PLAN.md#3-testing-strategy)
- [Master Plan - Section 4: Hardware Test Framework](MASTER_PLAN.md#4-hardware-test-framework)
- [InjectionTest1008.md](InjectionTest1008.md) - Test philosophy
- [OpusTestInject2.md](OpusTestInject2.md) - Hardware tests
- [QwenTestMerge.md](QwenTestMerge.md) - vNext plan

**Implementation:**
- [Master Plan - Section 5: Implementation Roadmap](MASTER_PLAN.md#5-implementation-roadmap)
- [OpusCodeInject.md](OpusCodeInject.md) - Code examples

**Success & Risks:**
- [Master Plan - Section 6: Success Criteria](MASTER_PLAN.md#6-success-criteria)
- [Master Plan - Section 7: Risk Management](MASTER_PLAN.md#7-risk-management)

### By Role

**Project Manager:**
- Start: [Master Plan - Executive Summary](MASTER_PLAN.md#executive-summary)
- Focus: [Implementation Roadmap](MASTER_PLAN.md#5-implementation-roadmap), [Risk Management](MASTER_PLAN.md#7-risk-management)

**Developer (Backend):**
- Start: [Text Injection Architecture](MASTER_PLAN.md#2-text-injection-architecture)
- Deep Dive: [OpusCodeInject.md](OpusCodeInject.md) for complete implementations

**Developer (Testing):**
- Start: [Testing Strategy](MASTER_PLAN.md#3-testing-strategy)
- Deep Dive: [InjectionTest1008.md](InjectionTest1008.md), [OpusTestInject2.md](OpusTestInject2.md)

**QA Engineer:**
- Start: [Hardware Test Framework](MASTER_PLAN.md#4-hardware-test-framework)
- Deep Dive: [QwenTestMerge.md](QwenTestMerge.md) for test matrix

**Release Manager:**
- Start: [Success Criteria](MASTER_PLAN.md#6-success-criteria)
- Focus: Phase 6 in [Implementation Roadmap](MASTER_PLAN.md#56-phase-6-polish--release-weeks-11-12)

---

## Key Metrics & Targets

### Performance Budgets
- Pre-warm: <50ms
- Per-method injection: <50ms per stage
- Total end-to-end: <200ms p95
- Audio → text → injection: <500ms p95

### Success Rates
- AT-SPI apps: ≥95%
- Non-AT-SPI apps: ≥80%
- Electron apps: ≥90%
- Overall: ≥85%

### Test Performance
- Pre-commit: <3s
- Unit tests: <10s
- Integration tests: <30s
- E2E tests: <30s per test

---

## Related Documentation

**Architecture:**
- [../architecture.md](../architecture.md) - TUI architecture and robustness
- [../../CLAUDE.md](../../CLAUDE.md) - Workspace overview

**Implementation:**
- `crates/coldvox-text-injection/` - Text injection subsystem
- `crates/coldvox-audio/` - Audio capture and processing
- `crates/coldvox-vad/` - VAD core traits
- `crates/coldvox-stt/` - STT abstractions

---

## Questions?

For questions about these plans:
1. Check the [Master Plan](MASTER_PLAN.md) first
2. Review the relevant source document
3. Open an issue with the `planning` label
4. Tag the appropriate owner

**Last Updated:** 2025-10-08
