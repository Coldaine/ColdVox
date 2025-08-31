# Linter Remediation Plan (Batch 2)

This document outlines the plan to fix the second batch of linter errors.

## Error Remediation Details

### File: `crates/app/src/probes/vad_mic.rs`

1.  **Error:** `unresolved import `crate::audio::ring_buffer``
    -   **Line:** `use crate::audio::ring_buffer::AudioRingBuffer;
`    -   **Fix:** `use coldvox_audio::ring_buffer::AudioRingBuffer;
`
2.  **Error:** `could not find `chunker` in `audio``
    -   **Line:** `resampler_quality: crate::audio::chunker::ResamplerQuality::Balanced,
`    -   **Fix:** `resampler_quality: coldvox_audio::chunker::ResamplerQuality::Balanced,
`

### File: `crates/app/src/audio/vad_adapter.rs`

3.  **Error:** `use of undeclared type `Level3Vad``
    -   **Line:** `Box::new(Level3Vad::new(level3_config))
`    -   **Analysis:** The import for `Level3Vad` is incorrect. It should point to the `level3` module within the `coldvox-vad` crate.
    -   **Fix:** Change the import from `use coldvox_vad::Level3Vad;` to `use coldvox_vad::level3::Level3Vad;`.

4.  **Error:** `mismatched types`
    -   **Line:** `Box::new(SileroEngine::new(config.silero.clone())?)
`    -   **Analysis:** The `SileroEngine::new` function expects a `coldvox_vad_silero::SileroConfig`, but it is receiving a `coldvox_vad::config::SileroConfig`. A conversion is needed.
    -   **Fix:** Manually construct a `coldvox_vad_silero::SileroConfig` from the fields of `config.silero`. This assumes the field names are the same. A more robust solution might involve a `From` trait implementation.

### Outdated Error

-   **Error:** `error[E0599]: the function or associated item 'new' exists for struct 'std::boxed::Box<(dyn coldvox_vad::VadEngine + 'static)>', but its trait bounds were not satisfied`
    -   **Analysis:** The code pointed to by this error message does not seem to exist in the current version of the file. It's likely this error is from a previous compilation and has since been resolved. I will ignore it for now.
