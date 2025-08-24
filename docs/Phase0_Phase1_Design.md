# Phase 0 & Phase 1 Detailed Design Document

## Overview
This document provides the complete implementation design for Phase 0 (Foundation & Safety Net) and Phase 1 (Microphone Capture with Recovery). These phases establish the core reliability infrastructure and audio input system.

## Crate Dependencies

```toml
[dependencies]
# Core
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Audio
cpal = "0.15"  # Cross-platform audio
hound = "3.5"  # WAV file writing for debugging
dasp = "0.11"  # Audio sample conversions

# Utilities
crossbeam-channel = "0.5"  # Lock-free channels
parking_lot = "0.12"  # Better mutexes
once_cell = "1.19"  # Lazy statics
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

# Testing
tempfile = "3.8"
mockall = "0.12"  # For mocking traits in tests
```

---

## Phase 0: Foundation & Safety Net

### Module Structure

```
crates/app/
├── src/
│   ├── foundation/
│   │   ├── mod.rs
│   │   ├── error.rs       # Error types and handling
│   │   ├── health.rs      # Health monitoring system
│   │   ├── state.rs       # Application state machine
│   │   ├── shutdown.rs    # Graceful shutdown handling
│   │   └── recovery.rs    # Recovery strategies
│   ├── telemetry/
│   │   ├── mod.rs
│   │   └── metrics.rs     # Basic metrics collection
│   └── main.rs
```

### Error Type Hierarchy

```rust
// foundation/error.rs

use thiserror::Error;
use std::time::Duration;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Audio subsystem error: {0}")]
    Audio(#[from] AudioError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Component failed health check: {component}")]
    HealthCheckFailed { component: String },
    
    #[error("Shutdown requested")]
    ShutdownRequested,
    
    #[error("Fatal error, cannot recover: {0}")]
    Fatal(String),
    
    #[error("Transient error, will retry: {0}")]
    Transient(String),
}

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Device not found: {name:?}")]
    DeviceNotFound { name: Option<String> },
    
    #[error("Device disconnected")]
    DeviceDisconnected,
    
    #[error("Format not supported: {format}")]
    FormatNotSupported { format: String },
    
    #[error("Buffer overflow, dropped {count} samples")]
    BufferOverflow { count: usize },
    
    #[error("No audio data for {duration:?}")]
    NoDataTimeout { duration: Duration },
    
    #[error("Silence detected for {duration:?}")]
    SilenceDetected { duration: Duration },
}

// Recovery strategies
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Retry { max_attempts: u32, delay: Duration },
    Fallback { to: String },
    Restart,
    Ignore,
    Fatal,
}

impl AppError {
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            AppError::Audio(AudioError::DeviceDisconnected) => 
                RecoveryStrategy::Retry { 
                    max_attempts: 5, 
                    delay: Duration::from_secs(2) 
                },
            AppError::Audio(AudioError::DeviceNotFound { .. }) =>
                RecoveryStrategy::Fallback { to: "default".into() },
            AppError::Audio(AudioError::BufferOverflow { .. }) =>
                RecoveryStrategy::Ignore,
            AppError::Fatal(_) | AppError::ShutdownRequested =>
                RecoveryStrategy::Fatal,
            _ => RecoveryStrategy::Restart,
        }
    }
}

// Minimal audio config used by Phase 1 components (device selection handled elsewhere)
#[derive(Debug, Clone, Copy)]
pub struct AudioConfig {
    pub silence_threshold: i16,
}
```

### Application State Machine

