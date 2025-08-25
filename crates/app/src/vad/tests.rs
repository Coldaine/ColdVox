use crate::vad::{Level3Vad, VadConfig, VadEvent, VadProcessor};

mod test_utils {
    pub fn generate_silence(samples: usize) -> Vec<i16> {
        vec![0; samples]
    }

    pub fn generate_noise(samples: usize, amplitude: f32) -> Vec<i16> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..samples)
            .map(|_| (rng.gen::<f32>() - 0.5) * amplitude * 2.0)
            .map(|x| x as i16)
            .collect()
    }

    pub fn generate_sine_wave(samples: usize, frequency: f32, amplitude: f32, sample_rate: f32) -> Vec<i16> {
        (0..samples)
            .map(|i| {
                let phase = 2.0 * std::f32::consts::PI * frequency * i as f32 / sample_rate;
                (phase.sin() * amplitude) as i16
            })
            .collect()
    }

    pub fn generate_chirp(samples: usize, start_freq: f32, end_freq: f32, amplitude: f32, sample_rate: f32) -> Vec<i16> {
        (0..samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                let freq = start_freq + (end_freq - start_freq) * t * sample_rate / samples as f32;
                let phase = 2.0 * std::f32::consts::PI * freq * t;
                (phase.sin() * amplitude) as i16
            })
            .collect()
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use test_utils::*;

    #[test]
    fn test_complete_speech_cycle() {
        let config = VadConfig {
            onset_threshold_db: -25.0,
            offset_threshold_db: -30.0,
            initial_floor_db: -50.0,
            speech_debounce_ms: 100,
            silence_debounce_ms: 200,
            ema_alpha: 0.05,
            ..Default::default()
        };

        let mut vad = Level3Vad::new(config);

        let silence = generate_silence(320);
        let speech = generate_sine_wave(320, 440.0, 8000.0, 16000.0);
        let noise = generate_noise(320, 500.0);

        let mut events = Vec::new();

        for _ in 0..10 {
            if let Some(event) = vad.process(&silence).unwrap() {
                events.push(event);
            }
        }

        for _ in 0..10 {
            if let Some(event) = vad.process(&speech).unwrap() {
                events.push(event);
            }
        }

        for _ in 0..5 {
            if let Some(event) = vad.process(&noise).unwrap() {
                events.push(event);
            }
        }

        for _ in 0..15 {
            if let Some(event) = vad.process(&silence).unwrap() {
                events.push(event);
            }
        }

        assert_eq!(events.len(), 2);
        
        match &events[0] {
            VadEvent::SpeechStart { .. } => {},
            _ => panic!("Expected SpeechStart as first event"),
        }
        
        match &events[1] {
            VadEvent::SpeechEnd { duration_ms, .. } => {
                assert!(*duration_ms > 0);
            },
            _ => panic!("Expected SpeechEnd as second event"),
        }
    }

    #[test]
    fn test_noise_adaptation() {
        let config = VadConfig {
            ema_alpha: 0.1,
            initial_floor_db: -60.0,
            onset_threshold_db: 15.0,
            offset_threshold_db: 12.0,
            ..Default::default()
        };

        let mut vad = Level3Vad::new(config);

        let quiet_noise = generate_noise(320, 100.0);
        let loud_noise = generate_noise(320, 2000.0);
        let very_loud_speech = generate_sine_wave(320, 440.0, 16000.0, 16000.0);

        for _ in 0..20 {
            vad.process(&quiet_noise).unwrap();
        }
        let floor_after_quiet = vad.metrics().current_noise_floor_db;

        for _ in 0..20 {
            vad.process(&loud_noise).unwrap();
        }
        let floor_after_loud = vad.metrics().current_noise_floor_db;

        assert!(floor_after_loud > floor_after_quiet);

        let mut speech_detected = false;
        for _ in 0..10 {
            if let Some(VadEvent::SpeechStart { .. }) = vad.process(&very_loud_speech).unwrap() {
                speech_detected = true;
                break;
            }
        }
        
        assert!(speech_detected);
    }

    #[test]
    fn test_different_audio_types() {
        let mut vad = Level3Vad::new(VadConfig::default());

        let test_cases = vec![
            ("Sine wave", generate_sine_wave(320, 440.0, 8000.0, 16000.0)),
            ("Chirp", generate_chirp(320, 200.0, 2000.0, 6000.0, 16000.0)),
            ("White noise", generate_noise(320, 4000.0)),
            ("Silence", generate_silence(320)),
        ];

        for (name, frame) in test_cases {
            let result = vad.process(&frame);
            assert!(result.is_ok(), "Failed to process {}: {:?}", name, result);
        }
    }

    #[test]
    fn test_metrics_collection() {
        let config = VadConfig {
            speech_debounce_ms: 60,
            silence_debounce_ms: 60,
            ..Default::default()
        };
        let mut vad = Level3Vad::new(config);

        let silence = generate_silence(320);
        let speech = generate_sine_wave(320, 440.0, 8000.0, 16000.0);

        for _ in 0..10 {
            vad.process(&silence).unwrap();
        }

        for _ in 0..10 {
            vad.process(&speech).unwrap();
        }

        for _ in 0..10 {
            vad.process(&silence).unwrap();
        }

        let metrics = vad.metrics();
        assert_eq!(metrics.frames_processed, 30);
        assert!(metrics.total_silence_ms > 0);
        assert!(metrics.total_speech_ms > 0);
        assert!(metrics.speech_segments >= 1);
    }

    #[test]
    fn test_long_duration_stability() {
        let mut vad = Level3Vad::new(VadConfig::default());

        let silence = generate_silence(320);
        let speech = generate_sine_wave(320, 440.0, 8000.0, 16000.0);

        for cycle in 0..100 {
            for _ in 0..50 {
                let frame = if cycle % 10 < 3 { &speech } else { &silence };
                let result = vad.process(frame);
                assert!(result.is_ok());
            }
        }

        let metrics = vad.metrics();
        assert_eq!(metrics.frames_processed, 5000);
        assert!(metrics.speech_segments > 0);
    }
}