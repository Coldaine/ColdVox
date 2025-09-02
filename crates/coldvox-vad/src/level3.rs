use crate::{
    energy::EnergyCalculator,
    engine::VadEngine,
    state::VadStateMachine,
    threshold::AdaptiveThreshold,
    types::{VadConfig, VadEvent, VadMetrics, VadState},
    VadProcessor,
};

// INTENTIONAL: Level3 VAD is currently DISABLED by default in favor of Silero VAD
// This energy-based VAD implementation is kept for:
// 1. Fallback capability if Silero fails
// 2. Testing and comparison purposes
// 3. Potential future hybrid VAD approaches
// To enable: Set config.level3.enabled = true (see vad/config.rs)
pub struct Level3Vad {
    config: VadConfig,
    energy_calc: EnergyCalculator,
    threshold: AdaptiveThreshold,
    state_machine: VadStateMachine,
    metrics: VadMetrics,
}

impl Level3Vad {
    pub fn new(config: VadConfig) -> Self {
        Self {
            threshold: AdaptiveThreshold::new(&config),
            state_machine: VadStateMachine::new(&config),
            energy_calc: EnergyCalculator::new(),
            metrics: VadMetrics::default(),
            config,
        }
    }

    pub fn builder() -> Level3VadBuilder {
        Level3VadBuilder::new()
    }

    pub fn metrics(&self) -> &VadMetrics {
        &self.metrics
    }

    fn update_metrics(&mut self, energy_db: f32, event: Option<&VadEvent>) {
        self.metrics.frames_processed += 1;
        self.metrics.last_energy_db = energy_db;
        self.metrics.current_noise_floor_db = self.threshold.current_floor();

        let frame_duration_ms = self.config.frame_duration_ms() as u64;

        match self.state_machine.current_state() {
            VadState::Speech => {
                self.metrics.total_speech_ms += frame_duration_ms;
            }
            VadState::Silence => {
                self.metrics.total_silence_ms += frame_duration_ms;
            }
        }

        if let Some(VadEvent::SpeechStart { .. }) = event {
            self.metrics.speech_segments += 1;
        }
    }
}

impl VadProcessor for Level3Vad {
    fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String> {
        if frame.len() != self.config.frame_size_samples {
            return Err(format!(
                "Expected {} samples, got {}",
                self.config.frame_size_samples,
                frame.len()
            ));
        }

        let energy_db = self.energy_calc.calculate_dbfs(frame);

        let current_state = self.state_machine.current_state();
        let is_speech_candidate = match current_state {
            VadState::Silence => self.threshold.should_activate(energy_db),
            VadState::Speech => !self.threshold.should_deactivate(energy_db),
        };

        self.threshold
            .update(energy_db, current_state == VadState::Speech);

        let event = self.state_machine.process(is_speech_candidate, energy_db);

        self.update_metrics(energy_db, event.as_ref());

        Ok(event)
    }

    fn reset(&mut self) {
        self.state_machine.reset();
        self.threshold.reset(self.config.initial_floor_db);
        self.metrics = VadMetrics::default();
    }

    fn current_state(&self) -> VadState {
        self.state_machine.current_state()
    }
}

impl VadEngine for Level3Vad {
    fn process(&mut self, frame: &[i16]) -> Result<Option<VadEvent>, String> {
        VadProcessor::process(self, frame)
    }

    fn reset(&mut self) {
        VadProcessor::reset(self)
    }

    fn current_state(&self) -> VadState {
        VadProcessor::current_state(self)
    }

    fn required_sample_rate(&self) -> u32 {
        self.config.sample_rate_hz
    }

    fn required_frame_size_samples(&self) -> usize {
        self.config.frame_size_samples
    }
}

pub struct Level3VadBuilder {
    config: VadConfig,
}

impl Level3VadBuilder {
    pub fn new() -> Self {
        Self {
            config: VadConfig::default(),
        }
    }

    pub fn onset_threshold(mut self, db: f32) -> Self {
        self.config.onset_threshold_db = db;
        self
    }

    pub fn offset_threshold(mut self, db: f32) -> Self {
        self.config.offset_threshold_db = db;
        self
    }

    pub fn ema_alpha(mut self, alpha: f32) -> Self {
        self.config.ema_alpha = alpha.clamp(0.001, 1.0);
        self
    }

    pub fn speech_debounce_ms(mut self, ms: u32) -> Self {
        self.config.speech_debounce_ms = ms;
        self
    }

    pub fn silence_debounce_ms(mut self, ms: u32) -> Self {
        self.config.silence_debounce_ms = ms;
        self
    }

    pub fn initial_floor_db(mut self, db: f32) -> Self {
        self.config.initial_floor_db = db;
        self
    }

    pub fn frame_size(mut self, samples: usize) -> Self {
        self.config.frame_size_samples = samples;
        self
    }

