//! Built-in STT plugin implementations

pub mod mock;
pub mod noop;
// whisper backend temporarily removed; will be reintroduced as pure Rust implementation
// pub mod whisper_plugin;

// Pure Rust Candle Whisper implementation
#[cfg(feature = "candle-whisper")]
pub mod candle_whisper;

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
// pub use whisper_plugin::{WhisperPlugin, WhisperPluginFactory};

#[cfg(feature = "candle-whisper")]
pub use candle_whisper::{CandleWhisperPlugin, CandleWhisperPluginFactory};

#[cfg(feature = "parakeet")]
pub use parakeet::ParakeetPluginFactory;
