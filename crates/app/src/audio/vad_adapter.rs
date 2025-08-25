use crate::vad::{
    config::{UnifiedVadConfig, VadMode},
    engine::{VadEngine, VadEngineBox},
    level3::Level3Vad,
    silero_wrapper::SileroEngine,
    types::{VadConfig, VadEvent, VadState},
};

pub struct VadAdapter {
    engine: VadEngineBox,
    config: UnifiedVadConfig,
    resampler: Option<AudioResampler>,
}

impl VadAdapter {
    pub fn new(config: UnifiedVadConfig) -> Result<Self, String> {
        let engine: Box<dyn VadEngine> = match config.mode {
            VadMode::Level3 => {
                if !config.level3.enabled {
                    return Err("Level3 VAD is disabled in configuration. Use Silero mode instead.".to_string());
                }
                let level3_config = VadConfig {
                    onset_threshold_db: config.level3.onset_threshold_db,
                    offset_threshold_db: config.level3.offset_threshold_db,
                    ema_alpha: config.level3.ema_alpha,
                    speech_debounce_ms: config.level3.speech_debounce_ms,
                    silence_debounce_ms: config.level3.silence_debounce_ms,
                    initial_floor_db: config.level3.initial_floor_db,
                    frame_size_samples: config.frame_size_samples,
                    sample_rate_hz: config.sample_rate_hz,
                };
                Box::new(Level3Vad::new(level3_config))
            }
            VadMode::Silero => {
                Box::new(SileroEngine::new(config.silero.clone())?)
            }
        };
        
        let resampler = if engine.required_sample_rate() != config.sample_rate_hz
            || engine.required_frame_size_samples() != config.frame_size_samples
        {
            Some(AudioResampler::new(
                config.sample_rate_hz,
                engine.required_sample_rate(),
                config.frame_size_samples,
                engine.required_frame_size_samples(),
            )?)
        } else {
            None
        };
        
        Ok(Self {
            engine: VadEngineBox::new(engine),
            config,
            resampler,
        })
    }
    
    pub fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String> {
        if let Some(resampler) = &mut self.resampler {
            let processed_frame = resampler.process(frame)?;
            // Only process if we got a complete frame
            if !processed_frame.is_empty() {
                self.engine.process(&processed_frame)
            } else {
                // Not enough samples yet, waiting for more
                Ok(None)
            }
        } else {
            self.engine.process(frame)
        }
    }
    
    pub fn reset(&mut self) {
        self.engine.reset();
        if let Some(resampler) = &mut self.resampler {
            resampler.reset();
        }
    }
    
    pub fn current_state(&self) -> VadState {
        self.engine.current_state()
    }
    
    pub fn config(&self) -> &UnifiedVadConfig {
        &self.config
    }
}

struct AudioResampler {
    input_rate: u32,
    output_rate: u32,
    input_frame_size: usize,
    output_frame_size: usize,
    accumulator: Vec<i16>,
    output_buffer: Vec<i16>,
    phase: f32,
}

impl AudioResampler {
    fn new(
        input_rate: u32,
        output_rate: u32,
        input_frame_size: usize,
        output_frame_size: usize,
    ) -> Result<Self, String> {
        Ok(Self {
            input_rate,
            output_rate,
            input_frame_size,
            output_frame_size,
            accumulator: Vec::new(),
            output_buffer: Vec::with_capacity(output_frame_size * 2),
            phase: 0.0,
        })
    }
    
    fn process(&mut self, input: &[i16]) -> Result<Vec<i16>, String> {
        if input.len() != self.input_frame_size {
            return Err(format!(
                "Expected {} samples, got {}",
                self.input_frame_size,
                input.len()
            ));
        }
        
        // Add input to accumulator
        self.accumulator.extend_from_slice(input);
        
        // If sample rates are the same, just handle frame size conversion
        if self.input_rate == self.output_rate {
            // Simple frame size conversion without resampling
            while self.accumulator.len() >= self.output_frame_size {
                let frame: Vec<i16> = self.accumulator.drain(..self.output_frame_size).collect();
                self.output_buffer.extend_from_slice(&frame);
            }
        } else {
            // Resample with linear interpolation
            let ratio = self.input_rate as f32 / self.output_rate as f32;
            
            while (self.phase as usize) + 1 < self.accumulator.len() {
                let index = self.phase as usize;
                let fraction = self.phase - index as f32;
                
                let s0 = self.accumulator[index] as f32;
                let s1 = self.accumulator[index + 1] as f32;
                let sample = (s0 * (1.0 - fraction) + s1 * fraction) as i16;
                
                self.output_buffer.push(sample);
                self.phase += ratio;
                
                // If we have enough samples for a frame, stop
                if self.output_buffer.len() >= self.output_frame_size {
                    break;
                }
            }
            
            // Remove consumed samples
            let consumed = (self.phase as usize).min(self.accumulator.len());
            if consumed > 0 {
                self.accumulator.drain(..consumed);
                self.phase -= consumed as f32;
            }
        }
        
        // Return a complete frame if available, otherwise return empty vector
        if self.output_buffer.len() >= self.output_frame_size {
            Ok(self.output_buffer.drain(..self.output_frame_size).collect())
        } else {
            // Not enough samples yet - return empty vector
            Ok(Vec::new())
        }
    }
    
    fn reset(&mut self) {
        self.output_buffer.clear();
        self.accumulator.clear();
        self.phase = 0.0;
    }
}