//! Timestamp extraction and processing for Whisper transcriptions.
//!
//! This module implements the logic for converting timestamp tokens into
//! actual time values (in seconds) for transcription segments.
//!
//! Based on the Candle Whisper example implementation.

/// Timestamp token constants
///
/// # Token ID Source
///
/// TIMESTAMP_BEGIN = 50364 comes from the Whisper tokenizer vocabulary:
/// - Tokens 0-50256: Standard GPT-2 BPE vocabulary (text tokens)
/// - Token 50257: End of transcript <|endoftext|>
/// - Token 50258: Start of transcript <|startoftranscript|>
/// - Tokens 50259-50362: Language tokens (<|en|>, <|es|>, etc.)
/// - Token 50363: <|notimestamps|>
/// - **Tokens 50364-51864**: Timestamp tokens (0.00s to 30.00s in 0.02s increments)
///
/// This is defined in the Whisper model architecture and tokenizer config.
/// Reference: openai/whisper repository, tokenizer.json
pub const TIMESTAMP_BEGIN: u32 = 50364;

/// Time precision for timestamp tokens
///
/// # Why 0.02 seconds (20ms)?
///
/// This value is hardcoded in the Whisper model architecture:
/// 1. Whisper's encoder produces features at 50 Hz (one feature per 20ms)
/// 2. Each timestamp token represents one encoder time step
/// 3. 30 seconds of audio = 1500 time steps = 1500 timestamp tokens
/// 4. Token range: 50364 (0.00s) to 51864 (30.00s)
///
/// The model was trained with this granularity, so changing it would require
/// retraining the entire model.
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
            //
            // WHY +1 second?
            // - When the model doesn't generate a final timestamp, we need to estimate duration
            // - 1 second is a reasonable default for typical speech cadence
            // - It's long enough to cover most short phrases but not so long as to be misleading
            // - This matches the behavior in the original Whisper implementation
            // - Alternative: Could use average segment duration or text length heuristic,
            //   but simple heuristics are more robust
            let end_time = start + 1.0;
            segments.push((start, end_time, current_text.trim().to_string()));
        }
    }

    segments
}

