//! Audio processing constants for VAD pipeline

/// Standard sample rate for all VAD processing (Hz)
pub const SAMPLE_RATE_HZ: u32 = 16_000;

/// Standard frame size for all VAD processing (samples)
/// At 16kHz, 512 samples = 32ms frames
pub const FRAME_SIZE_SAMPLES: usize = 512;

/// Standard number of channels for mono audio processing
pub const CHANNELS_MONO: u16 = 1;

/// Frame duration in milliseconds (derived constant)
pub const FRAME_DURATION_MS: f32 = (FRAME_SIZE_SAMPLES as f32 * 1000.0) / SAMPLE_RATE_HZ as f32;
