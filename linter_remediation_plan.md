# Linter Remediation Plan (First 10 Errors)

This document outlines the plan to fix the first 10 linter errors found by `cargo clippy`. All errors are unresolved imports within the `coldvox-app` crate.

The root cause is that modules within the `coldvox-app` library are incorrectly referencing other workspace crates (e.g., `coldvox-telemetry`, `coldvox-vad`, `coldvox-audio`) using `crate::` paths instead of the proper crate names.

The remediation is to replace the incorrect `use` statements with the correct ones that use the external crate names.

## Error Remediation Details

### File: `crates/app/src/probes/vad_mic.rs`

1.  **Error:** `failed to resolve: could not find 'telemetry' in the crate root`
    -   **Line:** `use crate::telemetry::pipeline_metrics::PipelineMetrics;`
    -   **Fix:** `use coldvox_telemetry::pipeline_metrics::PipelineMetrics;`

2.  **Error:** `failed to resolve: could not find 'vad' in the crate root`
    -   **Line:** `use crate::vad::types::VadEvent;`
    -   **Fix:** `use coldvox_vad::types::VadEvent;`

3.  **Error:** `failed to resolve: could not find 'vad' in the crate root`
    -   **Line:** `use crate::vad::config::{UnifiedVadConfig, VadMode};
    -   **Fix:** `use coldvox_vad::config::{UnifiedVadConfig, VadMode};

4.  **Error:** `failed to resolve: unresolved import`
    -   **Line:** `use crate::foundation::error::AudioConfig;
    -   **Fix:** `use coldvox_foundation::error::AudioConfig;

5.  **Error:** `unresolved import 
`crate::audio::capture
`
    -   **Line:** `use crate::audio::capture::AudioCaptureThread;
    -   **Fix:** `use coldvox_audio::capture::AudioCaptureThread;

6.  **Error:** `unresolved import 
`crate::audio::chunker
`
    -   **Line:** `use crate::audio::chunker::{AudioChunker, ChunkerConfig};
    -   **Fix:** `use coldvox_audio::chunker::{AudioChunker, ChunkerConfig};

7.  **Error:** `unresolved import 
`crate::audio::frame_reader
`
    -   **Line:** `use crate::audio::frame_reader::FrameReader;
    -   **Fix:** `use coldvox_audio::frame_reader::FrameReader;

### File: `crates/app/src/probes/text_injection.rs`

8.  **Error:** `failed to resolve: could not find 'telemetry' in the crate root`
    -   **Line:** `use crate::telemetry::pipeline_metrics::PipelineMetrics;
    -   **Fix:** `use coldvox_telemetry::pipeline_metrics::PipelineMetrics;

### File: `crates/app/src/text_injection/processor.rs`

9.  **Error:** `failed to resolve: could not find 'telemetry' in the crate root`
    -   **Line:** `use crate::telemetry::pipeline_metrics::PipelineMetrics;
    -   **Fix:** `use coldvox_telemetry::pipeline_metrics::PipelineMetrics;

### File: `crates/app/src/hotkey/listener.rs`

10. **Error:** `failed to resolve: could not find 'vad' in the crate root`
    -   **Line:** `use crate::vad::types::VadEvent;
    -   **Fix:** `use coldvox_vad::types::VadEvent;
