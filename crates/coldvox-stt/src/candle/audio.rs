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
pub fn pcm_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| s as f32 / 32768.0)
        .collect()
}

/// Pad or trim audio to exactly N_SAMPLES (30 seconds at 16kHz)
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
    let samples_tensor = Tensor::from_vec(samples_padded.clone(), samples_padded.len(), device)?;

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
    fn test_pcm_to_f32() {
        let pcm = vec![0i16, 16384, -16384, 32767, -32768];
        let f32_samples = pcm_to_f32(&pcm);

        assert_eq!(f32_samples.len(), 5);
        assert!((f32_samples[0] - 0.0).abs() < 1e-6);
        assert!((f32_samples[1] - 0.5).abs() < 1e-3);
        assert!((f32_samples[2] - -0.5).abs() < 1e-3);
    }

    #[test]
    fn test_pad_or_trim() {
        // Test padding
        let short = vec![1.0, 2.0, 3.0];
        let padded = pad_or_trim(&short);
        assert_eq!(padded.len(), N_SAMPLES);
        assert_eq!(padded[0], 1.0);
        assert_eq!(padded[1], 2.0);
        assert_eq!(padded[2], 3.0);
        assert_eq!(padded[3], 0.0);

        // Test trimming
        let long = vec![1.0; N_SAMPLES + 1000];
        let trimmed = pad_or_trim(&long);
        assert_eq!(trimmed.len(), N_SAMPLES);
        assert_eq!(trimmed[0], 1.0);
        assert_eq!(trimmed[N_SAMPLES - 1], 1.0);
    }
}
