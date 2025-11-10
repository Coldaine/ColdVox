//! Token decoding helpers for Whisper.
//!
//! The actual inference loop will be implemented once the Candle model wiring
//! is in place. For now we focus on robust token → text conversion, token
//! suppression, and transcript assembly which will be shared by the future
//! greedy decoder.

use std::collections::HashSet;

use tokenizers::Tokenizer;

use super::types::{Segment, Transcript};

#[derive(Debug, Clone)]
pub struct DecoderSettings {
    /// Maximum number of tokens to emit in a single segment.
    pub max_tokens: usize,
    /// Tokens that should be removed before decoding (notimestamps etc.).
    pub suppress_tokens: HashSet<u32>,
}

impl Default for DecoderSettings {
    fn default() -> Self {
        Self {
            max_tokens: 448,
            suppress_tokens: HashSet::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecoderError {
    #[error("tokenizer decode failed: {0}")]
    Tokenizer(#[from] tokenizers::Error),
    #[error("no tokens to decode")]
    EmptyTokens,
}

#[derive(Debug)]
pub struct Decoder {
    tokenizer: Tokenizer,
    settings: DecoderSettings,
}

impl Decoder {
    pub fn new(tokenizer: Tokenizer, settings: DecoderSettings) -> Self {
        Self { tokenizer, settings }
    }

    pub fn tokenizer(&self) -> &Tokenizer {
        &self.tokenizer
    }

    /// Decode a batch of token ids into a transcript.
    pub fn decode_tokens(&self, tokens: &[u32]) -> Result<Transcript, DecoderError> {
        if tokens.is_empty() {
            return Err(DecoderError::EmptyTokens);
        }
        let filtered = self.filter_tokens(tokens);
        if filtered.is_empty() {
            return Err(DecoderError::EmptyTokens);
        }
        let clamped = &filtered[..filtered.len().min(self.settings.max_tokens)];
        let text = self.tokenizer.decode(clamped, true)?;
        Ok(Transcript {
            segments: vec![Segment::new(0.0, 0.0, text)],
            language: None,
        })
    }

    fn filter_tokens(&self, tokens: &[u32]) -> Vec<u32> {
        tokens
            .iter()
            .copied()
            .filter(|id| !self.settings.suppress_tokens.contains(id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tokenizer() -> Tokenizer {
        let data = r#"{
            "version": "1.0",
            "truncation": null,
            "padding": null,
            "model": {
                "type": "WordLevel",
                "vocab": {"<unk>":0, "hello":1, ",":2, "world":3},
                "unk_token": "<unk>"
            }
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tokenizer.json");
        std::fs::write(&path, data).unwrap();
        Tokenizer::from_file(path).unwrap()
    }

    #[test]
    fn decodes_token_sequence() {
        let tokenizer = make_tokenizer();
        let decoder = Decoder::new(tokenizer, DecoderSettings::default());
        let transcript = decoder.decode_tokens(&[1, 2, 3]).unwrap();
        assert_eq!(transcript.segments.len(), 1);
        assert!(
            transcript.segments[0]
                .text
                .to_lowercase()
                .contains("hello")
        );
    }

    #[test]
    fn suppresses_tokens() {
        let tokenizer = make_tokenizer();
        let mut settings = DecoderSettings::default();
        settings.suppress_tokens.insert(2);
        let decoder = Decoder::new(tokenizer, settings);
        let transcript = decoder.decode_tokens(&[1, 2, 3]).unwrap();
        assert!(
            !transcript.segments[0]
                .text
                .contains(","),
            "Suppressed commas should be absent"
        );
    }
}