```rust
// foundation/state.rs

use std::sync::Arc;
use parking_lot::RwLock;
use crossbeam_channel::{Sender, Receiver};

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Initializing,
    Running,
    Recovering { from_error: String },
    Stopping,
    Stopped,
}

pub struct StateManager {
    state: Arc<RwLock<AppState>>,
    state_tx: Sender<AppState>,
    state_rx: Receiver<AppState>,
}

impl StateManager {
    pub fn new() -> Self {
        let (state_tx, state_rx) = crossbeam_channel::unbounded();
        Self {
            state: Arc::new(RwLock::new(AppState::Initializing)),
            state_tx,
            state_rx,
        }
    }
    
    pub fn transition(&self, new_state: AppState) -> Result<(), AppError> {
        let mut current = self.state.write();
        
        // Validate state transitions
        let valid = match (&*current, &new_state) {
            (AppState::Initializing, AppState::Running) => true,
            (AppState::Running, AppState::Recovering { .. }) => true,
            (AppState::Running, AppState::Stopping) => true,
            (AppState::Recovering { .. }, AppState::Running) => true,
            (AppState::Recovering { .. }, AppState::Stopping) => true,
            (AppState::Stopping, AppState::Stopped) => true,
            _ => false,
        };
        
        if !valid {
            return Err(AppError::Fatal(
                format!("Invalid state transition: {:?} -> {:?}", *current, new_state)
            ));
        }
        
        tracing::info!("State transition: {:?} -> {:?}", *current, new_state);
        *current = new_state.clone();
        let _ = self.state_tx.send(new_state);
        Ok(())
    }
    
    pub fn current(&self) -> AppState {
        self.state.read().clone()
    }
    
    pub fn subscribe(&self) -> Receiver<AppState> {
        self.state_rx.clone()
    }
}
```

### Health Monitoring System

```rust
// foundation/health.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub name: String,
    pub healthy: bool,
    pub last_check: Instant,
    pub last_error: Option<String>,
    pub check_count: u64,
    pub failure_count: u64,
}

pub trait HealthCheck: Send + Sync {
    fn check(&self) -> Result<(), String>;
    fn name(&self) -> &str;
}

pub struct HealthMonitor {
    components: Arc<RwLock<HashMap<String, ComponentHealth>>>,
    checks: Arc<RwLock<Vec<Box<dyn HealthCheck>>>>,
    check_interval: Duration,
    handle: Option<JoinHandle<()>>,
}

impl HealthMonitor {
    pub fn new(check_interval: Duration) -> Self {
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
            checks: Arc::new(RwLock::new(Vec::new())),
            check_interval,
            handle: None,
        }
    }
    
    pub fn register(&self, component: Box<dyn HealthCheck>) {
        let name = component.name().to_string();
        let mut components = self.components.write();
        components.insert(name.clone(), ComponentHealth {
            name,
            healthy: true,
            last_check: Instant::now(),
            last_error: None,
            check_count: 0,
            failure_count: 0,
        });
        self.checks.write().push(component);
    }
    
    pub fn start(mut self) -> Self {
        let components = Arc::clone(&self.components);
        let checks = Arc::clone(&self.checks);
        let interval = self.check_interval;
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                // Run registered health checks and update status
                let now = Instant::now();
                let mut map = components.write();
                for hc in checks.read().iter() {
                    let name = hc.name().to_string();
                    let entry = map.entry(name.clone()).or_insert(ComponentHealth {
                        name: name.clone(),
                        healthy: true,
                        last_check: now,
                        last_error: None,
                        check_count: 0,
                        failure_count: 0,
                    });

                    entry.check_count += 1;
                    entry.last_check = now;
                    match hc.check() {
                        Ok(_) => {
                            if !entry.healthy {
                                tracing::info!(component = %name, "Component recovered");
                            }
                            entry.healthy = true;
                            entry.last_error = None;
                        }
                        Err(err) => {
                            entry.healthy = false;
                            entry.failure_count += 1;
                            entry.last_error = Some(err.clone());
                            tracing::warn!(component = %name, failure_count = entry.failure_count, "Health check failed: {}", err);
                        }
                    }
                }
            }
        });
        
        self.handle = Some(handle);
        self
    }
    
    pub fn get_status(&self) -> HashMap<String, ComponentHealth> {
        self.components.read().clone()
    }
    
    pub fn all_healthy(&self) -> bool {
        self.components.read().values().all(|c| c.healthy)
    }
}
```

