---
title: "[P2] God method intact: inject() remains monolithic"
labels: ["refactor", "priority:P2", "component:text-injection"]
---

## Problem

The `inject()` method in `StrategyManager` is a monolithic function that handles too many responsibilities: focus checking, app detection, pause checking, allowlist checking, method selection, injection attempts, metrics, logging, and error handling.

## Current Behavior

In `crates/coldvox-text-injection/src/manager.rs`:

```rust
pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
    // 1. Redaction
    // 2. Pause check
    // 3. App detection
    // 4. Allowlist check
    // 5. Focus status check
    // 6. Focus requirement check
    // 7. Method ordering
    // 8. Method iteration
    // 9. Cooldown checking
    // 10. Injection attempt
    // 11. Success/failure handling
    // 12. Metrics recording
    // 13. Error propagation
    // ... 100+ lines
}
```

## Expected Behavior

Break into smaller, focused functions:

```rust
pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
    let context = self.prepare_injection_context(text).await?;
    
    self.validate_injection_context(&context)?;
    
    let methods = self.select_injection_methods(&context);
    
    self.attempt_injection_with_fallback(text, &methods, &context).await
}

async fn prepare_injection_context(&mut self, text: &str) -> Result<InjectionContext, InjectionError> {
    // Focus, app detection, pause check
}

fn validate_injection_context(&self, context: &InjectionContext) -> Result<(), InjectionError> {
    // Allowlist, focus requirements
}

fn select_injection_methods(&mut self, context: &InjectionContext) -> Vec<InjectionMethod> {
    // Method ordering, cooldown filtering
}

async fn attempt_injection_with_fallback(
    &mut self,
    text: &str,
    methods: &[InjectionMethod],
    context: &InjectionContext
) -> Result<(), InjectionError> {
    // Try each method with proper error handling
}
```

## Benefits of Refactoring

1. **Testability**: Each sub-function can be unit tested independently
2. **Readability**: Clear separation of concerns
3. **Maintainability**: Changes are localized to specific functions
4. **Reusability**: Sub-functions can be used elsewhere if needed
5. **Debugging**: Easier to trace which step failed

## Suggested Structure

```rust
struct InjectionContext {
    app_id: String,
    focus_status: FocusStatus,
    is_paused: bool,
    redacted_text: String,
}

// Preparation phase
async fn prepare_injection_context(&mut self, text: &str) -> Result<InjectionContext, InjectionError>;

// Validation phase
fn validate_injection_context(&self, ctx: &InjectionContext) -> Result<(), InjectionError>;
fn check_allowlist(&self, app_id: &str) -> Result<(), InjectionError>;
fn check_focus_requirements(&self, ctx: &InjectionContext) -> Result<(), InjectionError>;

// Method selection phase
fn select_injection_methods(&mut self, ctx: &InjectionContext) -> Vec<InjectionMethod>;
fn filter_methods_by_cooldown(&self, app_id: &str, methods: Vec<InjectionMethod>) -> Vec<InjectionMethod>;

// Execution phase
async fn attempt_injection_with_fallback(
    &mut self,
    text: &str,
    methods: &[InjectionMethod],
    ctx: &InjectionContext,
) -> Result<(), InjectionError>;

async fn attempt_single_injection(
    &mut self,
    text: &str,
    method: InjectionMethod,
    ctx: &InjectionContext,
) -> Result<(), InjectionError>;

// Post-processing
fn record_injection_result(&mut self, app_id: &str, method: InjectionMethod, success: bool);
```

## Impact

- **Code Quality**: More maintainable and testable
- **Onboarding**: Easier for new contributors to understand
- **Bugs**: Easier to isolate and fix issues
- **Performance**: No performance impact (compiler inlines)

## Location

- File: `crates/coldvox-text-injection/src/manager.rs`
- Function: `inject()`

## Migration Strategy

1. Extract smallest functions first (e.g., validation helpers)
2. Test each extraction to ensure behavior unchanged
3. Gradually refactor larger pieces
4. Keep the main `inject()` signature stable for API compatibility
5. Update tests to cover new sub-functions
