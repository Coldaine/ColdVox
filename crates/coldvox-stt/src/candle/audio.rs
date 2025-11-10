//! Audio preprocessing utilities for the Candle Whisper backend.
//!
//! The implementation is ported from the official Candle Whisper example:
//! https://github.com/huggingface/candle/tree/main/candle-examples/examples/whisper
//! with light adaptations for the ColdVox architecture.

use std::cmp;
use std::sync::Arc;
use std::thread;

use candle_core::utils::get_num_threads;
use coldvox_foundation::error::{ColdVoxError, SttError};

/// Whisper operates on 16 kHz mono PCM samples.
pub const SAMPLE_RATE: usize = 16_000;
/// FFT window size used by Whisper.
pub const N_FFT: usize = 400;
/// Hop length (stride) between FFT windows.
pub const HOP_LENGTH: usize = 160;
/// Whisper chunks audio into 30 second windows.
pub const CHUNK_LENGTH: usize = 30;
/// Number of PCM samples in a single Whisper chunk.
pub const N_SAMPLES: usize = CHUNK_LENGTH * SAMPLE_RATE;
/// Number of mel frames produced for a single chunk.
pub const N_FRAMES: usize = N_SAMPLES / HOP_LENGTH;

/// Configuration for mel spectrogram generation.
#[derive(Debug, Clone, Copy)]
pub struct WhisperAudioConfig {
    pub num_mel_bins: usize,
    pub speed_up: bool,
}

impl Default for WhisperAudioConfig {
    fn default() -> Self {
        Self {
            num_mel_bins: 80,
            speed_up: false,
        }
    }
}

/// Load mel filter coefficients for the requested bin count.
pub fn mel_filters(num_mel_bins: usize) -> Result<Vec<f32>, ColdVoxError> {
    match num_mel_bins {
        80 => Ok(decode_mel_filters(include_bytes!("melfilters.bytes"))),
        128 => Ok(decode_mel_filters(include_bytes!("melfilters128.bytes"))),
        other => Err(ColdVoxError::Stt(SttError::InvalidConfig(format!(
            "unsupported mel bin count: {other} (expected 80 or 128)"
        )))),
    }
}

/// Convert raw PCM samples (normalized f32) into a log-mel spectrogram.
pub fn pcm_to_mel(
    cfg: &WhisperAudioConfig,
    samples: &[f32],
    filters: &[f32],
) -> Vec<f32> {
    log_mel_spectrogram(
        samples,
        filters,
        N_FFT,
        HOP_LENGTH,
        cfg.num_mel_bins,
        cfg.speed_up,
    )
}

