---
doc_type: playbook
subsystem: foundation
version: 1.0.0
status: active
owners: Documentation Working Group
last_reviewed: 2025-11-01
---

# AppError to ColdVoxError Migration Guide

## Overview

This guide helps developers migrate from the deprecated `AppError` to the new `ColdVoxError` unified error handling system introduced in PR #202. This change affects 46 files throughout the ColdVox codebase and breaks all public APIs that previously used `AppError`.

## Why This Change Was Made

The migration from `AppError` to `ColdVoxError` was implemented to:

1. **Unify Error Handling**: Create a consistent error handling approach across all ColdVox components
2. **Improve Error Context**: Provide structured error information with domain-specific details
3. **Enable Recovery Strategies**: Add built-in recovery mechanisms for different error types
4. **Enhance Debugging**: Better error messages and structured logging capabilities

## Breaking Changes

### 1. Error Type Replacement

All references to `AppError` must be replaced with `ColdVoxError`:

```rust
// Before (deprecated)
use crate::error::AppError;

// After (current)
use coldvox_foundation::error::ColdVoxError;
```

### 2. Import Path Changes

```rust
// Before
use crate::error::{AppError, AudioError};

// After
use coldvox_foundation::error::{ColdVoxError, AudioError};
```

### 3. Function Signature Updates

```rust
// Before
fn process_audio() -> Result<(), AppError> {
    // implementation
}

// After
fn process_audio() -> Result<(), ColdVoxError> {
    // implementation
}
```

### 4. Error Handling Pattern Changes

```rust
// Before
match error {
    AppError::Audio(msg) => handle_audio_error(msg),
    AppError::Config(msg) => handle_config_error(msg),
}

// After
match error {
    ColdVoxError::Audio(AudioError::DeviceNotFound { name }) => {
        handle_device_not_found(name)
    },
    ColdVoxError::Config(ConfigError::Validation { field, reason }) => {
        handle_validation_error(field, reason)
    },
}
```

## Migration Steps

### Step 1: Update Imports

Replace all `AppError` imports with `ColdVoxError` and domain-specific error types:

```rust
// Replace this
use crate::error::AppError;

// With this
use coldvox_foundation::error::{ColdVoxError, AudioError, SttError, VadError, InjectionError, ConfigError, PluginError};
```

### Step 2: Update Error Types

Replace generic `AppError` variants with appropriate domain-specific errors:

```rust
// Before
AppError::Audio("Device not found".to_string())

// After
ColdVoxError::Audio(AudioError::DeviceNotFound {
    name: Some("default".to_string())
})
```

### Step 3: Update Error Handling

Update match statements and error handling logic:

```rust
// Before
match result {
    Ok(data) => process_data(data),
    Err(AppError::Audio(msg)) => {
        log::error!("Audio error: {}", msg);
        return Err(AppError::Audio(format!("Failed to process: {}", msg)));
    }
}

// After
match result {
    Ok(data) => process_data(data),
    Err(ColdVoxError::Audio(audio_err)) => {
        match audio_err {
            AudioError::DeviceNotFound { name } => {
                log::error!("Device not found: {:?}", name);
                // Attempt recovery strategy
                match error.recovery_strategy() {
                    RecoveryStrategy::Fallback { to } => {
                        log::info!("Attempting fallback to: {}", to);
                        try_fallback_device(to)
                    }
                    RecoveryStrategy::Retry { max_attempts, delay } => {
                        retry_with_backoff(max_attempts, delay)
                    }
                    _ => return Err(ColdVoxError::Audio(audio_err)),
                }
            }
            AudioError::DeviceDisconnected => {
                log::warn!("Device disconnected, attempting reconnection");
                attempt_reconnection()
            }
            // Handle other audio errors...
        }
    }
}
```

### Step 4: Update Error Conversions

Update `From` trait implementations for error conversions:

```rust
// Before
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Audio(err.to_string())
    }
}

// After
impl From<std::io::Error> for ColdVoxError {
    fn from(err: std::io::Error) -> Self {
        ColdVoxError::Injection(InjectionError::Io(err))
    }
}
```

