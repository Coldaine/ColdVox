use serde::{Deserialize, Serialize};

use super::constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VadMode {
    Level3, // Energy-based VAD - INTENTIONALLY DISABLED (see Level3Config.enabled)
    Silero, // ML-based VAD using ONNX - DEFAULT ACTIVE VAD
}

impl Default for VadMode {
    fn default() -> Self {
        // INTENTIONAL: Silero is the default VAD mode
        // Level3 (energy-based) VAD is disabled by default - see Level3Config.enabled
        Self::Silero
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level3Config {
    // INTENTIONAL: Level3 VAD is disabled by default
    // This energy-based VAD is kept for fallback/testing but not used in production
    pub enabled: bool,
    pub onset_threshold_db: f32,
    pub offset_threshold_db: f32,
    pub ema_alpha: f32,
    pub speech_debounce_ms: u32,
    pub silence_debounce_ms: u32,
    pub initial_floor_db: f32,
}

impl Default for Level3Config {
    fn default() -> Self {
        Self {
            // INTENTIONAL: Level3 VAD is disabled - using Silero VAD instead
            enabled: false,
            onset_threshold_db: 9.0,
            offset_threshold_db: 6.0,
            ema_alpha: 0.02,
            speech_debounce_ms: 200,
            silence_debounce_ms: 400,
            initial_floor_db: -50.0,
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedVadConfig {
    pub mode: VadMode,
    pub level3: Level3Config,
    pub silero: SileroConfig,
    pub frame_size_samples: usize,
    pub sample_rate_hz: u32,
}

impl Default for UnifiedVadConfig {
    fn default() -> Self {
        Self {
            mode: VadMode::default(), // Uses Silero by default now
            level3: Level3Config::default(),
            silero: SileroConfig::default(),
            // Align default frame size with default engine (Silero) requirement
            // Both Silero and Level3 now use 512-sample windows at 16 kHz
            frame_size_samples: FRAME_SIZE_SAMPLES,
            sample_rate_hz: SAMPLE_RATE_HZ,
        }
    }
}

impl UnifiedVadConfig {
    pub fn frame_duration_ms(&self) -> f32 {
        (self.frame_size_samples as f32 * 1000.0) / self.sample_rate_hz as f32
    }
}
