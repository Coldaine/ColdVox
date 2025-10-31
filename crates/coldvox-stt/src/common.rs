use crate::types::TranscriptionEvent;
use coldvox_foundation::error::{ColdVoxError, SttError};

/// Creates a NotAvailable error for unimplemented plugins.
#[allow(dead_code)]
pub(super) fn not_yet_available(id: &str) -> ColdVoxError {
    SttError::NotAvailable {
        plugin: id.to_string(),
        reason: format!("{} not yet implemented", id),
    }
    .into()
}

/// Common availability check for unavailable plugins.
#[allow(dead_code)]
pub(super) async fn unavailable_check() -> Result<bool, ColdVoxError> {
    Ok(false)
}

/// Common no-op reset implementation.
#[allow(dead_code)]
pub(super) async fn noop_reset() -> Result<(), ColdVoxError> {
    Ok(())
}

/// Common no-op finalize implementation.
#[allow(dead_code)]
pub(super) async fn noop_finalize() -> Result<Option<TranscriptionEvent>, ColdVoxError> {
    Ok(None)
}
