use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use parking_lot::{Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::detector::SilenceDetector;
use super::device::DeviceManager;
use super::monitor::DeviceMonitor;
// Test hook output

use super::ring_buffer::AudioProducer;
use super::watchdog::WatchdogTimer;
use coldvox_foundation::{AudioConfig, AudioError, DeviceEvent};

// This remains the primary data structure for audio data.
pub struct AudioCapture {
    device_manager: DeviceManager,
    stream: Option<Stream>,
    audio_producer: Arc<Mutex<AudioProducer>>,
    watchdog: WatchdogTimer,
    silence_detector: SilenceDetector,
    stats: Arc<CaptureStats>,
    running: Arc<AtomicBool>,
    restart_needed: Arc<AtomicBool>,
    config_tx: Option<tokio::sync::broadcast::Sender<DeviceConfig>>,
    device_event_tx: Option<tokio::sync::broadcast::Sender<DeviceEvent>>,
    current_device_name: Option<String>,
}

// Device configuration info
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub sample_rate: u32,
    pub channels: u16,
}

// A handle to the dedicated audio thread.
pub struct AudioCaptureThread {
    pub handle: JoinHandle<()>,
    pub shutdown: Arc<AtomicBool>,
    pub device_monitor_handle: Option<JoinHandle<()>>,
}

