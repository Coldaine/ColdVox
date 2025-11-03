# ColdVox Testing Documentation

This directory contains comprehensive documentation on ColdVox's **Pragmatic, Large-Span Testing Philosophy**.

## Quick Start

**New to the testing approach?** Start here:

1. Read [PRAGMATIC_TEST_ANALYSIS.md](PRAGMATIC_TEST_ANALYSIS.md) for the complete analysis of our current test suite
2. Review [TESTING_EXAMPLES.md](TESTING_EXAMPLES.md) to see good vs bad test examples
3. See [PRAGMATIC_TEST_IMPROVEMENTS.md](PRAGMATIC_TEST_IMPROVEMENTS.md) for planned improvements
4. Check [TEST_REMOVAL_PLAN.md](TEST_REMOVAL_PLAN.md) for consolidation strategy

**Writing a new test?**

- Follow the decision framework in [../domains/foundation/testing-guide.md](../domains/foundation/testing-guide.md)
- Review examples in [TESTING_EXAMPLES.md](TESTING_EXAMPLES.md)
- Ask yourself the Six Mental Models (below)

---

## Core Philosophy

ColdVox follows the **Pragmatic Test Architect** philosophy:

> **One comprehensive test that exercises real behavior beats ten fragmented unit tests.**

### The Six Mental Models

Before writing ANY test, ask yourself:

1. **External Observer**: What would a user expect to see happen?
2. **Real Action**: Can this test perform a real action that proves the system works?
3. **Larger Span**: Could this be part of a bigger, more meaningful test?
4. **Failure Clarity**: If this fails, will I know behavior is broken (not just code changed)?
5. **Story**: Does this test tell a complete story about user value?
6. **No-Mock Challenge**: How can I eliminate every mock in this test?

### Test Distribution Target

| Layer | Percentage | When to Use |
|-------|-----------|-------------|
| **Service/Integration** | 70% | DEFAULT - Use for all features |
| **E2E/Trace** | 15% | Critical user journeys only |
| **Pure Logic** | 10% | Complex algorithms (>20 lines) |
| **Contract** | 5% | External service boundaries |

---

## Document Index

### Core Documentation

| Document | Purpose | Audience |
|----------|---------|----------|
| **[PRAGMATIC_TEST_ANALYSIS.md](PRAGMATIC_TEST_ANALYSIS.md)** | Complete analysis of current test suite with grades | All developers |
| **[TESTING_EXAMPLES.md](TESTING_EXAMPLES.md)** | Concrete examples of good vs bad tests | Test writers |
| **[PRAGMATIC_TEST_IMPROVEMENTS.md](PRAGMATIC_TEST_IMPROVEMENTS.md)** | Specific code changes and new tests to write | Implementers |
| **[TEST_REMOVAL_PLAN.md](TEST_REMOVAL_PLAN.md)** | Which tests to remove/consolidate and why | Refactoring |

### Main Testing Guide

| Document | Purpose |
|----------|---------|
| **[../domains/foundation/testing-guide.md](../domains/foundation/testing-guide.md)** | Primary testing guide with philosophy and commands |

---

## Key Findings from Analysis

**Overall Grade**: **B+ (83/100)**

### ✅ Strengths

- Already emphasizes real hardware testing
- No mock-only test paths
- Strong E2E test (`end_to_end_wav.rs`)
- Good integration test coverage

### ⚠️ Opportunities

- **Consolidate fragmented tests**: 38 tests → 13 tests (-66%)
- Add trace-based testing for distributed flows
- Expand E2E coverage to 15% of suite
- Remove implementation-coupled tests
- Add missing critical journey tests

---

## Implementation Phases

### Phase 1: Documentation (COMPLETE)
- ✅ Analysis of existing tests
- ✅ Consolidation recommendations
- ✅ Testing philosophy documentation
- ✅ Examples of good/bad tests

### Phase 2: Immediate Improvements (Next)
1. Consolidate settings tests (1 hour)
2. Update main TESTING.md (done)
3. Add missing E2E test for error recovery (4 hours)

