#[cfg(test)]
mod tests {
    use coldvox_audio::detector::SilenceDetector;
    use coldvox_vad::constants::FRAME_SIZE_SAMPLES;
    use crate::common::test_utils::*;
    use std::time::Duration;

    #[test]
    fn test_rms_calculation() {
        let detector = SilenceDetector::new(100);

        // Test with known values
        let samples = vec![100, -100, 100, -100];
        let is_silent = detector.is_silent(&samples);
        assert!(!is_silent, "RMS of ±100 should not be silent with threshold 100");

        // Test with zeros
        let samples = vec![0; 100];
        let is_silent = detector.is_silent(&samples);
        assert!(is_silent, "Zero samples should be silent");

        // Test with single spike
        let mut samples = vec![0; 100];
        samples[50] = 1000;
        let is_silent = detector.is_silent(&samples);
        assert!(!is_silent, "Single spike should affect RMS");
    }

    #[test]
    fn test_silence_threshold_50() {
        let detector = SilenceDetector::new(50);

        // Generate samples just below threshold
        let quiet_samples = generate_noise(100, 40);
        assert!(detector.is_silent(&quiet_samples),
            "Samples with amplitude 40 should be silent with threshold 50");

        // Generate samples just above threshold
        let loud_samples = generate_noise(100, 60);
        assert!(!detector.is_silent(&loud_samples),
            "Samples with amplitude 60 should not be silent with threshold 50");
    }

    #[test]
    fn test_silence_threshold_500() {
        let detector = SilenceDetector::new(500);

        // Normal speech levels (~1000-3000) should not be silent
        let speech_samples = generate_sine_wave(200.0, 16000, 100);
        assert!(!detector.is_silent(&speech_samples),
            "Speech-level audio should not be silent with threshold 500");

        // Whisper levels (~100-400) should be silent
        let whisper_samples = generate_noise(100, 300);
        assert!(detector.is_silent(&whisper_samples),
            "Whisper-level audio should be silent with threshold 500");
    }

    #[test]
    fn test_continuous_silence_tracking() {
        let mut detector = SilenceDetector::new(100);
        let silent_samples = generate_silence(FRAME_SIZE_SAMPLES); // ~32ms at 16kHz

        // Track 3 seconds of silence (~94 frames of 32ms each)
        let mut continuous_silent = Duration::ZERO;
        let frame_duration = Duration::from_millis(32);

        for _ in 0..94 {
            if detector.is_silent(&silent_samples) {
                continuous_silent += frame_duration;
            } else {
                continuous_silent = Duration::ZERO;
            }
        }

        assert!(continuous_silent >= Duration::from_secs(3),
            "Should track 3 seconds of continuous silence");
    }

    #[test]
    fn test_activity_interrupts_silence() {
        let detector = SilenceDetector::new(100);
        let generator = TestDataGenerator::new(16000);

        // Pattern: 1s silence, 0.1s activity, 1s silence
        let pattern = vec![
            (false, 1000),  // Silent
            (true, 100),    // Active
            (false, 1000),  // Silent
        ];

        let samples = generator.generate_activity_pattern(&pattern);

        // Process in ~32ms frames
        let frame_size = FRAME_SIZE_SAMPLES; // ~32ms at 16kHz
        let mut max_continuous_silence = 0;
        let mut current_silence_count = 0;

        for chunk in samples.chunks(frame_size) {
            if chunk.len() == frame_size {
                if detector.is_silent(chunk) {
                    current_silence_count += 1;
                    max_continuous_silence = max_continuous_silence.max(current_silence_count);
                } else {
                    current_silence_count = 0;
                }
            }
        }

        // Should not have more than 1 second of continuous silence
        assert!(max_continuous_silence <= 50, // 50 * 20ms = 1 second
            "Activity should interrupt silence tracking");
    }

    #[test]
    fn test_edge_cases() {
        let detector = SilenceDetector::new(100);

        // Empty samples
        let empty: Vec<i16> = vec![];
        assert!(detector.is_silent(&empty), "Empty samples should be silent");

        // Single sample
        let single = vec![50];
        assert!(detector.is_silent(&single), "Single quiet sample should be silent");

        let single_loud = vec![500];
        assert!(!detector.is_silent(&single_loud), "Single loud sample should not be silent");

        // Max values
        let max_positive = vec![i16::MAX; 10];
        assert!(!detector.is_silent(&max_positive), "Max positive values should not be silent");

        let max_negative = vec![i16::MIN; 10];
        assert!(!detector.is_silent(&max_negative), "Max negative values should not be silent");

        // Alternating max values
        let alternating = vec![i16::MAX, i16::MIN, i16::MAX, i16::MIN];
        assert!(!detector.is_silent(&alternating), "Alternating max values should not be silent");
    }

    #[test]
    fn test_threshold_boundary_conditions() {
        // Test threshold of 0 (everything except absolute silence is active)
        let detector_zero = SilenceDetector::new(0);
        assert!(detector_zero.is_silent(&[0, 0, 0]));
        assert!(!detector_zero.is_silent(&[1, 0, 0]));

        // Test very high threshold
        let detector_high = SilenceDetector::new(10000);
        let loud_samples = generate_sine_wave(440.0, 16000, 100);
        assert!(detector_high.is_silent(&loud_samples),
            "Even loud audio should be 'silent' with very high threshold");
    }

    #[test]
    fn test_real_world_scenarios() {
        let detector = SilenceDetector::new(150); // Typical threshold

        // Simulate microphone background noise
        let bg_noise = generate_noise(1000, 80);
        assert!(detector.is_silent(&bg_noise),
            "Typical background noise should be detected as silence");

        // Simulate speech
        let speech = generate_sine_wave(250.0, 16000, 100);
        assert!(!detector.is_silent(&speech),
            "Speech frequencies should not be detected as silence");

        // Simulate breathing/wind noise
        let breathing = generate_noise(1000, 120);
        assert!(detector.is_silent(&breathing),
            "Breathing noise should be detected as silence with typical threshold");
    }
}
