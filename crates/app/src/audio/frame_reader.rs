use std::time::Instant;

use super::ring_buffer::AudioConsumer;
use super::capture::AudioFrame;

/// Reads audio frames from ring buffer and reconstructs metadata
pub struct FrameReader {
    consumer: AudioConsumer,
    sample_rate: u32,
    samples_read: u64,
    start_time: Instant,
}

impl FrameReader {
    /// Create a new FrameReader
    pub fn new(consumer: AudioConsumer, sample_rate: u32) -> Self {
        Self {
            consumer,
            sample_rate,
            samples_read: 0,
            start_time: Instant::now(),
        }
    }

    /// Read next audio frame, reconstructing timestamp from sample count
    pub fn read_frame(&mut self, max_samples: usize) -> Option<AudioFrame> {
        let mut buffer = vec![0i16; max_samples];
        let samples_read = self.consumer.read(&mut buffer);
        
        if samples_read == 0 {
            return None;
        }

        buffer.truncate(samples_read);
        
        // Calculate timestamp based on samples read
        let elapsed_samples = self.samples_read;
        let elapsed_ms = (elapsed_samples * 1000) / self.sample_rate as u64;
        let timestamp = self.start_time + std::time::Duration::from_millis(elapsed_ms);
        
        self.samples_read += samples_read as u64;

        Some(AudioFrame {
            samples: buffer,
            timestamp,
            sample_rate: self.sample_rate,
            channels: 1, // Mono
        })
    }

    /// Check how many samples are available to read
    pub fn available_samples(&self) -> usize {
        self.consumer.slots()
    }
}