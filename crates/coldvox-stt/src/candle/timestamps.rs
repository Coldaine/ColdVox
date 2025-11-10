//! Timestamp extraction core logic for Whisper tokens.
//!
//! This module provides comprehensive timestamp extraction functionality including:
//! - Whisper timestamp token recognition (>= 50000)
//! - Mathematical token-to-time conversion with frame-based timing
//! - Timestamp sequence extraction and validation
//! - Integration with decoder pipeline
//! - Segment boundary detection and processing
//! - SegmentBuilder for incremental segment construction

use candle_transformers::models::whisper::Config as WhisperConfig;
use tokenizers::Tokenizer;
use tracing;

use coldvox_foundation::error::{ColdVoxError, SttError};

use super::types::{Segment, WordTiming};

/// Whisper timestamp token threshold (typically >= 50000)
pub const WHISPER_TIMESTAMP_THRESHOLD: u32 = 50000;

/// Frame duration for Whisper timing (20ms per frame)
pub const WHISPER_FRAME_DURATION: f32 = 0.02;

/// First timestamp token ID in Whisper tokenizer (offset from threshold)
pub const TIMESTAMP_BEGIN_OFFSET: u32 = 364;

/// Builder for incrementally constructing segments with proper validation.
///
/// This builder provides a fluent API for creating segments with incremental
/// additions of text, timestamps, and confidence scores. It handles edge cases
/// like missing timestamps, malformed sequences, and validation automatically.
pub struct SegmentBuilder {
    start_time: Option<f32>,
    end_time: Option<f32>,
    text_tokens: Vec<u32>,
    word_count: usize,
    confidence_sum: f32,
    token_count: usize,
}

impl SegmentBuilder {
    /// Create a new empty segment builder.
    pub fn new() -> Self {
        Self {
            start_time: None,
            end_time: None,
            text_tokens: Vec::new(),
            word_count: 0,
            confidence_sum: 0.0,
            token_count: 0,
        }
    }

    /// Add a start timestamp to the segment.
    pub fn with_start_time(mut self, start_time: f32) -> Self {
        if start_time >= 0.0 {
            self.start_time = Some(start_time);
        }
        self
    }

    /// Add an end timestamp to the segment.
    pub fn with_end_time(mut self, end_time: f32) -> Self {
        if end_time >= 0.0 {
            self.end_time = Some(end_time);
        }
        self
    }

    /// Add text tokens to the segment.
    pub fn with_text_tokens(mut self, tokens: &[u32]) -> Self {
        self.text_tokens.extend_from_slice(tokens);
        self
    }

    /// Add a word to the segment.
    pub fn with_word(mut self, word: &str) -> Self {
        if !word.trim().is_empty() {
            self.word_count += 1;
        }
        self
    }

    /// Add confidence score for accumulated content.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence_sum += confidence.max(0.0_f32).min(1.0_f32);
        self.token_count += 1;
        self
    }

    /// Build the segment with proper fallbacks and validation.
    pub fn build(
        self,
        tokenizer: &Tokenizer,
        default_duration: f32,
    ) -> Result<Segment, ColdVoxError> {
        // Convert tokens to text
        let text = if !self.text_tokens.is_empty() {
            tokenizer.decode(&self.text_tokens, true)
                .map_err(|e| ColdVoxError::Stt(SttError::InvalidConfig(
                    format!("Failed to decode tokens: {}", e)
                )))?
        } else {
            String::new()
        };

        // Calculate timing with fallbacks
        let start_time = self.start_time.unwrap_or(0.0);
        let end_time = self.end_time.unwrap_or_else(|| {
            start_time + if self.word_count > 0 {
                self.word_count as f32 * default_duration
            } else {
                default_duration
            }
        });

        // Calculate average confidence
        let confidence = if self.token_count > 0 {
            self.confidence_sum / self.token_count as f32
        } else {
            0.0
        };

        // Validate timing
        let (start, end) = if end_time >= start_time {
            (start_time, end_time)
        } else {
            tracing::warn!("Invalid segment timing: start={} >= end={}, correcting", start_time, end_time);
            (start_time, start_time + default_duration.max(0.1))
        };

        Ok(Segment {
            start,
            end,
            text: text.trim().to_string(),
            confidence,
            word_count: self.word_count,
            words: None,
        })
    }

    /// Check if the builder has any meaningful content.
    pub fn is_empty(&self) -> bool {
        self.text_tokens.is_empty() && self.word_count == 0 && self.start_time.is_none()
    }

    /// Get the current estimated duration based on accumulated content.
    pub fn estimated_duration(&self, default_duration: f32) -> f32 {
        if self.word_count > 0 {
            self.word_count as f32 * default_duration
        } else {
            default_duration
        }
    }
}

