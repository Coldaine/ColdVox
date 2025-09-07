//! # Success types for the text injection crate.
//!
//! This module defines the structured success types that are returned by the
//! injection process, providing details about the operation.

use crate::probe::BackendId;
use serde::Serialize;

/// Represents the successful outcome of a text injection operation.
#[derive(Debug, Clone, Serialize)]
pub struct InjectionOutcome {
    /// The backend that successfully performed the injection.
    pub backend: BackendId,
    /// The total latency of the injection operation in milliseconds.
    pub latency_ms: u32,
    /// A flag indicating if the operation was degraded (e.g., required a retry).
    pub degraded: bool,
}
