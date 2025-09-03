//! Built-in STT plugin implementations

pub mod noop;
pub mod mock;

#[cfg(feature = "vosk")]
pub mod vosk_plugin;

#[cfg(feature = "whisper")]
pub mod whisper_plugin;

// Re-export commonly used plugins
pub use noop::NoOpPlugin;
pub use mock::MockPlugin;