/// Segment builder for incremental construction
///
/// # Visibility Notes
///
/// The fields `current_start` and `current_end` are marked `pub(crate)` rather than private
/// because they are accessed by `decode.rs` during token-by-token decoding to determine
/// when to finalize segments.
///
/// This is a deliberate tight coupling between the timestamp and decode modules:
/// - The decoder needs to peek at segment state to decide when to finalize
/// - Making these fields public would expose internal state to external consumers
/// - Using `pub(crate)` limits access to within the crate while allowing the decoder to inspect state
///
/// Alternative designs considered:
/// 1. Add a `has_complete_segment() -> bool` method - doesn't provide enough granularity
/// 2. Make SegmentBuilder fully opaque - would require duplicating state tracking in decoder
/// 3. Merge timestamp and decode modules - creates too much coupling
///
/// The current design balances encapsulation with practical needs.
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
    fn test_token_to_seconds_boundary_values() {
        // First timestamp token = 0.00s
        assert_eq!(token_to_seconds(TIMESTAMP_BEGIN), Some(0.0));

        // One step forward = 0.02s
        assert_eq!(token_to_seconds(TIMESTAMP_BEGIN + 1), Some(0.02));

        // 50 steps = 1.00s
        assert_eq!(token_to_seconds(TIMESTAMP_BEGIN + 50), Some(1.0));

        // 1500 steps = 30.00s (maximum)
        assert_eq!(token_to_seconds(TIMESTAMP_BEGIN + 1500), Some(30.0));
    }

    #[test]
    fn test_token_to_seconds_non_timestamp() {
        // Text tokens should return None
        assert_eq!(token_to_seconds(0), None);
        assert_eq!(token_to_seconds(100), None);
        assert_eq!(token_to_seconds(50363), None); // <|notimestamps|>
        assert_eq!(token_to_seconds(50257), None); // EOT
        assert_eq!(token_to_seconds(50258), None); // SOT
    }

    #[test]
    fn test_token_to_seconds_precision() {
        // Verify 20ms precision
        let t1 = token_to_seconds(TIMESTAMP_BEGIN + 1).unwrap();
        let t0 = token_to_seconds(TIMESTAMP_BEGIN).unwrap();
        assert!((t1 - t0 - 0.02).abs() < 1e-10);
    }

    #[test]
    fn test_extract_segments_basic() {
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
        assert_eq!(segments[0].0, 0.0);
        assert_eq!(segments[0].1, 1.0);
        assert_eq!(segments[0].2, "Hello world");

        assert_eq!(segments[1].0, 1.0);
        assert_eq!(segments[1].1, 2.0);
        assert_eq!(segments[1].2, "How are you");
    }

    #[test]
    fn test_extract_segments_empty() {
        // Empty input
        let segments = extract_segments(&[], &[]);
        assert_eq!(segments.len(), 0);
    }

    #[test]
    fn test_extract_segments_no_timestamps() {
        // Only text tokens, no timestamps
        let tokens = vec![100, 101, 102];
        let texts = vec!["Hello".to_string(), " ".to_string(), "world".to_string()];

        let segments = extract_segments(&tokens, &texts);
        assert_eq!(segments.len(), 0, "Should produce no segments without timestamps");
    }

    #[test]
    fn test_extract_segments_trailing_text() {
        // Text after last timestamp gets +1 second fallback
        let tokens = vec![
            50364, // 0.0s
            100, 101, // text
        ];

        let texts = vec![
            "".to_string(),
            "Trailing".to_string(),
            " text".to_string(),
        ];

        let segments = extract_segments(&tokens, &texts);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, 0.0);
        assert_eq!(segments[0].1, 1.0, "Should use +1 second fallback");
        assert_eq!(segments[0].2, "Trailing text");
    }

    #[test]
    fn test_extract_segments_whitespace_only() {
        // Segments with only whitespace should be filtered out
        let tokens = vec![
            50364, // 0.0s
            100,   // whitespace
            50414, // 1.0s
        ];

        let texts = vec![
            "".to_string(),
            "   ".to_string(),
            "".to_string(),
        ];

        let segments = extract_segments(&tokens, &texts);
        assert_eq!(segments.len(), 0, "Whitespace-only segments should be filtered");
    }

    #[test]
    fn test_segment_builder_basic() {
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

    #[test]
    fn test_segment_builder_no_end_timestamp() {
        // Segment without end timestamp uses +1 second default
        let mut builder = SegmentBuilder::new();

        builder.add_timestamp(5.0);
        builder.add_text("Incomplete segment");

        let segments = builder.segments();

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, 5.0);
        assert_eq!(segments[0].1, 6.0, "Should use +1 second default");
        assert_eq!(segments[0].2, "Incomplete segment");
    }

    #[test]
    fn test_segment_builder_empty_text() {
        // Empty/whitespace text should be filtered
        let mut builder = SegmentBuilder::new();

        builder.add_timestamp(0.0);
        builder.add_text("   ");
        builder.add_timestamp(1.0);

        let segments = builder.segments();

        assert_eq!(segments.len(), 0, "Empty segments should be filtered");
    }

    #[test]
    fn test_segment_builder_multiple_finalizations() {
        let mut builder = SegmentBuilder::new();

        // First segment
        builder.add_timestamp(0.0);
        builder.add_text("First");
        builder.add_timestamp(1.0);
        builder.finalize_segment();

        // Second segment
        builder.add_text("Second");
        builder.add_timestamp(2.0);
        builder.finalize_segment();

        // Third segment
        builder.add_text("Third");
        builder.add_timestamp(3.0);

        let segments = builder.segments();

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].2, "First");
        assert_eq!(segments[1].2, "Second");
        assert_eq!(segments[2].2, "Third");
    }

    #[test]
    fn test_segment_builder_no_timestamps() {
        // Text without any timestamps
        let mut builder = SegmentBuilder::new();

        builder.add_text("Text without timestamps");

        let segments = builder.segments();

        assert_eq!(segments.len(), 0);
    }

    #[test]
    fn test_segment_builder_timestamps_only() {
        // Timestamps without text
        let mut builder = SegmentBuilder::new();

        builder.add_timestamp(0.0);
        builder.add_timestamp(1.0);
        builder.finalize_segment();

        let segments = builder.segments();

        assert_eq!(segments.len(), 0, "Segments without text should be filtered");
    }

    #[test]
    fn test_segment_builder_state_fields() {
        // Test that pub(crate) fields are accessible within the crate
        let mut builder = SegmentBuilder::new();

        assert_eq!(builder.current_start, None);
        assert_eq!(builder.current_end, None);

        builder.add_timestamp(1.5);
        assert_eq!(builder.current_start, Some(1.5));
        assert_eq!(builder.current_end, None);

        builder.add_timestamp(3.7);
        assert_eq!(builder.current_start, Some(1.5));
        assert_eq!(builder.current_end, Some(3.7));
    }

    #[test]
    fn test_constants() {
        // Verify hardcoded constants match Whisper spec
        assert_eq!(TIMESTAMP_BEGIN, 50364);
        assert_eq!(TIME_PRECISION, 0.02);
    }
}
