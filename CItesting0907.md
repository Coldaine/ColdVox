# Comprehensive Test Review Plan - CI Testing Issues 09/07

## Executive Summary

This document provides a detailed plan for another agent to exhaustively review all tests that run during or after the failing/hanging portion of the ColdVox testing pipeline. Several tests are hanging (running >60 seconds), indicating potential issues with async operations, external dependencies, or infinite loops.

## Failing/Hanging Tests Analysis

**Hanging Tests (>60 seconds):**
- `test manager::tests::test_inject_success` - Manager injection test hanging
- `test tests::test_focus_tracking::tests::test_focus_cache_expiry` - Focus tracking cache test
- `test tests::test_focus_tracking::tests::test_focus_detection` - Focus detection test

**Passed Tests for Context:**
- `test processor::tests::test_injection_processor_basic_flow` - Basic processor flow
- `test tests::test_integration::integration_tests::test_app_allowlist_blocklist` - Integration test
- `test manager::tests::test_method_ordering` - Method ordering test
- `test tests::test_async_processor::async_processor_handles_final_and_ticks_without_panic` - Async processor test

## Root Cause Analysis of Hanging Issues

### 1. AT-SPI Connection Issues
**Location:** `crates/coldvox-text-injection/src/focus.rs:57`
- **Issue:** `AccessibilityConnection::new().await` has no timeout
- **Impact:** Blocks indefinitely if AT-SPI service unavailable
- **Files to Review:**
  - `crates/coldvox-text-injection/src/focus.rs` (lines 50-118)
  - `crates/coldvox-text-injection/src/atspi_injector.rs` (all timeout implementations)
  - `crates/coldvox-text-injection/src/manager.rs:283` (get_current_app_id method)

### 2. External Process Dependencies
**Affected Tests:** Focus detection and injection tests
- **Issue:** Tests may be waiting for unavailable system tools
- **Dependencies:**
  - ydotool (Wayland input simulation)
  - AT-SPI service (accessibility)
  - wl-clipboard (clipboard access)
  - xprop/X11 tools

### 3. Async Loop Timeout Issues
**Location:** `crates/coldvox-text-injection/src/processor.rs:334-380`
- **Issue:** AsyncInjectionProcessor runs infinite select! loop with 100ms intervals
- **Risk:** Test may not properly terminate the loop

## Complete Test Structure Map

### Text Injection Crate Tests (`crates/coldvox-text-injection/`)
```
src/tests/mod.rs - Test module organization
├── test_adaptive_strategy.rs - Strategy adaptation tests
├── test_allow_block.rs - Allowlist/blocklist filtering
├── test_async_processor.rs - HANGING: Async processor tests
├── test_focus_enforcement.rs - Focus requirement enforcement
├── test_focus_tracking.rs - HANGING: Focus detection and cache
├── test_integration.rs - Integration flow tests
├── test_mock_injectors.rs - Mock injector implementations
├── test_permission_checking.rs - Permission validation
├── test_util.rs - Test utilities
└── test_window_manager.rs - Window management tests
```

### Main Application Tests (`crates/app/`)
```
tests/
├── integration/
│   ├── capture_integration_test.rs - Audio capture integration
│   ├── mock_injection_tests.rs - Mock text injection tests
│   └── text_injection_integration_test.rs - Real injection tests
├── unit/
│   ├── silence_detector_test.rs - Silence detection unit tests
│   └── watchdog_test.rs - Audio watchdog tests
└── common/
    ├── mod.rs - Common test infrastructure
    └── test_utils.rs - Shared test utilities
```

### Source Tests (Embedded)
- `crates/app/src/stt/tests.rs` - STT processor tests
- `crates/coldvox-text-injection/src/processor.rs` (lines 402-500) - HANGING: Processor tests
- `crates/coldvox-text-injection/src/manager.rs` (lines 1117-1365) - HANGING: Manager tests

## Comprehensive Review Instructions

### Phase 1: Immediate Hanging Test Analysis

**PRIORITY 1 - Examine these files for infinite loops or blocking operations:**

1. **Focus Detection Tests** - `crates/coldvox-text-injection/src/tests/test_focus_tracking.rs`
   - Line 24: `test_focus_detection` - Check AT-SPI connection timeout
   - Line 39: `test_focus_cache_expiry` - Verify sleep duration vs cache timeout
   - **Look for:** AT-SPI calls without timeout wrappers

2. **Manager Injection Test** - `crates/coldvox-text-injection/src/manager.rs:1319`
   - Function: `test_inject_success`
   - **Look for:** Real injection attempts on headless systems
   - **Check:** External process calls (ydotool, clipboard operations)

