//! # ColdVox Text Injection Library
//!
//! This crate provides text injection capabilities for the ColdVox speech-to-text system.
//! It supports multiple backends for text injection across different platforms and environments.
//!
//! ## Backend Support Matrix
//!
//! | Backend      | Platform | Features           | Status |
//! |--------------|----------|--------------------|--------|
//! | AT-SPI       | Linux    | Accessibility API  | Stable |
//! | Clipboard Paste | Linux | wl-clipboard-rs + fallbacks | Stable |
//! | Enigo        | Cross    | Input simulation   | Beta   |
//! | KDotool      | Linux    | X11 automation     | Beta   |
//! | YDotool      | Linux    | uinput automation  | Beta   |
//!
//! ## Features
//!
//! - `atspi`: Linux AT-SPI accessibility backend
//! - `wl_clipboard`: Clipboard-based injection via wl-clipboard-rs
//! - `enigo`: Cross-platform input simulation
//! - `ydotool`: Linux uinput automation fallback for paste
//! - `kdotool`: KDE/X11 window activation assistance
//!
//! - `regex`: Precompile allow/block list patterns
//! - `all-backends`: Enable all available backends
//! - `linux-desktop`: Enable recommended Linux desktop backends

pub mod backend;
pub mod compat;
pub mod detection;
pub mod focus;
pub mod log_throttle;
pub mod logging;
pub mod manager;
pub mod processor;
pub mod session;
pub mod types;

// NOTE: window_manager intentionally violates the "no-sprawl" principle.
// Platform-specific fallbacks for app_id detection require this complexity budget.
// This is the ONLY allowed exception to keep lean architecture elsewhere.
pub mod window_manager;

// AT-SPI event confirmation module
pub mod confirm;

// Pre-warming module for injection components
pub mod prewarm;

// New modular injector organization
pub mod injectors;
pub mod orchestrator;

// Re-export orchestrator types and injector module
#[allow(deprecated)]
pub use injectors::{
    ClipboardBackup, ClipboardContext, ClipboardInjectionMode, ClipboardInjector,
    UnifiedClipboardInjector,
};
pub use orchestrator::{AtspiContext, DesktopEnvironment, StrategyOrchestrator};

// Re-export modular AT-SPI injector for backward compatibility
pub use injectors::atspi::AtspiInjector;

#[cfg(feature = "enigo")]
pub mod enigo_injector;

#[cfg(feature = "kdotool")]
pub mod kdotool_injector;

// Ydotool is Linux-only; provide real module on Unix and a stub elsewhere
#[cfg(all(unix, feature = "ydotool"))]
pub mod ydotool_injector;

#[cfg(any(not(unix), not(feature = "ydotool")))]
pub mod ydotool_injector {
    //! Windows/Non-Unix stub for ydotool injector to keep builds green on unsupported platforms.
    use crate::types::{InjectionConfig, InjectionResult};
    use crate::TextInjector;
    use async_trait::async_trait;

    /// Stub indicating ydotool is unavailable on this platform
    pub struct YdotoolInjector {
        #[allow(dead_code)]
        pub(crate) config: InjectionConfig,
    }

    impl YdotoolInjector {
        pub fn new(config: InjectionConfig) -> Self {
            Self { config }
        }

        #[allow(dead_code)]
        pub(crate) fn ydotool_runtime_available() -> bool {
            false
        }

        #[allow(dead_code)]
        pub(crate) fn apply_socket_env(_command: &mut tokio::process::Command) {}
    }

    #[async_trait]
    impl TextInjector for YdotoolInjector {
        async fn inject_text(
            &self,
            _text: &str,
            _context: Option<&crate::types::InjectionContext>,
        ) -> InjectionResult<()> {
            Err(crate::InjectionError::MethodUnavailable(
                "ydotool is not available on this platform".to_string(),
            ))
        }

        async fn is_available(&self) -> bool {
            false
        }

        fn backend_name(&self) -> &'static str {
            "ydotool"
        }

        fn backend_info(&self) -> Vec<(&'static str, String)> {
            vec![("platform", std::env::consts::OS.to_string())]
        }
    }
}

// NoOp fallback is always available
pub mod noop_injector;

// Re-export key components for easy access
pub use backend::Backend;
pub use coldvox_foundation::error::InjectionError;
pub use focus::{FocusProvider, FocusStatus};
pub use manager::StrategyManager;
pub use processor::{AsyncInjectionProcessor, InjectionProcessor, ProcessorMetrics};
pub use session::{InjectionSession, SessionConfig, SessionState};
pub use types::{
    InjectionConfig, InjectionContext, InjectionMethod, InjectionMode, InjectionResult,
};

/// Trait defining the core text injection interface
#[async_trait::async_trait]
pub trait TextInjector: Send + Sync {
    /// Inject text with optional context (pre-warmed data, focus info, mode override)
    ///
    /// # Arguments
    /// * `text` - The text to inject
    /// * `context` - Optional injection context with pre-warmed data and overrides
    async fn inject_text(
        &self,
        text: &str,
        context: Option<&InjectionContext>,
    ) -> InjectionResult<()>;

    /// Check if the injector is available and functional
    async fn is_available(&self) -> bool;

    /// Get the backend name for this injector
    fn backend_name(&self) -> &'static str;

    /// Get backend-specific configuration information
    fn backend_info(&self) -> Vec<(&'static str, String)>;
}

// Re-export confirmation module components
pub use confirm::{
    create_confirmation_context, text_changed, ConfirmationContext, ConfirmationResult,
    TextChangeListener,
};

// Test modules
#[cfg(test)]
mod tests;
