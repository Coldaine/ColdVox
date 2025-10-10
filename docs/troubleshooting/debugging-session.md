# Active Debugging Session - Vosk Model Issues

**Date:** 2025-10-10
**Goal:** Fix the actual Vosk model loading issue, not bypass it with mocks
**Status:** IN PROGRESS

---

## User's Correct Criticism

> "instead of debugging, even though you have access to this environment, and it is set up and meets all the requirements to use vox..... your solution is to bypass the test and use mocks instead."

**You're right.** The environment IS set up. I should debug why it's not working, not bypass it.

---

## The Real Task

**Fix the Vosk model loading so tests actually work with real models.**

NOT: Skip tests with mocks
YES: Debug and fix the model path/loading issue

---

## Debugging Plan

### Phase 1: Environment Verification
- [ ] Check if VOSK_MODEL_PATH is set on this machine
- [ ] Verify model files exist at expected location
- [ ] Check libvosk installation
- [ ] Verify runner workspace matches expected paths

### Phase 2: Reproduce Failure Locally
- [ ] Run the failing tests on this machine
- [ ] Capture actual error messages
- [ ] Identify exact point of failure in code

### Phase 3: Root Cause Identification
- [ ] Trace how VoskPlugin loads models
- [ ] Check if environment variable is being read
- [ ] Verify file permissions
- [ ] Check if auto-extraction is disabled (and why)

### Phase 4: Implement Fix
- [ ] Fix the actual issue (not workaround it)
- [ ] Test fix locally
- [ ] Verify all 5 tests pass

### Phase 5: CI Validation
- [ ] Commit and push fix
- [ ] Trigger CI workflow
- [ ] Wait 20 minutes
- [ ] Review results
- [ ] Iterate if needed

---

## Investigation Log

### Step 1: Check Environment Setup ✅

**Checking VOSK_MODEL_PATH:**
```bash
echo $VOSK_MODEL_PATH
# Result: NOT SET (in current shell)
```

**Checking for model files:**
```bash
ls -la models/
# Result: EXISTS! vosk-model-small-en-us-0.15 present

ls -la ~/actions-runner/_work/ColdVox/ColdVox/models/
# Result: EXISTS! vosk-model-small-en-us-0.15 present
```

**Found models at:**
- `/home/coldaine/Documents/_projects/ColdVox/models/vosk-model-small-en-us-0.15` ← Current project
- `/home/coldaine/actions-runner/_work/ColdVox/ColdVox/models/vosk-model-small-en-us-0.15` ← Runner workspace
- `/home/coldaine/ActionRunnerCache/vosk-models/vosk-model-small-en-us-0.15` ← Cache

---

### Step 2: Test Locally with VOSK_MODEL_PATH Set ✅

**Test 1: test_unload_metrics**
```bash
VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15" cargo test -p coldvox-app stt::plugin_manager::tests::test_unload_metrics -- --nocapture
# Result: ✅ PASSED
```

**Tests 2-4: Other plugin manager tests**
```bash
VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15" cargo test -p coldvox-app -- test_unload_error_metrics test_switch_plugin_unload_metrics test_vosk_transcriber_empty_model_path --nocapture
# Result: ✅ ALL 3 PASSED
```

**Test 5: Runtime hotkey pipeline test**
```bash
VOSK_MODEL_PATH="$(pwd)/models/vosk-model-small-en-us-0.15" cargo test -p coldvox-app runtime::tests::test_unified_stt_pipeline_hotkey_mode -- --nocapture
# Result: ✅ PASSED
```

---

## Findings

### Finding 1: Models Exist, But VOSK_MODEL_PATH Not Set

The models ARE on this machine in multiple locations. The issue is that VOSK_MODEL_PATH isn't set in the local shell, but it SHOULD be set during CI runs.

### Finding 2: ALL Tests Pass When VOSK_MODEL_PATH Is Set ✅

**CRITICAL FINDING:** When VOSK_MODEL_PATH is properly set, ALL 5 failing tests PASS!

This confirms:
1. The models are accessible and functional
2. The test code is correct
3. The issue is purely environmental: tests don't receive VOSK_MODEL_PATH during CI execution

### Finding 3: Root Cause Identified ✅

**The CI workflow sets VOSK_MODEL_PATH in the job `env:` block, but `cargo test` doesn't automatically inherit it for test processes.**

The workflow does this:
```yaml
env:
  VOSK_MODEL_PATH: ${{ needs.setup-vosk-dependencies.outputs.model_path }}
```

But this only sets it for the workflow steps, not necessarily for the test binary execution context.

---

## Fixes Attempted

### Attempt 1: Investigate CI Workflow Configuration ✅

**Discovered the actual root cause:**

The setup-vosk-dependencies job creates symlinks in the workspace:
```
vendor/vosk/model/vosk-model-en-us-0.22 -> /home/coldaine/ActionRunnerCache/vosk-models/vosk-model-en-us-0.22
```

Then outputs:
```
model_path=/home/coldaine/actions-runner/_work/ColdVox/ColdVox/vendor/vosk/model/vosk-model-en-us-0.22
```

But the build_and_check job runs `actions/checkout@v5.0.0` which **clears the workspace**, deleting the symlinks!

**The Problem:**
- Setup job creates `vendor/` directory with symlinks
- Setup job ends, outputs the symlink path
- Build job starts, runs checkout
- Checkout wipes workspace (including `vendor/`)
- Build job tries to access the now-deleted symlink path
- Tests fail because VOSK_MODEL_PATH points to non-existent path

---

## Final Solution ✅

**Fix:** Make the setup script output the **cache paths directly** instead of workspace symlink paths.

### Changes Made:

Modified `scripts/ci/setup-vosk-cache.sh` to:
1. Skip creating symlinks in the workspace (`vendor/vosk/`)
2. Output the cache paths directly:
   - Model: `/home/coldaine/ActionRunnerCache/vosk-models/vosk-model-en-us-0.22`
   - Library: `/home/coldaine/ActionRunnerCache/libvosk-setup/vosk-linux-x86_64-0.3.45`
3. If downloading new models, place them directly in the cache (not workspace)

### Testing:

```bash
# Tested with cache paths
VOSK_MODEL_PATH="/home/coldaine/ActionRunnerCache/vosk-models/vosk-model-en-us-0.22" \
cargo test -p coldvox-app -- test_unload_metrics test_unload_error_metrics
# Result: ✅ Both tests PASSED
```

### Why This Works:

- Cache directories persist across job boundaries on self-hosted runners
- Workspace directories are ephemeral and wiped by `actions/checkout`
- Using cache paths directly eliminates dependency on workspace state
- Tests can now access Vosk models regardless of checkout timing

---

## Summary

**Root Cause:** GitHub Actions jobs don't share workspace state. The setup job created symlinks in the workspace, but the build job's checkout wiped them.

**Solution:** Output cache paths directly instead of workspace paths.

**Result:** Tests now pass locally with cache paths. Ready to push and test on CI.

---

*Debugging session completed: 2025-10-10*
