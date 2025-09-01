use coldvox_vad::constants::FRAME_SIZE_SAMPLES;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SileroConfig {
    pub threshold: f32,
    pub min_speech_duration_ms: u32,
    pub min_silence_duration_ms: u32,
    pub window_size_samples: usize,
}

impl Default for SileroConfig {
    fn default() -> Self {
        Self {
            threshold: 0.3,
            min_speech_duration_ms: 250,
            min_silence_duration_ms: 100,
            window_size_samples: FRAME_SIZE_SAMPLES,
        }
    }
}