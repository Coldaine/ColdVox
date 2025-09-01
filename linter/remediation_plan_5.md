# Linter Remediation Plan (Batch 5 - coldvox-audio & coldvox-app)

This document outlines the plan to fix the final batch of linter warnings in the `coldvox-audio` and `coldvox-app` crates.

## `coldvox-audio` Crate

-   **File:** `crates/coldvox-audio/src/chunker.rs`
    -   **Warning:** `unused variable: `timestamp_ms``
    -   **Fix:** Prefix the variable with an underscore (`_timestamp_ms`) to indicate it is intentionally unused.

-   **File:** `crates/coldvox-audio/src/capture.rs`
    -   **Warning:** `initializer for `thread_local` value can be made `const``
    -   **Fix:** Wrap the initializer for the `thread_local` static in a `const` block as suggested by the linter.

## `coldvox-app` Crate

-   **Files:**
    -   `crates/app/src/text_injection/mod.rs`
    -   `crates/app/src/text_injection/focus.rs`
    -   `crates/app/src/text_injection/manager.rs`
    -   **Warning:** `unexpected `cfg` condition value`
    -   **Analysis:** The code uses numerous `#[cfg(feature = ...)]` attributes for features that are not defined in the `crates/app/Cargo.toml` file. This means the conditionally compiled code is never included.
    -   **Fix:** Add the following features to the `[features]` section of `crates/app/Cargo.toml`:
        -   `text-injection-atspi`
        -   `text-injection-clipboard`
        -   `text-injection-ydotool`
        -   `text-injection-enigo`
        -   `text-injection-mki`
        -   `text-injection-kdotool`
        -   `text-injection-regex`

-   **File:** `crates/app/src/text_injection/manager.rs`
    -   **Warning:** `unused variable: `has_wayland`` and `has_x11``
    -   **Fix:** Prefix the variables with an underscore (`_has_wayland`, `_has_x11`) to mark them as intentionally unused.

-   **File:** `crates/app/src/text_injection/types.rs`
    -   **Warning:** `empty line after outer attribute`
    -   **Fix:** Remove the extra blank line after the doc comment for better code formatting.
