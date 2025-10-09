use crate::plugin::SttPluginError;
use crate::types::TranscriptionEvent;

/// Creates a NotAvailable error for unimplemented plugins.
#[allow(dead_code)]
pub(super) fn not_yet_available(id: &str) -> SttPluginError {
    SttPluginError::NotAvailable {
        reason: format!("{} not yet implemented", id),
    }
}

/// Common availability check for unavailable plugins.
#[allow(dead_code)]
pub(super) async fn unavailable_check() -> Result<bool, SttPluginError> {
    Ok(false)
}

/// Common no-op reset implementation.
#[allow(dead_code)]
pub(super) async fn noop_reset() -> Result<(), SttPluginError> {
    Ok(())
}

/// Common no-op finalize implementation.
#[allow(dead_code)]
pub(super) async fn noop_finalize() -> Result<Option<TranscriptionEvent>, SttPluginError> {
    Ok(None)
}