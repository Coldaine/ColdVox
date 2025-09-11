pub mod capture;
pub mod chunker;
pub mod detector;
pub mod device;
pub mod frame_reader;
pub mod monitor;
pub mod resampler;
pub mod ring_buffer;
pub mod watchdog;

// Public API
pub use capture::{AudioCaptureThread, DeviceConfig};
pub use chunker::{AudioChunker, AudioFrame, ChunkerConfig, ResamplerQuality};
pub use device::{DeviceInfo, DeviceManager};
pub use frame_reader::FrameReader;
pub use monitor::DeviceMonitor;
pub use ring_buffer::AudioRingBuffer;
pub use watchdog::WatchdogTimer;
