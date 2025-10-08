# Issue Split Summary

## Overview

The monolithic issue about text-injection refactoring has been split into 20 separate, actionable issues organized by priority.

## Statistics

- **Total Issues**: 20
- **P0 (Correctness & Reliability)**: 7 issues
- **P1 (Performance & Maintainability)**: 8 issues
- **P2 (Structure, Testing, Documentation)**: 5 issues

## Status by Priority

### P0 - Critical Correctness Issues (7/20)

These issues affect correctness and reliability of the text injection system:

| ID | Title | Core Problem |
|----|-------|--------------|
| P0-01 | Cooldowns not per-app | `is_in_cooldown()` doesn't isolate by app |
| P0-02 | "unknown_app" hardcoded | Legacy methods use placeholder instead of real app_id |
| P0-03 | Metrics mutex poisoning | Silent failures on poisoned mutex |
| P0-04 | No timeouts on awaits | Can hang indefinitely on system service failures |
| P0-05 | Blocking runtime | Uses `std::process::Command` in async context |
| P0-06 | Silent failures in app detection | Returns "unknown" instead of proper errors |
| P0-07 | No cache invalidation | Stale cached method ordering |

**Estimated Impact**: High - These issues cause incorrect behavior and potential system hangs

### P1 - Performance & Maintainability Issues (8/20)

These issues affect performance, code quality, and maintainability:

| ID | Title | Core Problem |
|----|-------|--------------|
| P1-08 | Duplicate functions | Two functions compute method ordering |
| P1-09 | 32-bit hash, not zero-copy | Unnecessary allocations and hash collisions |
| P1-10 | Inefficient comparator | O(n²) sorting with repeated position searches |
| P1-11 | Unbatched metrics | Multiple lock acquisitions per operation |
| P1-12 | No cache cleanup | Unbounded memory growth |
| P1-13 | Magic numbers remain | Hardcoded values throughout |
| P1-14 | Dead paste/keystroke code | Unused functions with `#[allow(dead_code)]` |
| P1-15 | No app_id caching | Spawns processes on every call |

**Estimated Impact**: Medium - Performance degradation and code maintainability

### P2 - Structure, Testing & Documentation Issues (5/20)

These issues affect code structure, test coverage, and documentation:

| ID | Title | Core Problem |
|----|-------|--------------|
| P2-16 | God method intact | `inject()` is monolithic with many responsibilities |
| P2-17 | Missing targeted tests | No tests for per-app isolation, cache invalidation, etc. |
| P2-18 | Dead code preserved | Multiple `#[allow(dead_code)]` annotations |
| P2-19 | CI knobs in production | `cfg!(test)` and env checks in runtime code |
| P2-20 | Undocumented | Missing error docs and concurrency information |

**Estimated Impact**: Low-Medium - Affects long-term maintainability and code quality

## Issue Dependencies

```
P0-06 (Fix app detection)
  ├─> P0-02 (Stop hardcoding "unknown_app")
  │    └─> P0-01 (Fix per-app cooldowns)
  └─> P1-15 (Add app_id caching)

P1-08 (Remove duplicate functions)
  └─> P1-10 (Fix inefficient comparator)

P0-07 (Cache invalidation)
  └─> P1-12 (Cache cleanup)

P2-17 (Add tests)
  ├─> P2-16 (Refactor god method) - needs tests first
  └─> All P0/P1 issues - tests enable safe refactoring
```

## Recommended Work Phases

### Phase 1: Critical Fixes (2-3 days)
Focus on P0 issues that block other work:
- P0-06, P0-02, P0-01 (app detection & per-app cooldowns)
- P0-03, P0-04, P0-05 (async/mutex correctness)
- P0-07 (cache invalidation)

### Phase 2: Quick Wins (1-2 days)
Low-risk, high-value improvements:
- P1-08, P1-13, P1-14 (code cleanup)
- P1-09, P1-10 (simple optimizations)

### Phase 3: Complex Performance (2-3 days)
More involved performance work:
- P1-12, P1-15 (caching strategies)
- P1-11 (metrics batching)

### Phase 4: Quality & Structure (3-4 days)
Long-term maintainability:
- P2-17 (comprehensive tests)
- P2-16 (refactor god method)
- P2-18, P2-19 (cleanup)
- P2-20 (documentation)

**Total Estimated Effort**: 8-12 days for all issues

## Impact Analysis

### Lines of Code Affected
- **P0 issues**: ~200-300 lines (critical sections)
- **P1 issues**: ~300-400 lines (optimizations and cleanup)
- **P2 issues**: ~500+ lines (refactoring and docs)

### Risk Level
- **High Risk**: P0-01, P0-02, P2-16 (core behavior changes)
- **Medium Risk**: P0-07, P1-12, P1-15 (caching changes)
- **Low Risk**: All other issues (isolated improvements)

### Testing Requirements
- **Critical**: P0-01, P0-02, P0-07 (per-app behavior)
- **Important**: P1-12, P1-15 (cache behavior)
- **Standard**: All other issues (existing tests should pass)

## Success Metrics

After completing all issues:
- [ ] All cooldowns properly isolated per-app
- [ ] No silent failures in error paths
- [ ] No indefinite hangs on system calls
- [ ] No unbounded memory growth
- [ ] Reduced lock contention in metrics
- [ ] Clean codebase without dead code annotations
- [ ] Comprehensive test coverage
- [ ] Full API documentation

## Files Modified

Primary files affected:
- `crates/coldvox-text-injection/src/manager.rs` (all issues touch this)
- `crates/coldvox-text-injection/src/focus.rs` (P0-04)
- `crates/coldvox-text-injection/src/types.rs` (P1-13, P2-20)
- `crates/coldvox-text-injection/src/tests/` (P2-17)
- Various injector files (P1-14, P2-18)

## Original Issue Reference

All 20 issues were identified in the original analysis of the text-injection codebase. The original issue described all problems in a single document, which made it difficult to:
- Track progress on individual problems
- Assign work to different developers
- Review changes in focused PRs
- Prioritize work effectively

This split enables:
- ✅ Parallel development on independent issues
- ✅ Granular progress tracking
- ✅ Focused code reviews
- ✅ Clear prioritization
- ✅ Better issue lifecycle management

## Next Steps

1. **Create GitHub Issues**: Use the templates in this directory to create 20 separate GitHub issues
2. **Label Appropriately**: Apply priority labels (P0/P1/P2) and component labels
3. **Assign Ownership**: Distribute issues based on developer expertise and capacity
4. **Start with P0**: Begin work on critical correctness issues
5. **Track Progress**: Use GitHub project boards or milestones to track completion

---

*Issue split completed: 2025-10-08*
*Total templates created: 20*
*Ready for GitHub issue creation*
