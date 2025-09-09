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

/// Calculate Word Error Rate (WER) between reference and hypothesis strings.
/// Uses word-level Levenshtein distance divided by reference word count.
/// 
/// **Note:** This function is deprecated. Use `crate::stt::tests::wer_utils::calculate_wer` instead
/// for enhanced functionality and consistent return types.
#[deprecated(since = "0.1.0", note = "Use crate::stt::tests::wer_utils::calculate_wer instead")]
pub fn calculate_wer(reference: &str, hypothesis: &str) -> f32 {
    use crate::stt::tests::wer_utils;
    wer_utils::calculate_wer(reference, hypothesis) as f32
}

#[cfg(test)]
mod wer_tests {
    use super::calculate_wer;

    #[test]
    fn test_wer_basic() {
        assert_eq!(calculate_wer("hello world", "hello world"), 0.0);
        let w = calculate_wer("hello world", "hello there");
        assert!((w - 0.5).abs() < 1e-6);
        let w = calculate_wer("one two three", "one three");
        assert!((w - (1.0 / 3.0)).abs() < 1e-6);
        let w = calculate_wer("one two", "one two three");
        assert!((w - 0.5).abs() < 1e-6);
        assert_eq!(calculate_wer("one two", ""), 1.0);
    }
}
