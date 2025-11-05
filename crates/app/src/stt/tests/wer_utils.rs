//! Word Error Rate (WER) calculation utilities for STT testing.
//!
//! The implementation has been moved to `crate::test_utils::wer` to allow
//! sharing with integration tests.

#[cfg(test)]
mod tests {
    use crate::test_utils::wer::*;

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