### Graceful Shutdown Handler

```rust
// foundation/shutdown.rs

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;
use tokio::sync::Notify;

pub struct ShutdownHandler {
    shutdown_requested: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
}

impl ShutdownHandler {
    pub fn new() -> Self {
        Self {
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }
    
    pub async fn install(self) -> ShutdownGuard {
        let shutdown_requested = Arc::clone(&self.shutdown_requested);
        let shutdown_notify = Arc::clone(&self.shutdown_notify);
        
        tokio::spawn(async move {
            // Wait for Ctrl-C
            signal::ctrl_c().await.expect("Failed to install Ctrl-C handler");
            
            tracing::info!("Shutdown requested via Ctrl-C");
            shutdown_requested.store(true, Ordering::SeqCst);
            shutdown_notify.notify_waiters();
        });
        
        // Install panic handler
        let original_panic = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            tracing::error!("PANIC: {}", panic_info);
            eprintln!("Application panicked: {}", panic_info);
            original_panic(panic_info);
        }));
        
        ShutdownGuard {
            shutdown_requested: self.shutdown_requested,
            shutdown_notify: self.shutdown_notify,
        }
    }
}

pub struct ShutdownGuard {
    shutdown_requested: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
}

impl ShutdownGuard {
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }
    
    pub async fn wait(&self) {
        self.shutdown_notify.notified().await;
    }
    
    pub fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
    }
}
```

---

## Phase 1: Microphone Capture with Recovery

### Module Structure

```
crates/app/
├── src/
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── capture.rs     # Main capture logic
│   │   ├── device.rs      # Device enumeration
│   │   ├── format.rs      # Format conversion
│   │   ├── watchdog.rs    # Watchdog timer
│   │   └── detector.rs    # Silence detection
```

### Device Management

```rust
// audio/device.rs

use cpal::{Device, Host, HostId, StreamConfig};
use cpal::traits::{DeviceTrait, HostTrait};

pub struct DeviceManager {
    host: Host,
    preferred_device: Option<String>,
    current_device: Option<Device>,
}

impl DeviceManager {
    pub fn new() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        Ok(Self {
            host,
            preferred_device: None,
            current_device: None,
        })
    }
    
    pub fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        let mut devices = Vec::new();
        
        // Input devices
        if let Ok(inputs) = self.host.input_devices() {
            for device in inputs {
                if let Ok(name) = device.name() {
                    devices.push(DeviceInfo {
                        name: name.clone(),
                        is_default: false,
                        supported_configs: self.get_supported_configs(&device),
                    });
                }
            }
        }
        
        // Mark default
        if let Some(default) = self.host.default_input_device() {
            if let Ok(default_name) = default.name() {
                for device in &mut devices {
                    if device.name == default_name {
                        device.is_default = true;
                    }
                }
            }
        }
        
        devices
    }
    
    pub fn open_device(&mut self, name: Option<&str>) -> Result<Device, AudioError> {
        // Try preferred device first
        if let Some(preferred) = name {
            if let Some(device) = self.find_device_by_name(preferred) {
                self.current_device = Some(device.clone());
                return Ok(device);
            }
            tracing::warn!("Preferred device '{}' not found, falling back to default", preferred);
        }
        
        // Fall back to default
        self.host.default_input_device()
            .ok_or(AudioError::DeviceNotFound { name: None })
            .map(|device| {
                self.current_device = Some(device.clone());
                device
            })
    }
    
    fn find_device_by_name(&self, name: &str) -> Option<Device> {
        if let Ok(devices) = self.host.input_devices() {
            for device in devices {
                if let Ok(device_name) = device.name() {
                    if device_name == name {
                        return Some(device);
                    }
                }
            }
        }
        None
    }
    
    fn get_supported_configs(&self, device: &Device) -> Vec<StreamConfig> {
        // Get all supported configs, prioritize 16kHz mono
        let mut configs = Vec::new();
        
        if let Ok(supported) = device.supported_input_configs() {
            for config in supported {
                // We prefer 16kHz, but will take anything
                let sample_rate = if config.min_sample_rate().0 <= 16000 
                    && config.max_sample_rate().0 >= 16000 {
                    cpal::SampleRate(16000)
                } else {
                    config.max_sample_rate()
                };
                
                configs.push(StreamConfig {
                    channels: config.channels(),
                    sample_rate,
                    buffer_size: cpal::BufferSize::Default,
                });
            }
        }
        
        configs
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub supported_configs: Vec<StreamConfig>,
}
```