impl AudioCaptureThread {
    pub fn spawn(
        config: AudioConfig,
        audio_producer: Arc<Mutex<AudioProducer>>,
        device_name: Option<String>,
        enable_device_monitor: bool,
    ) -> Result<
        (
            Self,
            DeviceConfig,
            tokio::sync::broadcast::Receiver<DeviceConfig>,
            tokio::sync::broadcast::Receiver<DeviceEvent>,
        ),
        AudioError,
    > {
        // Start in running state so the device monitor thread stays alive
        // until we explicitly stop via `stop()`. Previously this was false,
        // causing the monitor to exit immediately and the capture loop to
        // detect a closed channel and terminate early.
        let running = Arc::new(AtomicBool::new(true));
        let shutdown = running.clone();
        let device_config = Arc::new(RwLock::new(None::<DeviceConfig>));
        let device_config_clone = device_config.clone();

        // Create device config broadcast channel
        let (config_tx, config_rx) = tokio::sync::broadcast::channel(16);
        let config_tx_clone = config_tx.clone();

        // Create device event broadcast channel
        let (device_event_tx, device_event_rx) = tokio::sync::broadcast::channel(32);
        let device_event_tx_clone = device_event_tx.clone();

        // Start device monitor with 2-second interval to reduce false positives from CPAL enumeration glitches
        let monitor_running = running.clone();
        let (monitor_rx_opt, monitor_handle) = if enable_device_monitor {
            let (device_monitor, monitor_rx) = DeviceMonitor::new(Duration::from_secs(2))?;
            let handle = device_monitor.start_monitoring(monitor_running);
            (Some(monitor_rx), Some(handle))
        } else {
            tracing::debug!("Device monitor disabled; skipping hotplug polling");
            (None, None)
        };

        let handle = thread::Builder::new()
            .name("audio-capture".to_string())
            .spawn(move || {
                let mut monitor_rx = monitor_rx_opt;
                let mut capture = match AudioCapture::new(config, audio_producer, running.clone()) {
                    Ok(c) => c.with_config_channel(config_tx_clone)
                              .with_device_event_channel(device_event_tx_clone),
                    Err(e) => {
                        tracing::error!("Failed to create AudioCapture: {}", e);
                        return;
                    }
                };

                // Preflight with fallback: try requested, otherwise candidate list until frames arrive
                let mut attempts: Vec<Option<String>> = Vec::new();
                if let Some(d) = device_name.clone() { attempts.push(Some(d)); }
                // Expand candidates from device manager priority
                let candidates = capture.device_manager.candidate_device_names();
                for name in candidates { attempts.push(Some(name)); }
                // Final attempt: None (let host decide)
                attempts.push(None);

                let mut dev_cfg: Option<DeviceConfig> = None;
                for attempt in attempts {
                    match capture.start(attempt.as_deref()) {
                        Ok(cfg) => {
                            tracing::info!("Audio stream started on device: {:?}", attempt);
                            capture.current_device_name = attempt.clone();

                            // Preflight: wait up to 3s for at least one frame
                            let start = Instant::now();
                            let mut ok = false;
                            while start.elapsed() < Duration::from_secs(3) {
                                if capture.stats.frames_captured.load(Ordering::Relaxed) > 0 {
                                    ok = true;
                                    break;
                                }
                                thread::sleep(Duration::from_millis(50));
                            }
                            if ok {
                                dev_cfg = Some(cfg);
                                break;
                            } else {
                                tracing::warn!("No audio frames within preflight timeout; falling back to next candidate");
                                capture.stop();
                                // small backoff before next attempt
                                thread::sleep(Duration::from_millis(200));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to start on {:?}: {}", attempt, e);
                            // try next candidate
                        }
                    }
                }
                let Some(dev_cfg) = dev_cfg else {
                    tracing::error!("All device candidates failed to produce audio; capture not started");
                    return;
                };

                *device_config_clone.write() = Some(dev_cfg);

                // Monitor for watchdog, error-triggered restarts, and device events
                while running.load(Ordering::Relaxed) {
                    let mut needs_restart = false;
                    let mut restart_reason = "unknown";

                    // Check for device monitor events
                    if let Some(rx) = monitor_rx.as_mut() {
                        match rx.try_recv() {
                            Ok(event) => {
                                tracing::debug!("Device event: {:?}", event);
                                let _ = capture.device_event_tx.as_ref().map(|tx| tx.send(event.clone()));

                                match event {
                                    DeviceEvent::CurrentDeviceDisconnected { name } => {
                                        if capture.current_device_name.as_ref() == Some(&name) {
                                            tracing::warn!("Current device {} disconnected, attempting recovery", name);
                                            needs_restart = true;
                                            restart_reason = "device disconnected";
                                        }
                                    }
                                    DeviceEvent::DeviceAdded { name } => {
                                        tracing::info!("New device available: {}", name);
                                        // Could implement automatic switching to preferred devices here
                                    }
                                    DeviceEvent::DeviceSwitchRequested { target } => {
                                        tracing::info!("Manual device switch requested to: {}", target);
                                        needs_restart = true;
                                        restart_reason = "manual device switch requested";
                                    }
                                    _ => {}
                                }
                            }
                            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                                // No events, continue
                            }
                            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                                tracing::warn!("Device monitor events lagged, some events may have been missed");
                            }
                            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                                tracing::error!("Device monitor channel closed");
                                break;
                            }
                        }
                    }

                    // Check existing restart conditions
                    if capture.watchdog.is_triggered() {
                        needs_restart = true;
                        restart_reason = "watchdog timeout";
                    }

                    if capture.restart_needed.load(Ordering::SeqCst) {
                        needs_restart = true;
                        restart_reason = "stream error";
                    }

                    if needs_restart {
                        tracing::warn!("Capture restart triggered ({})", restart_reason);
                        let old_device = capture.current_device_name.clone();
                        capture.stop();
                        capture.restart_needed.store(false, Ordering::SeqCst);

                        // Attempt re-open starting from current priority list
                        let mut restarted = false;
                        let mut attempts: Vec<Option<String>> = Vec::new();
                        let candidates = capture.device_manager.candidate_device_names();
                        for name in candidates { attempts.push(Some(name)); }
                        attempts.push(None);

                        for attempt in attempts {
                            match capture.start(attempt.as_deref()) {
                                Ok(cfg) => {
                                    tracing::info!("Capture restarted on device: {:?}", attempt);
                                    let new_device = attempt.clone();
                                    capture.current_device_name = new_device.clone();
                                    *device_config_clone.write() = Some(cfg);

                                    // Emit device switch event
                                    let switch_event = DeviceEvent::DeviceSwitched {
                                        from: old_device.clone(),
                                        to: new_device.unwrap_or_else(|| "default".to_string()),
                                    };
                                    let _ = capture.device_event_tx.as_ref().map(|tx| tx.send(switch_event));

                                    restarted = true;
                                    break;
                                }
                                Err(e) => {
                                    tracing::warn!("Restart failed on {:?}: {}", attempt, e);
                                }
                            }
                        }
                        if !restarted {
                            tracing::error!("Failed to restart capture on any candidate device");
                            let switch_event = DeviceEvent::DeviceSwitchFailed {
                                attempted: old_device.unwrap_or_else(|| "unknown".to_string()),
                                fallback: None,
                            };
                            let _ = capture.device_event_tx.as_ref().map(|tx| tx.send(switch_event));
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                tracing::debug!("Audio capture thread shutting down.");
                capture.stop();
            })
            .map_err(|e| AudioError::Fatal(format!("Failed to spawn audio thread: {}", e)))?;

        // Wait for device config to be set with timeout
        let start = Instant::now();
        let mut cfg = None;
        while start.elapsed() < Duration::from_secs(3) {
            if let Some(config) = device_config.read().clone() {
                cfg = Some(config);
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        let cfg = cfg.ok_or_else(|| {
            AudioError::Fatal("Failed to get device configuration within timeout".to_string())
        })?;

        Ok((
            Self {
                handle,
                shutdown,
                device_monitor_handle: monitor_handle,
            },
            cfg,
            config_rx,
            device_event_rx,
        ))
    }

    pub fn stop(self) {
        self.shutdown.store(false, Ordering::Relaxed);
        let _ = self.handle.join();
        if let Some(monitor_handle) = self.device_monitor_handle {
            let _ = monitor_handle.join();
        }
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
        audio_producer: Arc<Mutex<AudioProducer>>,
        running: Arc<AtomicBool>,
    ) -> Result<Self, AudioError> {
        let self_ = Self {
            device_manager: DeviceManager::new()?,
            stream: None,
            audio_producer,
            watchdog: WatchdogTimer::new(Duration::from_secs(5)),
            silence_detector: SilenceDetector::new(config.silence_threshold),
            stats: Arc::new(CaptureStats::default()),
            running,
            restart_needed: Arc::new(AtomicBool::new(false)),
            config_tx: None,
            device_event_tx: None,
            current_device_name: None,
        };

        // Check audio setup for PipeWire compatibility (baseline timing)
        let setup_start = std::time::Instant::now();
        if let Err(e) = self_.device_manager.check_audio_setup() {
            tracing::error!("Audio setup check failed: {}", e);
        }
        let setup_elapsed = setup_start.elapsed();
        const SETUP_BUDGET_MS: u64 = 100;
        let budget = std::time::Duration::from_millis(SETUP_BUDGET_MS);

        if setup_elapsed > budget {
            tracing::warn!(
                "Audio setup check exceeded budget: {:.2?} > {:.2?}. Consider optimizing setup path.",
                setup_elapsed, budget
            );
        } else {
            tracing::info!(
                "Audio setup check baseline: {:.2?} (under {:?} budget)",
                setup_elapsed,
                budget
            );
        }

        Ok(self_)
    }

    pub fn with_config_channel(
        mut self,
        config_tx: tokio::sync::broadcast::Sender<DeviceConfig>,
    ) -> Self {
        self.config_tx = Some(config_tx);
        self
    }

    pub fn with_device_event_channel(
        mut self,
        device_event_tx: tokio::sync::broadcast::Sender<DeviceEvent>,
    ) -> Self {
        self.device_event_tx = Some(device_event_tx);
        self
    }

    fn start(&mut self, device_name: Option<&str>) -> Result<DeviceConfig, AudioError> {
        self.running.store(true, Ordering::SeqCst);

        let device = self.device_manager.open_device(device_name)?;
        if let Ok(desc) = device.description() {
            tracing::info!(
                "Selected input device: {} (host: {:?})",
                desc.to_string(),
                self.device_manager.host_id()
            );
        }
        let (config, sample_format) = self.negotiate_config(&device)?;

        let device_config = DeviceConfig {
            sample_rate: config.sample_rate,
            channels: config.channels,
        };

        // Broadcast the device config change if we have a channel
        if let Some(ref tx) = self.config_tx {
            let _ = tx.send(device_config.clone());
        }

        let stream = self.build_stream(device, config.clone(), sample_format)?;
        stream.play()?;

        self.stream = Some(stream);
        self.watchdog.start(Arc::clone(&self.running));
        Ok(device_config)
    }

    fn build_stream(
        &mut self,
        device: cpal::Device,
        config: StreamConfig,
        sample_format: SampleFormat,
    ) -> Result<Stream, AudioError> {
        let audio_producer = Arc::clone(&self.audio_producer);
        let stats = Arc::clone(&self.stats);
        let watchdog = self.watchdog.clone();
        let detector = Arc::new(RwLock::new(self.silence_detector.clone()));
        let running = Arc::clone(&self.running);
        let restart_needed = Arc::clone(&self.restart_needed);

        let err_fn = move |err: cpal::StreamError| {
            tracing::error!("Audio stream error: {}", err);
            // Signal recovery path to restart the stream
            restart_needed.store(true, Ordering::SeqCst);
        };

        // Common handler after converting to i16
        let handle_i16 = move |i16_data: &[i16]| {
            if !running.load(Ordering::SeqCst) {
                return;
            }
            watchdog.feed();
            let mut det = detector.write();
            if det.is_silence(i16_data) {
                stats.silent_frames.fetch_add(1, Ordering::Relaxed);
            } else {
                stats.active_frames.fetch_add(1, Ordering::Relaxed);
            }

            // Use the shared producer
            if let Ok(written) = audio_producer.lock().write(i16_data) {
                if written == i16_data.len() {
                    stats.frames_captured.fetch_add(1, Ordering::Relaxed);
                } else {
                    stats.frames_dropped.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                stats.frames_dropped.fetch_add(1, Ordering::Relaxed);
            }
            *stats.last_frame_time.write() = Some(Instant::now());
        };

        // Build the CPAL input stream with proper conversion to i16
        // Use thread-local buffers to avoid allocations in the audio callback
        // Pre-allocate buffer BEFORE building callbacks to ensure zero-allocation in real-time path
        thread_local! {
            static CONVERT_BUFFER: std::cell::RefCell<Vec<i16>> = const { std::cell::RefCell::new(Vec::new()) };
        }

        // Pre-allocate buffer capacity outside of audio callback (critical for real-time safety)
        // 131072 samples = 8.192s at 16kHz, sufficient for any audio callback size
        CONVERT_BUFFER.with(|buf| {
            let mut v = buf.borrow_mut();
            let current_cap = v.capacity();
            if current_cap < 131072 {
                v.reserve_exact(131072 - current_cap);
            }
            // Verify allocation succeeded
            debug_assert!(
                v.capacity() >= 131072,
                "Failed to pre-allocate CONVERT_BUFFER to required capacity"
            );
        });

        let stream = match sample_format {
            SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _: &_| {
                    handle_i16(data);
                },
                err_fn,
                None,
            )?,
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _: &_| {
                    CONVERT_BUFFER.with(|buf| {
                        let mut converted = buf.borrow_mut();
                        converted.clear();
                        // Real-time safety: capacity must be pre-allocated, never allocate here
                        debug_assert!(
                            converted.capacity() >= data.len(),
                            "CONVERT_BUFFER capacity {} insufficient for {} samples",
                            converted.capacity(),
                            data.len()
                        );
                        for &s in data {
                            let clamped = s.clamp(-1.0, 1.0);
                            let v = (clamped * 32767.0).round() as i16;
                            converted.push(v);
                        }
                        handle_i16(&converted);
                    });
                },
                err_fn,
                None,
            )?,
            SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _: &_| {
                    CONVERT_BUFFER.with(|buf| {
                        let mut converted = buf.borrow_mut();
                        converted.clear();
                        debug_assert!(
                            converted.capacity() >= data.len(),
                            "CONVERT_BUFFER capacity {} insufficient for {} samples",
                            converted.capacity(),
                            data.len()
                        );
                        for &s in data {
                            let v = (s as i32 - 32768) as i16;
                            converted.push(v);
                        }
                        handle_i16(&converted);
                    });
                },
                err_fn,
                None,
            )?,
            SampleFormat::U32 => device.build_input_stream(
                &config,
                move |data: &[u32], _: &_| {
                    CONVERT_BUFFER.with(|buf| {
                        let mut converted = buf.borrow_mut();
                        converted.clear();
                        debug_assert!(
                            converted.capacity() >= data.len(),
                            "CONVERT_BUFFER capacity {} insufficient for {} samples",
                            converted.capacity(),
                            data.len()
                        );
                        for &s in data {
                            let centered = s as i64 - 2_147_483_648i64;
                            let v = (centered >> 16) as i16;
                            converted.push(v);
                        }
                        handle_i16(&converted);
                    });
                },
                err_fn,
                None,
            )?,
            SampleFormat::F64 => device.build_input_stream(
                &config,
                move |data: &[f64], _: &_| {
                    CONVERT_BUFFER.with(|buf| {
                        let mut converted = buf.borrow_mut();
                        converted.clear();
                        debug_assert!(
                            converted.capacity() >= data.len(),
                            "CONVERT_BUFFER capacity {} insufficient for {} samples",
                            converted.capacity(),
                            data.len()
                        );
                        for &s in data {
                            let clamped = s.clamp(-1.0, 1.0);
                            let v = (clamped * 32767.0).round() as i16;
                            converted.push(v);
                        }
                        handle_i16(&converted);
                    });
                },
                err_fn,
                None,
            )?,
            other => {
                return Err(AudioError::FormatNotSupported {
                    format: format!("{:?}", other),
                });
            }
        };

        Ok(stream)
    }

    fn negotiate_config(
        &self,
        device: &cpal::Device,
    ) -> Result<(StreamConfig, SampleFormat), AudioError> {
        // Try to get default config first
        if let Ok(default_config) = device.default_input_config() {
            return Ok((
                StreamConfig {
                    channels: default_config.channels(),
                    sample_rate: default_config.sample_rate(),
                    buffer_size: cpal::BufferSize::Default,
                },
                default_config.sample_format(),
            ));
        }

        // Fallback to first available config
        if let Ok(configs) = device.supported_input_configs() {
            if let Some(config) = configs.into_iter().next() {
                return Ok((config.with_max_sample_rate().into(), config.sample_format()));
            }
        }

        Err(AudioError::FormatNotSupported {
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

#[cfg(test)]
mod convert_tests {
    // unit tests for sample format conversions

    #[test]
    fn f32_to_i16_basic() {
        let src = [-1.0f32, -0.5, 0.0, 0.5, 1.0];
        let expected = [-32767i16, -16384, 0, 16384, 32767];
        let mut out = Vec::new();
        for &s in &src {
            out.push((s.clamp(-1.0, 1.0) * 32767.0).round() as i16);
        }
        assert_eq!(&out[..], &expected);
    }

    #[test]
    fn u16_to_i16_centering() {
        let src = [0u16, 32768, 65535];
        let expected = [-32768i16, 0, 32767];
        let out: Vec<i16> = src.iter().map(|&s| (s as i32 - 32768) as i16).collect();
        assert_eq!(&out[..], &expected);
    }

    #[test]
    fn u32_to_i16_scaling() {
        let src = [0u32, 2_147_483_648u32, 4_294_967_295u32];
        let out: Vec<i16> = src
            .iter()
            .map(|&s| ((s as i64 - 2_147_483_648i64) >> 16) as i16)
            .collect();
        assert_eq!(out[1], 0);
        assert!(out[0] < 0 && out[2] > 0);
    }

    #[test]
    fn f64_to_i16_basic() {
        let src = [-1.0f64, -0.25, 0.25, 1.0];
        let out: Vec<i16> = src
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();
        assert_eq!(out.len(), 4);
        assert!(out[0] <= -32767 && out[3] >= 32766);
    }
}
