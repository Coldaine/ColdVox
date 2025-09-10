/// Word Error Rate (WER) calculation utilities for test assertions and validation.
/// 
/// WER is computed using word-level Levenshtein (edit) distance, normalized by
/// the reference word count. This is standard for speech recognition evaluation.

use std::cmp;

/// Calculate Word Error Rate (WER) between reference and hypothesis strings.
/// 
/// WER = (insertions + deletions + substitutions) / reference_word_count
/// 
/// Uses word-level Levenshtein distance algorithm for accurate error counting.
/// Returns a value between 0.0 (perfect match) and potentially > 1.0 (if hypothesis
/// has many more words than reference).
/// 
/// # Arguments
/// * `reference` - The ground truth text 
/// * `hypothesis` - The predicted/transcribed text
/// 
/// # Returns
/// * `f64` - Word Error Rate as a decimal (0.0 = perfect, 1.0 = 100% error)
/// 
/// # Examples
/// ```
/// use crate::wer::calculate_wer;
/// 
/// assert_eq!(calculate_wer("hello world", "hello world"), 0.0);
/// assert_eq!(calculate_wer("hello world", "hello there"), 0.5);
/// assert_eq!(calculate_wer("", "any text"), 1.0);
/// ```
pub fn calculate_wer(reference: &str, hypothesis: &str) -> f64 {
    let ref_words: Vec<&str> = reference.split_whitespace().collect();
    let hyp_words: Vec<&str> = hypothesis.split_whitespace().collect();
    
    if ref_words.is_empty() {
        return if hyp_words.is_empty() { 0.0 } else { 1.0 };
    }
    
    let ref_len = ref_words.len();
    let hyp_len = hyp_words.len();
    
    // Dynamic programming table for Levenshtein distance
    // dp[i][j] = minimum edits to transform first i reference words into first j hypothesis words
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
            let substitution_cost = if ref_words[i-1] == hyp_words[j-1] { 0 } else { 1 };
            
            dp[i][j] = cmp::min(
                cmp::min(
                    dp[i-1][j] + 1,         // deletion
                    dp[i][j-1] + 1          // insertion
                ),
                dp[i-1][j-1] + substitution_cost  // substitution (or match)
            );
        }
    }
    
    dp[ref_len][hyp_len] as f64 / ref_len as f64
}

/// Format WER as a percentage string with specified decimal places.
/// 
/// # Arguments
/// * `wer` - WER value as decimal (e.g., 0.15)
/// * `decimal_places` - Number of decimal places to display (default: 1)
/// 
/// # Returns
/// * `String` - Formatted percentage (e.g., "15.0%")
/// 
/// # Examples
/// ```
/// assert_eq!(format_wer_percentage(0.15, 1), "15.0%");
/// assert_eq!(format_wer_percentage(0.1234, 2), "12.34%");
/// ```
pub fn format_wer_percentage(wer: f64, decimal_places: usize) -> String {
    format!("{:.prec$}%", wer * 100.0, prec = decimal_places)
}

/// Format WER as a percentage string with 1 decimal place (convenience function).
pub fn format_wer(wer: f64) -> String {
    format_wer_percentage(wer, 1)
}

/// Assert that WER is below a given threshold, with detailed error message.
/// 
/// This provides better test failure messages than manual assertions.
/// 
/// # Arguments
/// * `reference` - The ground truth text
/// * `hypothesis` - The predicted text to validate
/// * `threshold` - Maximum acceptable WER (e.g., 0.3 for 30%)
/// * `test_name` - Optional test identifier for error messages
/// 
/// # Panics
/// * If WER exceeds the threshold
/// 
/// # Examples
/// ```
/// assert_wer_below_threshold("hello world", "hello there", 0.6, Some("basic test"));
/// // This will panic if WER > 0.6
/// ```
pub fn assert_wer_below_threshold(
    reference: &str, 
    hypothesis: &str, 
    threshold: f64,
    test_name: Option<&str>
) {
    let wer = calculate_wer(reference, hypothesis);
    
    if wer > threshold {
        let test_info = test_name.map(|name| format!("[{}] ", name)).unwrap_or_default();
        panic!(
            "{}WER {} exceeds threshold {}\n  Reference:  '{}'\n  Hypothesis: '{}'\n  Reference words: {}, Hypothesis words: {}",
            test_info,
            format_wer(wer),
            format_wer(threshold),
            reference,
            hypothesis,
            reference.split_whitespace().count(),
            hypothesis.split_whitespace().count()
        );
    }
}

/// Compute detailed WER metrics including breakdown of error types.
/// 
/// This is useful for more detailed analysis beyond just the final WER score.
/// 
/// # Returns
/// * `WerMetrics` - Detailed metrics including insertions, deletions, substitutions
#[derive(Debug, Clone, PartialEq)]
pub struct WerMetrics {
    pub wer: f64,
    pub reference_words: usize,
    pub hypothesis_words: usize,
    pub total_errors: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub substitutions: usize,
}

