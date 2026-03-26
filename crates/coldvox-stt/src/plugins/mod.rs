//! Built-in STT plugin implementations

pub mod mock;
pub mod noop;

#[cfg(feature = "http-remote")]
pub mod http_remote;

#[cfg(feature = "parakeet")]
pub mod parakeet;

#[cfg(feature = "moonshine")]
pub mod moonshine;

// Re-export commonly used plugins
pub use mock::MockPlugin;
pub use noop::NoOpPlugin;

#[cfg(feature = "http-remote")]
pub use http_remote::HttpRemotePluginFactory;

#[cfg(feature = "parakeet")]
pub use parakeet::ParakeetPluginFactory;

#[cfg(feature = "moonshine")]
pub use moonshine::{MoonshineModelSize, MoonshinePlugin, MoonshinePluginFactory};
