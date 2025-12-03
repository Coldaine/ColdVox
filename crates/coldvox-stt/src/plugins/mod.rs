//! Built-in STT plugin implementations

pub mod mock;
pub mod noop;

#[cfg(feature = "parakeet")]
pub mod parakeet;

// Re-export commonly used plugins
pub use mock::MockPlugin;
pub use noop::NoOpPlugin;

#[cfg(feature = "parakeet")]
pub use parakeet::ParakeetPluginFactory;
