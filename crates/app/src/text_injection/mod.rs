pub mod backend;
pub mod focus;
pub mod manager;
pub mod processor;
pub mod session;
pub mod types;

#[cfg(feature = "text-injection")]
pub mod probes;

// Re-export key components
pub use processor::{AsyncInjectionProcessor, InjectionMetrics, InjectionProcessor};
pub use session::{InjectionSession, SessionConfig, SessionState};
pub use types::{InjectionConfig, InjectionError, InjectionMethod, InjectionResult};
pub use backend::Backend;

#[cfg(feature = "text-injection")]
pub use manager::StrategyManager;