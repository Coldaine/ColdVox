#[cfg(feature = "silero")]
pub mod silero_wrapper;
pub mod config;

pub use config::SileroConfig;

#[cfg(feature = "silero")]
pub use silero_wrapper::SileroEngine;