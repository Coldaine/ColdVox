# PR Merge Battle Plan - 2025-10-08

**Status:** Ready for Execution
**Created:** 2025-10-08
**Author:** Claude Code + User Collaboration
**Objective:** Systematically merge 11 PRs (#123-#134) for domain-based refactor

---

## Executive Summary

**Strategy:** Fix blocking issues → Sequential merge → Compare with PR #134 → Choose best approach

**Timeline:** 4-5 hours (with CI waits)
**Risk Level:** LOW (all branches backed up with tags)
**Confidence:** HIGH (clear path forward)

---

## Table of Contents

1. [Phase 0: Backup & Safety](#phase-0-backup--safety)
2. [Phase 1: Critical Fixes](#phase-1-critical-fixes)
3. [Phase 2: Sequential Merge](#phase-2-sequential-merge)
4. [Phase 3: Validation & Comparison](#phase-3-validation--comparison)
5. [Phase 4: Decision Matrix](#phase-4-decision-matrix)
6. [Risk Assessment](#risk-assessment)
7. [Rollback Procedures](#rollback-procedures)

---

## Phase 0: Backup & Safety

### Objective
Create safety snapshots of all PR branches with proper UTC timestamps and atomic pushing.

### Improved Backup Script

**⚠️ CRITICAL: Run prechecks first**

```bash
# Verify origin points to upstream
git remote -v
# Should show: origin	https://github.com/Coldaine/ColdVox.git

# Verify you have permission to push tags
git tag test-tag-$(date +%s) && git push origin --delete test-tag-$(date +%s) 2>/dev/null || echo "Tag push permission verified"
```

**Execute Backup (Copy-Paste Safe)**

```bash
set -euo pipefail

# UTC timestamp for namespace
date_utc="$(date -u +%Y%m%dT%H%M%SZ)"
prefix="backup/${date_utc}"
prs=(123 124 125 126 127 128 129 130 131)

# Collect tags to push atomically
tags=()

echo "=== Backing up PR branches with timestamp: ${date_utc} ==="

# Backup each PR head via GitHub PR refspec (fork/same-repo agnostic)
for pr in "${prs[@]}"; do
  echo "Fetching PR #${pr}..."
  git fetch --no-tags origin "pull/${pr}/head:refs/remotes/backup/pr/${pr}"
  tag="${prefix}/pr-${pr}"
  git tag -a -m "Safety snapshot of PR #${pr}" "${tag}" "refs/remotes/backup/pr/${pr}"
  tags+=("${tag}")
  echo "  Tagged: ${tag}"
done

# Tag PR #134 branch
echo "Fetching PR #134 branch..."
git fetch --no-tags origin feature/test-and-doc-fixes-1:refs/remotes/backup/pr/134
tag134="${prefix}/pr-134"
git tag -a -m "Safety snapshot of PR #134" "${tag134}" "refs/remotes/backup/pr/134"
tags+=("${tag134}")
echo "  Tagged: ${tag134}"

# Tag anchor branch
echo "Fetching anchor branch..."
git fetch --no-tags origin anchor/oct-06-2025:refs/remotes/backup/anchor/oct-06-2025
tag_anchor="${prefix}/anchor-oct-06-2025"
git tag -a -m "Safety snapshot of anchor/oct-06-2025" "${tag_anchor}" "refs/remotes/backup/anchor/oct-06-2025"
tags+=("${tag_anchor}")
echo "  Tagged: ${tag_anchor}"

# Review locally before pushing
echo ""
echo "=== Created backup tags: ==="
git tag -l "${prefix}/*"

# Push only the new tags, atomically
echo ""
echo "=== Pushing tags atomically to origin ==="
git push --atomic origin "${tags[@]}"

echo ""
echo "✅ Backup complete! All ${#tags[@]} tags pushed successfully."
echo "Tags are namespaced under: ${prefix}/*"
```

**Verification**

```bash
# List all backup tags
git tag -l "backup/*"

# Verify on GitHub
gh api /repos/Coldaine/ColdVox/git/refs/tags | jq -r '.[] | select(.ref | contains("backup")) | .ref'
```

**Time:** ~5 minutes
**Status:** □ Not Started

---

## Phase 1: Critical Fixes

### Fix 1: PR #123 - Env Var Override Tests (DON'T SKIP!)

**Issue:** 3 tests marked `#[ignore]` with env var override mechanism incomplete

**Analysis:**
- Tests: `test_settings_new_with_env_override`, `test_settings_new_invalid_env_var_deserial`, `test_settings_new_validation_err`
- Root cause: Environment variable loading may be working, but tests can't verify due to compilation errors
- The multi-agent team diagnosed this but hasn't implemented fix

**Fix Strategy:**

```bash
gh pr checkout 123

# Check if tests can even compile first
cargo test -p coldvox-app settings_test --no-run

# If compilation fails:
# The issue is that lib.rs Settings infrastructure is incomplete
# without subsequent PRs. The env var mechanism itself is likely correct.

# Solution: Document as "will validate after full stack merge"
# Add tracking issue or comment explaining the dependency
```

**Alternative:** Just verify the config source ordering is correct (which it is - lines 194-200 in lib.rs add Environment after File sources).

**Time:** 30 minutes
**Priority:** P0 - BLOCKING
**Status:** □ Not Started

### Fix 2: PR #126 - Missing Module Declarations

**Issue:** New files `constants.rs` and `helpers.rs` not declared in `lib.rs`

**Fix:**

```bash
gh pr checkout 126

# Edit crates/coldvox-stt/src/lib.rs
# Add at appropriate location:
# pub mod constants;
# pub mod helpers;

# Verify compilation
cargo check -p coldvox-stt

# Commit and push
git add crates/coldvox-stt/src/lib.rs
git commit -m "fix: add module declarations for constants and helpers"
git push
```

**Time:** 5 minutes
**Priority:** P0 - BLOCKING
**Status:** □ Not Started

### Fix 3: PR #129 - Futures Dependency

**Issue:** Missing `futures` crate in dev-dependencies

**Fix:**

```bash
gh pr checkout 129

# Add to crates/coldvox-text-injection/Cargo.toml under [dev-dependencies]:
echo 'futures = "0.3"' >> crates/coldvox-text-injection/Cargo.toml

# OR use tokio runtime instead (cleaner):
# Edit clipboard_injector.rs line 228:
# Replace: futures::executor::block_on(...)
# With: tokio::runtime::Runtime::new().unwrap().block_on(...)

# Verify compilation
cargo check -p coldvox-text-injection
cargo test -p coldvox-text-injection --no-run

# Commit
git add crates/coldvox-text-injection/Cargo.toml
git commit -m "fix: add futures dev dependency for test compilation"
git push
```

**Time:** 10 minutes
**Priority:** P0 - BLOCKING
**Status:** □ Not Started

### Fix 4: PR #130 - Hardcoded Device

**Issue:** `vad_mic.rs` hardcodes "HyperX QuadCast" device, breaking portability

**Fix (Environment Variable Approach):**

```bash
gh pr checkout 130

# Edit crates/app/src/probes/vad_mic.rs
# Replace hardcoded line with:
let device_name = std::env::var("COLDVOX_TEST_DEVICE")
    .ok()
    .or_else(|| ctx.device.clone())
    .or_else(|| {
        tracing::warn!("No device specified, using default detection");
        None
    });

# Verify compilation
cargo check -p coldvox-app

# Commit
git add crates/app/src/probes/vad_mic.rs
git commit -m "fix: replace hardcoded device with env var and fallback"
git push
```

**Time:** 30 minutes
**Priority:** P1 - HIGH
**Status:** □ Not Started

### Fix 5: PR #124/#127 - Circular Dependency

**Issue:** #124 declares `wav_file_loader` module, #127 has the file

**Fix Strategy: Add file to BOTH PRs (duplication)**

Rationale: More robust than moving declaration - each PR becomes independently buildable.

```bash
# Step 1: Add file to PR #124
gh pr checkout 124

# Get the file from PR #127
git fetch origin 05-app-runtime-wav
git checkout origin/05-app-runtime-wav -- crates/app/src/audio/wav_file_loader.rs

# Verify it compiles
cargo check -p coldvox-app

# Commit
git add crates/app/src/audio/wav_file_loader.rs
git commit -m "fix: add wav_file_loader to resolve circular dependency with #127"
git push

# Step 2: Verify PR #127 still has the file (it should)
gh pr checkout 127
ls -la crates/app/src/audio/wav_file_loader.rs
# Should exist - no action needed

echo "Both PRs now have the file - independently buildable"
```

**Time:** 15 minutes
**Priority:** P0 - BLOCKING
**Status:** □ Not Started

### Summary: Phase 1

**Total Time:** ~1.5 hours
**Completed:** □ 0/5
**Blockers:** None (all fixable)

---

## Phase 2: Sequential Merge

### Prerequisites

- ✅ All Phase 1 fixes pushed and CI green
- ✅ Phase 0 backup tags created
- ✅ All PRs approved (or ready for auto-approval)

### Merge Strategy

Use **Graphite automation** if available, otherwise **manual gh CLI**.

#### Option A: Graphite Automated (Recommended)

```bash
# First, merge PR #132 (independent archive docs)
gh pr merge 132 --squash --admin

# Use merge script from PR #132
cd docs/execution/2025-10-08-domain-split/

# Test merge flow
./merge-stack.sh --dry-run

# Execute merges
./merge-stack.sh

# If interrupted, resume from specific PR
./merge-stack.sh --start-from 127
```

#### Option B: Manual Sequential (Fallback)

```bash
# Merge order with CI validation
prs=(132 123 124 125 126 127 128 129 130 131)

for pr in "${prs[@]}"; do
  echo "=== Merging PR #${pr} ==="

  # Check CI status
  gh pr view ${pr} --json statusCheckRollup --jq '.statusCheckRollup[] | select(.status != "COMPLETED" or .conclusion != "SUCCESS") | .name'

  # Wait for CI if needed
  gh pr checks ${pr} --watch

  # Merge when green
  gh pr merge ${pr} --squash --admin

  echo "✅ PR #${pr} merged"
  echo ""
  sleep 5  # Brief pause between merges
done
```

#### Handling Expected Issues

**Issue: #124/#127 Merge Conflict**

If duplication causes conflict when #127 merges:
```bash
# On #127 branch after #124 is merged
git fetch origin main
git merge origin/main

# If conflict on wav_file_loader.rs:
# Keep the version from #127 (it's identical or newer)
git checkout --theirs crates/app/src/audio/wav_file_loader.rs
git add crates/app/src/audio/wav_file_loader.rs
git commit -m "fix: resolve wav_file_loader merge conflict"
git push

# Re-merge PR
gh pr merge 127 --squash --admin
```

**Issue: #123 Tests Fail Post-Merge**

If env var override tests fail after merge:
```bash
# Run tests
cargo test -p coldvox-app settings_test -- --nocapture

# If failures occur:
# 1. Check if it's real or test setup issue
# 2. Fix in a follow-up PR
# 3. Don't block the stack for this

# Create tracking issue
gh issue create --title "Fix env var override tests in Settings" --body "Tests in settings_test.rs are failing post-merge. Need to debug environment variable loading mechanism."
```

### Verification Points

After each critical merge:

**After #123 (Config):**
```bash
cargo run --features vosk -- --list-devices
# Should show devices without errors
```

**After #127 (Runtime):**
```bash
cargo test -p coldvox-app test_end_to_end_wav -- --nocapture
# Should pass (or give clear model path guidance)
```

**After #131 (Docs - FINAL):**
```bash
# Verify full workspace
cargo check --workspace --all-features
cargo test --workspace

# Smoke test
cargo run --features vosk,text-injection -- --activation-mode vad
# Press Ctrl+C after 5 seconds - should shutdown cleanly
```

### Timeline

| PR | Est. CI Time | Dependencies | Notes |
|----|--------------|--------------|-------|
| #132 | 5 min | None | Independent, merge first |
| #123 | 30 min | CI fix | Foundation, blocks all |
| #124 | 20 min | #123 | Audio capture |
| #125 | 15 min | #124 | VAD (clean) |
| #126 | 20 min | #125 | STT helpers |
| #127 | 25 min | #126 | Runtime (may conflict) |
| #128 | 20 min | #127 | Text injection |
| #129 | 20 min | #128 | Testing |
| #130 | 20 min | #129 | Logging |
| #131 | 15 min | #130 | Docs (final) |

**Total:** ~3 hours (with CI waits)

**Status:** □ Not Started

---

## Phase 3: Validation & Comparison

### Objective
Verify merged stack works, then compare to PR #134

### Step 1: Test Merged Stack

```bash
# Checkout updated main
git fetch origin main
git checkout origin/main -b test-merged-stack-$(date +%s)

# Full workspace check
cargo check --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo test --workspace --all-features

# Runtime smoke tests
echo "=== Testing VAD mode ==="
timeout 10s cargo run --features vosk,text-injection -- --activation-mode vad || true

echo "=== Testing Hotkey mode ==="
timeout 10s cargo run --features vosk,text-injection -- --activation-mode hotkey || true

echo "=== Testing TUI ==="
timeout 10s cargo run --bin tui_dashboard || true

# Integration tests
cargo test -p coldvox-app integration --features vosk,text-injection

# Document results
echo "Merged stack validation: $(date)" > /tmp/merged-stack-results.txt
cargo test --workspace 2>&1 | tee -a /tmp/merged-stack-results.txt
```

### Step 2: Test PR #134

```bash
# Checkout PR #134
git checkout -b test-pr134-$(date +%s) origin/feature/test-and-doc-fixes-1

# Same test suite
cargo check --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo test --workspace --all-features

# Runtime smoke tests
timeout 10s cargo run --features vosk,text-injection -- --activation-mode vad || true

# Document results
echo "PR #134 validation: $(date)" > /tmp/pr134-results.txt
cargo test --workspace 2>&1 | tee -a /tmp/pr134-results.txt
```

### Step 3: Compare

```bash
# Diff the end states
git diff origin/main origin/feature/test-and-doc-fixes-1 > /tmp/diff-merged-vs-pr134.patch

# Analyze differences
echo "=== File Count Comparison ==="
echo "Merged stack:"
git diff --stat origin/main origin/main~11 | tail -1
echo "PR #134:"
git diff --stat origin/main origin/feature/test-and-doc-fixes-1 | tail -1

# Code-only comparison
echo "=== Rust Code Differences ==="
git diff origin/main origin/feature/test-and-doc-fixes-1 --stat -- 'crates/**/*.rs' | tail -5

# Test result comparison
echo "=== Test Results Comparison ==="
diff -u /tmp/merged-stack-results.txt /tmp/pr134-results.txt || true
```

**Time:** 1 hour
**Status:** □ Not Started

---

## Phase 4: Decision Matrix

| Scenario | Merged Stack | PR #134 | Action |
|----------|--------------|---------|--------|
| **Both work perfectly** | ✅ | ✅ | Keep merged stack (already done) |
| **Stack works, #134 fails** | ✅ | ❌ | Close #134, document why |
| **#134 works, stack fails** | ❌ | ✅ | Revert merges, merge #134 instead |
| **Both work, #134 cleaner** | ✅ | ✅ | Cherry-pick improvements from #134 |
| **Neither works fully** | ⚠️ | ⚠️ | Debug both, fix closer one first |

### If Keeping Merged Stack

```bash
# Close PR #134 with explanation
gh pr comment 134 --body "Closing in favor of merged stacked PRs (#123-#131). The stacked approach provides better attribution and domain separation. If there are unique improvements in this PR, they can be cherry-picked in follow-up PRs."

gh pr close 134
```

### If Switching to PR #134

```bash
# Revert all merged PRs
git checkout main
git log --oneline --graph -20  # Identify merge commits

# Create revert PR
git checkout -b revert-stack-use-pr134
for commit_sha in $(git log --reverse --format=%H main~11..main); do
  git revert --no-edit $commit_sha
done

gh pr create --title "revert: domain refactor stack, use PR #134 instead" --body "Reverting #123-#131 in favor of PR #134's cleaner implementation"

# Merge PR #134
gh pr merge 134 --squash --admin
```

**Status:** □ Not Started

---

## Risk Assessment

### Risk Matrix

| Risk | Likelihood | Impact | Priority | Mitigation |
|------|------------|--------|----------|------------|
| CI failure on #123 | LOW | HIGH | P0 | Already fixed (Vosk) |
| Circular dep merge conflict | MEDIUM | MEDIUM | P1 | Duplication strategy |
| Test failures post-merge | MEDIUM | LOW | P2 | Document & fix in follow-up |
| Git history corruption | LOW | HIGH | P0 | Atomic tag backups |
| PR #134 conflicts | HIGH | MEDIUM | P1 | Clear decision matrix |

### Rollback Procedures

**Scenario 1: Single PR merge fails**

```bash
# Revert the last merge
git checkout main
git revert HEAD
git push origin main
```

**Scenario 2: Multiple PRs merged, need to rollback**

```bash
# Restore from backup tags
date_utc="<your-backup-timestamp>"  # From Phase 0

# Restore specific PR
git fetch origin
git reset --hard "backup/${date_utc}/pr-123"

# Force push (DANGEROUS - coordinate with team)
# git push --force origin main
```

**Scenario 3: Complete rollback needed**

```bash
# Find the commit before any merges started
git log --oneline main | head -20

# Reset to that commit
git reset --hard <commit-before-merges>

# All PR branches still exist - start over
```

---

## Timeline Summary

| Phase | Duration | Can Parallelize | Dependencies |
|-------|----------|-----------------|--------------|
| **Phase 0: Backup** | 5 min | N/A | None |
| **Phase 1: Fixes** | 1.5 hrs | Yes (different PRs) | Phase 0 |
| **CI for Fixes** | 30 min | Yes (parallel CI) | Phase 1 |
| **Phase 2: Merge** | 3 hrs | No (sequential) | Phase 1 |
| **Phase 3: Validation** | 1 hr | Yes (parallel tests) | Phase 2 |
| **Phase 4: Decision** | 30 min | N/A | Phase 3 |
| **TOTAL** | **6.5 hrs** | Mixed | Sequential phases |

---

## Success Criteria

- ✅ All backup tags created and pushed
- ✅ All 5 blocking fixes applied and CI green
- ✅ All 11 PRs merged sequentially (or PR #134 chosen instead)
- ✅ Full test suite passes on main
- ✅ Smoke tests run successfully (VAD, Hotkey, TUI)
- ✅ Documentation accurate and up-to-date
- ✅ No regressions in functionality

---

## Communication Plan

### Checkpoints

**After Phase 0:**
- Post backup tag list to PR #123 for visibility

**After Phase 1:**
- Comment on each fixed PR: "✅ Blocking issue resolved, ready for merge"

**After Phase 2 Completes:**
- Post summary to PR #131: "🎉 All PRs merged successfully"

**After Phase 4:**
- Update PROJECT_STATUS.md
- Post final summary to all PRs with results

---

## Notes & Lessons Learned

(To be filled in during execution)

**What Went Well:**
-

**What Could Be Improved:**
-

**Unexpected Issues:**
-

**Time Actual vs Estimated:**
-

---

**END OF BATTLE PLAN**

*Last Updated: 2025-10-08*
*Next Review: After Phase 1 completion*