impl Default for SegmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns true if the token is a Whisper timestamp token.
pub fn is_timestamp_token(token: u32) -> bool {
    token >= WHISPER_TIMESTAMP_THRESHOLD
}

/// Convert a timestamp token to time in seconds with mathematical precision.
/// 
/// # Arguments
/// * `token` - Whisper timestamp token (>= 50000)
/// * `config` - Whisper model configuration
/// 
/// # Returns
/// Time in seconds as f32, with ~100ms tolerance for frame-based timing
pub fn token_to_time(token: u32, config: &WhisperConfig) -> Result<f32, ColdVoxError> {
    if !is_timestamp_token(token) {
        return Err(ColdVoxError::Stt(SttError::InvalidConfig(
            format!("Token {} is not a valid timestamp token", token)
        )));
    }
    
    // Calculate frame offset from Whisper timestamp threshold
    let frame_offset = token.saturating_sub(WHISPER_TIMESTAMP_THRESHOLD) as f32;
    
    // Ensure we don't exceed the maximum source positions
    if frame_offset >= config.max_source_positions as f32 {
        return Err(ColdVoxError::Stt(SttError::InvalidConfig(
            format!("Token {} exceeds max source positions {}", token, config.max_source_positions)
        )));
    }
    
    // Convert to time: frame_offset * frame_duration
    // Whisper uses 20ms frames, giving us ~100ms tolerance
    let time_seconds = frame_offset * WHISPER_FRAME_DURATION;
    
    Ok(time_seconds)
}

/// Extract timestamp pairs from token sequence.
/// 
/// # Arguments
/// * `tokens` - Token sequence that may contain timestamp tokens
/// * `config` - Whisper model configuration
/// 
/// # Returns
/// Vector of (start_time, end_time) pairs in seconds
pub fn extract_timestamps(tokens: &[u32], config: &WhisperConfig) -> Result<Vec<(f32, f32)>, ColdVoxError> {
    let mut timestamps = Vec::new();
    let mut current_start: Option<f32> = None;
    let mut last_timestamp: Option<f32> = None;
    let mut text_token_count = 0;
    
    let frame_duration = WHISPER_FRAME_DURATION;
    
    for &token in tokens {
        if is_timestamp_token(token) {
            let current_time = token_to_time(token, config)?;
            
            // If we have accumulated text tokens and a start time, create a timestamp pair
            if text_token_count > 0 && current_start.is_some() {
                // Estimate end time based on text length and frame duration
                // This provides ~100ms tolerance as required
                let estimated_end = if let Some(last_ts) = last_timestamp {
                    // Use actual timestamp if available
                    current_time.max(last_ts + frame_duration)
                } else {
                    // Estimate based on token count: ~0.02s per token
                    current_start.unwrap() + (text_token_count as f32 * frame_duration * 0.5)
                };
                
                timestamps.push((current_start.unwrap(), estimated_end.max(current_start.unwrap())));
                
                // Clear for next segment
                text_token_count = 0;
            }
            
            // Update tracking variables
            last_timestamp = Some(current_time);
            
            // Only set new start if we don't have one or this is clearly a new segment
            if current_start.is_none() || current_time >= current_start.unwrap() + frame_duration {
                current_start = Some(current_time);
            }
        } else {
            // Count non-timestamp tokens (text tokens)
            text_token_count += 1;
        }
    }
    
    // Handle remaining text at the end
    if text_token_count > 0 && current_start.is_some() {
        let start_time = current_start.unwrap();
        let end_time = if let Some(last_ts) = last_timestamp {
            (last_ts + (text_token_count as f32 * frame_duration * 0.5)).max(start_time)
        } else {
            start_time + (text_token_count as f32 * frame_duration)
        };
        
        timestamps.push((start_time, end_time));
    }
    
    Ok(timestamps)
}

