pub mod capture;
pub mod chunker;
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

/// Zero-copy shared audio frame used for broadcasting to multiple consumers.
///
/// - samples: i16 PCM at the configured sample rate (typically 16kHz mono)
/// - timestamp: monotonic Instant approximating capture time
/// - sample_rate: sample rate in Hz for the samples buffer
#[derive(Debug, Clone)]
pub struct SharedAudioFrame {
    pub samples: Arc<[i16]>,
    pub timestamp: Instant,
    pub sample_rate: u32,
}
