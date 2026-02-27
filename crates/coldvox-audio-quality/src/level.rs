//! Level monitoring: RMS and peak calculation with rolling windows.

use std::collections::VecDeque;

/// Monitors audio levels (RMS and peak) with rolling windows.
///
/// This struct maintains a rolling window for RMS calculation and peak hold
/// with decay. It's designed to be called from audio callback threads.
pub struct LevelMonitor {
    sample_rate: u32,

    // RMS calculation
    rms_window: VecDeque<f32>,      // Rolling window of mean-square values
    rms_window_capacity: usize,      // Max window size
    current_rms_linear: f32,         // Current RMS in linear scale [0, 1]

    // Peak detection
    current_peak_linear: f32,        // Current peak in linear scale [0, 1]
    peak_hold_frames: usize,         // How many frames to hold peak
    peak_hold_counter: usize,        // Frames since last peak update
}

impl LevelMonitor {
    /// Create a new level monitor.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Sample rate in Hz
    /// * `rms_window_ms` - RMS window duration in milliseconds
    /// * `peak_hold_ms` - Peak hold duration in milliseconds
    ///
    /// # Frame Size Assumptions
    ///
    /// This implementation assumes a typical frame size of 512 samples (32ms @ 16kHz).
    /// Window sizes are calculated based on this assumption. If your audio callback
    /// uses different frame sizes, timing accuracy may vary:
    ///
    /// - 256 samples (16ms): RMS/peak windows will be ~2x longer than specified
    /// - 1024 samples (64ms): RMS/peak windows will be ~0.5x shorter than specified
    ///
    /// For most applications, this variance is acceptable. If precise timing is required,
    /// consider using sample-count-based windows instead of time-based windows.
    pub fn new(sample_rate: u32, rms_window_ms: u64, peak_hold_ms: u64) -> Self {
        // Calculate window sizes in frames
        // ASSUMPTION: Typical frame size of 512 samples (32ms @ 16kHz)
        // This is a reasonable default for most audio callbacks, but timing accuracy
        // depends on actual frame size used at runtime.
        let typical_frame_ms = 32;
        let rms_window_frames = (rms_window_ms / typical_frame_ms).max(1) as usize;
        let peak_hold_frames = (peak_hold_ms / typical_frame_ms).max(1) as usize;

        Self {
            sample_rate,
            rms_window: VecDeque::with_capacity(rms_window_frames),
            rms_window_capacity: rms_window_frames,
            current_rms_linear: 0.0,
            current_peak_linear: 0.0,
            peak_hold_frames,
            peak_hold_counter: 0,
        }
    }

    /// Update RMS calculation with new audio frame.
    ///
    /// Returns the current RMS level in dBFS.
    pub fn update_rms(&mut self, samples: &[i16]) -> f32 {
        // Calculate mean-square for this frame
        let frame_mean_square = Self::calculate_frame_mean_square(samples);

        // Add to rolling window
        self.rms_window.push_back(frame_mean_square);
        if self.rms_window.len() > self.rms_window_capacity {
            self.rms_window.pop_front();
        }

        // Calculate average mean-square over window, then take sqrt
        // This is mathematically correct: RMS = sqrt(mean(x²))
        let sum: f32 = self.rms_window.iter().sum();
        let avg_mean_square = if self.rms_window.is_empty() {
            0.0
        } else {
            sum / self.rms_window.len() as f32
        };

        let avg_rms = avg_mean_square.sqrt();
        self.current_rms_linear = avg_rms;

        // Convert to dBFS
        Self::linear_to_dbfs(avg_rms)
    }

    /// Update peak detection with new audio frame.
    ///
    /// Returns the current peak level in dBFS.
    pub fn update_peak(&mut self, samples: &[i16]) -> f32 {
        // Find peak in this frame
        let frame_peak = Self::calculate_frame_peak(samples);

        // Update peak with hold
        if frame_peak > self.current_peak_linear {
            // New peak detected
            self.current_peak_linear = frame_peak;
            self.peak_hold_counter = 0;
        } else {
            // Holding or decaying
            self.peak_hold_counter += 1;
            if self.peak_hold_counter >= self.peak_hold_frames {
                // Hold period expired, start decay
                // Decay by 1 dB per frame (~30 dB/sec @ 32ms frames)
                let decay_db = 1.0;
                let current_db = Self::linear_to_dbfs(self.current_peak_linear);
                let new_db = current_db - decay_db;
                self.current_peak_linear = Self::dbfs_to_linear(new_db);
            }
        }

        // Convert to dBFS
        Self::linear_to_dbfs(self.current_peak_linear)
    }

    /// Get current RMS level in dBFS.
    pub fn current_rms_dbfs(&self) -> f32 {
        Self::linear_to_dbfs(self.current_rms_linear)
    }

    /// Get current peak level in dBFS.
    pub fn current_peak_dbfs(&self) -> f32 {
        Self::linear_to_dbfs(self.current_peak_linear)
    }

    /// Calculate mean-square for a single frame of samples.
    ///
    /// Mean-square = mean(x²)
    /// Used for RMS rolling window calculation (mathematically correct).
    fn calculate_frame_mean_square(samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f64 = samples
            .iter()
            .map(|&s| {
                // Convert to [-1.0, 1.0] range
                let normalized = s as f64 / 32768.0;
                normalized * normalized
            })
            .sum();

        (sum_squares / samples.len() as f64) as f32
    }

