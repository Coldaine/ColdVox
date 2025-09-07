//! # Error types for the text injection crate.
//!
//! This module defines the structured error types that are returned by the
//! injection process. This allows for precise error handling and diagnostics.

use crate::probe::BackendId;
use serde::Serialize;
use thiserror::Error;

/// The primary error type for text injection operations.
#[derive(Debug, Error, Serialize)]
pub enum InjectionError {
    /// A required backend is definitively unavailable due to the environment.
    #[error("Backend {backend:?} is unavailable: {cause}")]
    Unavailable {
        backend: BackendId,
        cause: UnavailableCause,
    },

    /// An operation timed out.
    #[error("Operation for backend {backend:?} timed out during phase '{phase}' after {elapsed_ms}ms")]
    Timeout {
        backend: BackendId,
        phase: &'static str,
        elapsed_ms: u32,
    },

    /// A precondition for the injection was not met.
    #[error("A precondition was not met: {reason}")]
    PreconditionNotMet { reason: &'static str },

    /// A transient error occurred that might be resolved by a retry.
    #[error("A transient error occurred: {reason} (retryable: {retryable})")]
    Transient {
        reason: &'static str,
        retryable: bool,
    },

    /// The clipboard content could not be restored to its original state.
    /// This is often treated as a warning rather than a hard failure.
    #[error("Failed to restore clipboard: {details}")]
    ClipboardRestoreMismatch { details: String },

    /// An underlying I/O error occurred.
    #[error("I/O error for backend {backend:?}: {msg}")]
    Io { backend: BackendId, msg: String },

    /// A catch-all for other types of errors.
    #[error("An unexpected error occurred: {0}")]
    Other(String),
}

/// Reasons why a backend might be unavailable.
#[derive(Debug, Error, Serialize)]
pub enum UnavailableCause {
    /// The environment is missing required components (e.g., D-Bus, display server).
    #[error("Environment is not configured correctly: {causes:?}")]
    Environment { causes: Vec<String> },

    /// D-Bus connection failed or is not available.
    #[error("D-Bus is not available")]
    Dbus,

    /// The AT-SPI registry is not running or not responsive.
    #[error("AT-SPI registry is not available")]
    AtspiRegistry,

    /// A required clipboard utility (e.g., `wl-paste`, `xclip`) is not installed or not working.
    #[error("A required clipboard tool is missing or non-responsive")]
    ClipboardTool,

    /// All available injection methods were tried and failed.
    #[error("All available backends were attempted and failed")]
    Exhausted,
}

/// Errors that can occur during clipboard operations.
#[derive(Debug, Error)]
pub enum ClipboardError {
    /// Failed to launch the clipboard utility.
    #[error("Failed to launch clipboard tool: {0}")]
    Launch(String),

    /// The clipboard operation timed out.
    #[error("Clipboard operation timed out")]
    Timeout,

    /// The content from the clipboard was not valid UTF-8.
    #[error("Clipboard content is not valid UTF-8")]
    InvalidUtf8,

    /// An I/O error occurred while interacting with the tool.
    #[error("I/O error during clipboard operation: {0}")]
    Io(#[from] std::io::Error),
}
