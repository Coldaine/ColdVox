use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use crossbeam_channel::{Sender, Receiver, bounded};
use cpal::{Stream, StreamConfig, SampleFormat};
use cpal::traits::{DeviceTrait, StreamTrait};
use parking_lot::RwLock;

use crate::foundation::error::{AudioError, AudioConfig};
use super::device::DeviceManager;
use super::watchdog::WatchdogTimer;
use super::detector::SilenceDetector;

pub struct AudioCapture {
    device_manager: DeviceManager,
    stream: Option<Stream>,
    sample_tx: Sender<AudioFrame>,
    sample_rx: Receiver<AudioFrame>,
    watchdog: WatchdogTimer,
    silence_detector: SilenceDetector,
    stats: CaptureStats,
    running: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub samples: Vec<i16>,
    pub timestamp: Instant,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug)]
pub struct CaptureStats {
    pub frames_captured: AtomicU64,
    pub frames_dropped: AtomicU64,
    pub disconnections: AtomicU64,
    pub reconnections: AtomicU64,
    pub last_frame_time: Arc<RwLock<Option<Instant>>>,
}

impl Clone for CaptureStats {
    fn clone(&self) -> Self {
        Self {
            frames_captured: AtomicU64::new(self.frames_captured.load(Ordering::Relaxed)),
            frames_dropped: AtomicU64::new(self.frames_dropped.load(Ordering::Relaxed)),
            disconnections: AtomicU64::new(self.disconnections.load(Ordering::Relaxed)),
            reconnections: AtomicU64::new(self.reconnections.load(Ordering::Relaxed)),
            last_frame_time: Arc::new(RwLock::new(*self.last_frame_time.read())),
        }
    }
}

impl Default for CaptureStats {
    fn default() -> Self {
        Self {
            frames_captured: AtomicU64::new(0),
            frames_dropped: AtomicU64::new(0),
            disconnections: AtomicU64::new(0),
            reconnections: AtomicU64::new(0),
            last_frame_time: Arc::new(RwLock::new(None)),
        }
    }
}

#[derive(Debug)]
pub struct CaptureStatsSnapshot {
    pub frames_captured: u64,
    pub frames_dropped: u64,
    pub disconnections: u64,
    pub reconnections: u64,
    pub last_frame_age: Option<Duration>,
}

impl AudioCapture {
    pub fn new(config: AudioConfig) -> Result<Self, AudioError> {
        let (sample_tx, sample_rx) = bounded(100); // Buffer ~2 seconds at 20ms frames
        
        Ok(Self {
            device_manager: DeviceManager::new()?,
            stream: None,
            sample_tx,
            sample_rx,
            watchdog: WatchdogTimer::new(Duration::from_secs(5)),
            silence_detector: SilenceDetector::new(config.silence_threshold),
            stats: CaptureStats::default(),
            running: Arc::new(AtomicBool::new(false)),
        })
    }
    
    pub async fn start(&mut self, device_name: Option<&str>) -> Result<(), AudioError> {
        self.running.store(true, Ordering::SeqCst);
        
        // Open device with fallback
        let device = self.device_manager.open_device(device_name)?;
        let device_name = device.name().unwrap_or("Unknown".to_string());
        tracing::info!("Opening audio device: {}", device_name);
        
    // Get best config (prefer 16kHz, keep device channel count) and sample format
    let (config, sample_format) = self.negotiate_config(&device)?;
    tracing::info!("Audio config: {:?}, format: {:?}", config, sample_format);
        
        // Build stream with error recovery
    let stream = self.build_stream(device, config, sample_format)?;
        stream.play()?;
        
        self.stream = Some(stream);
        
        // Start watchdog
        self.watchdog.start(Arc::clone(&self.running));
        
        Ok(())
    }
    
