//! Constants for STT processing

/// Standard sample rate for STT processing (16 kHz)
pub(crate) const SAMPLE_RATE_HZ: u32 = 16_000;

/// Frame size in samples for STT processing
pub(crate) const FRAME_SIZE_SAMPLES: u32 = 512;

/// Default buffered audio duration used by the processor (10 seconds)
pub(crate) const DEFAULT_BUFFER_DURATION_SECONDS: usize = 10;

/// Default chunk size (1 second of audio at 16kHz)
pub(crate) const DEFAULT_CHUNK_SIZE_SAMPLES: usize = SAMPLE_RATE_HZ as usize;

/// Interval (in frames) for buffer logging
pub(crate) const LOGGING_INTERVAL_FRAMES: u64 = 100;

/// Timeout for sending transcription events (seconds)
pub(crate) const SEND_TIMEOUT_SECONDS: u64 = 5;

/// Default buffer size for plugin configurations, in milliseconds
pub(crate) const DEFAULT_BUFFER_SIZE_MS: u32 = 512;