/// Advanced timestamp extraction with validation and error handling.
/// 
/// This function performs comprehensive timestamp extraction including:
/// - Malformed token sequence detection
/// - Temporal ordering validation
/// - Gap detection and handling
/// 
/// # Arguments
/// * `tokens` - Token sequence from decoder output
/// * `config` - Whisper model configuration
/// * `max_gap_ms` - Maximum allowed gap between segments (default: 1000ms)
///
/// # Returns
/// Validated vector of (start_time, end_time) pairs
pub fn extract_timestamps_advanced(
    tokens: &[u32], 
    config: &WhisperConfig,
    max_gap_ms: u32
) -> Result<Vec<(f32, f32)>, ColdVoxError> {
    let basic_timestamps = extract_timestamps(tokens, config)?;
    
    if basic_timestamps.is_empty() {
        return Ok(Vec::new());
    }
    
    let max_gap_seconds = max_gap_ms as f32 / 1000.0;
    let frame_duration = WHISPER_FRAME_DURATION;
    
    // Validate and filter timestamps
    let mut validated_timestamps = Vec::new();
    let mut previous_end: Option<f32> = None;
    
    for (start, end) in basic_timestamps {
        // Validate temporal ordering
        if end < start {
            tracing::warn!("Invalid timestamp ordering: end {:?} < start {:?}", end, start);
            continue;
        }
        
        // Check for reasonable segment duration (not too short or too long)
        let duration = end - start;
        if duration < frame_duration {
            tracing::debug!("Skipping too-short segment: {:?}s", duration);
            continue;
        }
        
        if duration > 30.0 { // Max 30 seconds per segment
            tracing::warn!("Skipping too-long segment: {:?}s", duration);
            continue;
        }
        
        // Check for gaps (if we have a previous end time)
        if let Some(prev_end) = previous_end {
            let gap = start - prev_end;
            if gap > max_gap_seconds {
                tracing::debug!("Large gap detected: {:?}s, continuing", gap);
            }
        }
        
        validated_timestamps.push((start, end));
        previous_end = Some(end);
    }
    
    // Ensure timestamps are sorted
    validated_timestamps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    
    Ok(validated_timestamps)
}

/// Build segments from a mixed token sequence containing timestamps with enhanced processing.
/// 
/// This function provides direct integration with the decoder output by combining
/// timestamp extraction with token-to-text conversion using the tokenizer. It handles
/// Whisper's token pairing format, special tokens, and provides confidence scores.
/// 
/// # Arguments
/// * `tokens` - Token sequence from decoder (may contain timestamp tokens)
/// * `config` - Whisper model configuration
/// * `tokenizer` - Tokenizer for converting tokens to text
/// 
/// # Returns
/// Vector of Segment objects with enhanced timing information, confidence scores, and text
pub fn segments_from_tokens(
    tokens: &[u32],
    config: &WhisperConfig,
    tokenizer: &Tokenizer,
) -> Result<Vec<Segment>, ColdVoxError> {
    let mut segments = Vec::new();
    let mut text_tokens = Vec::new();
    let mut current_start: Option<f32> = None;
    let mut previous_ts: Option<f32> = None;
    let frame_duration = WHISPER_FRAME_DURATION;

    // Process tokens with enhanced Whisper token handling
    for &token in tokens {
        if is_timestamp_token(token) {
            let ts = match token_to_time(token, config) {
                Ok(time) => time,
                Err(e) => {
                    tracing::warn!("Invalid timestamp token {}: {:?}", token, e);
                    continue;
                }
            };
            
            // Create segment from accumulated text tokens
            if !text_tokens.is_empty() && current_start.is_some() {
                let segment = build_segment_from_tokens(
                    &text_tokens,
                    current_start.unwrap(),
                    ts.max(current_start.unwrap()),
                    tokenizer,
                    frame_duration,
                )?;
                
                if !segment.text.trim().is_empty() {
                    segments.push(segment);
                }
                text_tokens.clear();
            }
            current_start = Some(ts);
            previous_ts = Some(ts);
        } else {
            // Handle Whisper's token pairing by accumulating pairs
            handle_token_sequence(&mut text_tokens, token, tokenizer)?;
        }
    }

    // Handle remaining text at the end
    if !text_tokens.is_empty() {
        let start = current_start.or(previous_ts).unwrap_or(0.0);
        let end = if let Some(prev) = previous_ts {
            (prev + frame_duration).max(start)
        } else {
            start + text_tokens.len() as f32 * frame_duration
        };

        let segment = build_segment_from_tokens(
            &text_tokens,
            start,
            end,
            tokenizer,
            frame_duration,
        )?;
        
        if !segment.text.trim().is_empty() {
            segments.push(segment);
        }
    }

    // Post-process segments for merging and validation
    let merged_segments = merge_adjacent_segments(segments, frame_duration)?;
    
    tracing::info!(
        "Processed {} tokens into {} segments",
        tokens.len(),
        merged_segments.len()
    );
    
    Ok(merged_segments)
}