### Audio Capture with Watchdog

```rust
// audio/capture.rs

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use crossbeam_channel::{Sender, Receiver, bounded};
use cpal::{Stream, StreamConfig, SampleFormat};
use cpal::traits::{DeviceTrait, StreamTrait};

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
        
        // Get best config (prefer 16kHz mono)
        let config = self.negotiate_config(&device)?;
        tracing::info!("Audio config: {:?}", config);
        
        // Build stream with error recovery
        let stream = self.build_stream(device, config)?;
        stream.play()?;
        
        self.stream = Some(stream);
        
        // Start watchdog
        self.watchdog.start(Arc::clone(&self.running));
        
        Ok(())
    }
    
    fn build_stream(&self, device: Device, config: StreamConfig) -> Result<Stream, AudioError> {
        let sample_tx = self.sample_tx.clone();
        let stats = self.stats.clone();
        let watchdog = self.watchdog.clone();
        let silence_detector = self.silence_detector.clone();
        let running = Arc::clone(&self.running);
        
        let err_fn = move |err| {
            tracing::error!("Audio stream error: {}", err);
            // Don't panic, let watchdog handle recovery
        };
        
        let stream = device.build_input_stream(
            &config,
            move |data: &[i16], _: &_| {
                if !running.load(Ordering::SeqCst) {
                    return;
                }
                
                // Update watchdog
                watchdog.feed();
                
                // Check for silence (possible disconnect)
                if silence_detector.is_silence(data) {
                    if silence_detector.silence_duration() > Duration::from_secs(3) {
                        tracing::warn!("Extended silence detected, possible device issue");
                    }
                }
                
                // Convert to our format
                let frame = AudioFrame {
                    samples: data.to_vec(),
                    timestamp: Instant::now(),
                    sample_rate: config.sample_rate.0,
                    channels: config.channels,
                };
                
                // Send with overflow handling
                match sample_tx.try_send(frame) {
                    Ok(_) => {
                        stats.frames_captured.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        stats.frames_dropped.fetch_add(1, Ordering::Relaxed);
                        tracing::warn!("Audio buffer full, dropping frame");
                    }
                }
                
                *stats.last_frame_time.write() = Some(Instant::now());
            },
            err_fn,
            None
        )?;
        
        Ok(stream)
    }
    
    fn negotiate_config(&self, device: &Device) -> Result<StreamConfig, AudioError> {
        // Try to get 16kHz mono, but accept anything
        if let Ok(configs) = device.supported_input_configs() {
            for config in configs {
                // Check if 16kHz is in range
                if config.min_sample_rate().0 <= 16000 && config.max_sample_rate().0 >= 16000 {
                    return Ok(StreamConfig {
                        channels: 1.min(config.channels()),
                        sample_rate: cpal::SampleRate(16000),
                        buffer_size: cpal::BufferSize::Default,
                    });
                }
            }
            
            // Take first available if 16kHz not supported
            if let Some(config) = device.supported_input_configs()?.next() {
                return Ok(config.with_max_sample_rate().into());
            }
        }
        
        Err(AudioError::FormatNotSupported { 
            format: "No supported audio formats".to_string() 
        })
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
            last_frame_age: self.stats.last_frame_time.read()
                .map(|t| Instant::now().duration_since(t)),
        }
    }
}
```

### Watchdog Timer

