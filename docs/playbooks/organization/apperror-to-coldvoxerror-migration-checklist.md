---
doc_type: playbook
subsystem: foundation
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2025-11-01
owners: Documentation Working Group
version: 1.0.0
---

# AppError to ColdVoxError Migration Checklist

This checklist helps developers systematically migrate from `AppError` to `ColdVoxError` across their codebase. Use this to track progress and ensure all necessary changes are completed.

## Pre-Migration Preparation

### Code Analysis
- [ ] Search for all `AppError` references in your codebase
- [ ] Identify all public APIs that return `AppError`
- [ ] List all custom error variants specific to your code
- [ ] Document current error handling patterns
- [ ] Create backup of current code state

### Dependency Check
- [ ] Verify `coldvox-foundation` dependency is up to date
- [ ] Check if any crates need version updates
- [ ] Identify any transitive dependencies that might be affected

## Migration Tasks

### Import Updates
- [ ] Replace `use crate::error::AppError` with `use coldvox_foundation::error::{ColdVoxError, ...}`
- [ ] Add domain-specific error types: `AudioError`, `SttError`, `VadError`, `InjectionError`, `ConfigError`, `PluginError`
- [ ] Update all `use` statements in affected files
- [ ] Verify no remaining `AppError` imports exist

### Type Replacements
- [ ] Replace `AppError` return types with `ColdVoxError` in function signatures
- [ ] Replace `AppError::Variant` with appropriate domain-specific errors
- [ ] Update generic type parameters from `AppError` to `ColdVoxError`
- [ ] Replace `Result<T>` alias if it points to `AppError`

### Error Handling Updates
- [ ] Update match statements to handle domain-specific error variants
- [ ] Replace generic error handling with structured error patterns
- [ ] Add recovery strategy handling where appropriate
- [ ] Update error messages to use structured error fields

### Conversion Traits
- [ ] Update `From<AppError>` implementations to `From<ColdVoxError>`
- [ ] Add new `From` implementations for domain-specific errors
- [ ] Verify all error conversions work correctly
- [ ] Test error conversion paths

### Test Updates
- [ ] Update unit tests to expect `ColdVoxError` variants
- [ ] Update integration tests to use new error types
- [ ] Add tests for new error handling capabilities
- [ ] Verify all tests pass with new error handling

### Documentation Updates
- [ ] Update function documentation to reflect new error types
- [ ] Update examples in documentation
- [ ] Add migration notes to API documentation
- [ ] Update any README files with error examples

## Post-Migration Verification

### Compilation
- [ ] Verify all code compiles without errors
- [ ] Check for any remaining `AppError` references
- [ ] Ensure no deprecated warnings related to error types

### Testing
- [ ] Run full test suite
- [ ] Verify all error handling paths work correctly
- [ ] Test error recovery strategies
- [ ] Check error logging output for proper formatting

### Code Review
- [ ] Review all changed files for correctness
- [ ] Verify error handling follows best practices
- [ ] Check for any potential error handling gaps

### Integration
- [ ] Test with dependent crates
- [ ] Verify public API compatibility
- [ ] Check for any runtime errors in production scenarios

## Validation Criteria

### Functional Requirements
- [ ] All `AppError` references replaced with `ColdVoxError`
- [ ] Error handling maintains existing functionality
- [ ] Recovery strategies work as expected
- [ ] Error messages are informative and actionable

### Quality Requirements
- [ ] Code follows Rust error handling best practices
- [ ] Error types provide appropriate context
- [ ] No loss of error information during migration
- [ ] Consistent error handling patterns across codebase

### Performance Requirements
- [ ] No performance regression from error handling changes
- [ ] Error creation overhead is minimal
- [ ] Recovery strategies don't impact performance significantly

## Common Issues and Solutions

### Compilation Errors
**Issue**: `error[E0433]: expected type, found struct AppError`
**Solution**: Check for remaining `AppError` references in imports and type annotations

**Issue**: `error[E0277]`: mismatched types` in error handling
**Solution**: Ensure all match arms handle `ColdVoxError` variants correctly

### Runtime Errors
**Issue**: Panic from unwrapped error that doesn't match expected variant
**Solution**: Update error handling to cover all error cases

**Issue**: Error recovery not working as expected
**Solution**: Verify recovery strategy implementation and error type mapping

### Test Failures
**Issue**: Tests expecting `AppError` now fail
**Solution**: Update test assertions to expect `ColdVoxError` variants

**Issue**: Integration tests failing with type mismatches
**Solution**: Update integration test code to use new error types

## Notes

- Each completed item should be dated and signed off by the developer
- Run `cargo check` after each major change group to catch issues early
- Use `git grep` to verify complete removal of `AppError` references: `git grep -r "AppError" --exclude-dir=target`
- Test error recovery strategies in isolation before full integration
- Consider adding temporary logging to verify error handling behavior during migration

---

*This checklist should be used in conjunction with the [Migration Guide](apperror-to-coldvoxerror-migration-guide.md) for complete migration coverage.*
