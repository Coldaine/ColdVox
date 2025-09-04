use crate::config::SileroConfig;
use coldvox_vad::{VadEngine, VadEvent, VadState};
use std::time::Instant;
use voice_activity_detector::VoiceActivityDetector;

#[derive(Copy, Clone, Default)]
struct I16Sample(i16);

impl voice_activity_detector::Sample for I16Sample {
    fn to_f32(self) -> f32 {
        self.0 as f32 / i16::MAX as f32
    }
}

pub struct SileroEngine {
    detector: VoiceActivityDetector,
    config: SileroConfig,
    current_state: VadState,
    speech_start_time: Option<Instant>,
    silence_start_time: Option<Instant>,
    speech_start_timestamp_ms: u64,
    frames_processed: u64,
    last_probability: f32,
}

impl SileroEngine {
    pub fn new(config: SileroConfig) -> Result<Self, String> {
        let detector = VoiceActivityDetector::builder()
            .sample_rate(16000)
            .chunk_size(512_usize)
            .build()
            .map_err(|e| format!("Failed to create Silero VAD: {}", e))?;

        Ok(Self {
            detector,
            config,
            current_state: VadState::Silence,
            speech_start_time: None,
            silence_start_time: None,
            speech_start_timestamp_ms: 0,
            frames_processed: 0,
            last_probability: 0.0,
        })
    }

    fn process_probability(&mut self, probability: f32) -> Option<VadEvent> {
        let timestamp_ms = self.frames_processed * 512 * 1000 / 16000;

        match self.current_state {
            VadState::Silence => {
                if probability >= self.config.activation_threshold {
                    if self.speech_start_time.is_none() {
                        self.speech_start_time = Some(Instant::now());
                        self.speech_start_timestamp_ms = timestamp_ms;
                    } else if let Some(start) = self.speech_start_time {
                        if start.elapsed().as_millis() >= self.config.min_speech_duration_ms as u128
                        {
                            self.current_state = VadState::Speech;
                            self.speech_start_time = None;
                            self.silence_start_time = None;

                            let start_timestamp = self
                                .speech_start_timestamp_ms
                                .saturating_sub(self.config.speech_padding_ms as u64);
                            return Some(VadEvent::SpeechStart {
                                timestamp_ms: start_timestamp,
                                energy_db: probability_to_db(probability),
                            });
                        }
                    }
                } else {
                    self.speech_start_time = None;
                }
            }
            VadState::Speech => {
                // Check for max speech duration
                if let Some(max_duration) = self.config.max_speech_duration_ms {
                    let current_duration = timestamp_ms - self.speech_start_timestamp_ms;
                    if current_duration >= max_duration as u64 {
                        self.current_state = VadState::Silence;
                        self.speech_start_time = None;
                        self.silence_start_time = None;

                        let padded_start_ms = self
                            .speech_start_timestamp_ms
                            .saturating_sub(self.config.speech_padding_ms as u64);
                        let padded_end_ms =
                            timestamp_ms + self.config.speech_padding_ms as u64;
                        let padded_duration_ms = padded_end_ms - padded_start_ms;

                        return Some(VadEvent::SpeechEnd {
                            timestamp_ms: padded_end_ms,
                            duration_ms: padded_duration_ms,
                            energy_db: probability_to_db(probability),
                        });
                    }
                }

                if probability < self.config.deactivation_threshold {
                    if self.silence_start_time.is_none() {
                        self.silence_start_time = Some(Instant::now());
                    } else if let Some(start) = self.silence_start_time {
                        if start.elapsed().as_millis()
                            >= self.config.min_silence_duration_ms as u128
                        {
                            self.current_state = VadState::Silence;
                            self.speech_start_time = None;
                            self.silence_start_time = None;

                            let padded_start_ms = self
                                .speech_start_timestamp_ms
                                .saturating_sub(self.config.speech_padding_ms as u64);
                            let padded_end_ms =
                                timestamp_ms + self.config.speech_padding_ms as u64;
                            let padded_duration_ms = padded_end_ms - padded_start_ms;

                            return Some(VadEvent::SpeechEnd {
                                timestamp_ms: padded_end_ms,
                                duration_ms: padded_duration_ms,
                                energy_db: probability_to_db(probability),
                            });
                        }
                    }
                } else {
                    self.silence_start_time = None;
                }
            }
        }

        None
    }
}

impl VadEngine for SileroEngine {
    fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String> {
        if frame.len() != 512 {
            return Err(format!(
                "Silero VAD requires 512 samples, got {}",
                frame.len()
            ));
        }

        let energy_dbfs = calculate_energy_dbfs(frame);
        let mut probability = self.detector.predict(frame.iter().map(|&s| I16Sample(s)));

        if energy_dbfs < self.config.energy_floor_dbfs {
            probability = 0.0;
        }

        self.last_probability = probability;
        self.frames_processed += 1;

        Ok(self.process_probability(probability))
    }

    fn reset(&mut self) {
        self.detector.reset();
        self.current_state = VadState::Silence;
        self.speech_start_time = None;
        self.silence_start_time = None;
        self.speech_start_timestamp_ms = 0;
        self.frames_processed = 0;
        self.last_probability = 0.0;
    }

    fn current_state(&self) -> VadState {
        self.current_state
    }

    fn required_sample_rate(&self) -> u32 {
        16000
    }

    fn required_frame_size_samples(&self) -> usize {
        512
    }
}

fn probability_to_db(probability: f32) -> f32 {
    if probability <= 0.0 {
        -60.0
    } else {
        20.0 * probability.log10()
    }
}

fn calculate_energy_dbfs(frame: &[i16]) -> f32 {
    if frame.is_empty() {
        return -96.0; // Return a very low dBFS for empty frames
    }
    let sum_sq = frame.iter().map(|&s| (s as f64).powi(2)).sum::<f64>();
    let rms = (sum_sq / frame.len() as f64).sqrt();

    if rms == 0.0 {
        return -96.0; // Log of zero is undefined, return a low value
    }

    // Convert RMS to dBFS, where 0 dBFS is the max possible level for i16
    20.0 * (rms / i16::MAX as f64).log10() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silero_engine_creates_and_reports_requirements() {
        let cfg = SileroConfig::default();
        let engine = SileroEngine::new(cfg).expect("SileroEngine should create successfully");
        assert_eq!(engine.required_sample_rate(), 16000);
        assert_eq!(engine.required_frame_size_samples(), 512);
    }

    #[test]
    fn silero_engine_processes_silence_without_event() {
        let cfg = SileroConfig::default();
        let mut engine = SileroEngine::new(cfg).expect("SileroEngine should create successfully");
        let silence = vec![0i16; 512];
        let evt = engine.process(&silence).expect("Processing should succeed");
        assert!(evt.is_none(), "Silence should not emit VAD events");
    }

    #[test]
    fn silero_engine_rejects_incorrect_frame_sizes() {
        let cfg = SileroConfig::default();
        let mut engine = SileroEngine::new(cfg).expect("SileroEngine should create successfully");
        let too_short = vec![0i16; 511];
        let too_long = vec![0i16; 513];
        let err_short = engine.process(&too_short).unwrap_err();
        let err_long = engine.process(&too_long).unwrap_err();
        assert!(
            err_short.contains("512"),
            "Error should mention required frame size: {err_short}"
        );
        assert!(
            err_long.contains("512"),
            "Error should mention required frame size: {err_long}"
        );
    }
}
