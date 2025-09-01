use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use cpal::{SampleFormat, StreamConfig};
use coldvox_audio::capture::{AudioFrame, CaptureStats};
use coldvox_audio::ring_buffer::AudioProducer;

/// Write samples into the audio ring buffer producer in fixed-size chunks.
/// Returns the total number of samples successfully written.
pub fn feed_samples_to_ring_buffer(
    producer: &mut AudioProducer,
    samples: &[i16],
    chunk_size: usize,
) -> usize {
    if chunk_size == 0 { return 0; }
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
pub fn calculate_wer(reference: &str, hypothesis: &str) -> f32 {
    let ref_words: Vec<&str> = reference.split_whitespace().collect();
    let hyp_words: Vec<&str> = hypothesis.split_whitespace().collect();

    let n = ref_words.len();
    let m = hyp_words.len();
    if n == 0 { return if m > 0 { 1.0 } else { 0.0 }; }

    // dp[i][j]: min edits to transform first i ref words into first j hyp words
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n { dp[i][0] = i; }
    for j in 0..=m { dp[0][j] = j; }

    for i in 1..=n {
        for j in 1..=m {
            let cost = if ref_words[i - 1] == hyp_words[j - 1] { 0 } else { 1 };
            let sub = dp[i - 1][j - 1] + cost;
            let del = dp[i - 1][j] + 1;
            let ins = dp[i][j - 1] + 1;
            dp[i][j] = sub.min(del).min(ins);
        }
    }

    dp[n][m] as f32 / n as f32
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
        assert!((w - (1.0/3.0)).abs() < 1e-6);
        let w = calculate_wer("one two", "one two three");
        assert!((w - 0.5).abs() < 1e-6);
        assert_eq!(calculate_wer("one two", ""), 1.0);
    }
}