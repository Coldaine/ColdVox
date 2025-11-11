//! Audio preprocessing for Whisper inference
//!
//! This module provides mel spectrogram computation for Whisper models.
//! The implementation is adapted from the Candle Whisper examples.
//!
//! Attribution: This code is derived from the Candle project's Whisper examples
//! (https://github.com/huggingface/candle/tree/main/candle-examples/examples/whisper)

use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use rustfft::{FftPlanner, num_complex::Complex};

/// Whisper audio constants
pub const SAMPLE_RATE: usize = 16000;
pub const N_FFT: usize = 400;
pub const N_MELS: usize = 80;
pub const HOP_LENGTH: usize = 160;
pub const CHUNK_LENGTH: usize = 30;
pub const N_SAMPLES: usize = CHUNK_LENGTH * SAMPLE_RATE; // 30 seconds at 16kHz

/// Mel filterbank frequencies
const MEL_FILTERS: &[u8] = include_bytes!("mel_filters.bytes");

/// Convert frequency to mel scale
fn hz_to_mel(freq: f64) -> f64 {
    2595.0 * (1.0 + freq / 700.0).log10()
}

/// Convert mel scale to frequency
fn mel_to_hz(mel: f64) -> f64 {
    700.0 * (10_f64.powf(mel / 2595.0) - 1.0)
}

/// Generate mel filterbank
fn mel_filterbank(device: &Device) -> Result<Tensor> {
    // Load pre-computed mel filters from included bytes
    let mut filters = vec![0f32; N_MELS * (N_FFT / 2 + 1)];
    let bytes = MEL_FILTERS;

    if bytes.len() != filters.len() * 4 {
        // If pre-computed filters not available, compute them
        return compute_mel_filterbank(device);
    }

    use byteorder::{ByteOrder, LittleEndian};
    for (i, chunk) in bytes.chunks_exact(4).enumerate() {
        filters[i] = LittleEndian::read_f32(chunk);
    }

    Tensor::from_vec(filters, (N_MELS, N_FFT / 2 + 1), device)
        .context("Failed to create mel filterbank tensor")
}

/// Compute mel filterbank from scratch
fn compute_mel_filterbank(device: &Device) -> Result<Tensor> {
    let n_freqs = N_FFT / 2 + 1;
    let mut filterbank = vec![0f32; N_MELS * n_freqs];

    // Frequency bins
    let freqs: Vec<f64> = (0..n_freqs)
        .map(|i| i as f64 * SAMPLE_RATE as f64 / N_FFT as f64)
        .collect();

    // Mel scale points
    let mel_min = hz_to_mel(0.0);
    let mel_max = hz_to_mel(SAMPLE_RATE as f64 / 2.0);
    let mel_pts: Vec<f64> = (0..=N_MELS + 1)
        .map(|i| mel_min + (mel_max - mel_min) * i as f64 / (N_MELS + 1) as f64)
        .collect();

    let hz_pts: Vec<f64> = mel_pts.iter().map(|&m| mel_to_hz(m)).collect();

    // Create triangular filters
    for m in 0..N_MELS {
        let left = hz_pts[m];
        let center = hz_pts[m + 1];
        let right = hz_pts[m + 2];

        for (f, &freq) in freqs.iter().enumerate() {
            let weight = if freq < left || freq > right {
                0.0
            } else if freq <= center {
                (freq - left) / (center - left)
            } else {
                (right - freq) / (right - center)
            };

            filterbank[m * n_freqs + f] = weight as f32;
        }
    }

    Tensor::from_vec(filterbank, (N_MELS, n_freqs), device)
        .context("Failed to create computed mel filterbank tensor")
}

/// Apply Hann window to a tensor
fn hann_window(n_fft: usize, device: &Device) -> Result<Tensor> {
    let window: Vec<f32> = (0..n_fft)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / n_fft as f32).cos())
        })
        .collect();

    Tensor::from_vec(window, n_fft, device)
        .context("Failed to create Hann window")
}

/// Compute Short-Time Fourier Transform (STFT) using FFT
fn stft(pcm: &[f32], device: &Device) -> Result<Tensor> {
    let n_frames = (pcm.len() - N_FFT) / HOP_LENGTH + 1;

    // Pre-compute Hann window and extract values once (avoids repeated allocations)
    let window_tensor = hann_window(N_FFT, device)?;
    let window_values = window_tensor.to_vec1::<f32>()
        .context("Failed to extract window values")?;

    // Setup FFT planner
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(N_FFT);

    let mut spectrogram = Vec::new();

    for frame_idx in 0..n_frames {
        let start = frame_idx * HOP_LENGTH;
        let end = start + N_FFT;

        if end > pcm.len() {
            break;
        }

        // Apply window (using pre-extracted window values)
        let mut buffer: Vec<Complex<f32>> = pcm[start..end]
            .iter()
            .zip(window_values.iter())
            .map(|(&x, &w)| Complex::new(x * w, 0.0))
            .collect();

        // Compute FFT using rustfft (O(N log N) instead of O(N²))
        fft.process(&mut buffer);

        // Extract magnitude spectrum (only first half due to symmetry)
        let frame_fft: Vec<f32> = buffer[0..=N_FFT / 2]
            .iter()
            .map(|c| c.norm())
            .collect();

        spectrogram.extend(frame_fft);
    }

    Tensor::from_vec(spectrogram, (n_frames, N_FFT / 2 + 1), device)
        .context("Failed to create STFT tensor")
}

