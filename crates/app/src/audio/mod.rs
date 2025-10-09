pub mod vad_adapter;
pub mod vad_processor;

// Re-export modules from coldvox-audio crate
pub use coldvox_audio::{
    capture::CaptureStats,
    chunker::{AudioChunker, ChunkerConfig, ResamplerQuality},
    frame_reader::FrameReader,
    ring_buffer::{AudioProducer, AudioRingBuffer},
};

pub use coldvox_audio::AudioFrame;
pub use vad_adapter::*;
pub use vad_processor::*;