    pub fn sample_rate(mut self, hz: u32) -> Self {
        self.config.sample_rate_hz = hz;
        self
    }

    pub fn build(self) -> Level3Vad {
        Level3Vad::new(self.config)
    }
}

impl Default for Level3VadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::FRAME_SIZE_SAMPLES;

    #[test]
    fn test_builder_pattern() {
        let vad = Level3Vad::builder()
            .onset_threshold(12.0)
            .offset_threshold(8.0)
            .ema_alpha(0.05)
            .speech_debounce_ms(150)
            .silence_debounce_ms(300)
            .build();

        assert_eq!(vad.config.onset_threshold_db, 12.0);
        assert_eq!(vad.config.offset_threshold_db, 8.0);
        assert_eq!(vad.config.ema_alpha, 0.05);
        assert_eq!(vad.config.speech_debounce_ms, 150);
        assert_eq!(vad.config.silence_debounce_ms, 300);
    }

    #[test]
    fn test_frame_size_validation() {
        let mut vad = Level3Vad::new(VadConfig::default());
        let wrong_size_frame = vec![0i16; 160];

        let result = VadProcessor::process(&mut vad, &wrong_size_frame);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Expected 512 samples"));
    }

    #[test]
    fn test_silence_processing() {
        let mut vad = Level3Vad::new(VadConfig::default());
        let silence_frame = vec![0i16; FRAME_SIZE_SAMPLES];

        for _ in 0..100 {
            let event = VadProcessor::process(&mut vad, &silence_frame).unwrap();
            assert!(event.is_none());
            assert_eq!(VadProcessor::current_state(&vad), VadState::Silence);
        }

        let metrics = vad.metrics();
        assert_eq!(metrics.frames_processed, 100);
        assert_eq!(metrics.speech_segments, 0);
        assert!(metrics.total_silence_ms > 0);
        assert_eq!(metrics.total_speech_ms, 0);
    }

    #[test]
    fn test_speech_detection() {
        let config = VadConfig {
            onset_threshold_db: -30.0,
            offset_threshold_db: -35.0,
            initial_floor_db: -60.0,
            speech_debounce_ms: 60,
            silence_debounce_ms: 80,
            ..Default::default()
        };
        let mut vad = Level3Vad::new(config);

        let speech_frame: Vec<i16> = (0..FRAME_SIZE_SAMPLES)
            .map(|i| {
                let phase = 2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0;
                (phase.sin() * 8000.0) as i16
            })
            .collect();

        let mut speech_started = false;

        for frame_num in 0..10 {
            let event = VadProcessor::process(&mut vad, &speech_frame).unwrap();

            if let Some(VadEvent::SpeechStart { .. }) = event {
                speech_started = true;
                // With 60ms speech debounce and ~32ms frames, should trigger by frame 2 (60/32 = 2 frames)
                assert!(frame_num >= 1);
            }
        }

        assert!(speech_started);
        assert_eq!(VadProcessor::current_state(&vad), VadState::Speech);

        let silence_frame = vec![0i16; FRAME_SIZE_SAMPLES];
        let mut speech_ended = false;

        for _ in 0..10 {
            let event = VadProcessor::process(&mut vad, &silence_frame).unwrap();

            if let Some(VadEvent::SpeechEnd { duration_ms, .. }) = event {
                speech_ended = true;
                assert!(duration_ms > 0);
            }
        }

        assert!(speech_ended);
        assert_eq!(VadProcessor::current_state(&vad), VadState::Silence);
    }

    #[test]
    fn test_adaptive_threshold() {
        let config = VadConfig {
            ema_alpha: 0.1,
            initial_floor_db: -50.0,
            ..Default::default()
        };
        let mut vad = Level3Vad::new(config);

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let noisy_background: Vec<i16> = (0..FRAME_SIZE_SAMPLES)
            .map(|_| (rng.gen::<f32>() - 0.5) * 1000.0)
            .map(|x| x as i16)
            .collect();

        let initial_floor = vad.threshold.current_floor();

        for _ in 0..50 {
            VadProcessor::process(&mut vad, &noisy_background).unwrap();
        }

        let adapted_floor = vad.threshold.current_floor();
        assert_ne!(initial_floor, adapted_floor);
    }

    #[test]
    fn test_reset_functionality() {
        let mut vad = Level3Vad::new(VadConfig::default());

        let speech_frame = vec![16000i16; FRAME_SIZE_SAMPLES];
        for _ in 0..20 {
            VadProcessor::process(&mut vad, &speech_frame).unwrap();
        }

        assert!(vad.metrics().frames_processed > 0);

        VadProcessor::reset(&mut vad);

        assert_eq!(vad.metrics().frames_processed, 0);
        assert_eq!(VadProcessor::current_state(&vad), VadState::Silence);
    }
}
