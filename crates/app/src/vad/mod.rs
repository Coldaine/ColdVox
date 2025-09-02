//! VAD (Voice Activity Detection) module re-exports
//!
//! This module provides a unified interface to VAD functionality
//! by re-exporting types from the coldvox-vad and coldvox-vad-silero crates.

pub use coldvox_vad::{
    config::{UnifiedVadConfig, VadMode},
    constants::{FRAME_DURATION_MS, FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ},
    engine::VadEngine,
    types::{VadEvent, VadMetrics, VadState},
    VadProcessor,
};

#[cfg(feature = "level3")]
pub use coldvox_vad::level3::{Level3Vad, Level3VadBuilder};

#[cfg(feature = "silero")]
pub use coldvox_vad_silero::SileroEngine;
