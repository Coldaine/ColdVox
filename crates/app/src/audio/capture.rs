use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::detector::SilenceDetector;
use super::device::DeviceManager;
// Test hook output

use super::resampler::StreamResampler;
use super::ring_buffer::{AudioProducer};
use super::watchdog::WatchdogTimer;
use crate::foundation::error::{AudioConfig, AudioError};

// This remains the primary data structure for audio data.
pub struct AudioCapture {
    device_manager: DeviceManager,
    stream: Option<Stream>,
    audio_producer: Option<AudioProducer>,
    watchdog: WatchdogTimer,
    silence_detector: SilenceDetector,
    stats: Arc<CaptureStats>,
    running: Arc<AtomicBool>,
}

// A handle to the dedicated audio thread.
pub struct AudioCaptureThread {
    pub handle: JoinHandle<()>,
    pub shutdown: Arc<AtomicBool>,
}

impl AudioCaptureThread {
    pub fn spawn(
        config: AudioConfig,
        audio_producer: AudioProducer,
        device_name: Option<String>,
    ) -> Result<(Self, u32), AudioError> {
        let running = Arc::new(AtomicBool::new(false));
        let shutdown = running.clone();

        let handle = thread::Builder::new()
            .name("audio-capture".to_string())
            .spawn(move || {
                let mut capture = match AudioCapture::new(config, audio_producer, running.clone()) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Failed to create AudioCapture: {}", e);
                        return;
                    }
                };

                if let Err(e) = capture.start(device_name.as_deref()) {
                    tracing::error!("Failed to start audio capture: {}", e);
                    return;
                }

                while running.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(100));
                }

                tracing::info!("Audio capture thread shutting down.");
                capture.stop();
            })
            .map_err(|e| AudioError::Fatal(format!("Failed to spawn audio thread: {}", e)))?;

        let output_sample_rate = 16_000; // The resampler always targets 16kHz

        Ok((Self { handle, shutdown }, output_sample_rate))
    }

    pub fn stop(self) {
        self.shutdown.store(false, Ordering::Relaxed);
        let _ = self.handle.join();
    }
}

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub samples: Vec<i16>,
    pub timestamp: Instant,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Default)]
pub struct CaptureStats {
    pub frames_captured: AtomicU64,
    pub frames_dropped: AtomicU64,
    pub disconnections: AtomicU64,
    pub reconnections: AtomicU64,
    pub active_frames: AtomicU64,
    pub silent_frames: AtomicU64,
    pub last_frame_time: Arc<RwLock<Option<Instant>>>,
}

impl AudioCapture {
    pub fn new(
        config: AudioConfig,
        audio_producer: AudioProducer,
        running: Arc<AtomicBool>,
    ) -> Result<Self, AudioError> {
        Ok(Self {
            device_manager: DeviceManager::new()?,
            stream: None,
            audio_producer: Some(audio_producer),
            watchdog: WatchdogTimer::new(Duration::from_secs(5)),
            silence_detector: SilenceDetector::new(config.silence_threshold),
            stats: Arc::new(CaptureStats::default()),
            running,
        })
    }

    fn start(&mut self, device_name: Option<&str>) -> Result<(), AudioError> {
        self.running.store(true, Ordering::SeqCst);

        let device = self.device_manager.open_device(device_name)?;
        let (config, sample_format) = self.negotiate_config(&device)?;

        let stream = self.build_stream(device, config, sample_format)?;
        stream.play()?;

        self.stream = Some(stream);
        self.watchdog.start(Arc::clone(&self.running));
        Ok(())
    }

    fn build_stream(
        &mut self,
        device: cpal::Device,
        config: StreamConfig,
        sample_format: SampleFormat,
    ) -> Result<Stream, AudioError> {
        let mut audio_producer = self.audio_producer.take().unwrap();
        let stats = Arc::clone(&self.stats);
        let watchdog = self.watchdog.clone();
        let detector = Arc::new(RwLock::new(self.silence_detector.clone()));
        let running = Arc::clone(&self.running);

        let err_fn = move |err: cpal::StreamError| {
            tracing::error!("Audio stream error: {}", err);
        };

        let channels = config.channels as usize;
        let input_sample_rate = config.sample_rate.0;
        let target_sample_rate: u32 = 16_000;
        let need_resample = input_sample_rate != target_sample_rate;
        let resampler =
            if need_resample {
                Some(Arc::new(parking_lot::Mutex::new(StreamResampler::new(
                    input_sample_rate,
                    target_sample_rate,
                ))))
            } else {
                None
            };

        let stream_callback = move |data: &[i16], _: &_| {
            if !running.load(Ordering::SeqCst) {
                return;
            }
            watchdog.feed();
            let samples_mono: Vec<i16> = if channels == 1 {
                data.to_vec()
            } else {
                data.chunks_exact(channels)
                    .map(|chunk| (chunk.iter().map(|&s| s as i32).sum::<i32>() / channels as i32) as i16)
                    .collect()
            };
            let out_samples: Vec<i16> = if let Some(rs) = &resampler {
                rs.lock().process(&samples_mono)
            } else {
                samples_mono
            };

            let mut det = detector.write();
            if det.is_silence(&out_samples) {
                stats.silent_frames.fetch_add(1, Ordering::Relaxed);
            } else {
                stats.active_frames.fetch_add(1, Ordering::Relaxed);
            }

            if let Ok(written) = audio_producer.write(&out_samples) {
                if written == out_samples.len() {
                    stats.frames_captured.fetch_add(1, Ordering::Relaxed);
                } else {
                    stats.frames_dropped.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                stats.frames_dropped.fetch_add(1, Ordering::Relaxed);
            }
            *stats.last_frame_time.write() = Some(Instant::now());
        };

        let stream = match sample_format {
            SampleFormat::I16 => device.build_input_stream(&config, stream_callback, err_fn, None)?,
            // Similar callbacks for F32, U16 etc., converting to I16
            _ => return Err(AudioError::FormatNotSupported { format: format!("{:?}", sample_format) }),
        };

        Ok(stream)
    }

    fn negotiate_config(
        &self,
        device: &cpal::Device,
    ) -> Result<(StreamConfig, SampleFormat), AudioError> {
        let mut first_any: Option<(StreamConfig, SampleFormat)> = None;
        if let Ok(configs) = device.supported_input_configs() {
            for supported in configs {
                let fmt = supported.sample_format();
                if first_any.is_none() {
                    first_any = Some((supported.with_max_sample_rate().into(), fmt));
                }
                if supported.min_sample_rate().0 <= 16000 && supported.max_sample_rate().0 >= 16000 {
                    return Ok((
                        StreamConfig {
                            channels: supported.channels(),
                            sample_rate: cpal::SampleRate(16000),
                            buffer_size: cpal::BufferSize::Default,
                        },
                        fmt,
                    ));
                }
            }
        }
        first_any.ok_or_else(|| AudioError::FormatNotSupported {
            format: "No supported audio formats".to_string(),
        })
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        self.watchdog.stop();
    }
}