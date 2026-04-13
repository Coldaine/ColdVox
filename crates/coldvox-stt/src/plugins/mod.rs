//! Built-in STT plugin implementations

pub mod mock;
pub mod noop;
// whisper backend temporarily removed; will be reintroduced as pure Rust implementation
// pub mod whisper_plugin;

#[cfg(feature = "parakeet")]
pub mod parakeet;

#[cfg(feature = "moonshine")]
pub mod moonshine;

// Re-export commonly used plugins
pub use mock::MockPlugin;
pub use noop::NoOpPlugin;
// pub use whisper_plugin::{WhisperPlugin, WhisperPluginFactory};

#[cfg(feature = "parakeet")]
pub use parakeet::ParakeetPluginFactory;

#[cfg(feature = "moonshine")]
pub use moonshine::{MoonshineModelSize, MoonshinePlugin, MoonshinePluginFactory};
