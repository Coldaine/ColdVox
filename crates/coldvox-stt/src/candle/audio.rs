//! Audio preprocessing for Whisper models.
//!
//! This module implements the audio preprocessing pipeline required by Whisper:
//! 1. Convert PCM to f32 samples
//! 2. Pad or trim to 30 seconds
//! 3. Compute log-mel spectrogram
//!
//! The implementation is based on the Candle Whisper examples with attribution
//! to the original source code.
//!
//! Reference: https://github.com/huggingface/candle/tree/main/candle-examples/examples/whisper

#[cfg(feature = "whisper")]
use candle_core::{Device, Result, Tensor};
#[cfg(feature = "whisper")]
use candle_transformers::models::whisper::audio;

/// Whisper expects 16kHz audio
pub const SAMPLE_RATE: usize = 16000;

/// Whisper processes 30-second chunks
pub const N_SAMPLES: usize = SAMPLE_RATE * 30; // 480,000 samples

/// Number of mel filterbanks
pub const N_MELS: usize = 80;

/// Convert i16 PCM samples to f32 normalized to [-1, 1]
///
/// # Normalization Constant
///
/// Uses 32768.0 (2^15) for normalization because:
/// - i16 range is -32768 to 32767
/// - Dividing by 32768.0 maps this to approximately [-1.0, 1.0]
/// - The value -32768 maps to exactly -1.0
/// - The value 32767 maps to ~0.999969482 (slightly less than 1.0)
/// - This asymmetry is inherent to signed integer representation
/// - The alternative (dividing by 32767.0) would make -32768 map to -1.00003,
///   which could exceed the [-1, 1] range expected by audio models
///
/// This is the standard PCM normalization used across audio processing libraries.
pub fn pcm_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| s as f32 / 32768.0)
        .collect()
}

/// Pad or trim audio to exactly N_SAMPLES (30 seconds at 16kHz)
///
/// # Zero Padding Rationale
///
/// Padding uses zeros (silence) because:
/// 1. Zeros represent true silence in normalized PCM audio
/// 2. Whisper is trained on variable-length audio padded with silence
/// 3. The model's attention mechanism learns to ignore padded regions
/// 4. Alternative padding strategies (e.g., edge repetition) could confuse the model
///    by creating artificial discontinuities or fake speech patterns
///
/// # Empty Input Handling
///
/// If the input is empty (0 samples), the function returns a full buffer of zeros,
/// representing 30 seconds of silence. This is safe and won't crash the model,
/// though it will likely produce an empty transcription.
pub fn pad_or_trim(samples: &[f32]) -> Vec<f32> {
    let mut result = vec![0.0; N_SAMPLES];
    let copy_len = samples.len().min(N_SAMPLES);
    result[..copy_len].copy_from_slice(&samples[..copy_len]);
    result
}

/// Compute log-mel spectrogram from PCM audio samples.
///
/// This function:
/// 1. Normalizes i16 PCM to f32 [-1, 1]
/// 2. Pads or trims to 30 seconds
/// 3. Computes mel spectrogram using Candle's built-in implementation
///
/// # Arguments
/// * `pcm_audio` - 16kHz mono i16 PCM samples
/// * `device` - Candle device to use for computation
///
/// # Returns
/// A tensor of shape [N_MELS, time_steps] containing the log-mel spectrogram
#[cfg(feature = "whisper")]
pub fn log_mel_spectrogram(pcm_audio: &[i16], device: &Device) -> Result<Tensor> {
    // Convert to f32 and normalize
    let samples_f32 = pcm_to_f32(pcm_audio);

    // Pad or trim to 30 seconds
    let samples_padded = pad_or_trim(&samples_f32);

    // Create tensor from samples
    // PERF: Tensor::from_vec takes ownership, so we pass the vector directly without cloning
    let samples_tensor = Tensor::from_vec(samples_padded, N_SAMPLES, device)?;

    // Use Candle's built-in mel spectrogram computation
    // This handles STFT, mel filterbank, and log scaling
    audio::log_mel_spectrogram(&samples_tensor, N_MELS)
}

#[cfg(feature = "whisper")]
pub fn log_mel_spectrogram_from_f32(pcm_audio: &[f32], device: &Device) -> Result<Tensor> {
    // Pad or trim to 30 seconds
    let samples_padded = pad_or_trim(pcm_audio);

    // Create tensor from samples
    let samples_tensor = Tensor::from_vec(samples_padded, samples_padded.len(), device)?;

    // Use Candle's built-in mel spectrogram computation
    audio::log_mel_spectrogram(&samples_tensor, N_MELS)
}

#[cfg(test)]
#[cfg(feature = "whisper")]
mod tests {
    use super::*;

    #[test]
    fn test_pcm_to_f32_boundary_values() {
        let pcm = vec![0i16, 16384, -16384, 32767, -32768];
        let f32_samples = pcm_to_f32(&pcm);

        assert_eq!(f32_samples.len(), 5);

        // Test zero maps to zero
        assert!((f32_samples[0] - 0.0).abs() < 1e-6);

        // Test half scale (~0.5)
        assert!((f32_samples[1] - 0.5).abs() < 1e-3);
        assert!((f32_samples[2] - -0.5).abs() < 1e-3);

        // Test max positive value maps to ~1.0 (but slightly less)
        assert!(f32_samples[3] > 0.999 && f32_samples[3] < 1.0);

        // Test min negative value maps to exactly -1.0
        assert_eq!(f32_samples[4], -1.0);
    }

