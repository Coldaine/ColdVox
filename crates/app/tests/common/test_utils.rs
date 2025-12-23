#![allow(dead_code)] // Utility functions may not be used in all test binaries

use coldvox_audio::ring_buffer::AudioProducer;

/// Write samples into the audio ring buffer producer in fixed-size chunks.
/// Returns the total number of samples successfully written.
pub fn feed_samples_to_ring_buffer(
    producer: &mut AudioProducer,
    samples: &[i16],
    chunk_size: usize,
) -> usize {
    if chunk_size == 0 {
        return 0;
    }
    let mut written_total = 0usize;
    let mut offset = 0usize;
    while offset < samples.len() {
        let end = (offset + chunk_size).min(samples.len());
        match producer.write(&samples[offset..end]) {
            Ok(written) => {
                written_total += written;
                offset += written;
            }
            Err(_) => {
                // Buffer full; stop to avoid busy-wait in tests
                break;
            }
        }
    }
    written_total
}
