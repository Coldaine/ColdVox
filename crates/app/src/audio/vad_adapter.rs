#[cfg(feature = "vad")]
use voice_activity_detector::VoiceActivityDetector;

#[derive(Debug, Clone, Copy)]
pub struct ColdVoxVadConfig {
    pub energy_threshold: f32,
    pub vad_threshold: f32,
}

impl Default for ColdVoxVadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.0,
            vad_threshold: 0.5,
        }
    }
}

pub struct ColdVoxVAD {
    #[cfg(feature = "vad")]
    detector: VoiceActivityDetector,
    cfg: ColdVoxVadConfig,
    frame_buffer: Vec<i16>,
}

impl ColdVoxVAD {
    pub fn new(cfg: ColdVoxVadConfig) -> Result<Self, String> {
        #[cfg(feature = "vad")]
        let detector = VoiceActivityDetector::builder()
            .sample_rate(16000)
            .build()
            .map_err(|e| format!("build VAD: {e}"))?;
        Ok(Self {
            #[cfg(feature = "vad")]
            detector,
            cfg,
            frame_buffer: Vec::new(),
        })
    }

    pub fn process_coldvox_frame(&mut self, frame: &[i16]) -> Result<bool, String> {
        // Basic buffering hook for future use
        self.frame_buffer.clear();
        self.frame_buffer.extend_from_slice(frame);

        let mut is_voice = false;
        #[cfg(feature = "vad")]
        {
            // The upstream expects 16 kHz PCM i16; caller should ensure rate
            let probability = self
                .detector
                .predict(frame.iter().copied())
                .map_err(|e| format!("vad error: {e}"))?;
            is_voice = probability >= self.cfg.vad_threshold;
        }
        if is_voice || self.check_energy_fallback(frame) {
            return Ok(true);
        }
        Ok(false)
    }

    fn check_energy_fallback(&self, frame: &[i16]) -> bool {
        if self.cfg.energy_threshold <= 0.0 {
            return false;
        }
        let sum: f32 = frame.iter().map(|&s| {
            let v = s as f32 / 32768.0;
            v * v
        })
        .sum();
        let rms = (sum / (frame.len().max(1) as f32)).sqrt();
        rms >= self.cfg.energy_threshold
    }
}