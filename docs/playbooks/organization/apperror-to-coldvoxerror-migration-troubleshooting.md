---
doc_type: troubleshooting
subsystem: foundation
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2025-11-01
owners: Documentation Working Group
version: 1.0.0
---

# AppError to ColdVoxError Migration Troubleshooting

This guide helps developers resolve common issues encountered when migrating from `AppError` to `ColdVoxError` in the ColdVox codebase.

## Common Migration Issues

### 1. Compilation Errors

#### Issue: `error[E0433]: expected type, found struct AppError`
**Cause**: Remaining references to old `AppError` type after imports have been updated

**Symptoms**:
```
error[E0433]: expected type, found struct `AppError`
   --> src/my_module.rs:42:15
    |
42 |     return Err(AppError::Audio("Device not found".to_string()))
    |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Solutions**:
1. **Search for remaining AppError references**:
   ```bash
   git grep -r "AppError" --exclude-dir=target
   ```

2. **Update all imports**:
   ```rust
   // Replace this
   use crate::error::AppError;

   // With this
   use coldvox_foundation::error::{ColdVoxError, AudioError};
   ```

3. **Check for type aliases**:
   ```bash
   git grep -r "type.*Result.*=.*AppError" --exclude-dir=target
   ```

#### Issue: `error[E0277]: mismatched types` in error handling
**Cause**: Match statement expects `AppError` but receives `ColdVoxError`

**Symptoms**:
```
error[E0277]: mismatched types
   --> src/my_module.rs:85:19
    |
85 |         AppError::Audio(msg) => handle_audio_error(msg),
    |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
