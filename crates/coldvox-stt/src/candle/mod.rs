//! Candle (Rust Whisper) backend.
//!
//! This module tree will eventually host the full WhisperEngine port. For now
//! it focuses on the audio preprocessing routines (mel filters + spectrogram).

pub mod audio;
pub mod loader;
pub mod decode;
pub mod types;
pub mod timestamps;
pub mod model;
pub mod engine;
pub mod decoder;
