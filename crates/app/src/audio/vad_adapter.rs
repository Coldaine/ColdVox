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
        let processed_frame = if let Some(resampler) = &mut self.resampler {
            resampler.process(frame)?
        } else {
            frame.to_vec()
        };
        self.engine.process(&processed_frame)
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
    buffer: Vec<i16>,
    accumulator: Vec<i16>,
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
            buffer: Vec::with_capacity(output_frame_size * 2),
            accumulator: Vec::new(),
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
        
        self.accumulator.extend_from_slice(input);
        
        let ratio = self.input_rate as f32 / self.output_rate as f32;
        let mut output = Vec::with_capacity(self.output_frame_size);
        
        while output.len() < self.output_frame_size && (self.phase as usize) < self.accumulator.len() - 1 {
            let index = self.phase as usize;
            let fraction = self.phase - index as f32;
            
            let sample = if index + 1 < self.accumulator.len() {
                let s0 = self.accumulator[index] as f32;
                let s1 = self.accumulator[index + 1] as f32;
                (s0 * (1.0 - fraction) + s1 * fraction) as i16
            } else {
                self.accumulator[index]
            };
            
            output.push(sample);
            self.phase += ratio;
        }
        
        let consumed = (self.phase as usize).min(self.accumulator.len());
        self.accumulator.drain(..consumed);
        self.phase -= consumed as f32;
        
        while output.len() < self.output_frame_size {
            output.push(0);
        }
        
        Ok(output)
    }
    
    fn reset(&mut self) {
        self.buffer.clear();
        self.accumulator.clear();
        self.phase = 0.0;
    }
}