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
//! | Clipboard    | Linux    | wl-clipboard-rs    | Stable |
//! | Enigo        | Cross    | Input simulation   | Beta   |
//! | KDotool      | Linux    | X11 automation     | Beta   |
//! | YDotool      | Linux    | uinput automation  | Beta   |

//!
//! ## Features
//!
//! - `atspi`: Linux AT-SPI accessibility backend
//! - `wl_clipboard`: Clipboard-based injection via wl-clipboard-rs
//! - `enigo`: Cross-platform input simulation
//! - `ydotool`: Linux uinput automation
//! - `kdotool`: KDE/X11 window activation assistance

//! - `regex`: Precompile allow/block list patterns
//! - `all-backends`: Enable all available backends
//! - `linux-desktop`: Enable recommended Linux desktop backends

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

#[cfg(all(feature = "wl_clipboard", feature = "atspi"))]
pub mod combo_clip_atspi;

#[cfg(feature = "enigo")]
pub mod enigo_injector;

#[cfg(feature = "kdotool")]
pub mod kdotool_injector;

pub mod ydotool_injector;

// NoOp fallback is always available
pub mod noop_injector;

#[cfg(test)]
mod tests;

// Re-export key components for easy access
pub use backend::Backend;
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

/// Trait defining text injection session management
#[async_trait::async_trait]
pub trait TextInjectionSession: Send + Sync {
    type Config;
    type Error;

    /// Start a new injection session
    async fn start(&mut self, config: Self::Config) -> Result<(), Self::Error>;

    /// Stop the current injection session
    async fn stop(&mut self) -> Result<(), Self::Error>;

    /// Check if session is currently active
    fn is_active(&self) -> bool;

    /// Get session statistics
    fn get_stats(&self) -> SessionStats;
}

/// Statistics for text injection sessions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionStats {
    pub injections_count: u64,
    pub total_characters: u64,
    pub session_duration: std::time::Duration,
    pub last_injection: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for SessionStats {
    fn default() -> Self {
        Self {
            injections_count: 0,
            total_characters: 0,
            session_duration: std::time::Duration::ZERO,
            last_injection: None,
        }
    }
}
