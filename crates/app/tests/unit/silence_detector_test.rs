#[cfg(test)]
mod tests {
    use coldvox_audio::detector::SilenceDetector;
    use coldvox_vad::constants::FRAME_SIZE_SAMPLES;
    use crate::common::test_utils::*;

    /// Comprehensive test of RMS calculation and threshold behavior.
    ///
    /// Tests the core algorithm: RMS calculation should correctly classify
    /// audio as silent or active based on the threshold. This covers:
    /// - Basic RMS calculation accuracy
    /// - Threshold boundary conditions
    /// - Edge cases (empty, single sample, max values)
    #[test]
    fn test_silence_detection_algorithm() {
        // Test various threshold levels with different audio patterns

        // Low threshold (50) - sensitive detection
        let sensitive_detector = SilenceDetector::new(50);
        assert!(sensitive_detector.is_silent(&generate_noise(100, 40)),
            "Quiet noise (40) should be silent with threshold 50");
        assert!(!sensitive_detector.is_silent(&generate_noise(100, 60)),
            "Louder noise (60) should not be silent with threshold 50");

        // Medium threshold (150) - typical production setting
        let typical_detector = SilenceDetector::new(150);
        assert!(typical_detector.is_silent(&generate_noise(1000, 80)),
            "Background noise (80) should be silent with typical threshold");
        assert!(!typical_detector.is_silent(&generate_sine_wave(250.0, 16000, 100)),
            "Speech frequencies should not be silent with typical threshold");

        // High threshold (500) - only detect loud audio
        let high_detector = SilenceDetector::new(500);
        assert!(high_detector.is_silent(&generate_noise(100, 300)),
            "Whisper-level audio (300) should be silent with high threshold");
        assert!(!high_detector.is_silent(&generate_sine_wave(200.0, 16000, 100)),
            "Normal speech should not be silent with high threshold");

        // Edge case: Zero threshold (everything except true silence is active)
        let zero_detector = SilenceDetector::new(0);
        assert!(zero_detector.is_silent(&[0, 0, 0]), "True silence should be silent");
        assert!(!zero_detector.is_silent(&[1, 0, 0]), "Any non-zero should not be silent");

        // Edge case: Empty and single samples
        let detector = SilenceDetector::new(100);
        assert!(detector.is_silent(&[]), "Empty samples should be silent");
        assert!(detector.is_silent(&[50]), "Single quiet sample should be silent");
        assert!(!detector.is_silent(&[500]), "Single loud sample should not be silent");

        // Edge case: Max values
        assert!(!detector.is_silent(&[i16::MAX; 10]), "Max positive should not be silent");
        assert!(!detector.is_silent(&[i16::MIN; 10]), "Max negative should not be silent");
        assert!(!detector.is_silent(&vec![i16::MAX, i16::MIN, i16::MAX, i16::MIN]),
            "Alternating max should not be silent");
    }

    /// Test continuous silence tracking with activity interruption.
    ///
    /// Tests user-facing behavior: the detector should correctly identify
    /// periods of continuous silence and reset when activity is detected.
    /// This is the behavior that VAD and audio pipeline depend on.
    #[test]
    fn test_continuous_silence_tracking() {
        let detector = SilenceDetector::new(100);
        let generator = TestDataGenerator::new(16000);

        // Pattern: 2s silence, 0.1s activity, 2s silence
        // This simulates real-world scenario: silence → speech → silence
        let pattern = vec![
            (false, 2000),  // 2 seconds silence
            (true, 100),    // 0.1 seconds activity (interruption)
            (false, 2000),  // 2 seconds silence
        ];

        let samples = generator.generate_activity_pattern(&pattern);
        let frame_size = FRAME_SIZE_SAMPLES; // ~32ms at 16kHz

        // Track silence periods
        let mut max_continuous_silence_frames = 0;
        let mut current_silence_frames = 0;

        for chunk in samples.chunks(frame_size) {
            if chunk.len() == frame_size {
                if detector.is_silent(chunk) {
                    current_silence_frames += 1;
                    max_continuous_silence_frames =
                        max_continuous_silence_frames.max(current_silence_frames);
                } else {
                    current_silence_frames = 0; // Reset on activity
                }
            }
        }

        // Verify silence tracking behavior
        let max_silence_ms = max_continuous_silence_frames * 32;

        // Should have detected continuous silence, but not more than 2 seconds
        // (because activity interrupts it)
        assert!(max_silence_ms >= 1900 && max_silence_ms <= 2100,
            "Should track ~2 seconds of continuous silence (got {} ms), \
             activity should interrupt tracking",
            max_silence_ms);
    }
}