## Code Examples

### Basic Error Handling

```rust
use coldvox_foundation::error::{ColdVoxError, AudioError, Result};

fn capture_audio() -> Result<Vec<i16>, ColdVoxError> {
    let device = find_audio_device()
        .ok_or_else(|| ColdVoxError::Audio(AudioError::DeviceNotFound {
            name: Some("default".to_string())
        }))?;

    let buffer = allocate_buffer()
        .map_err(|e| ColdVoxError::Audio(AudioError::BufferOverflow {
            count: e.samples_lost
        }))?;

    Ok(buffer)
}
```

### Plugin Development

```rust
use coldvox_foundation::error::{ColdVoxError, SttError, PluginError};
use async_trait::async_trait;

#[async_trait]
pub trait MySttPlugin {
    async fn process(&mut self, audio: &[i16]) -> Result<String, ColdVoxError>;
}

struct MyPlugin {
    initialized: bool,
}

#[async_trait]
impl MySttPlugin for MyPlugin {
    async fn process(&mut self, audio: &[i16]) -> Result<String, ColdVoxError> {
        if !self.initialized {
            return Err(ColdVoxError::Stt(SttError::NotAvailable {
                plugin: "my_plugin".to_string(),
                reason: "Plugin not initialized".to_string(),
            }));
        }

        // Process audio...
        Ok("transcription".to_string())
    }
}
```

### Error Recovery Strategies

```rust
use coldvox_foundation::error::{ColdVoxError, RecoveryStrategy};

fn handle_error_with_recovery(error: ColdVoxError) -> Result<(), ColdVoxError> {
    match error.recovery_strategy() {
        RecoveryStrategy::Retry { max_attempts, delay } => {
            for attempt in 1..=max_attempts {
                match retry_operation() {
                    Ok(result) => return Ok(result),
                    Err(retry_error) if attempt < max_attempts => {
                        log::warn!("Attempt {} failed: {}, retrying in {:?}ms",
                                 attempt, retry_error, delay);
                        tokio::time::sleep(delay).await;
                    }
                    Err(final_error) => return Err(final_error),
                }
            }
        }
        RecoveryStrategy::Fallback { to } => {
            log::info!("Primary operation failed, attempting fallback to: {}", to);
            fallback_operation(to)
        }
        RecoveryStrategy::Restart => {
            log::error!("Fatal error, requiring restart: {}", error);
            trigger_restart_sequence();
        }
        RecoveryStrategy::Ignore => {
            log::debug!("Ignoring non-critical error: {}", error);
            Ok(())
        }
        RecoveryStrategy::Fatal => {
            log::error!("Fatal error, cannot recover: {}", error);
            return Err(error);
        }
    }
}
```

## Migration Checklist

### Pre-Migration

- [ ] Identify all files using `AppError`
- [ ] List all public APIs that return `AppError`
- [ ] Document current error handling patterns
- [ ] Identify custom error variants specific to your code
- [ ] Plan test updates for new error handling

### Migration Tasks

- [ ] Update all `use` statements to import `ColdVoxError`
- [ ] Replace `AppError` with appropriate domain-specific errors
- [ ] Update function signatures to return `ColdVoxError`
- [ ] Modify error handling logic to use structured error variants
- [ ] Update `From` trait implementations
- [ ] Add recovery strategies where appropriate
- [ ] Update error messages to use structured fields

### Post-Migration

- [ ] Run full test suite to verify changes
- [ ] Update integration tests to use new error types
- [ ] Verify error logging displays correctly
- [ ] Test error recovery mechanisms
- [ ] Update documentation examples
- [ ] Validate that all error paths are covered

### Validation

- [ ] All imports updated successfully
- [ ] No compilation errors related to error types
- [ ] All tests pass with new error handling
- [ ] Error messages are informative and actionable
- [ ] Recovery strategies work as expected
- [ ] No runtime panics from error handling

## Troubleshooting

### Common Issues

#### 1. Compilation Errors

