//! # ColdVox Text Injection Library
//!
//! This crate provides robust, resilient, and fast-failing text injection
//! capabilities for the ColdVox speech-to-text system. It is designed to
//! degrade gracefully in environments where desktop components are missing
//! and to provide clear diagnostics.
//!
//! ## Core Concepts
//!
//! - **Environment Probe**: Before any injection, a fast, async probe checks
//!   for required components (D-Bus, AT-SPI, clipboard tools) with strict
//!   timeouts.
//! - **Structured Outcomes**: Injection attempts return `InjectionOutcome` on
//!   success or a detailed `InjectionError` on failure, preventing hangs and
//!   making failures easy to diagnose.
//! - **Strict Timeouts**: Every potentially blocking operation, from a single
//!   D-Bus call to a full injection attempt, is wrapped in a timeout to
//!   prevent cascading failures and test hangs.

// New modules for the refactored design.
pub mod constants;
pub mod error;
pub mod metrics;
pub mod outcome;
pub mod probe;
pub mod subprocess;

// Existing modules that will be refactored.
pub mod backend;
pub mod focus;
pub mod manager;
pub mod processor;
pub mod session;
pub mod types;
pub mod window_manager;

// Individual injector modules with feature gates
#[cfg(feature = "atspi")]
pub mod atspi_injector;

#[cfg(feature = "wl_clipboard")]
pub mod clipboard_injector;

#[cfg(all(feature = "wl_clipboard", feature = "ydotool"))]
pub mod combo_clip_ydotool;

#[cfg(feature = "enigo")]
pub mod enigo_injector;

#[cfg(feature = "kdotool")]
pub mod kdotool_injector;

pub mod ydotool_injector;

// NoOp fallback is always available
pub mod noop_injector;

#[cfg(test)]
mod tests;

// Re-export key components for easy access by consumers of the crate.
pub use async_trait::async_trait;
pub use error::{InjectionError, UnavailableCause};
pub use manager::StrategyManager;
pub use metrics::{InjectionMetrics, MetricsSink};
pub use outcome::InjectionOutcome;
pub use probe::{probe_environment, BackendId, ProbeState};
pub use types::{InjectionConfig, InjectionMethod};

/// # TextInjector Trait
///
/// This trait defines the core interface for a text injection backend.
/// Each backend (AT-SPI, Clipboard, etc.) implements this trait.
#[async_trait]
pub trait TextInjector: Send + Sync {
    /// Returns the specific backend ID for this injector.
    fn backend_id(&self) -> BackendId;

    /// Performs a quick, non-blocking check to see if the backend is likely
    /// to be available. This is a preliminary check before the main probe.
    async fn is_available(&self) -> bool;

    /// Injects the given text into the active application.
    ///
    /// This is the primary method for a backend. It should perform the injection
    /// and return a structured outcome or a detailed error. Implementations
    /// must be mindful of timeouts and resource cleanup.
    ///
    /// ## Arguments
    ///
    /// * `text` - The text string to inject.
    ///
    /// ## Returns
    ///
    /// * `Ok(InjectionOutcome)` - On success, provides details about the
    ///   operation, including latency.
    /// * `Err(InjectionError)` - On failure, provides a structured error
    ///   indicating the cause (e.g., timeout, unavailable, precondition).
    async fn inject_text(&self, text: &str) -> Result<InjectionOutcome, InjectionError>;
}
