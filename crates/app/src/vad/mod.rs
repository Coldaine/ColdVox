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

#[cfg(feature = "silero")]
pub use coldvox_vad_silero::SileroEngine;
