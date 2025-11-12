use candle::{Result, Tensor, Device};

const N_FFT: usize = 400;
const N_MELS: usize = 80;
const HOP_LENGTH: usize = 160;
const CHUNK_LENGTH: usize = 30;
const SAMPLING_RATE: usize = 16000;

pub fn log_mel_spectrogram(pcm: &[f32], device: &Device) -> Result<Tensor> {
    let pcm_len = pcm.len();
    let n_samples = CHUNK_LENGTH * SAMPLING_RATE;
    let pcm = if pcm_len < n_samples {
        let mut padded = vec![0.0; n_samples];
        padded[..pcm_len].copy_from_slice(pcm);
        padded
    } else {
        pcm.to_vec()
    };

    let stft = stft(&pcm, N_FFT, HOP_LENGTH)?;
    let magnitudes = stft.abs()?.powf(2.0)?;
    let mel_filters = mel_filters(device, N_MELS, N_FFT)?;
    let mel_spec = magnitudes.matmul(&mel_filters.t()?)?;
    let log_spec = (mel_spec.max(1e-10)?).log10()?;
    let log_spec = (log_spec.max(log_spec.max_all()?.to_scalar::<f64>()? - 8.0)?)?;
    let log_spec = (log_spec + 4.0)? / 4.0?;
    Ok(log_spec)
}

fn stft(pcm: &[f32], n_fft: usize, hop_length: usize) -> Result<Tensor> {
    let window = hamming_window(n_fft, &Device::Cpu)?;
    let n_frames = (pcm.len() - n_fft) / hop_length + 1;
    let mut frames = Vec::with_capacity(n_frames);
    for i in 0..n_frames {
        let start = i * hop_length;
        let end = start + n_fft;
        frames.extend_from_slice(&pcm[start..end]);
    }
    let frames = Tensor::new(frames, &Device::Cpu)?.reshape((n_frames, n_fft))?;
    let frames = frames.broadcast_mul(&window)?;
    let stft = frames.fft(n_fft)?;
    Ok(stft.i((.., ..n_fft / 2 + 1))?)
}

fn hamming_window(n: usize, device: &Device) -> Result<Tensor> {
    let ts = Tensor::arange(0, n as u32, device)?.to_dtype(candle::DType::F32)?;
    let cos = (ts * (2.0 * std::f64::consts::PI / (n - 1) as f64))?.cos()?;
    (0.54 - 0.46 * cos)?
}

fn hz_to_mel(hz: f64) -> f64 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f64) -> f64 {
    700.0 * (10.0f64.powf(mel / 2595.0) - 1.0)
}

fn mel_filters(device: &Device, n_mels: usize, n_fft: usize) -> Result<Tensor> {
    let f_min = 0.0;
    let f_max = SAMPLING_RATE as f64 / 2.0;
    let mel_min = hz_to_mel(f_min);
    let mel_max = hz_to_mel(f_max);
    let mel_points = (0..=n_mels + 1)
        .map(|i| mel_min + (mel_max - mel_min) * i as f64 / (n_mels + 1) as f64)
        .collect::<Vec<_>>();
    let fft_freqs = (0..=n_fft / 2)
        .map(|i| i as f64 * SAMPLING_RATE as f64 / n_fft as f64)
        .collect::<Vec<_>>();
    let mel_edges = mel_points.windows(3).map(|w| (w[0], w[1], w[2])).collect::<Vec<_>>();

    let mut filters = vec![0.0; n_mels * (n_fft / 2 + 1)];
    for (i, (mel_start, mel_center, mel_end)) in mel_edges.iter().enumerate() {
        for (j, &freq) in fft_freqs.iter().enumerate() {
            let mel_freq = hz_to_mel(freq);
            let slope = if mel_freq >= *mel_start && mel_freq <= *mel_center {
                (mel_freq - mel_start) / (mel_center - mel_start)
            } else if mel_freq >= *mel_center && mel_freq <= *mel_end {
                (mel_end - mel_freq) / (mel_end - mel_center)
            } else {
                0.0
            };
            filters[i * (n_fft / 2 + 1) + j] = slope as f32;
        }
    }
    Tensor::new(filters, device)?.reshape((n_mels, n_fft / 2 + 1))
}