3. **Async Processor Test** - `crates/coldvox-text-injection/src/tests/test_async_processor.rs`
   - Line 9: `async_processor_handles_final_and_ticks_without_panic`
   - **Look for:** Missing timeout on processor.run() method
   - **Check:** tokio::select! loop termination conditions

### Phase 2: Systematic Test File Review

**For each test file, analyze:**

1. **Timeout Configuration**
   - Search for `Duration::from_millis`, `timeout`, `sleep`
   - Verify all async operations have reasonable timeouts
   - Check that timeouts are shorter than CI timeout limits

2. **External Dependencies**
   - Identify calls to system binaries (ydotool, xprop, etc.)
   - Check for proper availability checks before usage
   - Verify mock implementations for headless testing

3. **Async Operation Patterns**
   - Look for `tokio::select!` loops without proper exit conditions
   - Check channel receivers that may block indefinitely
   - Verify spawn_blocking operations have timeouts

### Phase 3: Test Execution Flow Analysis

**Map out test execution dependencies:**

1. **Feature Flag Dependencies**
   - Check which tests require `atspi`, `wl_clipboard`, etc.
   - Verify CI environment has required features enabled
   - Document tests that should be skipped in headless environments

2. **Test Environment Setup**
   - Review test initialization in `init_test_tracing()` functions
   - Check for proper cleanup in test teardown
   - Verify no resource leaks between tests

3. **Integration Test Flow**
   - Map dependencies between integration tests
   - Check for proper state isolation between tests
   - Verify shared resource management

### Phase 4: Specific Code Locations to Review

**Critical Files Requiring Line-by-Line Analysis:**

1. **AT-SPI Integration** - Potential blocking operations
   ```
   crates/coldvox-text-injection/src/focus.rs:57-118
   crates/coldvox-text-injection/src/atspi_injector.rs:45-120
   crates/coldvox-text-injection/src/manager.rs:277-324
   ```

2. **Process Execution** - External tool timeouts
   ```
   crates/coldvox-text-injection/src/ydotool_injector.rs:60-95
   crates/coldvox-text-injection/src/kdotool_injector.rs:85-150
   ```

3. **Async Loops** - Infinite loop conditions
   ```
   crates/coldvox-text-injection/src/processor.rs:332-379
   ```

4. **Test Setup** - Environment dependencies
   ```
   crates/coldvox-text-injection/src/tests/test_focus_tracking.rs:10-21
   crates/coldvox-text-injection/src/tests/test_integration.rs:8-20
   ```

### Phase 5: Recommendations and Fixes

**For each identified issue, provide:**

1. **Root Cause Analysis**
   - Exact line numbers and function names
   - Explanation of why the code blocks/hangs
   - Environmental conditions that trigger the issue

2. **Proposed Solution**
   - Specific code changes required
   - Timeout values or retry logic
   - Mock implementations for headless testing

3. **Test Environment Fixes**
   - CI configuration changes needed
   - Feature flag adjustments
   - Environment variable requirements

## Analysis Results - Critical Issues Identified

### 🔴 Issue Catalog

#### 1. **AT-SPI Connection Without Timeout** (crates/coldvox-text-injection/src/focus.rs:57)
- **Root Cause**: `AccessibilityConnection::new().await` has NO timeout wrapper
- **Impact**: Blocks indefinitely if AT-SPI service is unavailable or unresponsive
- **Affected Tests**:
  - `test_focus_detection` (test_focus_tracking.rs:24)
  - `test_focus_cache_expiry` (test_focus_tracking.rs:39)
- **Evidence**: Unlike atspi_injector.rs:79 which has proper timeout, focus.rs lacks timeout protection

#### 2. **Manager Test Real Injection Attempts** (crates/coldvox-text-injection/src/manager.rs:1319)
- **Root Cause**: `test_inject_success` performs real text injection operations
- **Impact**: Tries actual AT-SPI/clipboard operations in headless CI environment
- **Environment Issue**: CI has Xvfb+fluxbox+D-Bus but AT-SPI may not respond properly

#### 3. **Async Processor Test Incomplete Termination** (crates/coldvox-text-injection/src/tests/test_async_processor.rs:9)
- **Root Cause**: Test creates AsyncInjectionProcessor but doesn't properly await `run()` with shutdown
- **Impact**: Processor's internal tokio::select! loop (processor.rs:339-376) may continue running
- **Missing**: Proper shutdown signal and task termination

### 🎯 Fix Priority Matrix

