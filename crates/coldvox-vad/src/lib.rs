pub mod config;
pub mod constants;
pub mod engine;
pub mod energy;
pub mod state;
pub mod threshold;
pub mod types;

#[cfg(feature = "level3")]
pub mod level3;

// Core exports
pub use constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ, FRAME_DURATION_MS};
pub use types::{VadConfig, VadEvent, VadState, VadMetrics};
pub use config::{UnifiedVadConfig, VadMode};
pub use engine::VadEngine;

// Level3 VAD exports when feature is enabled
#[cfg(feature = "level3")]
pub use level3::{Level3Vad, Level3VadBuilder};

/// Main VAD trait for processing audio frames
pub trait VadProcessor: Send {
    fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String>;
    fn reset(&mut self);
    fn current_state(&self) -> VadState;
}