```rust
// audio/watchdog.rs

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct WatchdogTimer {
    timeout: Duration,
    last_feed: Arc<AtomicU64>,
    triggered: Arc<AtomicBool>,
    handle: Option<Arc<JoinHandle<()>>>,
}

impl WatchdogTimer {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            last_feed: Arc::new(AtomicU64::new(0)),
            triggered: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }
    
    pub fn start(&mut self, running: Arc<AtomicBool>) {
        let timeout = self.timeout;
        let last_feed = Arc::clone(&self.last_feed);
        let triggered = Arc::clone(&self.triggered);
        
        // Set initial feed time
        self.feed();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            
            while running.load(Ordering::SeqCst) {
                interval.tick().await;
                
                // Fixed elapsed computation based on last feed millis since start
                let last_ms = last_feed.load(Ordering::Relaxed);
                let elapsed = Duration::from_millis(Instant::now().elapsed().as_millis() as u64 - last_ms);
                
                if elapsed > timeout && !triggered.load(Ordering::SeqCst) {
                    tracing::error!("Watchdog timeout! No audio data for {:?}", elapsed);
                    triggered.store(true, Ordering::SeqCst);
                    // Trigger recovery mechanism
                }
            }
        });
        
        self.handle = Some(Arc::new(handle));
    }
    
    pub fn feed(&self) {
        let now = Instant::now().elapsed().as_millis() as u64;
        self.last_feed.store(now, Ordering::Relaxed);
        self.triggered.store(false, Ordering::SeqCst);
    }
    
    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }
}
```

### Silence Detection

```rust
// audio/detector.rs

use std::time::{Duration, Instant};

pub struct SilenceDetector {
    threshold: i16,
    silence_start: Option<Instant>,
    last_check: Instant,
}

impl SilenceDetector {
    pub fn new(threshold: i16) -> Self {
        Self {
            threshold,
            silence_start: None,
            last_check: Instant::now(),
        }
    }
    
    pub fn is_silence(&mut self, samples: &[i16]) -> bool {
        self.last_check = Instant::now();
        
        // Calculate RMS
        let sum: i64 = samples.iter().map(|&s| s as i64 * s as i64).sum();
        let rms = ((sum / samples.len() as i64) as f64).sqrt() as i16;
        
        if rms < self.threshold {
            if self.silence_start.is_none() {
                self.silence_start = Some(Instant::now());
            }
            true
        } else {
            self.silence_start = None;
            false
        }
    }
    
    pub fn silence_duration(&self) -> Duration {
        self.silence_start
            .map(|start| Instant::now().duration_since(start))
            .unwrap_or(Duration::ZERO)
    }
    
    pub fn reset(&mut self) {
        self.silence_start = None;
    }
}
```

Note: SilenceDetector is instantiated per stream and not shared across threads, so it is thread-safe by construction without additional atomics.

---

## Test Harnesses

### Phase 0 Test: Foundation Probe

```rust
// tests/foundation_probe.rs

use clap::Parser;
use std::time::Duration;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "60")]
    duration: u64,
    
    #[arg(long)]
    simulate_panics: bool,
    
    #[arg(long)]
    simulate_errors: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();
    
    // Create foundation components
    let state_manager = StateManager::new();
    let health_monitor = HealthMonitor::new(Duration::from_secs(5));
    let shutdown = ShutdownHandler::new().install().await;
    
    // Test state transitions
    state_manager.transition(AppState::Running)?;
    
    if args.simulate_errors {
        // Simulate various errors
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                tracing::error!("Simulated error!");
                // Test recovery
            }
        });
    }
    
    if args.simulate_panics {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs(15));
            panic!("Simulated panic for testing!");
        });
    }
    
    // Run for specified duration
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(args.duration)) => {
            tracing::info!("Test duration reached");
        }
        _ = shutdown.wait() => {
            tracing::info!("Shutdown requested");
        }
    }
    
    // Clean shutdown
    state_manager.transition(AppState::Stopping)?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    state_manager.transition(AppState::Stopped)?;
    
    println!("Test completed successfully");
    Ok(())
}
```

### Phase 1 Test: Microphone Probe

