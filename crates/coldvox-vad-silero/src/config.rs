use coldvox_vad::constants::FRAME_SIZE_SAMPLES;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SileroConfig {
    pub activation_threshold: f32,
    pub deactivation_threshold: f32,
    pub min_speech_duration_ms: u32,
    pub min_silence_duration_ms: u32,
    pub window_size_samples: usize,
    pub speech_padding_ms: u32,
    pub energy_floor_dbfs: f32,
    pub max_speech_duration_ms: Option<u32>,
}

impl Default for SileroConfig {
    fn default() -> Self {
        Self {
            activation_threshold: 0.35,
            deactivation_threshold: 0.25,
            min_speech_duration_ms: 250,
            min_silence_duration_ms: 250,
            window_size_samples: FRAME_SIZE_SAMPLES,
            speech_padding_ms: 150,
            energy_floor_dbfs: -55.0,
            max_speech_duration_ms: Some(30000),
        }
    }
}
