//! Built-in STT plugin implementations

pub mod mock;
pub mod noop;

#[cfg(feature = "vosk")]
pub mod vosk_plugin;

#[cfg(feature = "whisper")]
pub mod whisper_plugin;

// Re-export commonly used plugins
pub use mock::MockPlugin;
pub use noop::NoOpPlugin;

#[cfg(feature = "vosk")]
pub use vosk_plugin::{VoskPlugin, VoskPluginFactory};

#[cfg(feature = "whisper")]
pub use whisper_plugin::{WhisperPlugin, WhisperPluginFactory};