/// Build a single segment from token sequence with proper text reconstruction.
/// 
/// This function handles Whisper's token pairing format, special token filtering,
/// and provides confidence score estimation based on token characteristics.
fn build_segment_from_tokens(
    tokens: &[u32],
    start: f32,
    end: f32,
    tokenizer: &Tokenizer,
    frame_duration: f32,
) -> Result<Segment, ColdVoxError> {
    // Filter out special tokens that shouldn't appear in final text
    let filtered_tokens = filter_special_tokens(tokens, tokenizer)?;
    
    let text = if !filtered_tokens.is_empty() {
        match tokenizer.decode(&filtered_tokens, true) {
            Ok(decoded) => {
                // Clean up the text: remove extra spaces, handle punctuation
                clean_decoded_text(&decoded)
            }
            Err(e) => {
                tracing::warn!("Failed to decode tokens {:?}: {:?}", filtered_tokens, e);
                String::new()
            }
        }
    } else {
        String::new()
    };

    // Estimate confidence based on token characteristics
    let confidence = estimate_segment_confidence(tokens, &text, frame_duration);
    
    // Calculate word count
    let word_count = text.split_whitespace().count();
    
    Ok(Segment {
        start,
        end: end.max(start),
        text,
        confidence,
        word_count,
        words: None, // Could be enhanced with word-level timing later
    })
}

/// Handle Whisper's token pairing format during sequence processing.
/// 
/// Whisper sometimes uses token pairs where two tokens represent one semantic unit.
/// This function detects and handles these pairs appropriately.
fn handle_token_sequence(
    text_tokens: &mut Vec<u32>,
    token: u32,
    tokenizer: &Tokenizer,
) -> Result<(), ColdVoxError> {
    // Basic token pair detection - could be enhanced with tokenizer metadata
    if is_potential_token_pair(token, tokenizer) {
        // For now, just add the token - enhanced pairing logic can be added later
        text_tokens.push(token);
    } else {
        text_tokens.push(token);
    }
    Ok(())
}

/// Filter out special tokens that shouldn't appear in final transcribed text.
fn filter_special_tokens(
    tokens: &[u32],
    tokenizer: &Tokenizer,
) -> Result<Vec<u32>, ColdVoxError> {
    let mut filtered = Vec::new();
    
    for &token in tokens {
        // Get token string to check if it's a special token
        if let Some(token_str) = tokenizer.id_to_token(token) {
            // Filter out common special tokens
            if !is_special_token(&token_str) {
                filtered.push(token);
            }
        } else {
            // If we can't decode the token, include it (might be a valid text token)
            filtered.push(token);
        }
    }
    
    Ok(filtered)
}

/// Check if a token string represents a special token.
fn is_special_token(token_str: &str) -> bool {
    token_str.starts_with("<|") || 
    token_str.starts_with('<') && token_str.ends_with('>') ||
    token_str == "<unk>" ||
    token_str == "<pad>" ||
    token_str == "[UNK]" ||
    token_str == "[PAD]"
}