    fn build_stream(&self, device: cpal::Device, config: StreamConfig, sample_format: SampleFormat) -> Result<Stream, AudioError> {
        let sample_tx = self.sample_tx.clone();
        let stats = self.stats.clone();
        let watchdog = self.watchdog.clone();
        let detector = Arc::new(RwLock::new(self.silence_detector.clone()));
        let running = Arc::clone(&self.running);
        
    let err_fn = move |err: cpal::StreamError| {
            tracing::error!("Audio stream error: {}", err);
            // Don't panic, let watchdog handle recovery
        };
        
        let channels = config.channels as usize;
        let sample_rate = config.sample_rate.0;

        let stream = match sample_format {
            SampleFormat::I16 => {
                let running = Arc::clone(&self.running);
                let watchdog = self.watchdog.clone();
                let detector = Arc::new(RwLock::new(self.silence_detector.clone()));
                let sample_tx = self.sample_tx.clone();
                let stats = self.stats.clone();
                let err_fn = move |err| { tracing::error!("Audio stream error: {}", err); };
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &_| {
                        if !running.load(Ordering::SeqCst) { return; }
                        watchdog.feed();
                        let samples_mono: Vec<i16> = if channels == 1 {
                            data.to_vec()
                        } else {
                            data.chunks_exact(channels)
                                .map(|chunk| {
                                    let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
                                    (sum / channels as i32) as i16
                                })
                                .collect()
                        };
                        {
                            let mut det = detector.write();
                            let is_sil = det.is_silence(&samples_mono);
                            let dur = det.silence_duration();
                            if is_sil && dur > Duration::from_secs(3) {
                                tracing::warn!("Extended silence detected, possible device issue");
                            }
                        }
                        let frame = AudioFrame { samples: samples_mono, timestamp: Instant::now(), sample_rate, channels: config.channels };
                        match sample_tx.try_send(frame) {
                            Ok(_) => { stats.frames_captured.fetch_add(1, Ordering::Relaxed); }
                            Err(_) => { stats.frames_dropped.fetch_add(1, Ordering::Relaxed); tracing::warn!("Audio buffer full, dropping frame"); }
                        }
                        *stats.last_frame_time.write() = Some(Instant::now());
                    },
                    err_fn,
                    None
                )?
            }
            SampleFormat::F32 => {
                let running = Arc::clone(&self.running);
                let watchdog = self.watchdog.clone();
                let detector = Arc::new(RwLock::new(self.silence_detector.clone()));
                let sample_tx = self.sample_tx.clone();
                let stats = self.stats.clone();
                let err_fn = move |err| { tracing::error!("Audio stream error: {}", err); };
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &_| {
                        if !running.load(Ordering::SeqCst) { return; }
                        watchdog.feed();
                        let to_i16 = |x: f32| -> i16 { (x.clamp(-1.0, 1.0) * 32767.0) as i16 };
                        let samples_mono: Vec<i16> = if channels == 1 {
                            data.iter().map(|&s| to_i16(s)).collect()
                        } else {
                            data.chunks_exact(channels)
                                .map(|chunk| {
                                    let sum: f32 = chunk.iter().copied().sum();
                                    to_i16(sum / channels as f32)
                                })
                                .collect()
                        };
                        {
                            let mut det = detector.write();
                            let is_sil = det.is_silence(&samples_mono);
                            let dur = det.silence_duration();
                            if is_sil && dur > Duration::from_secs(3) {
                                tracing::warn!("Extended silence detected, possible device issue");
                            }
                        }
                        let frame = AudioFrame { samples: samples_mono, timestamp: Instant::now(), sample_rate, channels: config.channels };
                        match sample_tx.try_send(frame) {
                            Ok(_) => { stats.frames_captured.fetch_add(1, Ordering::Relaxed); }
                            Err(_) => { stats.frames_dropped.fetch_add(1, Ordering::Relaxed); tracing::warn!("Audio buffer full, dropping frame"); }
                        }
                        *stats.last_frame_time.write() = Some(Instant::now());
                    },
                    err_fn,
                    None
                )?
            }
            SampleFormat::U16 => {
                let running = Arc::clone(&self.running);
                let watchdog = self.watchdog.clone();
                let detector = Arc::new(RwLock::new(self.silence_detector.clone()));
                let sample_tx = self.sample_tx.clone();
                let stats = self.stats.clone();
                let err_fn = move |err| { tracing::error!("Audio stream error: {}", err); };
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _: &_| {
                        if !running.load(Ordering::SeqCst) { return; }
                        watchdog.feed();
                        let to_i16 = |x: u16| -> i16 { (x as i32 - 32768) as i16 };
                        let samples_mono: Vec<i16> = if channels == 1 {
                            data.iter().map(|&s| to_i16(s)).collect()
                        } else {
                            data.chunks_exact(channels)
                                .map(|chunk| {
                                    let sum: i32 = chunk.iter().map(|&s| to_i16(s) as i32).sum();
                                    (sum / channels as i32) as i16
                                })
                                .collect()
                        };
                        {
                            let mut det = detector.write();
                            let is_sil = det.is_silence(&samples_mono);
                            let dur = det.silence_duration();
                            if is_sil && dur > Duration::from_secs(3) {
                                tracing::warn!("Extended silence detected, possible device issue");
                            }
                        }
                        let frame = AudioFrame { samples: samples_mono, timestamp: Instant::now(), sample_rate, channels: config.channels };
                        match sample_tx.try_send(frame) {
                            Ok(_) => { stats.frames_captured.fetch_add(1, Ordering::Relaxed); }
                            Err(_) => { stats.frames_dropped.fetch_add(1, Ordering::Relaxed); tracing::warn!("Audio buffer full, dropping frame"); }
                        }
                        *stats.last_frame_time.write() = Some(Instant::now());
                    },
                    err_fn,
                    None
                )?
            }
            other => return Err(AudioError::FormatNotSupported { format: format!("{:?}", other) }),
        };
        
        Ok(stream)
    }
    
    fn negotiate_config(&self, device: &cpal::Device) -> Result<(StreamConfig, SampleFormat), AudioError> {
        // Prefer 16kHz if available; keep device channel count; return actual sample format
        let mut first_any: Option<(StreamConfig, SampleFormat)> = None;
        if let Ok(configs) = device.supported_input_configs() {
            for supported in configs {
                let fmt = supported.sample_format();
                let channels = supported.channels();

                // Capture a fallback first option
                if first_any.is_none() {
                    let cfg: StreamConfig = supported.clone().with_max_sample_rate().into();
                    first_any = Some((cfg, fmt));
                }

                // Check if 16kHz supported
                if supported.min_sample_rate().0 <= 16000 && supported.max_sample_rate().0 >= 16000 {
                    let cfg = StreamConfig {
                        channels,
                        sample_rate: cpal::SampleRate(16000),
                        buffer_size: cpal::BufferSize::Default,
                    };
                    return Ok((cfg, fmt));
                }
            }
        }
        if let Some(pair) = first_any { return Ok(pair); }
        Err(AudioError::FormatNotSupported { format: "No supported audio formats".to_string() })
    }
    
    pub async fn recover(&mut self) -> Result<(), AudioError> {
        tracing::info!("Attempting audio recovery");
        self.stats.disconnections.fetch_add(1, Ordering::Relaxed);
        
        // Stop current stream
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        
        // Wait a bit
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Try to restart
        for attempt in 1..=3 {
            tracing::info!("Recovery attempt {}/3", attempt);
            
            match self.start(None).await {
                Ok(_) => {
                    self.stats.reconnections.fetch_add(1, Ordering::Relaxed);
                    tracing::info!("Audio recovery successful");
                    return Ok(());
                }
                Err(e) => {
                    tracing::error!("Recovery attempt {} failed: {}", attempt, e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
        
        Err(AudioError::Fatal("Failed to recover audio after 3 attempts".to_string()))
    }
    
    pub fn get_stats(&self) -> CaptureStatsSnapshot {
        CaptureStatsSnapshot {
            frames_captured: self.stats.frames_captured.load(Ordering::Relaxed),
            frames_dropped: self.stats.frames_dropped.load(Ordering::Relaxed),
            disconnections: self.stats.disconnections.load(Ordering::Relaxed),
            reconnections: self.stats.reconnections.load(Ordering::Relaxed),
            last_frame_age: self.stats.last_frame_time.read().clone().map(|t| Instant::now().duration_since(t)),
        }
    }

    pub fn get_watchdog(&self) -> &WatchdogTimer {
        &self.watchdog
    }

    pub fn get_receiver(&self) -> Receiver<AudioFrame> {
        self.sample_rx.clone()
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(stream) = self.stream.take() { drop(stream); }
        self.watchdog.stop();
    }
}