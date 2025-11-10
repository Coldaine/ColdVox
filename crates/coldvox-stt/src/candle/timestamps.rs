//! Timestamp extraction helpers for Whisper tokens.

use candle_transformers::models::whisper::Config as WhisperConfig;
use tokenizers::Tokenizer;

use super::audio::{HOP_LENGTH, SAMPLE_RATE};
use super::types::Segment;

/// First timestamp token ID in Whisper tokenizer.
pub const TIMESTAMP_BEGIN: u32 = 50364;

/// Resolution between timestamp tokens (seconds).
pub const TIMESTAMP_RESOLUTION: f32 = 0.02;

/// Returns true if the token encodes a timestamp.
pub fn is_timestamp_token(token: u32, cfg: &WhisperConfig) -> bool {
    token >= TIMESTAMP_BEGIN
        && token < TIMESTAMP_BEGIN + cfg.max_source_positions as u32
}

/// Convert a timestamp token into a floating-point second offset.
pub fn token_to_time(token: u32) -> f32 {
    let frames = token.saturating_sub(TIMESTAMP_BEGIN) as f32;
    frames * TIMESTAMP_RESOLUTION
}

/// Build segments from a mixed token sequence containing timestamps.
pub fn segments_from_tokens(
    tokens: &[u32],
    cfg: &WhisperConfig,
    tokenizer: &Tokenizer,
) -> Result<Vec<Segment>, tokenizers::Error> {
    let mut segments = Vec::new();
    let mut text_tokens = Vec::new();
    let mut current_start: Option<f32> = None;
    let mut previous_ts: Option<f32> = None;

    for &token in tokens {
        if is_timestamp_token(token, cfg) {
            let ts = token_to_time(token);
            if !text_tokens.is_empty() && current_start.is_some() {
                let text = tokenizer.decode(&text_tokens, true)?;
                let segment = Segment::new(
                    current_start.unwrap(),
                    ts.max(current_start.unwrap()),
                    text,
                );
                segments.push(segment);
                text_tokens.clear();
            }
            current_start = Some(ts);
            previous_ts = Some(ts);
        } else {
            text_tokens.push(token);
        }
    }

    if !text_tokens.is_empty() {
        let text = tokenizer.decode(&text_tokens, true)?;
        let start = current_start.or(previous_ts).unwrap_or(0.0);

        // Estimate duration based on Whisper hop length.
        let frame_duration = HOP_LENGTH as f32 / SAMPLE_RATE as f32;
        let end = if let Some(prev) = previous_ts {
            (prev + frame_duration).max(start)
        } else {
            start + text_tokens.len() as f32 * frame_duration
        };

        segments.push(Segment::new(start, end, text));
    }

    Ok(segments)
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
        assert!(is_timestamp_token(TIMESTAMP_BEGIN, &cfg));
        assert!(!is_timestamp_token(1, &cfg));
    }

    #[test]
    fn builds_segments() {
        let cfg = test_config();
        let tokenizer = make_tokenizer();
        let tokens = vec![
            TIMESTAMP_BEGIN,
            1,
            2,
            TIMESTAMP_BEGIN + 50,
        ];
        let segments = segments_from_tokens(&tokens, &cfg, &tokenizer).unwrap();
        assert_eq!(segments.len(), 1);
        assert!(segments[0].text.contains("hi"));
        assert!(segments[0].end >= segments[0].start);
    }
}
