/// STT test utilities and end-to-end tests
///
/// This module provides utilities for testing speech-to-text functionality,
/// including WER calculation, timeout handling, and integration tests.
///
/// Note: More comprehensive versions of these utilities exist in `crates/app/tests/common/`
/// for integration tests. These simpler versions are kept for unit test convenience.
#[allow(dead_code)]
pub mod timeout_utils;
#[allow(dead_code)]
pub mod wer_utils;