### Phase 3: Major Refactor (Next Sprint)
1. Consolidate watchdog tests (3 hours)
2. Consolidate silence detector tests (4 hours)
3. Consolidate capture integration tests (2 hours)
4. Consolidate text injection tests (2 hours)

### Phase 4: New Infrastructure (Future)
1. Add OpenTelemetry tracing (8 hours)
2. Add trace-based tests (4 hours)
3. Add performance regression tests (6 hours)

---

## Quick Reference: Test Decision Tree

```
Need to test new feature?
│
├─ Complex algorithm (>20 lines)?
│  └─ YES → 1 algorithm test + integration test
│  └─ NO → Integration test only
│
├─ Critical user journey?
│  └─ YES → E2E test
│  └─ NO → Integration test
│
├─ Can extend existing test?
│  └─ YES → Extend existing
│  └─ NO → New integration test
│
└─ When in doubt → Integration test (70% target)
```

---

## Test Quality Metrics

### Target Metrics (After Implementation)

| Metric | Current | Target | Change |
|--------|---------|--------|--------|
| Test Count | ~60 | ~35 | -42% |
| Integration % | 50% | 70% | +40% |
| E2E % | 10% | 15% | +50% |
| Unit % | 40% | 15% | -62% |
| Behavior Coverage | 65% | 85% | +31% |
| Test Flakiness | <5% | <1% | -80% |

---

## Examples: Before & After

### Before: Fragmented
```rust
#[test] fn test_watchdog_creation() { ... }
#[test] fn test_watchdog_feed() { ... }
#[test] fn test_watchdog_timeout() { ... }
#[test] fn test_watchdog_reset() { ... }
#[test] fn test_watchdog_restart() { ... }
#[test] fn test_watchdog_concurrent() { ... }
// 6 tests, implementation-focused, no user value
```

### After: Comprehensive
```rust
#[tokio::test]
async fn test_audio_pipeline_auto_recovers_from_disconnect() {
    // ONE test proves: "Dictation continues when mic disconnects"
    // Tests watchdog + capture + recovery together
    // Verifies user-facing outcome
    // ~80 lines, far more valuable
}

#[test]
fn test_watchdog_timer_algorithm() {
    // ONE focused test for algorithm edge cases
    // ~30 lines
}
// 2 tests, behavior-focused, proves feature works
```

---

## Contributing

When adding new tests:

1. **Apply the Six Mental Models** before writing
2. **Default to integration tests** (70% target)
3. **Use real dependencies** whenever possible
4. **Tell complete stories** about user value
5. **Consolidate related tests** instead of fragmenting

When reviewing tests:

- [ ] Does this test tell a complete user story?
- [ ] Could this be part of a larger test?
- [ ] Are we using real dependencies or unnecessary mocks?
- [ ] Will this test break on refactor?
- [ ] Does failure indicate broken behavior or just changed code?

---

## Further Reading

### External Resources

- **Martin Fowler**: [Test Pyramid](https://martinfowler.com/articles/practical-test-pyramid.html)
- **Kent Beck**: [Test Desiderata](https://kentbeck.github.io/TestDesiderata/)
- **Google Testing Blog**: [Just Say No to More End-to-End Tests](https://testing.googleblog.com/2015/04/just-say-no-to-more-end-to-end-tests.html)

### Internal ColdVox Resources

- [Main Testing Guide](../domains/foundation/testing-guide.md)
- [Text Injection Testing](../../crates/coldvox-text-injection/TESTING.md)
- [Project Status](../PROJECT_STATUS.md)

---

## Contact & Questions

Questions about testing philosophy or implementation?

1. Review this documentation first
2. Check examples in `TESTING_EXAMPLES.md`
3. See specific improvements in `PRAGMATIC_TEST_IMPROVEMENTS.md`
4. Open an issue for discussion

---

**Last Updated**: 2025-10-23
**Version**: 1.0
**Status**: Active - Phase 1 Complete
