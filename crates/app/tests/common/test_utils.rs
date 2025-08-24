use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use cpal::{SampleFormat, StreamConfig};
use coldvox_app::audio::{AudioFrame, AudioCapture, CaptureStats};

/// Generate test audio samples as a sine wave
pub fn generate_sine_wave(freq: f32, sample_rate: u32, duration_ms: u32) -> Vec<i16> {
    let num_samples = (sample_rate * duration_ms / 1000) as usize;
    let mut samples = Vec::with_capacity(num_samples);
    
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (t * freq * 2.0 * std::f32::consts::PI).sin();
        samples.push((sample * i16::MAX as f32) as i16);
    }
    
    samples
}

/// Generate silent samples
pub fn generate_silence(sample_count: usize) -> Vec<i16> {
    vec![0; sample_count]
}

/// Generate noise samples for testing
pub fn generate_noise(sample_count: usize, amplitude: i16) -> Vec<i16> {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};
    
    let mut samples = Vec::with_capacity(sample_count);
    let mut hasher = RandomState::new().build_hasher();
    
    for i in 0..sample_count {
        i.hash(&mut hasher);
        let hash = hasher.finish();
        let normalized = (hash as i32 % (amplitude as i32 * 2)) - amplitude as i32;
        samples.push(normalized as i16);
    }
    
    samples
}

/// Convert f32 samples to i16
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples.iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

/// Convert u16 samples to i16
pub fn u16_to_i16(samples: &[u16]) -> Vec<i16> {
    samples.iter()
        .map(|&s| (s as i32 - 32768) as i16)
        .collect()
}

/// Convert u8 samples to i16
pub fn u8_to_i16(samples: &[u8]) -> Vec<i16> {
    samples.iter()
        .map(|&s| ((s as i32 - 128) * 256) as i16)
        .collect()
}

/// Convert i8 samples to i16
pub fn i8_to_i16(samples: &[i8]) -> Vec<i16> {
    samples.iter()
        .map(|&s| (s as i16) * 256)
        .collect()
}

/// Downmix stereo to mono by averaging channels
pub fn stereo_to_mono(stereo_samples: &[i16]) -> Vec<i16> {
    stereo_samples
        .chunks_exact(2)
        .map(|chunk| ((chunk[0] as i32 + chunk[1] as i32) / 2) as i16)
        .collect()
}

/// Mock audio device configuration for testing
#[derive(Clone, Debug)]
pub struct MockAudioConfig {
    pub name: String,
    pub format: SampleFormat,
    pub channels: u16,
    pub sample_rate: u32,
}

impl Default for MockAudioConfig {
    fn default() -> Self {
        Self {
            name: "test_device".to_string(),
            format: SampleFormat::I16,
            channels: 1,
            sample_rate: 16000,
        }
    }
}

impl MockAudioConfig {
    pub fn to_stream_config(&self) -> StreamConfig {
        StreamConfig {
            channels: self.channels,
            sample_rate: cpal::SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        }
    }
}

/// Test harness for timing verification
pub struct TimingHarness {
    start: Instant,
    checkpoints: Vec<(String, Instant)>,
}

impl TimingHarness {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            checkpoints: Vec::new(),
        }
    }
    
    pub fn checkpoint(&mut self, name: &str) {
        self.checkpoints.push((name.to_string(), Instant::now()));
    }
    
    pub fn assert_duration_between(&self, from: &str, to: &str, expected: Duration, tolerance: Duration) {
        let from_time = self.checkpoints.iter()
            .find(|(n, _)| n == from)
            .map(|(_, t)| *t)
            .unwrap_or(self.start);
            
        let to_time = self.checkpoints.iter()
            .find(|(n, _)| n == to)
            .map(|(_, t)| *t)
            .unwrap_or_else(|| Instant::now());
            
        let actual = to_time.duration_since(from_time);
        assert_duration_within(actual, expected, tolerance);
    }
}

/// Verify timing constraints
pub fn assert_duration_within(actual: Duration, expected: Duration, tolerance: Duration) {
    let diff = if actual > expected {
        actual - expected
    } else {
        expected - actual
    };
    assert!(
        diff <= tolerance,
        "Duration {:?} not within {:?} of expected {:?}",
        actual, tolerance, expected
    );
}

/// Create test AudioFrame
pub fn create_test_frame(samples: Vec<i16>, sample_rate: u32) -> AudioFrame {
    AudioFrame {
        samples,
        timestamp: Instant::now(),
        sample_rate,
        channels: 1,
    }
}

/// Stats snapshot helper for assertions
pub struct StatsSnapshot {
    pub frames_captured: u64,
    pub frames_dropped: u64,
    pub disconnections: u64,
    pub reconnections: u64,
    pub active_frames: u64,
    pub silent_frames: u64,
}

impl StatsSnapshot {
    pub fn from_stats(stats: &CaptureStats) -> Self {
        Self {
            frames_captured: stats.frames_captured.load(Ordering::Relaxed),
            frames_dropped: stats.frames_dropped.load(Ordering::Relaxed),
            disconnections: stats.disconnections.load(Ordering::Relaxed),
            reconnections: stats.reconnections.load(Ordering::Relaxed),
            active_frames: stats.active_frames.load(Ordering::Relaxed),
            silent_frames: stats.silent_frames.load(Ordering::Relaxed),
        }
    }
    
    pub fn assert_frames_increased(&self, after: &StatsSnapshot, min_increase: u64) {
        assert!(
            after.frames_captured >= self.frames_captured + min_increase,
            "Expected at least {} new frames, got {}",
            min_increase,
            after.frames_captured - self.frames_captured
        );
    }
}

/// Test data generator for various scenarios
pub struct TestDataGenerator {
    sample_rate: u32,
}

impl TestDataGenerator {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }
    
    /// Generate alternating silence and activity pattern
    pub fn generate_activity_pattern(&self, pattern: &[(bool, u32)]) -> Vec<i16> {
        let mut samples = Vec::new();
        
        for &(is_active, duration_ms) in pattern {
            let num_samples = (self.sample_rate * duration_ms / 1000) as usize;
            
            if is_active {
                samples.extend(generate_sine_wave(440.0, self.sample_rate, duration_ms));
            } else {
                samples.extend(generate_silence(num_samples));
            }
        }
        
        samples
    }
    
    /// Generate samples that will trigger silence detection
    pub fn generate_below_threshold(&self, threshold: i16, duration_ms: u32) -> Vec<i16> {
        let num_samples = (self.sample_rate * duration_ms / 1000) as usize;
        generate_noise(num_samples, threshold / 2)
    }
    
    /// Generate samples that will not trigger silence detection
    pub fn generate_above_threshold(&self, threshold: i16, duration_ms: u32) -> Vec<i16> {
        let num_samples = (self.sample_rate * duration_ms / 1000) as usize;
        generate_noise(num_samples, threshold * 2)
    }
}