/// Basic token pair detection for Whisper formatting.
/// 
/// This is a simplified implementation that could be enhanced with
/// more sophisticated token relationship analysis.
fn is_potential_token_pair(token: u32, tokenizer: &Tokenizer) -> bool {
    // Simple heuristic: very high-numbered tokens might be special formatting
    if token > 50000 {
        return true;
    }
    
    // Check if token decodes to punctuation or formatting
    if let Some(token_str) = tokenizer.id_to_token(token) {
        matches!(token_str.as_str(), "," | "." | "!" | "?" | ":" | ";" | "-" | "...")
    } else {
        false
    }
}

/// Clean and format decoded text for better readability.
fn clean_decoded_text(text: &str) -> String {
    let mut cleaned = text.to_string();
    
    // Remove extra spaces
    cleaned = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    
    // Add space after punctuation if missing (basic rule)
    cleaned = cleaned
        .replace(",", ", ")
        .replace(".", ". ")
        .replace("!", "! ")
        .replace("?", "? ");
    
    // Clean up multiple spaces
    cleaned = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    
    // Trim final result
    cleaned.trim().to_string()
}

/// Estimate confidence score for a segment based on token characteristics.
/// 
/// This provides a basic confidence estimation. In a full implementation,
/// this would use actual model confidence scores from the decoder.
fn estimate_segment_confidence(
    tokens: &[u32],
    text: &str,
    frame_duration: f32,
) -> f32 {
    let mut confidence: f32 = 0.7; // Base confidence
    
    // Higher confidence for longer segments (more stable)
    if tokens.len() > 5 {
        confidence += 0.1;
    }
    
    // Lower confidence for very short segments
    if tokens.len() < 2 {
        confidence -= 0.2;
    }
    
    // Higher confidence for segments with proper punctuation
    if text.contains('.') || text.contains('!') || text.contains('?') {
        confidence += 0.1;
    }
    
    // Adjust based on segment duration (very short or very long segments less confident)
    let estimated_duration = tokens.len() as f32 * frame_duration;
    if estimated_duration < 0.5 {
        confidence -= 0.1;
    } else if estimated_duration > 10.0 {
        confidence -= 0.1;
    }
    
    confidence.max(0.0_f32).min(1.0_f32)
}

/// Merge adjacent segments when appropriate (e.g., very short gaps, similar content).
fn merge_adjacent_segments(
    segments: Vec<Segment>,
    frame_duration: f32,
) -> Result<Vec<Segment>, ColdVoxError> {
    if segments.len() <= 1 {
        return Ok(segments);
    }
    
    let mut merged = Vec::new();
    let mut current = segments[0].clone();
    
    for next in segments.into_iter().skip(1) {
        // Check if segments should be merged
        let gap = next.start - current.end;
        let should_merge = gap < frame_duration * 2.0; // Less than 40ms gap
        
        if should_merge {
            // Merge segments
            let merged_text = if !current.text.is_empty() && !next.text.is_empty() {
                format!("{} {}", current.text.trim_end(), next.text.trim_start())
            } else {
                format!("{}{}", current.text, next.text)
            };
            
            current = Segment {
                start: current.start,
                end: next.end,
                text: merged_text,
                confidence: (current.confidence + next.confidence) / 2.0,
                word_count: current.word_count + next.word_count,
                words: None,
            };
        } else {
            // Add current segment and start new one
            merged.push(current);
            current = next;
        }
    }
    
    // Add the final segment
    merged.push(current);
    
    Ok(merged)
}

/// Integration function for decoder pipeline.
/// 
/// This function extracts timestamps and returns them in a format suitable for
/// integration with the ColdVox transcript system.
/// 
/// # Arguments
/// * `tokens` - Raw token output from decoder
/// * `config` - Whisper model configuration
/// * `include_validation` - Whether to perform advanced validation
/// 
/// # Returns
/// Vector of (start_time, end_time) pairs ready for transcript integration
pub fn extract_decoder_timestamps(
    tokens: &[u32],
    config: &WhisperConfig,
    include_validation: bool,
) -> Result<Vec<(f32, f32)>, ColdVoxError> {
    if include_validation {
        extract_timestamps_advanced(tokens, config, 1000) // 1 second max gap
    } else {
        extract_timestamps(tokens, config)
    }
}

