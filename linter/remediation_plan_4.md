# Linter Remediation Plan (Batch 4 - coldvox-text-injection)

This document outlines the plan to fix linter warnings within the `coldvox-text-injection` crate.

## Unused Imports 

-   **Files:**
    -   `crates/coldvox-text-injection/src/mki_injector.rs`
    -   `crates/coldvox-text-injection/src/ydotool_injector.rs`
    -   `crates/coldvox-text-injection/src/noop_injector.rs`
-   **Warning:** `unused import`
-   **Fix:** Remove the unused `use` statements for `std::time::Duration`, `info`, `InjectionMethod`, and `error`.

## Unused Variables and Fields 

-   **Files:**
    -   `crates/coldvox-text-injection/src/ydotool_injector.rs`
    -   `crates/coldvox-text-injection/src/noop_injector.rs`
    -   `crates/coldvox-text-injection/src/backend.rs`
    -   `crates/coldvox-text-injection/src/focus.rs`
-   **Warning:** `unused variable`, `field is never read`
-   **Fix:** Prefix the unused variables (`duration`) and fields (`config`) with an underscore to mark them as intentionally unused (e.g., `_duration`, `_config`).

## Unused Methods 

-   **Files:**
    -   `crates/coldvox-text-injection/src/manager.rs`
    -   `crates/coldvox-text-injection/src/ydotool_injector.rs`
-   **Warning:** `method is never used`
-   **Fix:** Prefix the unused methods (`get_method_priority`, `type_text`) with an underscore to mark them as intentionally unused (e.g., `_get_method_priority`).

## Code Style and Idiomatic Rust 

-   **File:** `crates/coldvox-text-injection/src/types.rs`
    -   **Warning:** `empty line after outer attribute`
    -   **Fix:** Remove the extra blank line after the doc comment.
-   **File:** `crates/coldvox-text-injection/src/backend.rs`
    -   **Warning:** `manual implementation of `Iterator::find``
    -   **Fix:** Replace the manual `for` loop with the suggested `find()` iterator method for a more idiomatic solution.
-   **File:** `crates/coldvox-text-injection/src/manager.rs`
    -   **Warning:** `the borrowed expression implements the required traits`
    -   **Fix:** Remove the unnecessary `&` where the compiler indicates a needless borrow.
-   **File:** `crates/coldvox-text-injection/src/manager.rs`
    -   **Warning:** `redundant redefinition of a binding`
    -   **Fix:** Remove the redundant `let app_id = app_id;` statement.
-   **File:** `crates/coldvox-text-injection/src/ydotool_injector.rs`
    -   **Warning:** `redundant closure`
    -   **Fix:** Simplify the code by replacing the closure with a direct function reference as suggested.
-   **File:** `crates/coldvox-text-injection/src/session.rs`
    -   **Warning:** `this `impl` can be derived`
    -   **Fix:** Replace the manual `impl Default for SessionState` with `#[derive(Default)]` on the enum.

## Concurrency 

-   **File:** `crates/coldvox-text-injection/src/processor.rs`
    -   **Warning:** `this `MutexGuard` is held across an await point`
    -   **Analysis:** Holding a standard library mutex guard across an `.await` can lead to deadlocks.
    -   **Fix:** Use an async-aware mutex (like `tokio::sync::Mutex`) or ensure the mutex guard is dropped before the `await` call.
