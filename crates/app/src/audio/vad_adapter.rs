use coldvox_audio::StreamResampler;
use coldvox_vad::{UnifiedVadConfig, VadEngine, VadEvent, VadMode, VadState};
#[cfg(feature = "silero")]
use coldvox_vad_silero::SileroEngine;

pub struct VadAdapter {
    engine: Box<dyn VadEngine>,
    config: UnifiedVadConfig,
    resampler: Option<StreamResampler>,
}

impl VadAdapter {
    pub fn new(config: UnifiedVadConfig) -> Result<Self, String> {
        #[cfg(not(feature = "silero"))]
        {
            return Err("No VAD engine available. Enable 'silero' feature.".to_string());
        }

        #[cfg(feature = "silero")]
        let engine: Box<dyn VadEngine> = match config.mode {
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

        #[cfg(feature = "silero")]
        let resampler = if engine.required_sample_rate() != config.sample_rate_hz {
            Some(StreamResampler::new(
                config.sample_rate_hz,
                engine.required_sample_rate(),
            ))
        } else {
            None
        };

        #[cfg(feature = "silero")]
        Ok(Self {
            engine,
            config,
            resampler,
        })
    }

    pub fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String> {
        if let Some(resampler) = &mut self.resampler {
            let processed_frame = resampler.process(frame);
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