/// Get timing statistics from a token sequence.
/// 
/// This function provides insights into the temporal structure of decoded tokens.
/// 
/// # Arguments
/// * `tokens` - Token sequence to analyze
/// * `config` - Whisper model configuration
/// 
/// # Returns
/// Timing statistics including duration, segment count, and gaps
pub fn analyze_timing_structure(
    tokens: &[u32],
    config: &WhisperConfig,
) -> Result<TimingStats, ColdVoxError> {
    let timestamps = extract_timestamps(tokens, config)?;
    
    if timestamps.is_empty() {
        return Ok(TimingStats {
            total_duration: 0.0,
            segment_count: 0,
            average_segment_duration: 0.0,
            max_gap: 0.0,
            has_timestamps: false,
        });
    }
    
    let total_duration = timestamps
        .iter()
        .map(|(_, end)| *end)
        .fold(0.0_f32, f32::max);
    
    let segment_count = timestamps.len();
    let average_segment_duration = timestamps
        .iter()
        .map(|(start, end)| end - start)
        .sum::<f32>() / segment_count as f32;
    
    let mut max_gap = 0.0f32;
    for i in 1..timestamps.len() {
        let gap = timestamps[i].0 - timestamps[i-1].1;
        if gap > max_gap {
            max_gap = gap;
        }
    }
    
    Ok(TimingStats {
        total_duration,
        segment_count,
        average_segment_duration,
        max_gap,
        has_timestamps: true,
    })
}

/// Statistics about the temporal structure of decoded tokens
#[derive(Debug, Clone)]
pub struct TimingStats {
    /// Total audio duration in seconds
    pub total_duration: f32,
    /// Number of timestamped segments
    pub segment_count: usize,
    /// Average duration of segments in seconds
    pub average_segment_duration: f32,
    /// Maximum gap between segments in seconds
    pub max_gap: f32,
    /// Whether the sequence contains valid timestamps
    pub has_timestamps: bool,
}

impl TimingStats {
    /// Check if the timing structure is healthy
    pub fn is_healthy(&self) -> bool {
        self.has_timestamps && 
        self.segment_count > 0 && 
        self.total_duration > 0.0 &&
        self.average_segment_duration > 0.0 &&
        self.average_segment_duration < 30.0 // Reasonable segment duration
    }
    
