use crate::config::SileroConfig;
use coldvox_vad::{VadEngine, VadEvent, VadState};
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
    // Frame-based debouncing timestamps (in ms) rather than wall-clock Instants.
    speech_start_candidate_ms: Option<u64>,
    silence_start_candidate_ms: Option<u64>,
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
            speech_start_candidate_ms: None,
            silence_start_candidate_ms: None,
            speech_start_timestamp_ms: 0,
            frames_processed: 0,
            last_probability: 0.0,
        })
    }

    fn process_probability(&mut self, probability: f32) -> Option<VadEvent> {
        let timestamp_ms = self.frames_processed * 512 * 1000 / 16000;

        match self.current_state {
            VadState::Silence => {
                if probability >= self.config.threshold {
                    if self.speech_start_candidate_ms.is_none() {
                        self.speech_start_candidate_ms = Some(timestamp_ms);
                        self.speech_start_timestamp_ms = timestamp_ms;
                    } else if let Some(start_ms) = self.speech_start_candidate_ms {
                        if timestamp_ms.saturating_sub(start_ms)
                            >= self.config.min_speech_duration_ms as u64
                        {
                            self.current_state = VadState::Speech;
                            self.speech_start_candidate_ms = None;
                            self.silence_start_candidate_ms = None;

                            return Some(VadEvent::SpeechStart {
                                timestamp_ms: self.speech_start_timestamp_ms,
                                energy_db: probability_to_db(probability),
                            });
                        }
                    }
                } else {
                    self.speech_start_candidate_ms = None;
                }
            }
            VadState::Speech => {
                if probability < self.config.threshold {
                    if self.silence_start_candidate_ms.is_none() {
                        self.silence_start_candidate_ms = Some(timestamp_ms);
                    } else if let Some(start_ms) = self.silence_start_candidate_ms {
                        if timestamp_ms.saturating_sub(start_ms)
                            >= self.config.min_silence_duration_ms as u64
                        {
                            self.current_state = VadState::Silence;
                            self.speech_start_candidate_ms = None;
                            self.silence_start_candidate_ms = None;

                            let duration_ms =
                                timestamp_ms.saturating_sub(self.speech_start_timestamp_ms);

                            return Some(VadEvent::SpeechEnd {
                                timestamp_ms,
                                duration_ms,
                                energy_db: probability_to_db(probability),
                            });
                        }
                    }
                } else {
                    self.silence_start_candidate_ms = None;
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

        let probability = self.detector.predict(frame.iter().map(|&s| I16Sample(s)));

        self.last_probability = probability;
        self.frames_processed += 1;

        Ok(self.process_probability(probability))
    }

    fn reset(&mut self) {
        self.detector.reset();
        self.current_state = VadState::Silence;
        self.speech_start_candidate_ms = None;
        self.silence_start_candidate_ms = None;
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
