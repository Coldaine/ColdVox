use serde::{Deserialize, Serialize};

use super::constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum VadMode {
    // INTENTIONAL: Silero is the default VAD mode
    // Level3 (energy-based) VAD is disabled by default - see Level3Config.enabled
    #[default]
    Silero, // ML-based VAD using ONNX - DEFAULT ACTIVE VAD
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SileroConfig {
    /// Speech probability threshold.
    pub threshold: f32,
    /// Minimum duration of speech to trigger a speech event.
    pub min_speech_duration_ms: u32,
    /// Minimum duration of silence to treat as a pause.
    /// A longer duration can help "stitch" together utterances separated by short pauses,
    /// but increases latency. The application may override this default (see issue #61).
    pub min_silence_duration_ms: u32,
    /// The number of samples in a single processing window.
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

    pub silero: SileroConfig,
    pub frame_size_samples: usize,
    pub sample_rate_hz: u32,
}

impl Default for UnifiedVadConfig {
    fn default() -> Self {
        Self {
            mode: VadMode::default(), // Uses Silero by default now

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
