//! Common helper functions for STT plugins to reduce boilerplate.

use crate::plugin::SttPluginError;

/// A common implementation for the `reset` method for simple plugins.
pub async fn simple_reset() -> Result<(), SttPluginError> {
    Ok(())
}

/// A common implementation for the `finalize` method for plugins that do not
/// perform any special processing at the end of an utterance.
pub async fn noop_finalize() -> Result<Option<crate::types::TranscriptionEvent>, SttPluginError> {
    Ok(None)
}

/// A common implementation for `is_available` for plugins that are always available.
pub async fn always_available() -> Result<bool, SttPluginError> {
    Ok(true)
}
