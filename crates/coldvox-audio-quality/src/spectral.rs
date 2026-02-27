//! Spectral analysis for off-axis detection.

use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit, FrequencySpectrum};
use spectrum_analyzer::scaling::divide_by_N;
use tracing;

/// Spectral analyzer for detecting off-axis audio via frequency analysis.
///
/// When a speaker moves off-axis from a cardioid microphone, high frequencies
/// (4-8kHz) drop off significantly more than mid frequencies (500Hz-2kHz).
/// This analyzer measures this effect and classifies the audio as on-axis or off-axis.
pub struct SpectralAnalyzer {
    sample_rate: u32,
    off_axis_threshold: f32,
    last_spectral_ratio: f32,
    /// Pre-allocated buffer for i16 → f32 conversion (real-time safe, no allocations)
    conversion_buffer: Vec<f32>,
}

impl SpectralAnalyzer {
    /// Create a new spectral analyzer.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Sample rate in Hz
    /// * `off_axis_threshold` - Spectral ratio threshold for off-axis detection (default: 0.3)
    pub fn new(sample_rate: u32, off_axis_threshold: f32) -> Self {
        // Pre-allocate conversion buffer (up to 2048 samples)
        // This avoids allocations in the hot path (detect_off_axis)
        let conversion_buffer = Vec::with_capacity(2048);

        Self {
            sample_rate,
            off_axis_threshold,
            last_spectral_ratio: 1.0, // Start with neutral ratio
            conversion_buffer,
        }
    }

    /// Detect if audio is off-axis by analyzing frequency spectrum.
    ///
    /// Returns `true` if speaker appears to be off-axis from the microphone.
    ///
    /// # Arguments
    ///
    /// * `samples` - Audio samples as i16 PCM data
    ///
    /// # Algorithm
    ///
    /// 1. Compute FFT of audio samples
    /// 2. Calculate average energy in high-freq band (4-8kHz)
    /// 3. Calculate average energy in mid-freq band (500Hz-2kHz)
    /// 4. Compute ratio: high_freq / mid_freq
    /// 5. If ratio < threshold (0.3), classify as off-axis
    pub fn detect_off_axis(&mut self, samples: &[i16]) -> bool {
        // Need at least 512 samples for meaningful FFT
        if samples.len() < 512 {
            return false;
        }

        // Compute frequency spectrum
        let spectrum = match self.compute_spectrum(samples) {
            Some(s) => s,
            None => return false,
        };

        // Calculate spectral ratio
        self.last_spectral_ratio = self.calculate_spectral_ratio(&spectrum);

        // Threshold-based classification
        // Ratio below threshold indicates significant high-frequency rolloff (off-axis)
        self.last_spectral_ratio < self.off_axis_threshold
    }

    /// Get the last computed spectral ratio.
    ///
    /// This can be used for debugging or visualization.
    pub fn last_spectral_ratio(&self) -> f32 {
        self.last_spectral_ratio
    }

    /// Compute frequency spectrum from audio samples.
    ///
    /// **Real-time safe:** Uses pre-allocated buffer, no allocations on hot path.
    fn compute_spectrum(&mut self, samples: &[i16]) -> Option<FrequencySpectrum> {
        // Reuse pre-allocated buffer for conversion (real-time safe, no allocation)
        self.conversion_buffer.clear();

        // Convert i16 samples to f32 for FFT
        // Note: If samples.len() > capacity, this will allocate once, then reuse
        for &sample in samples.iter().take(2048) {
            self.conversion_buffer.push(sample as f32 / 32768.0);
        }

        // Compute FFT
        let samples_for_fft = &self.conversion_buffer[..];

        match samples_fft_to_spectrum(
            samples_for_fft,
            self.sample_rate,
            FrequencyLimit::All,
            Some(&divide_by_N),
        ) {
            Ok(spectrum) => Some(spectrum),
            Err(e) => {
                tracing::debug!(
                    error = ?e,
                    sample_count = samples_for_fft.len(),
                    "FFT computation failed, skipping off-axis detection for this frame"
                );
                None
            }
        }
    }

    /// Calculate spectral ratio: high_freq_energy / mid_freq_energy.
    ///
    /// High-freq band: 4-8kHz (consonants, sibilants)
    /// Mid-freq band: 500Hz-2kHz (fundamental speech frequencies)
    fn calculate_spectral_ratio(&self, spectrum: &FrequencySpectrum) -> f32 {
        // Define frequency bands
        const HIGH_FREQ_START: f32 = 4000.0; // 4 kHz
        const HIGH_FREQ_END: f32 = 8000.0;   // 8 kHz
        const MID_FREQ_START: f32 = 500.0;   // 500 Hz
        const MID_FREQ_END: f32 = 2000.0;    // 2 kHz

        // Calculate average energy in each band
        let high_freq_energy = Self::average_energy_in_band(
            spectrum,
            HIGH_FREQ_START,
            HIGH_FREQ_END,
        );

        let mid_freq_energy = Self::average_energy_in_band(
            spectrum,
            MID_FREQ_START,
            MID_FREQ_END,
        );

        // Avoid division by zero
        if mid_freq_energy < 1e-10 {
            return 0.0; // Silence or very quiet
        }

        // Calculate ratio
        high_freq_energy / mid_freq_energy
    }

    /// Calculate average energy in a frequency band.
    fn average_energy_in_band(
        spectrum: &FrequencySpectrum,
        start_hz: f32,
        end_hz: f32,
    ) -> f32 {
        let mut sum = 0.0;
        let mut count = 0;

        for (freq, val) in spectrum.data().iter() {
            if freq.val() >= start_hz && freq.val() <= end_hz {
                // Use magnitude squared (energy)
                sum += val.val() * val.val();
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
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

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
    fn test_spectrum_computation() {
        let mut analyzer = SpectralAnalyzer::new(16000, 0.3);

        // Generate 1kHz sine wave
        let samples = generate_sine_wave(1000.0, 16000, 1024);

        let spectrum = analyzer.compute_spectrum(&samples);
        assert!(spectrum.is_some());

        let spectrum = spectrum.unwrap();
        // Should have peak around 1kHz
        let peak_freq = spectrum.max();
        assert!(peak_freq.0.val() > 900.0 && peak_freq.0.val() < 1100.0);
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
