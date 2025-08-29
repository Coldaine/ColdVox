pub mod config;
pub mod constants;
pub mod engine;
pub mod energy;
pub mod level3;
pub mod silero_wrapper;
pub mod state;
pub mod threshold;
pub mod types;

#[cfg(test)]
mod tests;

pub use constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ, FRAME_DURATION_MS};
pub use level3::{Level3Vad, Level3VadBuilder};
pub use types::{VadConfig, VadEvent, VadState};

pub trait VadProcessor: Send {
    fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String>;
    
    fn reset(&mut self);
    
    fn current_state(&self) -> VadState;
}