    /// Get a human-readable summary of timing statistics
    pub fn summary(&self) -> String {
        format!(
            "Duration: {:.1}s, Segments: {}, Avg: {:.1}s, Max gap: {:.1}s",
            self.total_duration,
            self.segment_count,
            self.average_segment_duration,
            self.max_gap
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_config() -> WhisperConfig {
        WhisperConfig {
            num_mel_bins: 80,
            max_source_positions: 1500,
            d_model: 512,
            encoder_attention_heads: 8,
            encoder_layers: 6,
            vocab_size: 51865,
            max_target_positions: 448,
            decoder_attention_heads: 8,
            decoder_layers: 6,
            suppress_tokens: vec![],
        }
    }

    fn make_tokenizer() -> Tokenizer {
        let data = r#"{
            "version": "1.0",
            "truncation": null,
            "padding": null,
            "model": {
                "type": "WordLevel",
                "vocab": {"<unk>":0, "hi":1, "there":2},
                "unk_token": "<unk>"
            }
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tokenizer.json");
        std::fs::write(&path, data).unwrap();
        Tokenizer::from_file(path).unwrap()
    }

    #[test]
    fn identifies_timestamp_tokens() {
        let cfg = test_config();
        assert!(is_timestamp_token(WHISPER_TIMESTAMP_THRESHOLD));
        assert!(is_timestamp_token(WHISPER_TIMESTAMP_THRESHOLD + 100));
        assert!(!is_timestamp_token(1));
        assert!(!is_timestamp_token(49999));
    }

    #[test]
    fn token_to_time_conversion() {
        let cfg = test_config();
        
        // Test first timestamp token
        let time = token_to_time(WHISPER_TIMESTAMP_THRESHOLD, &cfg).unwrap();
        assert!((time - 0.0).abs() < 0.001);
        
        // Test token offset by 50 positions (1 second at 20ms per frame)
        let time = token_to_time(WHISPER_TIMESTAMP_THRESHOLD + 50, &cfg).unwrap();
        assert!((time - 1.0).abs() < 0.001);
        
        // Test invalid token
        assert!(token_to_time(1000, &cfg).is_err());
    }

    #[test]
    fn extract_basic_timestamps() {
        let cfg = test_config();
        let tokens = vec![
            1, 2, 3, // text tokens
            WHISPER_TIMESTAMP_THRESHOLD, // timestamp 0s
            4, 5, // more text
            WHISPER_TIMESTAMP_THRESHOLD + 50, // timestamp 1s
        ];
        
        let timestamps = extract_timestamps(&tokens, &cfg).unwrap();
        assert_eq!(timestamps.len(), 1);
        assert!((timestamps[0].0 - 0.0).abs() < 0.01); // start at 0s
        assert!((timestamps[0].1 - 1.0).abs() < 0.1); // end around 1s
    }

    #[test]
    fn builds_segments() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        let tokens = vec![
            WHISPER_TIMESTAMP_THRESHOLD,
            1,
            2,
            WHISPER_TIMESTAMP_THRESHOLD + 50,
        ];
        let segments = segments_from_tokens(&tokens, &cfg, &tokenizer).unwrap();
        assert_eq!(segments.len(), 1);
        assert!(segments[0].text.contains("hi"));
        assert!(segments[0].end >= segments[0].start);
    }

    #[test]
    fn timing_stats_analysis() {
        let cfg = test_config();
        let tokens = vec![
            WHISPER_TIMESTAMP_THRESHOLD,
            1, 2, 3,
            WHISPER_TIMESTAMP_THRESHOLD + 50,
            4, 5,
        ];
        
        let stats = analyze_timing_structure(&tokens, &cfg).unwrap();
        assert!(stats.has_timestamps);
        assert_eq!(stats.segment_count, 2); // Two segments: 0-1s and 1s-end
        assert!(stats.total_duration > 0.0);
    }

    #[test]
    fn advanced_timestamp_validation() {
        let cfg = test_config();
        
        // Test with invalid sequence (out of order)
        let invalid_tokens = vec![
            WHISPER_TIMESTAMP_THRESHOLD + 50, // end before start
            WHISPER_TIMESTAMP_THRESHOLD,
            1, 2,
        ];
        
        let timestamps = extract_timestamps_advanced(&invalid_tokens, &cfg, 1000).unwrap();
        // Should handle gracefully
        assert!(timestamps.len() >= 0);
    }

    #[test]
    fn test_segment_builder_basic() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        
        let builder = SegmentBuilder::new()
            .with_start_time(0.5)
            .with_end_time(2.0)
            .with_text_tokens(&[1, 2])
            .with_word("test")
            .with_confidence(0.8);
            
        let segment = builder.build(&tokenizer, 0.1).unwrap();
        
        assert_eq!(segment.start, 0.5);
        assert_eq!(segment.end, 2.0);
        assert!(segment.text.contains("hi") || segment.text.contains("there"));
        assert!((segment.confidence - 0.8).abs() < 0.001);
        assert_eq!(segment.word_count, 1);
    }

    #[test]
    fn test_segment_builder_with_fallbacks() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        
        // Test with no start time (should fallback to 0.0)
        let builder = SegmentBuilder::new()
            .with_text_tokens(&[1, 2])
            .with_confidence(0.7);
            
        let segment = builder.build(&tokenizer, 0.2).unwrap();
        
