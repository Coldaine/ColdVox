pub mod config;
pub mod constants;
pub mod energy;
pub mod engine;
pub mod state;
pub mod threshold;
pub mod types;

// Core exports - grouped and sorted alphabetically
pub use config::{UnifiedVadConfig, VadMode};
pub use constants::{FRAME_DURATION_MS, FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};
pub use engine::VadEngine;
pub use types::{VadConfig, VadEvent, VadMetrics, VadState};

/// Main VAD trait for processing audio frames
pub trait VadProcessor: Send {
    fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String>;
    fn reset(&mut self);
    fn current_state(&self) -> VadState;
}