impl WerMetrics {
    pub fn new(reference: &str, hypothesis: &str) -> Self {
        let ref_words: Vec<&str> = reference.split_whitespace().collect();
        let hyp_words: Vec<&str> = hypothesis.split_whitespace().collect();
        
        if ref_words.is_empty() {
            return Self {
                wer: if hyp_words.is_empty() { 0.0 } else { 1.0 },
                reference_words: 0,
                hypothesis_words: hyp_words.len(),
                total_errors: hyp_words.len(),
                insertions: hyp_words.len(),
                deletions: 0,
                substitutions: 0,
            };
        }
        
        let ref_len = ref_words.len();
        let hyp_len = hyp_words.len();
        
        // Enhanced DP to track operation types
        let mut dp = vec![vec![0; hyp_len + 1]; ref_len + 1];
        let mut ops = vec![vec!['N'; hyp_len + 1]; ref_len + 1]; // 'D'=deletion, 'I'=insertion, 'S'=substitution, 'M'=match
        
        // Initialize base cases
        for i in 0..=ref_len {
            dp[i][0] = i;
            if i > 0 { ops[i][0] = 'D'; }
        }
        
        for j in 0..=hyp_len {
            dp[0][j] = j;
            if j > 0 { ops[0][j] = 'I'; }
        }
        
        // Fill the DP table with operation tracking
        for i in 1..=ref_len {
            for j in 1..=hyp_len {
                let match_cost = if ref_words[i-1] == hyp_words[j-1] { 0 } else { 1 };
                let match_total = dp[i-1][j-1] + match_cost;
                let delete_total = dp[i-1][j] + 1;
                let insert_total = dp[i][j-1] + 1;
                
                if match_total <= delete_total && match_total <= insert_total {
                    dp[i][j] = match_total;
                    ops[i][j] = if match_cost == 0 { 'M' } else { 'S' };
                } else if delete_total <= insert_total {
                    dp[i][j] = delete_total;
                    ops[i][j] = 'D';
                } else {
                    dp[i][j] = insert_total;
                    ops[i][j] = 'I';
                }
            }
        }
        
        // Backtrack to count operation types
        let mut insertions = 0;
        let mut deletions = 0;
        let mut substitutions = 0;
        
        let mut i = ref_len;
        let mut j = hyp_len;
        
        while i > 0 || j > 0 {
            match ops[i][j] {
                'M' => { i -= 1; j -= 1; }, // Match - no error
                'S' => { substitutions += 1; i -= 1; j -= 1; },
                'D' => { deletions += 1; i -= 1; },
                'I' => { insertions += 1; j -= 1; },
                _ => break,
            }
        }
        
        let total_errors = insertions + deletions + substitutions;
        let wer = total_errors as f64 / ref_len as f64;
        
        Self {
            wer,
            reference_words: ref_len,
            hypothesis_words: hyp_len,
            total_errors,
            insertions,
            deletions,
            substitutions,
        }
    }
}

impl std::fmt::Display for WerMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, 
            "WER: {} ({}/{} errors) | I:{} D:{} S:{} | Ref:{} Hyp:{} words",
            format_wer(self.wer),
            self.total_errors,
            self.reference_words,
            self.insertions,
            self.deletions, 
            self.substitutions,
            self.reference_words,
            self.hypothesis_words
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_wer_perfect_match() {
        assert_eq!(calculate_wer("hello world", "hello world"), 0.0);
        assert_eq!(calculate_wer("", ""), 0.0);
    }

    #[test] 
    fn test_calculate_wer_complete_mismatch() {
        assert_eq!(calculate_wer("hello world", "foo bar"), 1.0);
        assert_eq!(calculate_wer("hello", ""), 1.0);
        assert_eq!(calculate_wer("", "something"), 1.0);
    }

    #[test]
    fn test_calculate_wer_partial_errors() {
        // 1 substitution out of 2 words = 50%
        assert!((calculate_wer("hello world", "hello there") - 0.5).abs() < 1e-10);
        
        // 1 deletion out of 3 words = 33.33%
        let wer = calculate_wer("one two three", "one three");
        assert!((wer - (1.0 / 3.0)).abs() < 1e-10);
        
        // 1 insertion, 2 reference words = 50% 
        assert!((calculate_wer("one two", "one two three") - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_format_wer_percentage() {
        assert_eq!(format_wer_percentage(0.0, 1), "0.0%");
        assert_eq!(format_wer_percentage(0.15, 1), "15.0%");
        assert_eq!(format_wer_percentage(0.1234, 2), "12.34%");
        assert_eq!(format_wer(0.333), "33.3%");
    }

    #[test]
    fn test_assert_wer_below_threshold_pass() {
        // Should not panic
        assert_wer_below_threshold("hello world", "hello there", 0.6, None);
    }

    #[test]
    #[should_panic(expected = "WER 50.0% exceeds threshold 40.0%")]
    fn test_assert_wer_below_threshold_fail() {
        assert_wer_below_threshold("hello world", "hello there", 0.4, None);
    }

    #[test]
    fn test_wer_metrics_basic() {
        let metrics = WerMetrics::new("hello world", "hello there");
        assert_eq!(metrics.wer, 0.5);
        assert_eq!(metrics.reference_words, 2);
        assert_eq!(metrics.hypothesis_words, 2);
        assert_eq!(metrics.total_errors, 1);
        assert_eq!(metrics.substitutions, 1);
        assert_eq!(metrics.insertions, 0);
        assert_eq!(metrics.deletions, 0);
    }

    #[test] 
    fn test_wer_metrics_insertion() {
        let metrics = WerMetrics::new("hello", "hello world");
        assert_eq!(metrics.wer, 1.0);
        assert_eq!(metrics.insertions, 1);
        assert_eq!(metrics.deletions, 0);
        assert_eq!(metrics.substitutions, 0);
    }

    #[test]
    fn test_wer_metrics_deletion() {
        let metrics = WerMetrics::new("hello world", "hello");
        // One deletion out of two reference words is a 50% error rate.
        assert_eq!(metrics.wer, 0.5);
        assert_eq!(metrics.deletions, 1);
        assert_eq!(metrics.insertions, 0);
        assert_eq!(metrics.substitutions, 0);
    }

    #[test]
    fn test_wer_metrics_display() {
        let metrics = WerMetrics::new("hello world test", "hello there test");
        let display = format!("{}", metrics);
        assert!(display.contains("WER: 33.3%"));
        assert!(display.contains("S:1"));
        assert!(display.contains("Ref:3"));
    }
}