# Commit History Rewrite Plan
## injection-orchestrator-lean Branch

**Date:** 2025-10-12  
**Branch:** `injection-orchestrator-lean`  
**Base:** `main` (commit 0bcd282 - "Fix clipboard paste hang and stabilize text-injection defaults")  
**Goal:** Rewrite 18 commits into a logical, reviewable story

---

## Current State Analysis

### Existing Commits (Oldest to Newest):
1. `fadb82a` - begin documentation refactoring and injection changes
2. `f84fa98` - feat(stt): Enhance Vosk model discovery logging for CI debugging
3. `e892ec4` - docs: add comprehensive text injection architecture and Vosk diagnostic guide
4. `00c25de` - docs: add Parakeet STT research and remove outdated architecture doc
5. `90a6019` - chore(text-injection): snapshot old implementation before orchestrator rewrite
6. `ffe3ae6` - feat(text-injection): implement orchestrator-based architecture (WIP)
7. `8f4c33e` - feat(text-injection): wire up orchestrator modules in library interface
8. `c631ac4` - test: add enigo live testing script
9. `c94b4c1` - chore: update Cargo.lock for new dependencies
10. `fd9bc26` - Ditch snapshot
11. `d2ca97c` - Add comprehensive testing framework for ColdVox text injection stack
12. `691a620` - feat(text-injection): implement targeted pre-warming and fix async safety
13. `72d908d` - chore: run cargo fmt to fix formatting issues
14. `33e049f` - chore(dev): add optional pre-commit hook to run cargo fmt and installer script
15. `db11736` - chore(ci): make formatting advisory and drop unused examples
16. `f93c887` - refactor(audio): centralize capture buffer configuration
17. `a539f01` - refactor(text-injection): Implement beneficial injection improvements
18. `af75f86` - test
19. `c05f459` - refactor(clipboard): Add timeouts to clipboard command executions and improve test reliability

### Problems Identified:
1. **"test" commit** (`af75f86`) - Non-descriptive, should be squashed
2. **Snapshot churn** - Create snapshot → Ditch snapshot (commits 5, 10)
3. **Formatting commits** - Should be squashed into relevant features (commits 13, 14, 15)
4. **Scattered documentation** - Docs spread across multiple commits (1, 3, 4)
5. **Dependency updates isolated** - Should be with relevant features (commit 9)
6. **STT logging mixed in** - Vosk changes unrelated to injection work (commit 2)
7. **Non-atomic changes** - Some commits mix concerns (audio + injection)
8. **WIP markers** - Commit 6 has "(WIP)" but is in main history

---

## Proposed Rewrite Strategy

### New Commit Structure (7-9 commits):

#### **Commit 1: docs: Add text injection architecture documentation and planning**
- **Combines:** commits 1, 3, 4
- **Purpose:** Establish the "why" before the "what"
- **Changes:**
  - Add comprehensive text injection architecture docs
  - Add Parakeet STT research
  - Remove outdated architecture doc
  - Document refactoring and injection strategy
- **Message:**
  ```
  docs: Add text injection architecture documentation and planning
  
  Add comprehensive documentation for the text injection refactor:
  - Text injection architecture and strategy overview
  - Parakeet STT research and evaluation
  - Vosk diagnostic guide for CI debugging
  - Remove outdated architecture documentation
  
  This establishes the foundation and rationale for the orchestrator
  refactor that follows.
  ```

#### **Commit 2: feat(stt): Enhance Vosk model discovery logging for CI**
- **Keeps:** commit 2 (mostly unchanged)
- **Purpose:** Standalone improvement, helps with CI/debugging
- **Note:** Could be moved to separate PR if we want pure injection focus
- **Message:**
  ```
  feat(stt): Enhance Vosk model discovery logging for CI debugging
  
  Improve Vosk model path detection and logging to help diagnose
  CI failures and model availability issues. Adds detailed logging
  throughout the model discovery process.
  ```

