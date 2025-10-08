# Text Injection Issue Templates

This directory contains 20 separate issue templates split from the original monolithic issue about text-injection refactoring needs.

## Organization

Issues are organized by priority:

- **P0 (Priority 0)**: Correctness & Reliability - Issues 1-7
- **P1 (Priority 1)**: Performance & Maintainability - Issues 8-15
- **P2 (Priority 2)**: Structure, Testing, and Documentation - Issues 16-20

## Creating Issues

### Manual Creation

You can manually create issues in GitHub by:

1. Go to https://github.com/Coldaine/ColdVox/issues/new
2. Copy the content from each markdown file
3. The frontmatter at the top of each file contains suggested labels and title
4. Create one issue per file

### Using GitHub CLI

If you have appropriate permissions, you can use the GitHub CLI to create issues in bulk:

```bash
cd issues-to-create

# Create a single issue
gh issue create --title "$(grep '^title:' P0-01-cooldowns-not-per-app.md | cut -d'"' -f2)" \
  --body-file P0-01-cooldowns-not-per-app.md \
  --label "bug" --label "priority:P0" --label "component:text-injection"

# Or create all issues at once (requires script)
for file in P0-*.md P1-*.md P2-*.md; do
  title=$(grep '^title:' "$file" | sed 's/title: "\(.*\)"/\1/')
  gh issue create --title "$title" --body-file "$file" --label "bug,priority:P0,component:text-injection"
done
```

Note: Adjust labels based on the frontmatter in each file.

## Issue List

### P0 - Correctness & Reliability

1. **P0-01**: Cooldowns not per-app: `is_in_cooldown()` checks any app
2. **P0-02**: "unknown_app" hardcoded in `update_cooldown()` and `clear_cooldown()`
3. **P0-03**: Metrics mutex poisoning not handled properly
4. **P0-04**: No timeouts on awaited operations (e.g., `get_focus_status`)
5. **P0-05**: Blocking runtime: `std::process::Command` in `get_active_window_class()`
6. **P0-06**: Silent failures in app detection: `get_current_app_id()` returns "unknown"
7. **P0-07**: No cache invalidation: `update_success_record()` doesn't clear `cached_method_order`

### P1 - Performance & Maintainability

8. **P1-08**: Duplicate functions: `_get_method_priority()` and `compute_method_order()`
9. **P1-09**: 32-bit hash and non-zero-copy: `redact_text()` returns `String` not `Cow`
10. **P1-10**: Inefficient comparator: `sort_by` uses `position()` which iterates
11. **P1-11**: Unbatched metrics: individual locks throughout code
12. **P1-12**: No cache cleanup: unbounded success/cooldown caches
13. **P1-13**: Magic numbers remain: hardcoded values like 2.0 for backoff factor
14. **P1-14**: Dead paste/keystroke code: `chunk_and_paste()` and `pace_type_text()`
15. **P1-15**: No app_id caching: spawns processes every call without TTL cache

### P2 - Structure, Testing, and Documentation

16. **P2-16**: God method intact: `inject()` remains monolithic
17. **P2-17**: Missing targeted tests for cooldown per-app, cache invalidation, mocked time
18. **P2-18**: Dead code preserved with `#[allow(dead_code)]` throughout
19. **P2-19**: CI knobs in production: `cfg!(test)` and `CI` checks in runtime code
20. **P2-20**: Undocumented: Missing `/// # Errors` and concurrency docs

## Dependencies Between Issues

Some issues depend on or relate to others:

- **P0-01 & P0-02**: Both relate to per-app cooldown tracking (can be tackled together)
- **P0-06**: Silent failures block proper implementation of P0-01 and P0-02
- **P0-07**: Cache invalidation becomes more important after P1-12 cleanup
- **P1-08**: Duplicate functions should be resolved before other refactoring
- **P1-15**: App ID caching depends on P0-06 (proper error handling)
- **P2-16**: God method refactor may expose issues in P2-18 (dead code)
- **P2-17**: Tests needed to safely implement P0 and P1 fixes

## Recommended Implementation Order

### Phase 1: Foundation (P0 Correctness)
1. P0-06 - Fix silent failures in app detection (enables other fixes)
2. P0-02 - Stop hardcoding "unknown_app" (depends on P0-06)
3. P0-01 - Fix per-app cooldown checks (depends on P0-02)
4. P0-03 - Handle metrics mutex poisoning
5. P0-04 - Add timeouts to async operations
6. P0-05 - Fix blocking runtime calls

### Phase 2: Performance (P1 Quick Wins)
7. P1-08 - Remove duplicate functions
8. P1-13 - Extract magic numbers to constants
9. P1-14 - Clean up dead code
10. P1-09 - Optimize redact_text with Cow
11. P1-10 - Fix inefficient comparator

### Phase 3: Caching & Cleanup (P1 Complex)
12. P0-07 - Implement cache invalidation
13. P1-12 - Add cache size limits and cleanup
14. P1-15 - Add app_id caching with TTL
15. P1-11 - Batch metrics updates

### Phase 4: Structure & Quality (P2)
16. P2-17 - Add comprehensive tests (do this early!)
17. P2-18 - Audit and remove dead code
18. P2-19 - Remove CI/test conditionals from production code
19. P2-16 - Refactor god method (with tests in place)
20. P2-20 - Add comprehensive documentation

## Notes

- Each issue is self-contained with problem description, current behavior, expected behavior, impact, and location
- Issues include code examples to illustrate problems and solutions
- Priority levels reflect urgency and impact on correctness, performance, and maintainability
- The original monolithic issue has been fully decomposed into actionable items

## Original Issue

The content in this directory was generated from splitting a large issue that identified 20 separate problems in the `crates/coldvox-text-injection` codebase. This decomposition makes the work more manageable and allows for:

- Parallel work on independent issues
- Better tracking of progress
- Focused PRs for each fix
- Clearer review scope

## Maintenance

If new related issues are discovered, they should follow the same template structure:
- Clear problem statement
- Current vs expected behavior
- Impact analysis
- Code location
- Suggested solution
- Related issues

---

*Generated from original issue analysis on 2025-10-08*
