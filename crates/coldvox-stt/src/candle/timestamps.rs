//! Timestamp extraction and processing for Whisper transcriptions.
//!
//! This module implements the logic for converting timestamp tokens into
//! actual time values (in seconds) for transcription segments.
//!
//! Based on the Candle Whisper example implementation.

/// Timestamp token constants
pub const TIMESTAMP_BEGIN: u32 = 50364;
const TIME_PRECISION: f64 = 0.02; // 20ms per timestamp token

/// Extract timestamp from a token ID
///
/// Whisper uses special tokens (>= 50364) to represent timestamps.
/// Each token represents a 20ms increment.
///
/// # Arguments
/// * `token` - Token ID to convert
///
/// # Returns
/// Time in seconds, or None if the token is not a timestamp token
pub fn token_to_seconds(token: u32) -> Option<f64> {
    if token >= TIMESTAMP_BEGIN {
        let offset = (token - TIMESTAMP_BEGIN) as f64;
        Some(offset * TIME_PRECISION)
    } else {
        None
    }
}

/// Apply timestamp rules to a sequence of tokens
///
/// This implements the heuristics from the Candle Whisper example:
/// 1. Timestamps should be monotonically increasing
/// 2. Consecutive timestamps define segment boundaries
/// 3. Text between timestamps forms a segment
///
/// # Arguments
/// * `tokens` - Sequence of token IDs
/// * `token_texts` - Corresponding text for each token
///
/// # Returns
/// Vector of (start_time, end_time, text) tuples
pub fn extract_segments(
    tokens: &[u32],
    token_texts: &[String],
) -> Vec<(f64, f64, String)> {
    let mut segments = Vec::new();
    let mut current_start: Option<f64> = None;
    let mut current_text = String::new();

    for (idx, &token) in tokens.iter().enumerate() {
        if let Some(timestamp) = token_to_seconds(token) {
            // This is a timestamp token
            if let Some(start) = current_start {
                // We have a segment: from current_start to this timestamp
                if !current_text.trim().is_empty() {
                    segments.push((start, timestamp, current_text.trim().to_string()));
                }
                current_text.clear();
            }
            current_start = Some(timestamp);
        } else if idx < token_texts.len() {
            // This is a text token
            current_text.push_str(&token_texts[idx]);
        }
    }

    // Handle any remaining text
    if let Some(start) = current_start {
        if !current_text.trim().is_empty() {
            // Use the last timestamp + 1 second as a fallback end time
            let end_time = start + 1.0;
            segments.push((start, end_time, current_text.trim().to_string()));
        }
    }

    segments
}

/// Segment builder for incremental construction
pub struct SegmentBuilder {
    segments: Vec<(f64, f64, String)>,
    pub(crate) current_start: Option<f64>,
    pub(crate) current_end: Option<f64>,
    current_text: String,
}

impl SegmentBuilder {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            current_start: None,
            current_end: None,
            current_text: String::new(),
        }
    }

    /// Add a timestamp token
    pub fn add_timestamp(&mut self, seconds: f64) {
        if self.current_start.is_none() {
            self.current_start = Some(seconds);
        } else {
            self.current_end = Some(seconds);
        }
    }

    /// Add text to the current segment
    pub fn add_text(&mut self, text: &str) {
        self.current_text.push_str(text);
    }

    /// Finalize the current segment and start a new one
    pub fn finalize_segment(&mut self) {
        if let (Some(start), Some(end)) = (self.current_start, self.current_end) {
            if !self.current_text.trim().is_empty() {
                self.segments.push((
                    start,
                    end,
                    self.current_text.trim().to_string(),
                ));
            }
        }

        // Reset for next segment
        self.current_start = self.current_end;
        self.current_end = None;
        self.current_text.clear();
    }

    /// Get all finalized segments
    pub fn segments(mut self) -> Vec<(f64, f64, String)> {
        // Finalize any remaining segment
        if self.current_start.is_some() && !self.current_text.trim().is_empty() {
            if self.current_end.is_none() {
                // Use a default duration if we don't have an end timestamp
                self.current_end = Some(self.current_start.unwrap() + 1.0);
            }
            self.finalize_segment();
        }
        self.segments
    }
}

impl Default for SegmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_to_seconds() {
        // Token 50364 = 0.00s (start)
        assert_eq!(token_to_seconds(50364), Some(0.0));

        // Token 50365 = 0.02s
        assert_eq!(token_to_seconds(50365), Some(0.02));

        // Token 50414 = 1.00s (50 * 0.02)
        assert_eq!(token_to_seconds(50414), Some(1.0));

        // Non-timestamp token
        assert_eq!(token_to_seconds(100), None);
    }

    #[test]
    fn test_extract_segments() {
        let tokens = vec![
            50364, // 0.0s
            100, 101, 102, // text tokens
            50414, // 1.0s
            200, 201, // text tokens
            50464, // 2.0s
        ];

        let texts = vec![
            "".to_string(), // timestamp
            "Hello".to_string(),
            " ".to_string(),
            "world".to_string(),
            "".to_string(), // timestamp
            "How".to_string(),
            " are you".to_string(),
            "".to_string(), // timestamp
        ];

        let segments = extract_segments(&tokens, &texts);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].0, 0.0); // start
        assert_eq!(segments[0].1, 1.0); // end
        assert_eq!(segments[0].2, "Hello world"); // text

        assert_eq!(segments[1].0, 1.0);
        assert_eq!(segments[1].1, 2.0);
        assert_eq!(segments[1].2, "How are you");
    }

    #[test]
    fn test_segment_builder() {
        let mut builder = SegmentBuilder::new();

        builder.add_timestamp(0.0);
        builder.add_text("Hello");
        builder.add_text(" world");
        builder.add_timestamp(1.0);
        builder.finalize_segment();

        builder.add_text("Second segment");
        builder.add_timestamp(2.0);

        let segments = builder.segments();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], (0.0, 1.0, "Hello world".to_string()));
        assert_eq!(segments[1], (1.0, 2.0, "Second segment".to_string()));
    }
}