#### **Commit 3: feat(text-injection): Implement orchestrator-based architecture**
- **Combines:** commits 5, 6, 7, 9, 11
- **Purpose:** The core refactor as one atomic change
- **Changes:**
  - Snapshot old implementation (preserving context)
  - Implement new orchestrator architecture
  - Wire up orchestrator modules in library interface
  - Add comprehensive testing framework
  - Update Cargo.lock for new dependencies
- **Message:**
  ```
  feat(text-injection): Implement orchestrator-based architecture
  
  Major refactor of the text injection system to use an orchestrator
  pattern for better separation of concerns and testability.
  
  Changes:
  - Preserve snapshot of previous implementation for reference
  - Implement new orchestrator with modular injector strategy
  - Add injector registry with runtime capability detection
  - Wire up orchestrator modules in library interface
  - Add comprehensive testing framework with mock injectors
  - Support for adaptive strategy selection based on environment
  
  This replaces the previous monolithic approach with a more
  maintainable and extensible architecture.
  ```

#### **Commit 4: feat(text-injection): Add targeted pre-warming and async safety**
- **Combines:** commits 12
- **Purpose:** Enhancement to the orchestrator
- **Message:**
  ```
  feat(text-injection): Add targeted pre-warming and async safety
  
  Enhance the orchestrator with pre-warming capabilities and async
  safety improvements:
  - Implement targeted pre-warming for AT-SPI and other backends
  - Fix async safety issues in orchestrator and manager
  - Add pre-warming cache with TTL
  - Improve error handling in async contexts
  
  Pre-warming reduces first-injection latency by establishing
  connections before they're needed.
  ```

#### **Commit 5: refactor(text-injection): Implement beneficial injection improvements**
- **Combines:** commits 17, 19
- **Purpose:** Refinements and fixes
- **Changes:**
  - Beneficial injection improvements
  - Clipboard timeout fixes
  - Test reliability improvements
- **Message:**
  ```
  refactor(text-injection): Implement beneficial injection improvements
  
  Refine the orchestrator implementation with several improvements:
  - Add timeouts to clipboard command executions
  - Improve test reliability and stability
  - Better error handling and recovery
  - Enhanced logging for debugging
  
  These changes address issues found during testing and improve
  overall reliability.
  ```

#### **Commit 6: refactor(audio): Centralize capture buffer configuration**
- **Keeps:** commit 16
- **Purpose:** Independent improvement (could be separate PR)
- **Message:**
  ```
  refactor(audio): Centralize capture buffer configuration
  
  Centralize audio capture buffer size configuration in AudioConfig
  rather than scattering magic numbers throughout the codebase.
  
  - Add capture_buffer_samples to AudioConfig
  - Update all audio components to use centralized config
  - Default to 65536 samples (~4.1s at 16kHz) for headroom
  - Document buffer sizing rationale
  
  This makes it easier to tune buffer sizes without hunting through
  the codebase.
  ```

#### **Commit 7: chore(ci): Make formatting advisory and improve dev workflow**
- **Combines:** commits 13, 14, 15
- **Purpose:** CI/dev workflow improvements as one logical change
- **Changes:**
  - Make formatting advisory in CI
  - Add optional pre-commit hooks
  - Remove unused examples
  - Format all code
- **Message:**
  ```
  chore(ci): Make formatting advisory and improve dev workflow
  
  Improve development workflow and CI reliability:
  - Make cargo fmt checks advisory rather than blocking
  - Add optional pre-commit hook for local formatting
  - Add installer script for git hooks
  - Remove unused example files to reduce maintenance
  - Run cargo fmt across codebase
  
  This reduces friction in the development process while still
  encouraging good formatting practices.
  ```

#### **Commit 8: test: Add enigo live testing utilities**
- **Keeps:** commit 8 (`c631ac4`)
- **Purpose:** Testing utilities
- **Changes:**
  - Add enigo live testing script
- **Message:**
  ```
  test: Add enigo live testing utilities
  
  Add utilities for testing the enigo backend and text injection:
  - Live testing script for enigo backend
  - Better test isolation and cleanup
  
  These tools help verify injection behavior across different
  desktop environments.
  ```

