//! Built-in STT plugin implementations

pub mod mock;
pub mod noop;

// Old Python-based whisper backend (deprecated)
// pub mod whisper_plugin;

// New Candle-based Whisper implementation
#[cfg(feature = "whisper")]
pub mod whisper_candle;

#[cfg(feature = "parakeet")]
pub mod parakeet;

#[cfg(feature = "whisper")]
pub mod whisper_cpp;

#[cfg(feature = "coqui")]
pub mod coqui;

#[cfg(feature = "leopard")]
pub mod leopard;

#[cfg(feature = "silero-stt")]
pub mod silero_stt;

// Re-export commonly used plugins
pub use mock::MockPlugin;
pub use noop::NoOpPlugin;

#[cfg(feature = "whisper")]
pub use whisper_candle::{WhisperCandlePlugin, WhisperCandlePluginFactory};

#[cfg(feature = "parakeet")]
pub use parakeet::ParakeetPluginFactory;
