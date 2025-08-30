pub mod backend;
pub mod focus;
pub mod manager;
pub mod processor;
pub mod session;
pub mod types;
pub mod window_manager;

// Individual injector modules
#[cfg(feature = "text-injection-atspi")]
pub mod atspi_injector;
#[cfg(feature = "text-injection-clipboard")]
pub mod clipboard_injector;
#[cfg(all(feature = "text-injection-clipboard", feature = "text-injection-atspi"))]
pub mod combo_clip_atspi;
#[cfg(feature = "text-injection-ydotool")]
pub mod ydotool_injector;
#[cfg(feature = "text-injection-enigo")]
pub mod enigo_injector;
#[cfg(feature = "text-injection-mki")]
pub mod mki_injector;
// NoOp fallback is always available
pub mod noop_injector;
#[cfg(feature = "text-injection-kdotool")]
pub mod kdotool_injector;

#[cfg(test)]
mod tests;

#[cfg(feature = "text-injection")]
pub mod probes;

// Re-export key components
pub use processor::{AsyncInjectionProcessor, ProcessorMetrics, InjectionProcessor};
pub use session::{InjectionSession, SessionConfig, SessionState};
pub use types::{InjectionConfig, InjectionError, InjectionMethod, InjectionResult};
pub use backend::Backend;

#[cfg(feature = "text-injection")]
pub use manager::StrategyManager;