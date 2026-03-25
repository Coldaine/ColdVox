use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::Arc;

/// Analyzes audio to detect if the speaker is off-axis.
///
/// When a speaker turns their head away from the microphone, high frequencies
/// (4-8kHz) drop off significantly more than mid frequencies (500Hz-2kHz).
/// This analyzer measures this effect and classifies the audio as on-axis or off-axis.
pub struct SpectralAnalyzer {
    sample_rate: u32,
    off_axis_threshold: f32,
    last_spectral_ratio: f32,
    /// Pre-allocated buffer for i16 → f32 conversion (real-time safe, no allocations)
    conversion_buffer: Vec<Complex<f32>>,
    planner: FftPlanner<f32>,
}

impl SpectralAnalyzer {
    /// Create a new spectral analyzer.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Sample rate in Hz
    /// * `off_axis_threshold` - Spectral ratio threshold for off-axis detection (default: 0.3)
    pub fn new(sample_rate: u32, off_axis_threshold: f32) -> Self {
        let conversion_buffer = Vec::with_capacity(2048);
        Self {
            sample_rate,
            off_axis_threshold,
            last_spectral_ratio: 1.0, // Start with neutral ratio
            conversion_buffer,
            planner: FftPlanner::new(),
        }
    }

    /// Detect if audio is off-axis by analyzing frequency spectrum.
    ///
    /// Returns `true` if speaker appears to be off-axis from the microphone.
    ///
    /// # Arguments
    ///
    /// * `samples` - Audio samples as i16 PCM data
    pub fn detect_off_axis(&mut self, samples: &[i16]) -> bool {
        // Need at least 512 samples for meaningful FFT
        if samples.len() < 512 {
            return false;
        }

        // We will process a power of 2 number of samples for FFT, up to 2048
        let mut len = 1;
        while len * 2 <= samples.len() && len * 2 <= 2048 {
            len *= 2;
        }

        self.conversion_buffer.clear();
        for &sample in samples.iter().take(len) {
            // Apply Hann window and scale
            // Hann window: 0.5 * (1 - cos(2 * PI * i / (N - 1)))
            let i = self.conversion_buffer.len();
            let window =
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (len - 1) as f32).cos());
            self.conversion_buffer.push(Complex {
                re: (sample as f32 / 32768.0) * window,
                im: 0.0,
            });
        }

        let fft = self.planner.plan_fft_forward(len);
        fft.process(&mut self.conversion_buffer);

        // Calculate magnitudes (we only need the first half, up to Nyquist)
        let magnitudes: Vec<f32> = self
            .conversion_buffer
            .iter()
            .take(len / 2)
            .map(|c| c.norm_sqr())
            .collect();

        // Calculate spectral ratio
        self.last_spectral_ratio = self.calculate_spectral_ratio(&magnitudes, len);

        self.last_spectral_ratio < self.off_axis_threshold
    }

    /// Get the last computed spectral ratio.
    pub fn last_spectral_ratio(&self) -> f32 {
        self.last_spectral_ratio
    }

    /// Calculate spectral ratio: high_freq_energy / mid_freq_energy.
    fn calculate_spectral_ratio(&self, magnitudes: &[f32], fft_len: usize) -> f32 {
        const HIGH_FREQ_START: f32 = 4000.0; // 4 kHz
        const HIGH_FREQ_END: f32 = 8000.0; // 8 kHz
        const MID_FREQ_START: f32 = 500.0; // 500 Hz
        const MID_FREQ_END: f32 = 2000.0; // 2 kHz

        let hz_per_bin = self.sample_rate as f32 / fft_len as f32;

        let high_freq_energy =
            Self::average_energy_in_band(magnitudes, hz_per_bin, HIGH_FREQ_START, HIGH_FREQ_END);
        let mid_freq_energy =
            Self::average_energy_in_band(magnitudes, hz_per_bin, MID_FREQ_START, MID_FREQ_END);

        if mid_freq_energy < 1e-10 {
            return 0.0; // Silence or very quiet
        }

        high_freq_energy / mid_freq_energy
    }

    fn average_energy_in_band(
        magnitudes: &[f32],
        hz_per_bin: f32,
        start_hz: f32,
        end_hz: f32,
    ) -> f32 {
        let mut sum = 0.0;
        let mut count = 0;

        let start_bin = (start_hz / hz_per_bin).floor() as usize;
        let end_bin = (end_hz / hz_per_bin).ceil() as usize;

        for bin in start_bin..=end_bin {
            if bin < magnitudes.len() {
                sum += magnitudes[bin];
                count += 1;
            }
        }

        if count == 0 {
            0.0
        } else {
            sum / count as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a test signal with specific frequency.
    fn generate_sine_wave(freq: f32, sample_rate: u32, duration_samples: usize) -> Vec<i16> {
        (0..duration_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let sample = (2.0 * std::f32::consts::PI * freq * t).sin();
                (sample * 30000.0) as i16 // Scale to reasonable amplitude
            })
            .collect()
    }

    /// Generate white noise (all frequencies equal).
    fn generate_white_noise(duration_samples: usize) -> Vec<i16> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        (0..duration_samples)
            .map(|i| {
                i.hash(&mut hasher);
                let hash = hasher.finish();
                let unsigned_val = (hash % 60000) as u32;
                (unsigned_val as i32 - 30000) as i16
            })
            .collect()
    }

    #[test]
    fn test_analyzer_creation() {
        let _analyzer = SpectralAnalyzer::new(16000, 0.3);
    }

    #[test]
    fn test_white_noise_ratio() {
        let mut analyzer = SpectralAnalyzer::new(16000, 0.3);

        // White noise should have balanced frequencies
        let noise = generate_white_noise(1024);

        let is_off_axis = analyzer.detect_off_axis(&noise);

        // White noise should NOT be detected as off-axis
        // (ratio should be close to 1.0)
        assert!(!is_off_axis, "White noise should not be off-axis");
        assert!(
            analyzer.last_spectral_ratio() > 0.3,
            "White noise ratio should be > 0.3, got {}",
            analyzer.last_spectral_ratio()
        );
    }

    #[test]
    fn test_high_freq_signal() {
        let mut analyzer = SpectralAnalyzer::new(16000, 0.3);

        // Generate 6kHz sine wave (high frequency)
        let samples = generate_sine_wave(6000.0, 16000, 1024);

        let _is_off_axis = analyzer.detect_off_axis(&samples);

        // Pure high-freq signal without mid-freq content may have very high ratio
        // (or undefined if no mid-freq energy). This is expected - real speech has both.
        // Just verify it doesn't crash
        let _ratio = analyzer.last_spectral_ratio();
    }

    #[test]
    fn test_low_freq_signal() {
        let mut analyzer = SpectralAnalyzer::new(16000, 0.3);

        // Generate 1kHz sine wave (mid frequency)
        let samples = generate_sine_wave(1000.0, 16000, 1024);

        let is_off_axis = analyzer.detect_off_axis(&samples);

        // Pure mid-freq signal might look off-axis
        // (no energy in high freqs)
        assert!(analyzer.last_spectral_ratio() < 0.5);
    }

    #[test]
    fn test_silence_handling() {
        let mut analyzer = SpectralAnalyzer::new(16000, 0.3);

        let silence = vec![0i16; 1024];

        let _is_off_axis = analyzer.detect_off_axis(&silence);

        // Silence should not crash or panic
        // Result depends on implementation (might be off-axis or not)
    }

    #[test]
    fn test_short_buffer() {
        let mut analyzer = SpectralAnalyzer::new(16000, 0.3);

        // Too short for meaningful FFT
        let short = vec![0i16; 128];

        let is_off_axis = analyzer.detect_off_axis(&short);

        // Should handle gracefully (return false)
        assert!(!is_off_axis);
    }

    #[test]
    fn test_mixed_frequencies() {
        let mut analyzer = SpectralAnalyzer::new(16000, 0.3);

        // Mix of mid and high frequencies (on-axis speech)
        let mid_freq = generate_sine_wave(1000.0, 16000, 1024);
        let high_freq = generate_sine_wave(5000.0, 16000, 1024);

        let mixed: Vec<i16> = mid_freq
            .iter()
            .zip(high_freq.iter())
            .map(|(&a, &b)| ((a as i32 + b as i32) / 2) as i16)
            .collect();

        let is_off_axis = analyzer.detect_off_axis(&mixed);

        // Mixed frequencies should look on-axis
        assert!(!is_off_axis, "Mixed frequencies should be on-axis");
    }
}