| Priority | Issue | Implementation Effort | Impact |
|----------|-------|----------------------|---------|
| **CRITICAL** | AT-SPI timeout in focus.rs | Low (5 lines) | High (fixes 2 hanging tests) |
| **HIGH** | Async processor termination | Medium (20 lines) | High (fixes 1 hanging test) |
| **HIGH** | CI environment detection | Medium (15 lines) | Medium (prevents false failures) |
| **MEDIUM** | Manager test headless compatibility | Low (10 lines) | Low (improves reliability) |

### 📋 Implementation Plan

#### Fix 1: Add Timeout to AT-SPI Connection in focus.rs (CRITICAL)

```rust
// crates/coldvox-text-injection/src/focus.rs:57
// REPLACE:
let conn = match AccessibilityConnection::new().await {

// WITH:
use tokio::time;
let timeout_duration = Duration::from_millis(250);
let conn = match time::timeout(timeout_duration, AccessibilityConnection::new()).await {
    Ok(Ok(c)) => c,
    Ok(Err(err)) => {
        debug!(error = ?err, "AT-SPI: failed to connect");
        return Ok(FocusStatus::Unknown);
    }
    Err(_) => {
        debug!("AT-SPI: connection timeout after {}ms", timeout_duration.as_millis());
        return Ok(FocusStatus::Unknown);
    }
};
```

#### Fix 2: Fix Async Processor Test Termination (HIGH)

```rust
// crates/coldvox-text-injection/src/tests/test_async_processor.rs:9
#[tokio::test]
async fn async_processor_handles_final_and_ticks_without_panic() {
    let (tx, rx) = mpsc::channel::<TranscriptionEvent>(8);
    let (sd_tx, sd_rx) = mpsc::channel::<()>(1);

    let config = InjectionConfig::default();
    let proc = AsyncInjectionProcessor::new(config, rx, sd_rx, None).await;

    // Spawn the processor in a task
    let proc_handle = tokio::spawn(async move {
        proc.run().await
    });

    // Send test event
    tx.send(TranscriptionEvent::Final {
        utterance_id: 1,
        text: "hello world".to_string(),
        words: None,
    }).await.unwrap();

    // Wait briefly for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send shutdown signal
    sd_tx.send(()).await.unwrap();

    // Wait for processor to exit with timeout
    let _ = timeout(Duration::from_secs(1), proc_handle).await.expect("Processor should shutdown gracefully");
}
```

#### Fix 3: Add CI Environment Detection (HIGH)

```rust
// crates/coldvox-text-injection/src/tests/test_util.rs (create new file)
pub fn skip_if_headless_ci() -> bool {
    // Skip tests that require real GUI/AT-SPI in CI
    if std::env::var("CI").is_ok() {
        if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
            return true;
        }
        // Additional check: try to verify AT-SPI is actually responding
        return !is_atspi_responsive();
    }
    false
}

fn is_atspi_responsive() -> bool {
    // Quick check if we can connect to AT-SPI within a short timeout
    use std::time::Duration;
    tokio::runtime::Handle::current().block_on(async {
        tokio::time::timeout(
            Duration::from_millis(100),
            test_atspi_connection()
        ).await.is_ok()
    })
}

async fn test_atspi_connection() -> bool {
    // Minimal AT-SPI connection test
    #[cfg(feature = "atspi")]
    {
        atspi::connection::AccessibilityConnection::new().await.is_ok()
    }
    #[cfg(not(feature = "atspi"))]
    false
}

// Update existing tests to use this:
#[tokio::test]
async fn test_focus_detection() {
    if skip_if_headless_ci() {
        eprintln!("Skipping: headless CI environment detected");
        return;
    }

    init_test_tracing();
    // ... rest of existing test code
}

#[tokio::test]
async fn test_focus_cache_expiry() {
    if skip_if_headless_ci() {
        eprintln!("Skipping: headless CI environment detected");
        return;
    }

    init_test_tracing();
    // ... rest of existing test code
}

#[tokio::test]
async fn test_inject_success() {
    if skip_if_headless_ci() {
        eprintln!("Skipping: headless CI environment detected");
        return;
    }

    // ... rest of existing test code
}
```

#### Fix 4: Update CI Configuration (MEDIUM)

```yaml
# .github/workflows/ci.yml - Add test timeout environment
- name: Test with real-injection-tests feature
  run: |
    dbus-run-session -- bash -lc '
      # Set per-test timeout to prevent hanging
      export RUST_TEST_TIME_UNIT="10000"  # 10 second timeout per test
      export RUST_TEST_TIME_INTEGRATION="30000"  # 30 second for integration tests
      cargo test -p coldvox-text-injection \
        --features real-injection-tests \
        -- --nocapture --test-threads=1 --timeout 600
    '
```

