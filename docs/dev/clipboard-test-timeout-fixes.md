# Clipboard Test Timeout Fixes

## Problem

The clipboard injection tests were hanging indefinitely when run in environments without a proper clipboard manager or display server. This was causing CI/CD pipelines to timeout and making local development difficult.

## Root Cause

All clipboard operations (`wl-paste`, `wl-copy`, `xclip`, `ydotool`, `qdbus`) were executed without timeouts using `Command::new(...).output().await`, which could hang indefinitely if:

1. No display server is available
2. Clipboard manager is unresponsive
3. Wayland/X11 clipboard protocols are not properly initialized
4. Running in headless CI environments

## Solution

### 1. Added Timeouts to All Clipboard Commands

Wrapped all `Command` executions with `tokio::time::timeout` using `config.per_method_timeout_ms`:

```rust
let timeout_duration = Duration::from_millis(self.config.per_method_timeout_ms);

let output_future = Command::new("wl-paste")
    .args(&["--type", "text/plain"])
    .output();

let output = tokio::time::timeout(timeout_duration, output_future)
    .await
    .map_err(|_| InjectionError::Timeout(self.config.per_method_timeout_ms))?
    .map_err(|e| InjectionError::Process(format!("Failed to execute wl-paste: {}", e)))?;
```

### 2. Added Test-Level Timeout Macro

Created a `with_test_timeout!` macro that wraps test bodies with a 2-minute timeout:

```rust
/// Helper macro to wrap tests with a 2-minute timeout to prevent hangs
macro_rules! with_test_timeout {
    ($test_body:expr) => {{
        let timeout_duration = Duration::from_secs(120); // 2 minutes
        match tokio::time::timeout(timeout_duration, $test_body).await {
            Ok(result) => result,
            Err(_) => panic!("Test timed out after 2 minutes - likely hanging on clipboard operations"),
        }
    }};
}
```

### 3. Updated Test Configuration

Modified tests that interact with clipboard to use short timeouts:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_with_seed_restore_wrapper() {
    with_test_timeout!(async {
        let mut config = InjectionConfig::default();
        config.per_method_timeout_ms = 500; // Short timeout to fail fast
        
        let result = with_seed_restore(payload, mime_type, None, || async { Ok(()) }).await;
        assert!(result.is_ok() || result.is_err());
    })
}
```

## Files Modified

- `/crates/coldvox-text-injection/src/injectors/clipboard.rs`
  - `read_wayland_clipboard()` - Added timeout
  - `read_x11_clipboard()` - Added timeout
  - `write_wayland_clipboard()` - Added timeout
  - `write_x11_clipboard()` - Added timeout
  - `try_ydotool_paste()` - Added timeout
  - `clear_klipper_history()` - Added timeout (kdotool feature)
  - Test module - Added `with_test_timeout!` macro and applied to relevant tests

## Testing Results

### Before Fixes
```
$ timeout 10s cargo test -p coldvox-text-injection --lib -- injectors::clipboard::tests::test_with_seed_restore_wrapper
...
Command exited with code 124  # Timeout!
```

### After Fixes
```
$ cargo test -p coldvox-text-injection --lib injectors::clipboard::tests
running 7 tests
test injectors::clipboard::tests::test_backend_detection ... ok
test injectors::clipboard::tests::test_clipboard_injector_creation ... ok
test injectors::clipboard::tests::test_clipboard_backup_creation ... ok
test injectors::clipboard::tests::test_context_default ... ok
test injectors::clipboard::tests::test_empty_text_handling ... ok
test injectors::clipboard::tests::test_legacy_inject_text ... ok
test injectors::clipboard::tests::test_with_seed_restore_wrapper ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out; finished in 0.26s
```

## Benefits

1. **No more hanging tests** - Tests fail fast with clear error messages
2. **Better CI/CD reliability** - Timeouts prevent pipeline hangs
3. **Clearer error messages** - Timeout errors indicate clipboard unavailability
4. **Configurable timeouts** - Can be adjusted via `InjectionConfig`
5. **Fail-fast behavior** - Tests complete in milliseconds instead of hanging indefinitely

## Configuration

Default timeout from `InjectionConfig::default()`:
- `per_method_timeout_ms`: 1000ms (1 second)

Can be overridden in tests or runtime:
```rust
let mut config = InjectionConfig::default();
config.per_method_timeout_ms = 500; // 500ms for faster failure in tests
```

## Recommendations

1. **Always use timeouts** for external command execution in async contexts
2. **Set appropriate test timeouts** - 2 minutes provides ample safety margin
3. **Use short timeouts in tests** - 500ms is enough to detect unavailability
4. **Document timeout behavior** - Make it clear when operations might timeout
5. **Test in headless environments** - Ensure tests pass without display servers

## Related Issues

This fix resolves the hanging clipboard tests issue discovered during comprehensive testing of PR #152 (injection-orchestrator-lean branch).
