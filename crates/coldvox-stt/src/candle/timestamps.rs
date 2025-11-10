//! Timestamp generation and processing for Whisper transcriptions
//!
//! This module implements the timestamp heuristics from the Candle Whisper examples
//! to convert token-level timestamps into segment-level start and end times.
//!
//! Attribution: This code is adapted from the Candle project's Whisper examples
//! (https://github.com/huggingface/candle/tree/main/candle-examples/examples/whisper)

use crate::candle::types::Segment;

/// Special token IDs for timestamp tokens
/// Whisper uses tokens >= 50364 for timestamps, where each token represents 0.02 seconds
const TIMESTAMP_BEGIN: u32 = 50364;

/// Convert a token ID to a timestamp in seconds
///
/// Whisper encodes timestamps as token IDs starting from 50364,
/// where each token represents 0.02 seconds.
pub fn token_to_seconds(token: u32) -> Option<f64> {
    if token >= TIMESTAMP_BEGIN {
        Some((token - TIMESTAMP_BEGIN) as f64 * 0.02)
    } else {
        None
    }
}

/// Check if a token is a timestamp token
pub fn is_timestamp_token(token: u32) -> bool {
    token >= TIMESTAMP_BEGIN
}

/// Represents a decoded segment with timestamps and text
#[derive(Debug, Clone)]
pub struct DecodedSegment {
    pub tokens: Vec<u32>,
    pub start: Option<f64>,
    pub end: Option<f64>,
}

/// Apply timestamp rules to refine segment boundaries
///
/// This implements heuristics to clean up timestamp predictions:
/// - Ensures segments don't overlap
/// - Fills in missing timestamps
/// - Handles edge cases
pub fn apply_timestamp_rules(segments: &mut [DecodedSegment]) {
    if segments.is_empty() {
        return;
    }

    // Fill in missing start timestamps
    let mut last_end = 0.0;
    for segment in segments.iter_mut() {
        if segment.start.is_none() {
            segment.start = Some(last_end);
        }

        if let Some(end) = segment.end {
            last_end = end;
        } else if let Some(start) = segment.start {
            // If no end timestamp, estimate based on segment length
            // Default to 1 second segments if we have no better estimate
            segment.end = Some(start + 1.0);
            last_end = start + 1.0;
        }
    }

    // Ensure no overlaps between segments
    for i in 1..segments.len() {
        if let (Some(prev_end), Some(curr_start)) = (segments[i - 1].end, segments[i].start) {
            if curr_start < prev_end {
                // Adjust current segment start to match previous end
                segments[i].start = Some(prev_end);
            }
        }
    }

    // Ensure each segment has valid start < end
    for segment in segments.iter_mut() {
        if let (Some(start), Some(end)) = (segment.start, segment.end) {
            if end <= start {
                // If end is before or equal to start, add a small duration
                segment.end = Some(start + 0.1);
            }
        }
    }
}

/// Extract segments from a sequence of tokens and their metadata
///
/// This processes the decoder output to create segments with timestamps.
/// It looks for timestamp tokens in the output and uses them to define segment boundaries.
pub fn extract_segments(
    tokens: &[u32],
    tokenizer: &tokenizers::Tokenizer,
    logprobs: &[f64],
    no_speech_probs: &[f64],
) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_segment = DecodedSegment {
        tokens: Vec::new(),
        start: None,
        end: None,
    };

    let mut current_start: Option<f64> = None;
    let mut text_tokens = Vec::new();

    for (i, &token) in tokens.iter().enumerate() {
        if is_timestamp_token(token) {
            let timestamp = token_to_seconds(token);

            if current_start.is_none() {
                // This is a start timestamp
                current_start = timestamp;
            } else {
                // This is an end timestamp - finalize the segment
                if !text_tokens.is_empty() {
                    let text = tokenizer
                        .decode(&text_tokens, true)
                        .unwrap_or_default()
                        .trim()
                        .to_string();

                    if !text.is_empty() {
                        // Calculate average log probability for this segment
                        let avg_logprob = if logprobs.is_empty() {
                            0.0
                        } else {
                            let start_idx = i.saturating_sub(text_tokens.len());
                            let end_idx = i.min(logprobs.len());
                            let segment_logprobs = &logprobs[start_idx..end_idx];
                            if segment_logprobs.is_empty() {
                                0.0
                            } else {
                                segment_logprobs.iter().sum::<f64>() / segment_logprobs.len() as f64
                            }
                        };

                        // Get no-speech probability for this segment
                        let no_speech_prob = no_speech_probs.get(segments.len())
                            .copied()
                            .unwrap_or(0.0);

                        segments.push(Segment {
                            start_seconds: current_start.unwrap_or(0.0),
                            end_seconds: timestamp.unwrap_or(current_start.unwrap_or(0.0) + 1.0),
                            text,
                            avg_logprob,
                            no_speech_prob,
                        });
                    }

                    text_tokens.clear();
                }

                current_start = timestamp; // This end becomes the next start
            }
        } else {
            // Regular text token
            text_tokens.push(token);
        }
    }

    // Handle any remaining tokens
    if !text_tokens.is_empty() {
        let text = tokenizer
            .decode(&text_tokens, true)
            .unwrap_or_default()
            .trim()
            .to_string();

        if !text.is_empty() {
            let avg_logprob = if logprobs.is_empty() {
                0.0
            } else {
                let start_idx = tokens.len().saturating_sub(text_tokens.len());
                let segment_logprobs = &logprobs[start_idx..];
                if segment_logprobs.is_empty() {
                    0.0
                } else {
                    segment_logprobs.iter().sum::<f64>() / segment_logprobs.len() as f64
                }
            };

            let no_speech_prob = no_speech_probs.get(segments.len())
                .copied()
                .unwrap_or(0.0);

            segments.push(Segment {
                start_seconds: current_start.unwrap_or(0.0),
                end_seconds: current_start.unwrap_or(0.0) + 1.0,
                text,
                avg_logprob,
                no_speech_prob,
            });
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_to_seconds() {
        assert_eq!(token_to_seconds(50364), Some(0.0));
        assert_eq!(token_to_seconds(50365), Some(0.02));
        assert_eq!(token_to_seconds(50464), Some(2.0));
        assert_eq!(token_to_seconds(100), None);
    }

    #[test]
    fn test_is_timestamp_token() {
        assert!(is_timestamp_token(50364));
        assert!(is_timestamp_token(50365));
        assert!(is_timestamp_token(60000));
        assert!(!is_timestamp_token(50363));
        assert!(!is_timestamp_token(100));
    }

    #[test]
    fn test_apply_timestamp_rules() {
        let mut segments = vec![
            DecodedSegment {
                tokens: vec![],
                start: Some(0.0),
                end: Some(2.0),
            },
            DecodedSegment {
                tokens: vec![],
                start: Some(1.5), // Overlaps with previous
                end: Some(3.0),
            },
            DecodedSegment {
                tokens: vec![],
                start: None, // Missing start
                end: Some(4.0),
            },
        ];

        apply_timestamp_rules(&mut segments);

        assert_eq!(segments[0].start, Some(0.0));
        assert_eq!(segments[0].end, Some(2.0));
        assert_eq!(segments[1].start, Some(2.0)); // Fixed overlap
        assert_eq!(segments[1].end, Some(3.0));
        assert_eq!(segments[2].start, Some(3.0)); // Filled in missing start
        assert_eq!(segments[2].end, Some(4.0));
    }
}
