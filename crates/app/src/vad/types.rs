use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VadEvent {
    SpeechStart {
        timestamp_ms: u64,
        energy_db: f32,
    },
    SpeechEnd {
        timestamp_ms: u64,
        duration_ms: u64,
        energy_db: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadState {
    Silence,
    Speech,
}

impl Default for VadState {
    fn default() -> Self {
        Self::Silence
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    pub onset_threshold_db: f32,
    
    pub offset_threshold_db: f32,
    
    pub ema_alpha: f32,
    
    pub speech_debounce_ms: u32,
    
    pub silence_debounce_ms: u32,
    
    pub initial_floor_db: f32,
    
    pub frame_size_samples: usize,
    
    pub sample_rate_hz: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            onset_threshold_db: 9.0,
            offset_threshold_db: 6.0,
            ema_alpha: 0.02,
            speech_debounce_ms: 200,
            silence_debounce_ms: 400,
            initial_floor_db: -50.0,
            frame_size_samples: 320,
            sample_rate_hz: 16000,
        }
    }
}

impl VadConfig {
    pub fn frame_duration_ms(&self) -> f32 {
        (self.frame_size_samples as f32 * 1000.0) / self.sample_rate_hz as f32
    }
    
    pub fn speech_debounce_frames(&self) -> u32 {
        (self.speech_debounce_ms as f32 / self.frame_duration_ms()).ceil() as u32
    }
    
    pub fn silence_debounce_frames(&self) -> u32 {
        (self.silence_debounce_ms as f32 / self.frame_duration_ms()).ceil() as u32
    }
}

#[derive(Debug, Clone, Default)]
pub struct VadMetrics {
    pub frames_processed: u64,
    
    pub speech_segments: u64,
    
    pub total_speech_ms: u64,
    
    pub total_silence_ms: u64,
    
    pub current_noise_floor_db: f32,
    
    pub last_energy_db: f32,
}