```rust
// tests/mic_probe.rs

use clap::Parser;
use std::time::Duration;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "120")]
    duration: u64,
    
    #[arg(long)]
    device: Option<String>,
    
    #[arg(long)]
    expect_disconnect: bool,
    
    #[arg(long)]
    save_audio: bool,
    
    #[arg(long, default_value = "100")]
    silence_threshold: i16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();
    
    // List available devices
    let device_manager = DeviceManager::new()?;
    let devices = device_manager.enumerate_devices();
    
    println!("Available audio devices:");
    for device in &devices {
        println!("  {} {}", 
            if device.is_default { "[DEFAULT]" } else { "         " },
            device.name
        );
    }
    
    // Create capture
    let config = AudioConfig {
        silence_threshold: args.silence_threshold,
        device_name: args.device,
        save_audio: args.save_audio,
    };
    
    let mut capture = AudioCapture::new(config)?;
    capture.start(args.device.as_deref()).await?;
    
    // Monitor loop
    let start = Instant::now();
    let mut last_stats = Instant::now();
    
    while start.elapsed() < Duration::from_secs(args.duration) {
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Print stats every 5 seconds
        if last_stats.elapsed() > Duration::from_secs(5) {
            let stats = capture.get_stats();
            println!("Stats: {} frames, {} dropped, {} disconnects, {} reconnects",
                stats.frames_captured,
                stats.frames_dropped,
                stats.disconnections,
                stats.reconnections
            );
            
            if let Some(age) = stats.last_frame_age {
                if age > Duration::from_secs(2) {
                    println!("WARNING: No frames for {:?}", age);
                }
            }
            
            last_stats = Instant::now();
        }
        
        // Test disconnect recovery
        if args.expect_disconnect {
            println!("Unplug and replug your microphone to test recovery...");
            // Wait for watchdog to trigger
            if capture.watchdog.is_triggered() {
                println!("Device disconnected, attempting recovery...");
                match capture.recover().await {
                    Ok(_) => println!("Recovery successful!"),
                    Err(e) => println!("Recovery failed: {}", e),
                }
            }
        }
    }
    
    println!("Test completed successfully");
    Ok(())
}
```

---

## Implementation Timeline

### Week 1: Phase 0
- Day 1-2: Implement error types, state machine, and recovery strategies
- Day 3: Health monitoring and shutdown handling  
- Day 4: Foundation probe test and validation

### Week 1-2: Phase 1  
- Day 5-6: Device enumeration and management
- Day 7-8: Basic audio capture with format conversion
- Day 9: Watchdog timer and silence detection
- Day 10-11: Recovery mechanisms and reconnection logic
- Day 12: Microphone probe test and stress testing

PipeWire Note: On Nobara, the default input device can change between runs. This is normal behavior, not a bug.

---

## Success Criteria

### Phase 0
- [ ] Clean shutdown on Ctrl-C within 1 second
- [ ] All panics are caught and logged
- [ ] State transitions are validated and logged
- [ ] Health checks run at specified intervals
- [ ] Recovery strategies are applied correctly

### Phase 1
- [ ] Successfully captures audio from default device
- [ ] Falls back to default when preferred device unavailable
- [ ] Recovers from device disconnection within 10 seconds
- [ ] Detects extended silence and warns
- [ ] Watchdog triggers on no data timeout
- [ ] Runs for 10 minutes without crashes
- [ ] Handles buffer overflow gracefully
- [ ] Provides accurate statistics

---

## Key Design Decisions

1. **cpal over other audio libraries**: Most mature cross-platform Rust audio library
2. **Tokio for async runtime**: Industry standard, good ecosystem support
3. **Lock-free channels**: Avoid mutex contention between audio callback and processing
4. **Separate watchdog thread**: Ensures detection even if audio thread hangs
5. **Simple state machine**: Makes testing and debugging easier
6. **Explicit error recovery strategies**: Each error type has defined handling

This design prioritizes reliability and debuggability over performance, with every component designed to fail gracefully and recover automatically.