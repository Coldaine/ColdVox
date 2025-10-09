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
pub use orchestrator::{StrategyOrchestrator, DesktopEnvironment, AtspiContext};
pub use injectors::{ClipboardBackup, ClipboardInjector, ClipboardContext};

// Re-export modular AT-SPI injector for backward compatibility
pub use injectors::atspi::AtspiInjector;

#[cfg(feature = "wl_clipboard")]
pub mod clipboard_paste_injector;

#[cfg(feature = "enigo")]
pub mod enigo_injector;

#[cfg(feature = "kdotool")]
pub mod kdotool_injector;

pub mod ydotool_injector;

// NoOp fallback is always available
pub mod noop_injector;

// Tests temporarily moved to .tests_temp/ during refactor
// #[cfg(test)]
// mod tests;

// Re-export key components for easy access
pub use backend::Backend;
pub use focus::{FocusProvider, FocusStatus};
pub use manager::StrategyManager;
pub use processor::{AsyncInjectionProcessor, InjectionProcessor, ProcessorMetrics};
pub use session::{InjectionSession, SessionConfig, SessionState};
pub use types::{InjectionConfig, InjectionError, InjectionMethod, InjectionResult};

/// Trait defining the core text injection interface
#[async_trait::async_trait]
pub trait TextInjector: Send + Sync {
    /// Inject text into the currently focused application
    async fn inject_text(&self, text: &str) -> InjectionResult<()>;

    /// Check if the injector is available and functional
    async fn is_available(&self) -> bool;

    /// Get the backend name for this injector
    fn backend_name(&self) -> &'static str;

    /// Get backend-specific configuration information
    fn backend_info(&self) -> Vec<(&'static str, String)>;
}

// Re-export confirmation module components
pub use confirm::{
    ConfirmationContext, ConfirmationResult, TextChangeListener,
    create_confirmation_context, text_changed,
};




