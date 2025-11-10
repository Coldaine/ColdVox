//! Candle (Rust Whisper) backend.
//!
//! This module tree hosts the full WhisperEngine implementation. It provides
//! pure Rust implementation of OpenAI's Whisper model with full local processing,
//! GPU acceleration support, and comprehensive audio processing capabilities.

pub mod audio;
pub mod loader;
pub mod decode;
pub mod types;
pub mod timestamps;
pub mod model;
pub mod engine;
pub mod decoder;

// Re-export the main components for public use
pub use engine::{WhisperEngine, WhisperEngineInit, DevicePreference, DeviceInfo, WhisperEngineError};
pub use types::{Segment, WordTiming, Transcript};