### 📊 Test Environment Assessment

**Current CI Environment (Ubuntu + Xvfb + D-Bus):**
- ✅ D-Bus session bus available
- ✅ Xvfb virtual display running
- ✅ Basic AT-SPI infrastructure present
- ❌ AT-SPI services may not respond within reasonable timeouts
- ❌ No timeout protection on blocking AT-SPI calls

**Required Dependencies:**
- ✅ dbus-x11, at-spi2-core (installed in CI)
- ✅ wl-clipboard, xclip (installed in CI)
- ✅ ydotool (installed in CI)

**Configuration Issues:**
- Missing timeout wrappers on AT-SPI connections
- Tests don't detect/adapt to headless environments
- No graceful degradation for unresponsive services

### 🎯 Expected Outcomes

After implementing these fixes:
1. **Immediate**: Tests will fail fast (250ms AT-SPI timeout) instead of hanging 60+ seconds
2. **Reliability**: Tests requiring real AT-SPI will be skipped in problematic environments
3. **Completeness**: Async processor will properly terminate its event loop
4. **CI Stability**: Pipeline will complete reliably within timeout bounds

**Verification Steps:**
1. Run tests locally to confirm timeout behavior
2. Test CI environment detection logic
3. Verify async processor shutdown in test
4. Confirm CI pipeline completes within 20-minute limit

## Expected Deliverables - COMPLETED

1. ✅ **Issue Catalog** - Complete list of hanging/blocking operations with line numbers
2. ✅ **Fix Priority Matrix** - Issues ranked by impact and implementation effort
3. ✅ **Test Environment Assessment** - Required dependencies and configuration
4. ✅ **Implementation Plan** - Step-by-step fixes for each identified issue

## Files Summary for Review

**Primary Focus Files (Likely containing blocking operations):**
- `crates/coldvox-text-injection/src/focus.rs`
- `crates/coldvox-text-injection/src/manager.rs`
- `crates/coldvox-text-injection/src/processor.rs`
- `crates/coldvox-text-injection/src/tests/test_focus_tracking.rs`
- `crates/coldvox-text-injection/src/tests/test_async_processor.rs`

**Supporting Files (For context and dependencies):**
- All files in `crates/coldvox-text-injection/src/tests/`
- All files in `crates/app/tests/`
- Cargo.toml files for feature flags
- Build scripts for platform detection

**Test Configuration:**
- `crates/coldvox-text-injection/Cargo.toml` - Feature flags
- `crates/app/Cargo.toml` - Platform dependencies
- `crates/app/build.rs` - Build-time platform detection

This comprehensive review should identify all blocking operations and provide actionable solutions to fix the hanging tests in the CI pipeline.

## Additional Unaccounted Concerns

### 1. **Fixed 500ms Timeout in `verify_injection`**
`test_harness.rs:119` - The hardcoded 500ms timeout may be insufficient in CI environments under load. This should be configurable or escalated.

### 2. **Process Cleanup Race Conditions**
`test_harness.rs:24-36` - The `Drop` implementation kills processes but doesn't handle:
- Zombie processes if kill fails
- Race conditions where process is already dead
- Cleanup of child processes spawned by test apps

### 3. **Temporary File Collisions**
`test_harness.rs:76,106` - Using PID-based temp files (`/tmp/coldvox_gtk_test_{pid}.txt`) without checking for existing files could cause test interference if PIDs get reused.

### 4. **Missing stdout/stderr Capture**
`test_harness.rs:71-72,101-102` - Test apps redirect to `/dev/null`, losing potentially valuable debug information when tests fail. Should capture to temp files in CI mode.

### 5. **No Test App Startup Verification**
`test_harness.rs:56-83` - After spawning processes, there's no verification that the app actually started successfully or is ready to receive input.

### 6. **Hardcoded is_ci: false Override**
`test_harness.rs:158-159` - The `TestEnvironment` forcibly sets `is_ci: false`, disabling CI detection logic. This prevents proper test skipping in CI.

### 7. **No Retry Logic for Transient Failures**
The test harness lacks retry mechanisms for operations that commonly fail transiently (process spawning, file I/O, etc.).

### 8. **Missing Test Isolation**
No mechanisms to ensure tests run in isolated environments - they could interfere with each other through shared system resources (clipboard, AT-SPI state).

### 9. **Insufficient Error Context**
Error messages lack context about system state (which display server, which session type, available backends).

### 10. **No Resource Limit Handling**
Tests don't check for or handle resource exhaustion (file descriptors, process limits, memory).

These additional concerns should be added to the implementation plan for comprehensive CI test stability.