#### **Commit 9: docs: Remove old text-injection snapshot**
- **Keeps:** commit 10 (`fd9bc26`)
- **Purpose:** Cleanup now that refactor is stable
- **Message:**
  ```
  docs: Remove old text-injection snapshot
  
  Remove the snapshot of the old text-injection implementation now
  that the orchestrator refactor is complete and stable. The old
  implementation is preserved in git history if needed.
  ```

---

## Execution Plan

### Phase 1: Preparation (5 minutes)
1. **Create backup branch:**
   ```bash
   git branch backup/injection-orchestrator-lean-$(date +%Y%m%d-%H%M%S)
   git push origin backup/injection-orchestrator-lean-$(date +%Y%m%d-%H%M%S)
   ```

2. **Verify clean working directory:**
   ```bash
   git status
   # Should be clean
   ```

3. **Identify the base commit:**
   ```bash
   # Base: 0bcd282 (Fix clipboard paste hang)
   # This is our starting point on main
   ```

### Phase 2: Interactive Rebase Setup (10 minutes)
1. **Start interactive rebase:**
   ```bash
   git rebase -i 0bcd282
   ```

2. **Initial rebase plan** (in the editor):
   ```
   pick fadb82a begin documentation refactoring and injection changes
   pick f84fa98 feat(stt): Enhance Vosk model discovery logging for CI debugging
   squash e892ec4 docs: add comprehensive text injection architecture and Vosk diagnostic guide
   squash 00c25de docs: add Parakeet STT research and remove outdated architecture doc
   pick 90a6019 chore(text-injection): snapshot old implementation before orchestrator rewrite
   fixup ffe3ae6 feat(text-injection): implement orchestrator-based architecture (WIP)
   fixup 8f4c33e feat(text-injection): wire up orchestrator modules in library interface
   fixup c94b4c1 chore: update Cargo.lock for new dependencies
   fixup d2ca97c Add comprehensive testing framework for ColdVox text injection stack
   pick 691a620 feat(text-injection): implement targeted pre-warming and fix async safety
   pick f93c887 refactor(audio): centralize capture buffer configuration
   pick a539f01 refactor(text-injection): Implement beneficial injection improvements
   fixup af75f86 test
   fixup c05f459 refactor(clipboard): Add timeouts to clipboard command executions and improve test reliability
   pick 33e049f chore(dev): add optional pre-commit hook to run cargo fmt and installer script
   fixup 72d908d chore: run cargo fmt to fix formatting issues
   fixup db11736 chore(ci): make formatting advisory and drop unused examples
   pick c631ac4 test: add enigo live testing script
   pick fd9bc26 Ditch snapshot
   ```

### Phase 3: Commit Message Refinement (15 minutes)
As the rebase proceeds, update commit messages using the templates above:

1. **First stop** - Documentation commit:
   - Combine the documentation changes
   - Use the comprehensive message from Commit 1 template

2. **Second stop** - STT logging:
   - Keep as-is with minor message cleanup

3. **Third stop** - Orchestrator implementation:
   - This will combine 5 commits
   - Use the detailed message from Commit 3 template

4. **Fourth stop** - Pre-warming:
   - Message from Commit 4 template

5. **Fifth stop** - Audio refactor:
   - Message from Commit 6 template

6. **Sixth stop** - Injection improvements:
   - Message from Commit 5 template

7. **Seventh stop** - CI/dev workflow:
   - Message from Commit 7 template

8. **Eighth stop** - Testing utilities:
   - Keep `c631ac4` as standalone commit
   - Message from Commit 8 template

9. **Ninth stop** - Remove snapshot:
   - Keep `fd9bc26` as standalone commit
   - Message from Commit 9 template

### Phase 4: Conflict Resolution (Variable)
**Expected conflicts:**
- Cargo.lock (due to reordering)
- Test files (due to consolidation)
- CI files (due to squashing)

