/// Word Error Rate (WER) calculation utilities for STT testing.
///
/// Centralized implementation to avoid code duplication between test files.

/// Calculate Word Error Rate (WER) between reference and hypothesis strings.
///
/// WER = (insertions + deletions + substitutions) / reference_word_count
///
/// Uses word-level Levenshtein distance algorithm for accurate error counting.
pub fn calculate_wer(reference: &str, hypothesis: &str) -> f64 {
    let ref_words: Vec<&str> = reference.split_whitespace().collect();
    let hyp_words: Vec<&str> = hypothesis.split_whitespace().collect();

    if ref_words.is_empty() {
        return if hyp_words.is_empty() { 0.0 } else { 1.0 };
    }

    let ref_len = ref_words.len();
    let hyp_len = hyp_words.len();

    // Dynamic programming table for Levenshtein distance
    let mut dp = vec![vec![0; hyp_len + 1]; ref_len + 1];

    // Initialize base cases
    for i in 0..=ref_len {
        dp[i][0] = i; // deletions
    }

    for j in 0..=hyp_len {
        dp[0][j] = j; // insertions
    }

    // Fill the DP table
    for i in 1..=ref_len {
        for j in 1..=hyp_len {
            let substitution_cost = if ref_words[i - 1] == hyp_words[j - 1] {
                0
            } else {
                1
            };

            dp[i][j] = std::cmp::min(
                std::cmp::min(
                    dp[i - 1][j] + 1, // deletion
                    dp[i][j - 1] + 1, // insertion
                ),
                dp[i - 1][j - 1] + substitution_cost, // substitution (or match)
            );
        }
    }

    dp[ref_len][hyp_len] as f64 / ref_len as f64
}

/// Format WER as a percentage string.
pub fn format_wer_percentage(wer: f64) -> String {
    format!("{:.1}%", wer * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_wer_basic() {
        assert_eq!(calculate_wer("hello world", "hello world"), 0.0);
        assert!((calculate_wer("hello world", "hello there") - 0.5).abs() < 1e-10);
        assert_eq!(calculate_wer("", ""), 0.0);
        assert_eq!(calculate_wer("hello", ""), 1.0);
        assert_eq!(calculate_wer("", "hello"), 1.0);
    }

    #[test]
    fn test_format_wer_percentage() {
        assert_eq!(format_wer_percentage(0.0), "0.0%");
        assert_eq!(format_wer_percentage(0.15), "15.0%");
        assert_eq!(format_wer_percentage(0.333), "33.3%");
    }
}
