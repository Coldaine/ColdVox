use coldvox_vad::{
    UnifiedVadConfig, VadMode, VadEngine, VadEvent, VadState,
};
#[cfg(feature = "level3")]
use coldvox_vad::VadConfig;
#[cfg(feature = "level3")]
use coldvox_vad::level3::Level3Vad;
#[cfg(feature = "silero")]  
use coldvox_vad_silero::SileroEngine;

pub struct VadAdapter {
    engine: Box<dyn VadEngine>,
    config: UnifiedVadConfig,
    resampler: Option<AudioResampler>,
}

impl VadAdapter {
    pub fn new(config: UnifiedVadConfig) -> Result<Self, String> {
        let engine: Box<dyn VadEngine> = match config.mode {
            #[cfg(feature = "level3")]
            VadMode::Level3 => {
                // INTENTIONAL: Level3 VAD is disabled by default
                // This check ensures it's not accidentally enabled without explicit configuration
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
            #[cfg(not(feature = "level3"))]
            VadMode::Level3 => {
                return Err("Level3 VAD is not available in this build. Use Silero mode instead.".to_string());
            }
            VadMode::Silero => {
                let silero_config = coldvox_vad_silero::SileroConfig {
                    threshold: config.silero.threshold,
                    min_speech_duration_ms: config.silero.min_speech_duration_ms,
                    min_silence_duration_ms: config.silero.min_silence_duration_ms,
                    window_size_samples: config.silero.window_size_samples,
                };
                Box::new(SileroEngine::new(silero_config)?)
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
            engine,
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

use rubato::{
    Resampler, SincFixedIn,
    SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

struct AudioResampler {
    input_rate: u32,
    output_rate: u32,
    input_frame_size: usize,
    output_frame_size: usize,
    accumulator: Vec<i16>,
    output_buffer: Vec<i16>,
    resampler: Option<SincFixedIn<f32>>,
    f32_input_buffer: Vec<f32>,
    f32_output_buffer: Vec<f32>,
    chunk_size: usize,
}

impl AudioResampler {
    fn new(
        input_rate: u32,
        output_rate: u32,
        input_frame_size: usize,
        output_frame_size: usize,
    ) -> Result<Self, String> {
        // Only create Rubato resampler if rates differ
        let (resampler, chunk_size) = if input_rate != output_rate {
            // Use a chunk size that works well with typical frame sizes
            let chunk_size = 512;
            
            // Configure for low-latency VAD processing
            let sinc_params = SincInterpolationParameters {
                sinc_len: 64,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Cubic,
                oversampling_factor: 128,
                window: WindowFunction::Blackman2,
            };
            
            let resampler = SincFixedIn::<f32>::new(
                output_rate as f64 / input_rate as f64,  // Resample ratio
                2.0,  // Max resample ratio change (not used in fixed mode)
                sinc_params,
                chunk_size,
                1,  // mono
            ).map_err(|e| format!("Failed to create Rubato resampler: {}", e))?;
            
            (Some(resampler), chunk_size)
        } else {
            (None, 512)  // Default chunk size even when not resampling
        };
        
        Ok(Self {
            input_rate,
            output_rate,
            input_frame_size,
            output_frame_size,
            accumulator: Vec::new(),
            output_buffer: Vec::with_capacity(output_frame_size * 2),
            resampler,
            f32_input_buffer: Vec::with_capacity(chunk_size * 2),
            f32_output_buffer: Vec::new(),
            chunk_size,
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
        } else if let Some(resampler) = &mut self.resampler {
            // Use Rubato for high-quality resampling
            
            // Convert accumulated i16 samples to f32
            for &sample in &self.accumulator {
                self.f32_input_buffer.push(sample as f32 / 32768.0);
            }
            self.accumulator.clear();
            
            // Process complete chunks through Rubato
            while self.f32_input_buffer.len() >= self.chunk_size {
                let chunk: Vec<f32> = self.f32_input_buffer.drain(..self.chunk_size).collect();
                let input_frames = vec![chunk];
                
                // Process with Rubato
                match resampler.process(&input_frames, None) {
                    Ok(output_frames) => {
                        if !output_frames.is_empty() && !output_frames[0].is_empty() {
                            self.f32_output_buffer.extend_from_slice(&output_frames[0]);
                        }
                    }
                    Err(e) => {
                        return Err(format!("Resampler error: {}", e));
                    }
                }
            }
            
            // Convert f32 output back to i16 and add to output buffer
            for &sample in &self.f32_output_buffer {
                let clamped = sample.clamp(-1.0, 1.0);
                let i16_sample = (clamped * 32767.0).round() as i16;
                self.output_buffer.push(i16_sample);
            }
            self.f32_output_buffer.clear();
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
        self.f32_input_buffer.clear();
        self.f32_output_buffer.clear();
        if let Some(resampler) = &mut self.resampler {
            resampler.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampler_pass_through_same_rate_same_size() {
        let mut rs = AudioResampler::new(16_000, 16_000, 512, 512).expect("init");
        let input = vec![0i16; 512];
        let out = rs.process(&input).expect("process");
        assert_eq!(out.len(), 512);
        assert!(out.iter().all(|&s| s == 0));
    }

    #[test]
    fn resampler_frame_aggregation_same_rate_diff_size() {
        // Aggregate two 256-sample inputs into one 512-sample output when rates match
        let mut rs = AudioResampler::new(16_000, 16_000, 256, 512).expect("init");
        let input = vec![1i16; 256];

        let out1 = rs.process(&input).expect("process1");
        assert!(out1.is_empty(), "First half-frame should not emit yet");

        let out2 = rs.process(&input).expect("process2");
        assert_eq!(out2.len(), 512);
        assert!(out2.iter().all(|&s| s == 1));
    }

    #[test]
    fn resampler_downsample_48k_to_16k_produces_full_frames() {
        // When downsampling 48k -> 16k, multiple input chunks are needed before a full 512-sample output is ready.
        let mut rs = AudioResampler::new(48_000, 16_000, 512, 512).expect("init");
        let input = vec![0i16; 512];

        let mut got = Vec::new();
        for _ in 0..10 {
            let out = rs.process(&input).expect("process");
            if !out.is_empty() {
                got = out;
                break;
            }
        }
        assert_eq!(got.len(), 512, "Should eventually produce one full 512-sample frame");
        assert!(got.iter().all(|&s| s == 0));
    }
}