**Resolution strategy:**
```bash
# For each conflict:
git status  # See what's conflicting
git diff    # Understand the conflict

# Edit conflicting files
# Then:
git add <resolved-files>
git rebase --continue
```

**If things go wrong:**
```bash
git rebase --abort
# Restore from backup
git reset --hard backup/injection-orchestrator-lean-TIMESTAMP
```

### Phase 5: Verification (10 minutes)
1. **Check commit history:**
   ```bash
   git log --oneline main..HEAD
   # Should show ~7-9 clean commits
   ```

2. **Verify nothing was lost:**
   ```bash
   git diff main..HEAD > /tmp/new-changes.diff
   git diff main..backup/injection-orchestrator-lean-TIMESTAMP > /tmp/old-changes.diff
   diff /tmp/new-changes.diff /tmp/old-changes.diff
   # Should be identical (or nearly so)
   ```

3. **Build and test:**
   ```bash
   cargo check --workspace
   cargo test --workspace
   cargo clippy --workspace
   ```

4. **Review each commit:**
   ```bash
   git log --patch main..HEAD | less
   # Verify each commit is atomic and makes sense
   ```

### Phase 6: Force Push (2 minutes)
⚠️ **Warning:** This will rewrite history on the remote branch

1. **Update PR branch:**
   ```bash
   git push --force-with-lease origin injection-orchestrator-lean
   ```

2. **Verify on GitHub:**
   - Check that PR #152 updates correctly
   - Verify force-push warning appears
   - Review new commit list in PR

3. **Update PR description:**
   Add a note at the top:
   ```markdown
   > **Note:** Commit history was rewritten on 2025-10-12 to create a more
   > cohesive story. Previous commits are preserved in backup branch
   > `backup/injection-orchestrator-lean-YYYYMMDD-HHMMSS`.
   ```

---

## Alternative: Manual Cherry-Pick Approach

If interactive rebase becomes too complex, use this approach:

```bash
# Create new branch from base
git checkout -b injection-orchestrator-lean-v2 0bcd282

# Cherry-pick and squash manually
git cherry-pick fadb82a
git cherry-pick f84fa98
git cherry-pick e892ec4
# Squash into previous: git reset --soft HEAD~1 && git commit --amend

# Continue for each logical grouping...

# When done, verify and replace original branch
git branch -D injection-orchestrator-lean
git branch -m injection-orchestrator-lean
git push --force-with-lease origin injection-orchestrator-lean
```

---

## Rollback Plan

If anything goes wrong:

```bash
# Abort rebase if in progress
git rebase --abort

# Restore from backup
git reset --hard backup/injection-orchestrator-lean-TIMESTAMP
git push --force-with-lease origin injection-orchestrator-lean

# Or restore from reflog
git reflog  # Find the commit before rebase
git reset --hard HEAD@{N}
```

---

## Benefits of This Rewrite

1. **Clear narrative:** Story progresses logically from planning → implementation → refinement
2. **Atomic commits:** Each commit is self-contained and buildable
3. **Better review:** Reviewers can understand the progression
4. **Clean history:** Removes WIP markers, formatting commits, and churn
5. **Easier bisect:** If bugs appear, git bisect will be more effective
6. **Reduced noise:** Consolidates 18 commits into 7-9 meaningful ones

---

## Post-Rewrite Checklist

- [ ] Backup branch created and pushed
- [ ] Rebase completed successfully
- [ ] All tests pass
- [ ] No functionality lost (diff check)
- [ ] Force-pushed to origin
- [ ] PR description updated
- [ ] Team notified of history rewrite
- [ ] Backup branch documented for reference

---

## Notes

- **Estimated time:** 45-60 minutes (including conflicts)
- **Risk level:** Medium (history rewrite always carries risk)
- **Mitigation:** Multiple backups, careful verification
- **Best done:** When no one else is working on the branch
- **Communication:** Notify team before and after

---

**Created:** 2025-10-12  
**Author:** Claude (Sonnet 4)  
**Status:** Ready for execution
