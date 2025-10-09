pub mod capture;
pub mod chunker;
pub mod constants;
pub mod detector;
pub mod device;
pub mod frame_reader;
pub mod monitor;
pub mod resampler;
pub mod ring_buffer;
#[cfg(unix)]
pub mod stderr_suppressor;
pub mod watchdog;

// Public API
pub use capture::{AudioCaptureThread, DeviceConfig};
pub use chunker::{AudioChunker, AudioFrame, ChunkerConfig, ResamplerQuality};
pub use device::{DeviceInfo, DeviceManager};
pub use frame_reader::FrameReader;
pub use monitor::DeviceMonitor;
pub use ring_buffer::AudioRingBuffer;
pub use watchdog::WatchdogTimer;

use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct SharedAudioFrame {
    pub samples: Arc<[i16]>,
    pub timestamp: Instant,
    pub sample_rate: u32,
}