    /// Calculate RMS for a single frame of samples.
    ///
    /// RMS = sqrt(mean(samples^2))
    /// Note: For rolling window RMS, use calculate_frame_mean_square instead.
    fn calculate_frame_rms(samples: &[i16]) -> f32 {
        Self::calculate_frame_mean_square(samples).sqrt()
    }

    /// Calculate peak for a single frame of samples.
    fn calculate_frame_peak(samples: &[i16]) -> f32 {
        samples
            .iter()
            .map(|&s| {
                // Convert to [0.0, 1.0] range (absolute value)
                // Use unsigned_abs() to avoid i16::MIN overflow (-32768)
                (s.unsigned_abs() as f32) / 32768.0
            })
            .fold(0.0f32, f32::max)
    }

    /// Convert linear amplitude [0, 1] to dBFS [-∞, 0].
    ///
    /// dBFS = 20 * log10(amplitude)
    fn linear_to_dbfs(linear: f32) -> f32 {
        if linear <= 0.0 {
            f32::NEG_INFINITY
        } else {
            20.0 * linear.log10()
        }
    }

    /// Convert dBFS [-∞, 0] to linear amplitude [0, 1].
    ///
    /// amplitude = 10^(dBFS / 20)
    fn dbfs_to_linear(dbfs: f32) -> f32 {
        if dbfs == f32::NEG_INFINITY {
            0.0
        } else {
            10.0f32.powf(dbfs / 20.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn test_linear_to_dbfs_conversions() {
        // Test key points
        assert_abs_diff_eq!(LevelMonitor::linear_to_dbfs(1.0), 0.0, epsilon = 0.01);
        assert_abs_diff_eq!(LevelMonitor::linear_to_dbfs(0.5), -6.02, epsilon = 0.1);
        assert_abs_diff_eq!(LevelMonitor::linear_to_dbfs(0.1), -20.0, epsilon = 0.1);
        assert_eq!(LevelMonitor::linear_to_dbfs(0.0), f32::NEG_INFINITY);
    }

    #[test]
    fn test_dbfs_to_linear_conversions() {
        assert_abs_diff_eq!(LevelMonitor::dbfs_to_linear(0.0), 1.0, epsilon = 0.01);
        assert_abs_diff_eq!(LevelMonitor::dbfs_to_linear(-6.0), 0.5, epsilon = 0.01);
        assert_abs_diff_eq!(LevelMonitor::dbfs_to_linear(-20.0), 0.1, epsilon = 0.01);
        assert_eq!(LevelMonitor::dbfs_to_linear(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    fn test_rms_silence() {
        let samples = vec![0i16; 512];
        let rms = LevelMonitor::calculate_frame_rms(&samples);
        assert_eq!(rms, 0.0);
    }

    #[test]
    fn test_rms_full_scale() {
        let samples = vec![32767i16; 512];
        let rms = LevelMonitor::calculate_frame_rms(&samples);
        assert_abs_diff_eq!(rms, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_rms_half_scale() {
        let samples = vec![16384i16; 512];
        let rms = LevelMonitor::calculate_frame_rms(&samples);
        assert_abs_diff_eq!(rms, 0.5, epsilon = 0.01);
    }

    #[test]
    fn test_peak_detection() {
        let samples = vec![0i16, 1000, 0, -2000, 0, 5000];
        let peak = LevelMonitor::calculate_frame_peak(&samples);
        assert_abs_diff_eq!(peak, 5000.0 / 32768.0, epsilon = 0.001);
    }

    #[test]
    fn test_monitor_with_silence() {
        let mut monitor = LevelMonitor::new(16000, 500, 1000);
        let silence = vec![0i16; 512];

        let rms_dbfs = monitor.update_rms(&silence);
        let peak_dbfs = monitor.update_peak(&silence);

        assert_eq!(rms_dbfs, f32::NEG_INFINITY);
        assert_eq!(peak_dbfs, f32::NEG_INFINITY);
    }

    #[test]
    fn test_monitor_with_signal() {
        let mut monitor = LevelMonitor::new(16000, 500, 1000);

        // Generate a half-scale signal
        let signal = vec![16384i16; 512];

        let rms_dbfs = monitor.update_rms(&signal);
        let peak_dbfs = monitor.update_peak(&signal);

        // Half scale = -6 dBFS
        assert_abs_diff_eq!(rms_dbfs, -6.0, epsilon = 0.5);
        assert_abs_diff_eq!(peak_dbfs, -6.0, epsilon = 0.5);
    }

    #[test]
    fn test_peak_hold() {
        let mut monitor = LevelMonitor::new(16000, 500, 1000);

        // Send a loud frame
        let loud = vec![30000i16; 512];
        monitor.update_peak(&loud);
        let peak1 = monitor.current_peak_dbfs();

        // Send silence - peak should hold
        let silence = vec![0i16; 512];
        for _ in 0..10 {
            // Within hold period
            monitor.update_peak(&silence);
        }
        let peak2 = monitor.current_peak_dbfs();

        // Peak should be similar (within hold period)
        assert_abs_diff_eq!(peak1, peak2, epsilon = 1.0);
    }

    #[test]
    fn test_rolling_window() {
        let mut monitor = LevelMonitor::new(16000, 100, 1000); // Short window for testing

        // Feed several frames of increasing amplitude
        for amp in [1000i16, 5000, 10000, 20000] {
            let samples = vec![amp; 512];
            monitor.update_rms(&samples);
        }

        // RMS should reflect average of recent frames
        let rms = monitor.current_rms_dbfs();
        assert!(rms > -40.0); // Should be reasonably loud
        assert!(rms < 0.0);   // But not clipping
    }
}
