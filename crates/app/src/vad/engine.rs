use crate::vad::types::{VadEvent, VadState};

pub trait VadEngine: Send {
    fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String>;
    
    fn reset(&mut self);
    
    fn current_state(&self) -> VadState;
    
    fn required_sample_rate(&self) -> u32;
    
    fn required_frame_size_samples(&self) -> usize;
}

pub struct VadEngineBox {
    engine: Box<dyn VadEngine>,
}

impl VadEngineBox {
    pub fn new(engine: Box<dyn VadEngine>) -> Self {
        Self { engine }
    }
    
    pub fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String> {
        self.engine.process(frame)
    }
    
    pub fn reset(&mut self) {
        self.engine.reset()
    }
    
    pub fn current_state(&self) -> VadState {
        self.engine.current_state()
    }
    
    pub fn required_sample_rate(&self) -> u32 {
        self.engine.required_sample_rate()
    }
    
    pub fn required_frame_size_samples(&self) -> usize {
        self.engine.required_frame_size_samples()
    }
}