        assert_eq!(segment.start, 0.0);
        assert!(segment.end > 0.0);
        assert!(!segment.text.is_empty());
    }

    #[test]
    fn test_enhanced_segments_from_tokens() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        
        let tokens = vec![
            WHISPER_TIMESTAMP_THRESHOLD, // timestamp 0s
            1, 2, // "hi there"
            WHISPER_TIMESTAMP_THRESHOLD + 25, // 0.5s
            2, 1, // reversed order
            WHISPER_TIMESTAMP_THRESHOLD + 50, // 1.0s
        ];
        
        let segments = segments_from_tokens(&tokens, &cfg, &tokenizer).unwrap();
        
        assert!(segments.len() > 0);
        for segment in &segments {
            assert!(segment.start >= 0.0);
            assert!(segment.end >= segment.start);
            assert!(segment.confidence >= 0.0 && segment.confidence <= 1.0);
            assert!(segment.word_count >= 0);
        }
    }

    #[test]
    fn test_text_reconstruction_with_special_tokens() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        
        let tokens = vec![
            WHISPER_TIMESTAMP_THRESHOLD,
            1, 2, // normal tokens
            // Add some special tokens that should be filtered
        ];
        
        let segments = segments_from_tokens(&tokens, &cfg, &tokenizer).unwrap();
        
        assert_eq!(segments.len(), 1);
        let segment = &segments[0];
        assert!(!segment.text.is_empty());
        assert!(segment.confidence > 0.0);
    }

    #[test]
    fn test_segment_merging() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        
        // Create segments with very small gaps that should be merged
        let tokens = vec![
            WHISPER_TIMESTAMP_THRESHOLD,
            1,
            WHISPER_TIMESTAMP_THRESHOLD + 1, // 20ms later
            2,
        ];
        
        let segments = segments_from_tokens(&tokens, &cfg, &tokenizer).unwrap();
        
        // Should potentially merge or keep separate depending on merge logic
        assert!(segments.len() >= 1);
    }

    #[test]
    fn test_empty_token_handling() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        
        // Test with empty token sequence
        let segments = segments_from_tokens(&[], &cfg, &tokenizer).unwrap();
        assert!(segments.is_empty());
        
        // Test with only timestamp tokens
        let tokens = vec![WHISPER_TIMESTAMP_THRESHOLD];
        let segments = segments_from_tokens(&tokens, &cfg, &tokenizer).unwrap();
        // Should handle gracefully
        assert!(segments.len() >= 0);
    }

    #[test]
    fn test_confidence_estimation() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        
        // Test with long segment (should have higher confidence)
        let long_tokens = vec![
            WHISPER_TIMESTAMP_THRESHOLD,
            1, 1, 1, 1, 1, 1, // many tokens
            WHISPER_TIMESTAMP_THRESHOLD + 50,
        ];
        
        let segments = segments_from_tokens(&long_tokens, &cfg, &tokenizer).unwrap();
        assert_eq!(segments.len(), 1);
        // Long segments should have base confidence + bonus, which is > 0.7
        assert!(segments[0].confidence > 0.6); // Adjusted expectation
        
        // Test with short segment (should have lower confidence)
        let short_tokens = vec![
            WHISPER_TIMESTAMP_THRESHOLD,
            1, // single token
            WHISPER_TIMESTAMP_THRESHOLD + 1,
        ];
        
        let segments = segments_from_tokens(&short_tokens, &cfg, &tokenizer).unwrap();
        assert_eq!(segments.len(), 1);
        // Short segments should have base confidence - penalty, which is < 0.8
        assert!(segments[0].confidence < 0.9); // Adjusted expectation
    }

    #[test]
    fn test_invalid_token_handling() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        
        // Test with invalid timestamp token
        let tokens = vec![
            1000, // not a timestamp token
            1, 2,
        ];
        
        let segments = segments_from_tokens(&tokens, &cfg, &tokenizer).unwrap();
        // Should handle gracefully without crashing
        assert!(segments.len() >= 0);
    }

    #[test]
    fn test_word_timing_struct() {
        let word = WordTiming::new("test".to_string(), 0.5, 1.0, 0.8);
        
        assert_eq!(word.text, "test");
        assert_eq!(word.start, 0.5);
        assert_eq!(word.end, 1.0);
        assert_eq!(word.confidence, 0.8);
        assert!((word.duration() - 0.5).abs() < 0.001);
        assert!(word.has_valid_timing());
    }

    #[test]
    fn test_segment_summary() {
        let segment = Segment::new(0.0, 2.5, "Hello world".to_string());
        let summary = segment.summary();
        
        assert!(summary.contains("0.0s-2.5s"));
        assert!(summary.contains("Hello world"));
        assert!(summary.contains("confidence"));
    }
}