/// Compute log mel spectrogram from PCM audio
///
/// # Arguments
/// * `pcm` - 16kHz mono PCM audio as f32 samples (normalized to [-1.0, 1.0])
/// * `device` - Candle device to use for computation
///
/// # Returns
/// A tensor of shape (N_MELS, n_frames) containing the log mel spectrogram
pub fn log_mel_spectrogram(pcm: &[f32], device: &Device) -> Result<Tensor> {
    // Ensure we have the right amount of audio (pad or trim if necessary)
    let mut audio = pcm.to_vec();

    if audio.len() < N_SAMPLES {
        // Pad with zeros
        audio.resize(N_SAMPLES, 0.0);
    } else if audio.len() > N_SAMPLES {
        // Trim to 30 seconds
        audio.truncate(N_SAMPLES);
    }

    // Compute STFT
    let stft = stft(&audio, device)?;

    // Get mel filterbank
    let mel_filters = mel_filterbank(device)?;

    // Apply mel filterbank: (N_MELS, N_FFT/2+1) @ (n_frames, N_FFT/2+1)^T
    let mel_spec = mel_filters
        .matmul(&stft.t()?)
        .context("Failed to apply mel filterbank")?;

    // Convert to log scale (with small epsilon to avoid log(0))
    let log_spec = mel_spec
        .clamp(1e-10, f32::MAX)?
        .log()?;

    // Normalize to match Whisper's expected input range
    let max_val = log_spec.max(1)?.max(0)?;
    let log_spec = (log_spec.broadcast_sub(&max_val)?)?;

    Ok(log_spec)
}

/// Convert i16 PCM samples to f32 normalized to [-1.0, 1.0]
pub fn pcm16_to_f32(pcm16: &[i16]) -> Vec<f32> {
    pcm16
        .iter()
        .map(|&sample| sample as f32 / i16::MAX as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hz_mel_conversion() {
        let hz = 1000.0;
        let mel = hz_to_mel(hz);
        let hz_back = mel_to_hz(mel);
        assert!((hz - hz_back).abs() < 0.01);
    }

    #[test]
    fn test_hz_mel_conversion_zero() {
        let hz = 0.0;
        let mel = hz_to_mel(hz);
        assert_eq!(mel, 0.0);
        let hz_back = mel_to_hz(mel);
        assert!((hz - hz_back).abs() < 0.01);
    }

    #[test]
    fn test_hz_mel_conversion_nyquist() {
        let hz = 8000.0; // Nyquist for 16kHz
        let mel = hz_to_mel(hz);
        let hz_back = mel_to_hz(mel);
        assert!((hz - hz_back).abs() < 0.1);
    }

    #[test]
    fn test_pcm16_to_f32() {
        let pcm16 = vec![0i16, i16::MAX, i16::MIN];
        let pcm_f32 = pcm16_to_f32(&pcm16);

        assert_eq!(pcm_f32.len(), 3);
        assert!((pcm_f32[0] - 0.0).abs() < 1e-6);
        assert!((pcm_f32[1] - 1.0).abs() < 0.01);
        assert!((pcm_f32[2] + 1.0).abs() < 0.01);
    }

    #[test]
    fn test_pcm16_to_f32_range() {
        let pcm16 = vec![16384, -16384, 8192, -8192]; // Various levels
        let pcm_f32 = pcm16_to_f32(&pcm16);

        assert_eq!(pcm_f32.len(), 4);
        // All values should be in [-1, 1] range
        for &val in &pcm_f32 {
            assert!(val >= -1.0 && val <= 1.0, "Value {} out of range", val);
        }
    }

    #[test]
    fn test_pcm16_to_f32_empty() {
        let pcm16: Vec<i16> = vec![];
        let pcm_f32 = pcm16_to_f32(&pcm16);
        assert_eq!(pcm_f32.len(), 0);
    }

    #[test]
    fn test_constants() {
        // Verify Whisper audio constants match expected values
        assert_eq!(SAMPLE_RATE, 16000);
        assert_eq!(N_FFT, 400);
        assert_eq!(N_MELS, 80);
        assert_eq!(HOP_LENGTH, 160);
        assert_eq!(CHUNK_LENGTH, 30);
        assert_eq!(N_SAMPLES, 480000); // 30 * 16000
    }

    #[test]
    fn test_hann_window_cpu() {
        let device = Device::Cpu;
        let window = hann_window(128, &device).expect("Failed to create Hann window");

        // Check shape
        assert_eq!(window.dims(), &[128]);

        // Get values and verify properties
        let values = window.to_vec1::<f32>().expect("Failed to convert to vec");

        // Hann window should start and end at 0
        assert!((values[0] - 0.0).abs() < 1e-6);
        assert!((values[127] - 0.0).abs() < 1e-6);

        // Peak should be near the middle
        let max_val = values.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        assert!(max_val > 0.9 && max_val <= 1.0);
    }

    #[test]
    fn test_compute_mel_filterbank_cpu() {
        let device = Device::Cpu;
        let filterbank = compute_mel_filterbank(&device).expect("Failed to create mel filterbank");

        // Check shape: (N_MELS, N_FFT/2+1)
        assert_eq!(filterbank.dims(), &[N_MELS, N_FFT / 2 + 1]);

        // Get values and verify they're non-negative
        let values = filterbank.flatten_all().expect("Failed to flatten")
            .to_vec1::<f32>().expect("Failed to convert to vec");

        // All filterbank values should be non-negative
        for (i, &val) in values.iter().enumerate() {
            assert!(val >= 0.0, "Negative filterbank value at index {}: {}", i, val);
        }

        // At least some values should be non-zero
        let non_zero_count = values.iter().filter(|&&v| v > 1e-6).count();
        assert!(non_zero_count > 0, "Mel filterbank is all zeros");
    }
}
