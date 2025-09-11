//! Built-in STT plugin implementations

pub mod mock;
pub mod noop;

#[cfg(feature = "vosk")]
pub mod vosk;

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
