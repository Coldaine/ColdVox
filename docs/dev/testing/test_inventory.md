# Test Inventory

This document provides a comprehensive inventory of the tests in the ColdVox project, categorized by type and crate. Each test is rated based on its scope, value, speed, and dependencies.

## Test Ratings

Tests are rated on the following criteria:

*   **Scope**: The breadth of the test's coverage (e.g., a single function, a module, a full pipeline).
*   **Value**: The importance of the test in ensuring the project's correctness and stability.
*   **Speed**: The execution time of the test.
*   **Dependencies**: The external dependencies of the test (e.g., hardware, network, specific OS features).

## `crates/app`

### Unit Tests

| Test File | Scope | Value | Speed | Dependencies |
| --- | --- | --- | --- | --- |
| `silence_detector_test.rs` | Function | High | Fast | None |
| `watchdog_test.rs` | Struct | Medium | Fast | None |

### Integration Tests

| Test File | Scope | Value | Speed | Dependencies |
| --- | --- | --- | --- | --- |
| `pipeline_integration.rs` | High | High | Medium | Whisper Model |
| `text_injection_integration_test.rs` | Medium | High | Medium | GUI |
| `capture_integration_test.rs` | Medium | High | Medium | Audio HW |
| `mock_injection_tests.rs` | Medium | Medium | Fast | None |
| `settings_test.rs` | Low | Medium | Fast | None |
| `verify_mock_injection_fix.rs` | Low | Low | Fast | None |
| `chunker_timing_tests.rs` | Low | Medium | Fast | None |

### Hardware Tests

| Test File | Scope | Value | Speed | Dependencies |
| --- | --- | --- | --- | --- |
| `hardware_check.rs` | High | High | Slow | Audio HW, GUI |

### Golden Master Tests

| Test File | Scope | Value | Speed | Dependencies |
| --- | --- | --- | --- | --- |
| `golden_master.rs` | High | High | Slow | Whisper Model |

## `crates/coldvox-audio`

### Integration Tests

| Test File | Scope | Value | Speed | Dependencies |
| --- | --- | --- | --- | --- |
| `default_mic_detection_it.rs` | Low | Medium | Fast | Audio HW |

### Hardware Tests

| Test File | Scope | Value | Speed | Dependencies |
| --- | --- | --- | --- | --- |
| `device_hotplug_tests.rs` | Medium | High | Slow | Audio HW |

## `crates/coldvox-stt`

### Integration Tests

| Test File | Scope | Value | Speed | Dependencies |
| --- | --- | --- | --- | --- |
| `moonshine_e2e.rs` | High | High | Slow | Python, ML Models |

## `crates/coldvox-text-injection`

### Unit Tests

| Test File | Scope | Value | Speed | Dependencies |
| --- | --- | --- | --- | --- |
| `test_async_processor.rs` | Struct | Medium | Fast | None |
| `test_focus_enforcement.rs` | Struct | Medium | Fast | None |
| `test_focus_tracking.rs` | Struct | Medium | Fast | None |
| `test_permission_checking.rs` | Struct | Medium | Fast | None |
| `test_regex_metrics.rs` | Struct | Medium | Fast | None |
| `test_window_manager.rs` | Struct | Medium | Fast | None |
| `wl_copy_stdin_test.rs` | Function | Low | Fast | `wl-copy` |
| `wl_copy_basic_test.rs` | Function | Low | Fast | `wl-copy` |
| `wl_copy_simple_test.rs` | Function | Low | Fast | `wl-copy` |

### Integration Tests

| Test File | Scope | Value | Speed | Dependencies |
| --- | --- | --- | --- | --- |
| `real_injection.rs` | High | High | Slow | GUI |
| `real_injection_smoke.rs` | Medium | High | Medium | GUI |
| `test_adaptive_strategy.rs` | Struct | Medium | Fast | None |
| `test_allow_block.rs` | Struct | Medium | Fast | None |
| `test_integration.rs` | High | High | Slow | GUI |
| `test_mock_injectors.rs` | Struct | Medium | Fast | None |
