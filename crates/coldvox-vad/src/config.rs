use serde::{Deserialize, Serialize};

use super::constants::{FRAME_SIZE_SAMPLES, SAMPLE_RATE_HZ};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VadMode {
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
    /// Production VAD configuration tuned for dictation quality.
    ///
    /// This configuration is optimized for voice dictation applications and represents
    /// carefully tuned values that differ from the conservative defaults.
    ///
    /// # Key Parameters
    ///
    /// - **threshold: 0.1** (vs default 0.3)
    ///   - More sensitive speech detection
    ///   - Catches quieter speech and natural volume variations
    ///   - Better for users with varying speaking volumes
    ///
    /// - **min_speech_duration_ms: 100** (vs default 250)
    ///   - Shorter minimum speech duration
    ///   - Captures brief utterances and short words
    ///   - Reduces false negatives for quick speech
    ///
    /// - **min_silence_duration_ms: 500** (vs default 100) - **CRITICAL**
    ///   - Much longer silence tolerance before ending utterance
    ///   - "Stitches together" speech segments separated by natural pauses
    ///   - Prevents fragmentation of single logical utterances
    ///   - See issue #61 for detailed rationale
    ///
    /// # Rationale for 500ms Silence Duration
    ///
    /// Shorter silence durations (e.g., 100-200ms) cause the VAD to split a single
    /// logical utterance into multiple speech events during natural pauses in speech.
    /// This fragmentation leads to:
    ///
    /// - Disjointed transcriptions (sentence split mid-thought)
    /// - Loss of context for STT engine (can't understand full sentence)
    /// - Increased overhead (multiple STT start/stop cycles)
    /// - Poor user experience (text appears in fragments)
    ///
    /// The 500ms duration acts as a buffer that stitches together speech segments,
    /// resulting in more coherent, sentence-like chunks being sent to the STT engine.
    /// This significantly improves transcription quality at the cost of slight
    /// additional latency (~500ms delay after user stops speaking).
    ///
    /// For dictation applications, this trade-off strongly favors quality over
    /// minimal latency.
    ///
    /// # Usage
    ///
    /// Use this configuration in production code and tests to ensure consistent
    /// VAD behavior:
    ///
    /// ```rust
    /// let vad_cfg = UnifiedVadConfig::production_default();
    /// ```
    ///
    /// Tests should use this instead of `Default::default()` to match production
    /// behavior and avoid configuration drift.
    pub fn production_default() -> Self {
        Self {
            mode: VadMode::Silero,
            frame_size_samples: FRAME_SIZE_SAMPLES,
            sample_rate_hz: SAMPLE_RATE_HZ,
            silero: SileroConfig {
                threshold: 0.1,
                min_speech_duration_ms: 100,
                min_silence_duration_ms: 500,
                window_size_samples: FRAME_SIZE_SAMPLES,
            },
        }
    }

    pub fn frame_duration_ms(&self) -> f32 {
        (self.frame_size_samples as f32 * 1000.0) / self.sample_rate_hz as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_default_values() {
        let config = UnifiedVadConfig::production_default();

        // Verify VAD mode
        assert_eq!(config.mode, VadMode::Silero);

        // Verify frame parameters
        assert_eq!(config.frame_size_samples, FRAME_SIZE_SAMPLES);
        assert_eq!(config.sample_rate_hz, SAMPLE_RATE_HZ);

        // Verify Silero configuration - these are the CRITICAL production values
        assert_eq!(
            config.silero.threshold, 0.1,
            "Production threshold must be 0.1 for sensitive speech detection"
        );
        assert_eq!(
            config.silero.min_speech_duration_ms, 100,
            "Production min_speech_duration must be 100ms to capture brief utterances"
        );
        assert_eq!(
            config.silero.min_silence_duration_ms, 500,
            "Production min_silence_duration must be 500ms to stitch speech segments (issue #61)"
        );
        assert_eq!(config.silero.window_size_samples, FRAME_SIZE_SAMPLES);
    }

    #[test]
    fn test_production_differs_from_default() {
        let production = UnifiedVadConfig::production_default();
        let default = UnifiedVadConfig::default();

        // These should differ - production is tuned, default is conservative
        assert_ne!(
            production.silero.threshold, default.silero.threshold,
            "Production and default thresholds should differ"
        );
        assert_ne!(
            production.silero.min_silence_duration_ms, default.silero.min_silence_duration_ms,
            "Production and default min_silence_duration should differ (500ms vs 100ms)"
        );
    }
}
