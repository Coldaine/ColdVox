use crate::types::{VadEvent, VadState};

/// A trait for Voice Activity Detection (VAD) engines.
///
/// This defines the common interface for different VAD implementations,
/// allowing them to be used interchangeably in the audio pipeline.
pub trait VadEngine: Send {
    fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String>;
    fn reset(&mut self);
    fn current_state(&self) -> VadState;
    fn required_sample_rate(&self) -> u32;
    fn required_frame_size_samples(&self) -> usize;
}