**Issue**: `error[E0433]: expected type, found struct AppError`
**Solution**: Ensure all imports are updated and old error type references are removed

```rust
// Check for these patterns and update them
use crate::error::AppError;  // Remove this
fn my_function() -> AppError { ... }  // Update this
```

#### 2. Missing Error Fields

**Issue**: Error variants require fields that weren't needed before
**Solution**: Use the structured error variants with appropriate field values

```rust
// Before
AppError::Audio("Device not found".to_string())

// After
ColdVoxError::Audio(AudioError::DeviceNotFound {
    name: Some("default".to_string())
})
```

#### 3. Recovery Strategy Confusion

**Issue**: Unsure which recovery strategy to use
**Solution**: Use the built-in `recovery_strategy()` method

```rust
match error {
    ColdVoxError::Audio(audio_err) => {
        // Let the error type determine recovery
        match error.recovery_strategy() {
            RecoveryStrategy::Retry { .. } => implement_retry(),
            RecoveryStrategy::Fallback { .. } => implement_fallback(),
            // Don't manually choose strategy
        }
    }
}
```

#### 4. Test Failures

**Issue**: Tests expecting `AppError` now fail
**Solution**: Update test assertions to expect `ColdVoxError` variants

```rust
// Before
assert!(matches!(result, Err(AppError::Audio(_)));

// After
assert!(matches!(result, Err(ColdVoxError::Audio(AudioError::DeviceNotFound { .. }))));
```

## New Error Handling Capabilities

### 1. Structured Error Context

`ColdVoxError` provides detailed context for each error type:

```rust
match error {
    ColdVoxError::Audio(AudioError::DeviceNotFound { name }) => {
        log::error!("Audio device '{}' not found", name.as_deref().unwrap_or(&"unknown".to_string()));
    }
    ColdVoxError::Stt(SttError::ModelNotFound { path }) => {
        log::error!("STT model not found at path: {}", path.display());
    }
}
```

### 2. Automatic Recovery Strategies

Each error type has a recommended recovery strategy:

```rust
fn handle_error(error: ColdVoxError) -> Result<(), ColdVoxError> {
    use coldvox_foundation::error::RecoveryStrategy;

    match error.recovery_strategy() {
        RecoveryStrategy::Retry { max_attempts, delay } => {
            // Automatically retry with exponential backoff
        }
        RecoveryStrategy::Fallback { to } => {
            // Switch to alternative component
        }
        RecoveryStrategy::Restart => {
            // Restart the affected subsystem
        }
    }
}
```

### 3. Enhanced Debugging

Better error messages and structured logging:

```rust
use tracing::{error, warn, debug};

match error {
    ColdVoxError::Injection(InjectionError::Timeout(ms)) => {
        error!(
            timeout_ms = ms,
            "Text injection timed out after {}ms",
            ms
        );
    }
    ColdVoxError::Config(ConfigError::Validation { field, reason }) => {
        warn!(
            field = field,
            reason = reason,
            "Configuration validation failed for field '{}': {}",
            field, reason
        );
    }
}
```

## Benefits of Migration

1. **Better Error Context**: Structured error information with specific fields
2. **Consistent Error Handling**: Unified approach across all ColdVox components
3. **Automatic Recovery**: Built-in strategies for common error scenarios
4. **Improved Debugging**: Enhanced logging and error tracing
5. **Type Safety**: Compile-time verification of error handling completeness
6. **Future-Proof**: Extensible error system for new features

## Additional Resources

- [ColdVox Error Reference](../reference/crates/coldvox-foundation.md)
- [Error Handling Best Practices](../domains/foundation/fdn-voice-pipeline-core-design.md)
- [Troubleshooting Guide](../troubleshooting/)
- [API Documentation](https://docs.rs/coldvox-foundation/latest/coldvox_foundation/error/)

## Getting Help

If you encounter issues during migration:

1. Check this guide for common solutions
2. Review existing code in the codebase for examples
3. Consult the troubleshooting section above
4. Reach out to the ColdVox development team for assistance

---

*This guide will be updated as new migration patterns are discovered. Last updated: 2025-11-01*
