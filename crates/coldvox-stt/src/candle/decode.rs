use candle::{Result, Tensor, D};
use candle_nn::ops::softmax_last_dim;
use crate::candle::timestamps::{perform_word_alignment, perform_timestamp_probs_alignment};
use candle_transformers::models::whisper::{self as whisper, Config, Whisper};
use crate::candle::timestamps::TranscriptionResult;
use crate::candle::WordTimestampHeuristic;

pub struct Decoder {
    model: Whisper,
    tokenizer: whisper::tokenizer::Tokenizer,
    heuristic: WordTimestampHeuristic,
}

impl Decoder {
    pub fn new(model: Whisper, tokenizer: whisper::tokenizer::Tokenizer, heuristic: &WordTimestampHeuristic) -> Self {
        Self { model, tokenizer, heuristic: heuristic.clone() }
    }

    pub fn run(&mut self, mel: &Tensor) -> Result<Vec<TranscriptionResult>> {
        let mut audio_features = self.model.encoder.forward(mel, true)?;
        let mut tokens = vec![self.tokenizer.sot_token() as i32];
        let mut words = vec![];

        for _ in 0..self.model.config.max_target_positions {
            let tokens_tensor = Tensor::new(tokens.as_slice(), mel.device())?.unsqueeze(0)?;
            let (logits, cross_attentions) = self.model.decoder.forward(&tokens_tensor, &audio_features, false)?;

            let next_token = self.argmax(&logits)?;

            tokens.push(next_token);

            if self.is_segment_end(next_token) {
                let segment_tokens = &tokens;
                let segment_words = match self.heuristic {
                    WordTimestampHeuristic::AttentionDtw => {
                        if let Some(cross_attentions) = cross_attentions {
                            perform_word_alignment(
                                segment_tokens,
                                &cross_attentions,
                                &self.tokenizer,
                                true, // Assuming space-based splitting
                            )?
                        } else {
                            vec![]
                        }
                    }
                    WordTimestampHeuristic::TimestampProbs => {
                        perform_timestamp_probs_alignment(segment_tokens, &logits, &self.tokenizer)?
                    }
                };
                words.extend(segment_words);

                if next_token == self.tokenizer.eot_token() as i32 {
                    break;
                }
                tokens = vec![self.tokenizer.sot_token() as i32];
            }
        }

        Ok(words)
    }

    fn argmax(&self, logits: &Tensor) -> Result<i32> {
        let logits = logits.i((0, logits.dim(D::Minus1)? - 1, ..))?;
        let next_token = logits.argmax(D::Minus1)?.to_scalar::<u32>()? as i32;
        Ok(next_token)
    }

    fn is_segment_end(&self, token: i32) -> bool {
        token >= self.tokenizer.timestamp_begin() as i32 || token == self.tokenizer.eot_token() as i32
    }
}
