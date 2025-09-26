use anyhow::Result;
use hound::WavReader;
use std::sync::Arc;
use std::path::Path;
use std::time::Duration;
use tracing::info;

use coldvox_audio::ring_buffer::AudioProducer;
use coldvox_vad::constants::FRAME_SIZE_SAMPLES;

/// Playback mode for WAV streaming
#[derive(Debug, Clone, Copy)]
pub enum PlaybackMode {
    /// Real-time playback (default)
    Realtime,
    /// Accelerated playback with speed multiplier
    Accelerated(f32),
    /// Deterministic playback (no sleeps, feed as fast as possible)
    Deterministic,
}

/// WAV file loader that feeds audio data through the pipeline
pub struct WavFileLoader {
    samples: Vec<i16>,
    sample_rate: u32,
    channels: u16,
    current_pos: usize,
    frame_size_total: usize,
    playback_mode: PlaybackMode,
}

impl WavFileLoader {
    /// Load WAV file and prepare for streaming (no resample/mono conversion)
    /// This mirrors live capture: raw device rate/channels into ring buffer.
    pub fn new<P: AsRef<Path>>(wav_path: P) -> Result<Self> {
        let mut reader = WavReader::open(wav_path)?;
        let spec = reader.spec();

        info!(
            "Loading WAV: {} Hz, {} channels, {} bits",
            spec.sample_rate, spec.channels, spec.bits_per_sample
        );

        // Read all samples as interleaved i16
        let samples: Vec<i16> = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;

        info!(
            "WAV loaded: {} samples (interleaved) at {} Hz, {} channels",
            samples.len(),
            spec.sample_rate,
            spec.channels
        );

        // Choose a chunk size close to ~32ms per channel to emulate callback pacing
        // FRAME_SIZE_SAMPLES is per mono channel; scale by channel count for total i16 samples
        let frame_size_total = FRAME_SIZE_SAMPLES * spec.channels as usize;

        // Get playback mode from environment (namespaced)
        let playback_mode = match std::env::var("COLDVOX_PLAYBACK_MODE") {
            Ok(mode) if mode.eq_ignore_ascii_case("deterministic") => PlaybackMode::Deterministic,
            Ok(mode) if mode.eq_ignore_ascii_case("accelerated") => {
                let speed = std::env::var("COLDVOX_PLAYBACK_SPEED_MULTIPLIER")
                    .unwrap_or_else(|_| "2.0".to_string())
                    .parse::<f32>()
                    .unwrap_or(2.0);
                PlaybackMode::Accelerated(speed)
            }
            _ => PlaybackMode::Realtime,
        };

        Ok(Self {
            samples,
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            current_pos: 0,
            frame_size_total,
            playback_mode,
        })
    }

    /// Stream audio data to ring buffer with realistic timing
    pub async fn stream_to_ring_buffer(&mut self, mut producer: AudioProducer) -> Result<()> {
        // Duration for one chunk of size `frame_size_total` (interleaved across channels)
        // time = samples_total / (sample_rate * channels)
        let nanos_per_sample_total =
            1_000_000_000u64 / (self.sample_rate as u64 * self.channels as u64);

        while self.current_pos < self.samples.len() {
            let end_pos = (self.current_pos + self.frame_size_total).min(self.samples.len());
            let chunk = &self.samples[self.current_pos..end_pos];

            // Try to write chunk to ring buffer
            let mut written = 0;
            while written < chunk.len() {
                match producer.write(&chunk[written..]) {
                    Ok(count) => written += count,
                    Err(_) => {
                        // Ring buffer full, wait a bit
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                }
            }

            self.current_pos = end_pos;

            // Maintain realistic timing for the total interleaved samples written
            let written_total = chunk.len() as u64;
            let sleep_nanos = written_total * nanos_per_sample_total;

            match self.playback_mode {
                PlaybackMode::Realtime => {
                    tokio::time::sleep(Duration::from_nanos(sleep_nanos)).await;
                }
                PlaybackMode::Accelerated(speed) => {
                    let accelerated_nanos = (sleep_nanos as f32 / speed) as u64;
                    let clamped = accelerated_nanos.max(50_000); // 50us minimum to yield
                    tokio::time::sleep(Duration::from_nanos(clamped)).await;
                }
                PlaybackMode::Deterministic => {
                    // No real sleep; logical frame progression (future: integrate TestClock)
                }
            }
        }

        info!(
            "WAV streaming completed ({} total samples processed), feeding silence to flush VAD.",
            self.current_pos
        );

        // After WAV is done, feed some silence to ensure VAD emits SpeechEnd.
        let silence_chunk = vec![0i16; self.frame_size_total];
        for _ in 0..15 {
            // Feed ~500ms of silence (15 * 32ms)
            let mut written = 0;
            while written < silence_chunk.len() {
                if let Ok(count) = producer.write(&silence_chunk[written..]) {
                    written += count;
                } else {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            tokio::time::sleep(Duration::from_millis(32)).await;
        }

        Ok(())
    }

    /// Stream audio using a shared producer protected by a parking_lot Mutex
    pub async fn stream_to_ring_buffer_locked(
        &mut self,
        producer: Arc<parking_lot::Mutex<AudioProducer>>,
    ) -> Result<()> {
        let nanos_per_sample_total =
            1_000_000_000u64 / (self.sample_rate as u64 * self.channels as u64);

        while self.current_pos < self.samples.len() {
            let end_pos = (self.current_pos + self.frame_size_total).min(self.samples.len());
            let chunk = &self.samples[self.current_pos..end_pos];

            let mut written = 0;
            while written < chunk.len() {
                let res = {
                    let mut guard = producer.lock();
                    guard.write(&chunk[written..])
                };
                match res {
                    Ok(count) => written += count,
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                }
            }

            self.current_pos = end_pos;

            let written_total = chunk.len() as u64;
            let sleep_nanos = written_total * nanos_per_sample_total;

            match self.playback_mode {
                PlaybackMode::Realtime => {
                    tokio::time::sleep(Duration::from_nanos(sleep_nanos)).await;
                }
                PlaybackMode::Accelerated(speed) => {
                    let accelerated_nanos = (sleep_nanos as f32 / speed) as u64;
                    let clamped = accelerated_nanos.max(50_000);
                    tokio::time::sleep(Duration::from_nanos(clamped)).await;
                }
                PlaybackMode::Deterministic => {}
            }
        }

        info!(
            "WAV streaming completed ({} total samples processed), feeding silence to flush VAD.",
            self.current_pos
        );

        let silence_chunk = vec![0i16; self.frame_size_total];
        for _ in 0..15 {
            let mut written = 0;
            while written < silence_chunk.len() {
                let res = {
                    let mut guard = producer.lock();
                    guard.write(&silence_chunk[written..])
                };
                if let Ok(count) = res {
                    written += count;
                } else {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            tokio::time::sleep(Duration::from_millis(32)).await;
        }

        Ok(())
    }

    pub fn duration_ms(&self) -> u64 {
        // Total interleaved samples divided by (rate * channels)
        let base_duration =
            ((self.samples.len() as u64) * 1000) / (self.sample_rate as u64 * self.channels as u64);

        match self.playback_mode {
            PlaybackMode::Realtime => base_duration,
            PlaybackMode::Accelerated(speed) => (base_duration as f32 / speed) as u64,
            PlaybackMode::Deterministic => 0, // Logical time only; test should not rely on wall time
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn channels(&self) -> u16 {
        self.channels
    }
}