    #[test]
    fn test_pcm_to_f32_empty() {
        // Edge case: empty input
        let pcm: Vec<i16> = vec![];
        let f32_samples = pcm_to_f32(&pcm);
        assert_eq!(f32_samples.len(), 0);
    }

    #[test]
    fn test_pcm_to_f32_single_sample() {
        // Edge case: single sample
        let pcm = vec![1000i16];
        let f32_samples = pcm_to_f32(&pcm);
        assert_eq!(f32_samples.len(), 1);
        assert!((f32_samples[0] - (1000.0 / 32768.0)).abs() < 1e-6);
    }

    #[test]
    fn test_pcm_to_f32_range() {
        // Verify all outputs are in [-1.0, 1.0] range
        let pcm = vec![i16::MIN, -1000, 0, 1000, i16::MAX];
        let f32_samples = pcm_to_f32(&pcm);

        for &sample in &f32_samples {
            assert!(sample >= -1.0 && sample <= 1.0, "Sample {} out of range", sample);
        }
    }

    #[test]
    fn test_pad_or_trim_short() {
        // Test padding short audio
        let short = vec![1.0, 2.0, 3.0];
        let padded = pad_or_trim(&short);

        assert_eq!(padded.len(), N_SAMPLES);
        assert_eq!(padded[0], 1.0);
        assert_eq!(padded[1], 2.0);
        assert_eq!(padded[2], 3.0);

        // Verify padding is zeros
        for i in 3..N_SAMPLES {
            assert_eq!(padded[i], 0.0, "Padding at index {} should be 0.0", i);
        }
    }

    #[test]
    fn test_pad_or_trim_long() {
        // Test trimming long audio
        let mut long = vec![1.0; N_SAMPLES + 1000];
        // Mark the boundary
        long[N_SAMPLES - 1] = 2.0;
        long[N_SAMPLES] = 3.0;

        let trimmed = pad_or_trim(&long);

        assert_eq!(trimmed.len(), N_SAMPLES);
        assert_eq!(trimmed[0], 1.0);
        assert_eq!(trimmed[N_SAMPLES - 1], 2.0);
        // Verify trimmed portion is not included
    }

    #[test]
    fn test_pad_or_trim_exact() {
        // Test exact length (no padding or trimming needed)
        let exact = vec![0.5; N_SAMPLES];
        let result = pad_or_trim(&exact);

        assert_eq!(result.len(), N_SAMPLES);
        for i in 0..N_SAMPLES {
            assert_eq!(result[i], 0.5);
        }
    }

    #[test]
    fn test_pad_or_trim_empty() {
        // Edge case: empty input should produce all zeros
        let empty: Vec<f32> = vec![];
        let result = pad_or_trim(&empty);

        assert_eq!(result.len(), N_SAMPLES);
        for i in 0..N_SAMPLES {
            assert_eq!(result[i], 0.0, "Empty input should pad with zeros");
        }
    }

    #[test]
    fn test_pad_or_trim_one_sample() {
        // Edge case: single sample
        let single = vec![0.75];
        let result = pad_or_trim(&single);

        assert_eq!(result.len(), N_SAMPLES);
        assert_eq!(result[0], 0.75);

        for i in 1..N_SAMPLES {
            assert_eq!(result[i], 0.0);
        }
    }

    #[test]
    fn test_constants() {
        // Verify expected constants
        assert_eq!(SAMPLE_RATE, 16000);
        assert_eq!(N_SAMPLES, 480_000); // 30 seconds * 16000 Hz
        assert_eq!(N_MELS, 80);
    }

    #[test]
    fn test_log_mel_spectrogram_empty() {
        // Test that empty audio produces valid output without crashing
        let empty_pcm: Vec<i16> = vec![];
        let device = Device::Cpu;

        let result = log_mel_spectrogram(&empty_pcm, &device);

        // Should succeed (empty audio gets padded to 30s of silence)
        assert!(result.is_ok());

        let mel = result.unwrap();
        let shape = mel.dims();
        assert_eq!(shape.len(), 2);
        assert_eq!(shape[0], N_MELS); // Should have N_MELS mel bins
    }

    #[test]
    fn test_log_mel_spectrogram_short() {
        // Test short audio (1 second)
        let one_second = vec![0i16; SAMPLE_RATE];
        let device = Device::Cpu;

        let result = log_mel_spectrogram(&one_second, &device);
        assert!(result.is_ok());

        let mel = result.unwrap();
        let shape = mel.dims();
        assert_eq!(shape[0], N_MELS);
    }

    #[test]
    fn test_log_mel_spectrogram_exact() {
        // Test exact 30 seconds
        let thirty_seconds = vec![0i16; N_SAMPLES];
        let device = Device::Cpu;

        let result = log_mel_spectrogram(&thirty_seconds, &device);
        assert!(result.is_ok());

        let mel = result.unwrap();
        let shape = mel.dims();
        assert_eq!(shape[0], N_MELS);
    }

    #[test]
    fn test_log_mel_spectrogram_from_f32_empty() {
        // Test f32 version with empty input
        let empty: Vec<f32> = vec![];
        let device = Device::Cpu;

        let result = log_mel_spectrogram_from_f32(&empty, &device);
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_mel_spectrogram_from_f32_valid() {
        // Test f32 version with valid normalized audio
        let audio = vec![0.0f32; SAMPLE_RATE]; // 1 second of silence
        let device = Device::Cpu;

        let result = log_mel_spectrogram_from_f32(&audio, &device);
        assert!(result.is_ok());

        let mel = result.unwrap();
        let shape = mel.dims();
        assert_eq!(shape[0], N_MELS);
    }
}