85 |     ColdVoxError::Audio(AudioError::DeviceNotFound { name }) => {
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Solutions**:
1. **Update match patterns**:
   ```rust
   // Replace this
   match error {
       AppError::Audio(msg) => handle_audio_error(msg),
   }

   // With this
   match error {
       ColdVoxError::Audio(AudioError::DeviceNotFound { name }) => {
           handle_device_not_found(name)
       }
   }
   ```

2. **Use wildcard patterns where appropriate**:
   ```rust
   match error {
       AppError::Audio(_) => handle_audio_error(),
       AppError::Stt(_) => handle_stt_error(),
       // ... other variants
   }

   // With this
   match error {
       ColdVoxError::Audio(audio_err) => handle_audio_error(audio_err),
       ColdVoxError::Stt(stt_err) => handle_stt_error(stt_err),
       // ... other variants
   }
   ```

### 2. Test Failures

#### Issue: Tests expecting `AppError` now fail
**Cause**: Test assertions use old error type

**Symptoms**:
```
test my_module::tests::test_error_handling ... FAILED
assert!(matches!(result, Err(AppError::Audio(_))));
```

**Solutions**:
1. **Update test assertions**:
   ```rust
   // Replace this
   assert!(matches!(result, Err(AppError::Audio(_))));

   // With this
   assert!(matches!(result, Err(ColdVoxError::Audio(AudioError::_))));
   ```

2. **Update test helper functions**:
   ```rust
   // Replace this
   fn create_audio_error() -> AppError {
       AppError::Audio("test error".to_string())
   }

   // With this
   fn create_audio_error() -> ColdVoxError {
       ColdVoxError::Audio(AudioError::DeviceNotFound {
           name: Some("test".to_string())
       })
   }
   ```

3. **Update integration tests**:
   ```rust
   // Replace this
   #[test]
   fn test_integration() {
       let result = my_function();
       assert!(result.is_err());
       assert!(matches!(result.unwrap_err(), AppError::Audio(_)));
   }

   // With this
   #[test]
   fn test_integration() {
       let result = my_function();
       assert!(result.is_err());
       assert!(matches!(result.unwrap_err(), ColdVoxError::Audio(AudioError::_))));
   }
   ```

### 3. Runtime Errors

#### Issue: Panic from unwrapped error
**Cause**: Error handling expects `AppError` but receives `ColdVoxError`

**Symptoms**:
```
thread 'main' panicked at 'src/main.rs:123:14:
called `Result::unwrap()` on an `Err` value: ColdVoxError::Audio(AudioError::DeviceNotFound { name: Some("default") })
```

**Solutions**:
1. **Use proper error propagation**:
   ```rust
   // Replace this
   fn process_result(result: Result<T, AppError>) -> T {
       match result {
           Ok(value) => value,
           Err(error) => panic!("Processing failed: {:?}", error),
       }
   }

   // With this
   fn process_result(result: Result<T, ColdVoxError>) -> T {
       match result {
           Ok(value) => value,
           Err(error) => {
               log::error!("Processing failed: {:?}", error);
               // Handle error appropriately instead of panicking
               default_value
           }
       }
   }
   ```

2. **Update error conversion traits**:
   ```rust
   // Replace this
   impl From<MyError> for AppError {
       fn from(err: MyError) -> Self {
           AppError::Audio(err.to_string())
       }
   }

   // With this
   impl From<MyError> for ColdVoxError {
       fn from(err: MyError) -> Self {
           ColdVoxError::Injection(InjectionError::Other(err.to_string()))
       }
   }
   ```

### 4. Documentation Build Errors

#### Issue: Documentation examples don't compile
**Cause**: Documentation code examples still use `AppError`

**Solutions**:
1. **Update documentation examples**:
   ```rust
   // Replace this
   /// # Example
   ///
   /// ```rust
   /// use crate::error::AppError;
   /// fn example() -> Result<(), AppError> {
   ///     // implementation
   /// }
   /// ```

   // With this
   /// # Example
   ///
   /// ```rust
   /// use coldvox_foundation::error::{ColdVoxError, AudioError};
   /// fn example() -> Result<(), ColdVoxError> {
   ///     // implementation
   /// }
   /// ```
   ```

2. **Run documentation tests**:
   ```bash
   cargo test --doc
   ```

### 5. Dependency Issues

#### Issue: Version conflicts with `coldvox-foundation`
**Cause**: Local or dependency versions don't include new error types

**Symptoms**:
```
error[E0432]: unresolved import `coldvox_foundation::error::ColdVoxError`
```

**Solutions**:
1. **Update dependency versions**:
   ```toml
   [dependencies]
   coldvox-foundation = "0.5.0"  # Use version with ColdVoxError
   ```

2. **Clean and rebuild**:
   ```bash
   cargo clean
   cargo update
   cargo build
   ```

## Debugging Techniques

### 1. Enable Detailed Error Logging

Add structured logging to trace error handling:

```rust
use tracing::{error, debug, warn};

match error {
    ColdVoxError::Audio(audio_err) => {
        error!(
            error_type = "audio",
            error = ?audio_err,
            "Audio processing failed: {:?}",
            audio_err
        );
    }
    ColdVoxError::Stt(stt_err) => {
        error!(
            error_type = "stt",
            error = ?stt_err,
            "STT processing failed: {:?}",
            stt_err
        );
    }
}
```

### 2. Use Error Context

Leverage the structured fields in new error types:

```rust
match error {
    ColdVoxError::Config(ConfigError::Validation { field, reason }) => {
        error!(
            field = field,
            reason = reason,
            "Configuration validation failed for field '{}': {}",
            field, reason
        );
    }
}
```

### 3. Test Error Recovery Strategies

Verify recovery mechanisms work as expected:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recovery() {
        // Create an error that should trigger recovery
        let error = ColdVoxError::Audio(AudioError::DeviceDisconnected);

        // Verify recovery strategy is correct
        match error.recovery_strategy() {
            RecoveryStrategy::Retry { .. } => {
                // Verify retry logic
            }
            RecoveryStrategy::Fallback { .. } => {
                // Verify fallback logic
            }
            _ => {}
        }
    }
}
```

## Getting Help

### 1. Check Existing Examples

Look for similar error handling patterns in the codebase:

```bash
# Find examples of ColdVoxError usage
git grep -r "ColdVoxError" --include="*.rs" | head -20
```

### 2. Consult Migration Guide

Reference the comprehensive migration guide:

- [Migration Guide](apperror-to-coldvoxerror-migration-guide.md)
- [Migration Checklist](apperror-to-coldvoxerror-migration-checklist.md)

### 3. Review Test Cases

Check how similar error scenarios are handled in tests:

```bash
# Find test files with error handling
find . -name "*.rs" -exec grep -l "ColdVoxError" {} \; | head -10
```

### 4. Ask for Help

If you encounter issues not covered here:

1. **Check the ColdVox documentation**:
   - [ColdVox Error Reference](../reference/crates/coldvox-foundation.md)
    - [Foundation Design Guide](../domains/foundation/fdn-voice-pipeline-core-design.md)

2. **Reach out to the team**:
   - Create an issue in the ColdVox repository
   - Ask in the relevant development channels
   - Consult with team members who have completed the migration

3. **Review similar migrations**:
   - Check for other migration guides in `docs/playbooks/organization/`
   - Look for similar error handling patterns in the codebase

---

*This guide will be updated as new issues are discovered during the migration process.*