fn decode_mel_filters(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

fn fft(inp: &[f32]) -> Vec<f32> {
    let n = inp.len();
    if n == 1 {
        return vec![inp[0], 0.0];
    }
    if n % 2 == 1 {
        return dft(inp);
    }
    let mut out = vec![0.0; n * 2];

    let mut even = Vec::with_capacity(n / 2);
    let mut odd = Vec::with_capacity(n / 2);

    for (i, &value) in inp.iter().enumerate() {
        if i % 2 == 0 {
            even.push(value);
        } else {
            odd.push(value);
        }
    }

    let even_fft = fft(&even);
    let odd_fft = fft(&odd);

    let two_pi = std::f32::consts::PI * 2.0;
    let n_float = n as f32;
    for k in 0..(n / 2) {
        let k_float = k as f32;
        let theta = two_pi * k_float / n_float;
        let re = theta.cos();
        let im = -theta.sin();

        let re_odd = odd_fft[2 * k];
        let im_odd = odd_fft[2 * k + 1];

        out[2 * k] = even_fft[2 * k] + re * re_odd - im * im_odd;
        out[2 * k + 1] = even_fft[2 * k + 1] + re * im_odd + im * re_odd;

        out[2 * (k + n / 2)] = even_fft[2 * k] - re * re_odd + im * im_odd;
        out[2 * (k + n / 2) + 1] =
            even_fft[2 * k + 1] - re * im_odd - im * re_odd;
    }
    out
}

fn dft(inp: &[f32]) -> Vec<f32> {
    let n = inp.len();
    let two_pi = std::f32::consts::PI * 2.0;

    let mut out = Vec::with_capacity(2 * n);
    let n_float = n as f32;
    for k in 0..n {
        let k_float = k as f32;
        let mut re = 0.0;
        let mut im = 0.0;

        for (j, &value) in inp.iter().enumerate() {
            let j_float = j as f32;
            let angle = two_pi * k_float * j_float / n_float;
            re += value * angle.cos();
            im -= value * angle.sin();
        }

        out.push(re);
        out.push(im);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn log_mel_spectrogram_worker(
    ith: usize,
    hann: &[f32],
    samples: &[f32],
    filters: &[f32],
    fft_size: usize,
    fft_step: usize,
    speed_up: bool,
    n_len: usize,
    n_mel: usize,
    n_threads: usize,
) -> Vec<f32> {
    let n_fft = if speed_up {
        1 + fft_size / 4
    } else {
        1 + fft_size / 2
    };
    let mut fft_in = vec![0.0; fft_size];
    let mut mel = vec![0.0; n_len * n_mel];
    let n_samples = samples.len();
    let end = cmp::min(n_samples / fft_step + 1, n_len);

    for i in (ith..end).step_by(n_threads) {
        let offset = i * fft_step;
        let copy_len = cmp::min(fft_size, n_samples.saturating_sub(offset));

        for j in 0..copy_len {
            fft_in[j] = hann[j] * samples[offset + j];
        }

        if copy_len < fft_size {
            for j in copy_len..fft_size {
                fft_in[j] = 0.0;
            }
        }

        let mut fft_out = fft(&fft_in);

        for j in 0..fft_size {
            fft_out[j] = fft_out[2 * j] * fft_out[2 * j]
                + fft_out[2 * j + 1] * fft_out[2 * j + 1];
        }
        for j in 1..fft_size / 2 {
            let v = fft_out[fft_size - j];
            fft_out[j] += v;
        }

        if speed_up {
            for j in 0..n_fft {
                fft_out[j] =
                    0.5 * (fft_out[2 * j] + fft_out[2 * j + 1]);
            }
        }

        for j in 0..n_mel {
            let mut sum = 0.0;
            let mut k = 0;
            while k + 3 < n_fft {
                sum += fft_out[k] * filters[j * n_fft + k]
                    + fft_out[k + 1] * filters[j * n_fft + k + 1]
                    + fft_out[k + 2] * filters[j * n_fft + k + 2]
                    + fft_out[k + 3] * filters[j * n_fft + k + 3];
                k += 4;
            }
            while k < n_fft {
                sum += fft_out[k] * filters[j * n_fft + k];
                k += 1;
            }
            mel[j * n_len + i] = sum.max(1e-10).log10();
        }
    }
    mel
}

fn log_mel_spectrogram(
    samples: &[f32],
    filters: &[f32],
    fft_size: usize,
    fft_step: usize,
    n_mel: usize,
    speed_up: bool,
) -> Vec<f32> {
    let mut hann = Vec::with_capacity(fft_size);
    let fft_size_f = fft_size as f32;
    for i in 0..fft_size {
        let angle = 2.0 * std::f32::consts::PI * (i as f32) / fft_size_f;
        hann.push(0.5 * (1.0 - angle.cos()));
    }

    let mut n_len = samples.len() / fft_step;
    let pad = 100 * CHUNK_LENGTH / 2;
    if n_len % pad != 0 {
        n_len = (n_len / pad + 1) * pad;
    }
    n_len += pad;
    let mut samples_padded = samples.to_vec();
    let to_add = n_len * fft_step - samples.len();
    if to_add > 0 {
        samples_padded.extend(std::iter::repeat(0.0).take(to_add));
    }

    let mut n_threads = get_num_threads();
    if n_threads == 0 {
        n_threads = 1;
    }
    n_threads -= n_threads % 2;
    n_threads = cmp::max(2, cmp::min(n_threads, 12));

    let hann = Arc::new(hann);
    let samples = Arc::new(samples_padded);
    let filters = Arc::new(filters.to_vec());

    let outputs = thread::scope(|scope| {
        (0..n_threads)
            .map(|thread_id| {
                let hann = Arc::clone(&hann);
                let samples = Arc::clone(&samples);
                let filters = Arc::clone(&filters);
                scope.spawn(move || {
                    log_mel_spectrogram_worker(
                        thread_id,
                        &hann,
                        &samples,
                        &filters,
                        fft_size,
                        fft_step,
                        speed_up,
                        n_len,
                        n_mel,
                        n_threads,
                    )
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("log-mel worker panicked"))
            .collect::<Vec<_>>()
    });

    let mut mel = vec![0.0; outputs[0].len()];
    for segment_start in (0..mel.len()).step_by(n_threads) {
        for thread_output in &outputs {
            for offset in 0..n_threads {
                let idx = segment_start + offset;
                if idx < mel.len() {
                    mel[idx] += thread_output[idx];
                }
            }
        }
    }

    let mut mmax = mel
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    if mmax.is_finite() {
        mmax -= 8.0;
    } else {
        mmax = -8.0;
    }
    for value in mel.iter_mut() {
        let v = value.max(mmax);
        *value = v / 4.0 + 1.0;
    }
    mel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_supported_filter_bank() {
        let filters = mel_filters(80).expect("filters");
        assert_eq!(filters.len(), 80 * (1 + N_FFT / 2));

        let filters_128 = mel_filters(128).expect("filters 128");
        assert_eq!(filters_128.len(), 128 * (1 + N_FFT / 2));
    }

    #[test]
    fn rejects_unsupported_filter_bank() {
        let err = mel_filters(42).unwrap_err();
        match err {
            ColdVoxError::Stt(SttError::InvalidConfig(msg)) => {
                assert!(msg.contains("unsupported mel bin"))
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn pcm_to_mel_returns_finite_values() {
        let filters = mel_filters(80).expect("filters");
        let cfg = WhisperAudioConfig::default();
        let samples = vec![0.0_f32; SAMPLE_RATE];

        let mel = pcm_to_mel(&cfg, &samples, &filters);
        assert_eq!(mel.len() % cfg.num_mel_bins, 0);
        assert!(mel.iter().all(|v| v.is_finite()));
    }
}
