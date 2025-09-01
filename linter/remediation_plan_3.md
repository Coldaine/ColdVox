# Linter Remediation Plan (Batch 3)

This document outlines the plan to fix the third batch of linter warnings.

## Error Remediation Details

### File: `.clippy.toml`

1.  **Warning:** `expected a function, found a macro`
    -   **Location:** Affects `std::println` and `std::eprintln` entries.
    -   **Analysis:** The linter is flagging these because they are macros, not functions. This can be suppressed.
    -   **Fix:** Add `allow-invalid = true` to the configuration for both `std::println` and `std::eprintln`.

### File: `crates/coldvox-telemetry/src/pipeline_metrics.rs`

2.  **Warning:** `you should consider adding a `Default` implementation for `FpsTracker``
    -   **Analysis:** The struct `FpsTracker` has a `new` function but no `Default` implementation, which is a common and useful trait to have.
    -   **Fix:** Implement the `Default` trait for `FpsTracker` as suggested by Clippy, by calling `Self::new()` within the `default()` function.

### File: `crates/coldvox-stt/src/processor.rs`

3.  **Warning:** `this expression creates a reference which is immediately dereferenced by the compiler`
    -   **Line:** `match self.transcriber.accept_frame(&audio_buffer)`
    -   **Analysis:** A needless borrow is being created and immediately dereferenced.
    -   **Fix:** Remove the unnecessary `&`, changing the call to `self.transcriber.accept_frame(audio_buffer)`.

