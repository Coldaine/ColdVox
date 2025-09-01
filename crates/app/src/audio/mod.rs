pub mod vad_adapter;
pub mod vad_processor;

// Re-export modules from coldvox-audio crate
pub use coldvox_audio::{
    chunker::{AudioChunker, ChunkerConfig, ResamplerQuality},
    frame_reader::FrameReader,
    ring_buffer::{AudioRingBuffer, AudioProducer},
    capture::CaptureStats,
};

pub use vad_adapter::*;
pub use vad_processor::*;
pub use coldvox_audio::AudioFrame;
