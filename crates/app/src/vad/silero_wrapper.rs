use crate::vad::{
    config::SileroConfig,
    engine::VadEngine,
    types::{VadEvent, VadState},
};
use voice_activity_detector::VoiceActivityDetector;
use std::time::Instant;

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
                if probability >= self.config.threshold {
                    if self.speech_start_time.is_none() {
                        self.speech_start_time = Some(Instant::now());
                        self.speech_start_timestamp_ms = timestamp_ms;
                    } else if let Some(start) = self.speech_start_time {
                        if start.elapsed().as_millis() >= self.config.min_speech_duration_ms as u128 {
                            self.current_state = VadState::Speech;
                            self.speech_start_time = None;
                            self.silence_start_time = None;
                            
                            return Some(VadEvent::SpeechStart {
                                timestamp_ms: self.speech_start_timestamp_ms,
                                energy_db: probability_to_db(probability),
                            });
                        }
                    }
                } else {
                    self.speech_start_time = None;
                }
            }
            VadState::Speech => {
                if probability < self.config.threshold {
                    if self.silence_start_time.is_none() {
                        self.silence_start_time = Some(Instant::now());
                    } else if let Some(start) = self.silence_start_time {
                        if start.elapsed().as_millis() >= self.config.min_silence_duration_ms as u128 {
                            self.current_state = VadState::Silence;
                            self.speech_start_time = None;
                            self.silence_start_time = None;
                            
                            let duration_ms = timestamp_ms - self.speech_start_timestamp_ms;
                            
                            return Some(VadEvent::SpeechEnd {
                                timestamp_ms,
                                duration_ms,
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
            return Err(format!("Silero VAD requires 512 samples, got {}", frame.len()));
        }
        
        
        let probability = self.detector.predict(frame.iter().map(|&s| I16Sample(s)))
            .map_err(|e| format!("Silero VAD prediction failed: {}", e))?;
        
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