pub mod audio;
pub mod decode;
pub mod loader;
pub mod timestamps;

use candle::{Device, Result, Tensor};
use crate::candle::audio::log_mel_spectrogram;
use crate::candle::decode::Decoder;
use crate::candle::loader::load_model;
use candle_transformers::models::whisper::{self as whisper, Config, Whisper};
use timestamps::TranscriptionResult;

pub enum WordTimestampHeuristic {
    AttentionDtw,
    TimestampProbs,
}

pub struct WhisperEngine {
    decoder: Decoder,
    config: WhisperEngineConfig,
}

pub struct WhisperEngineConfig {
    pub model_path: String,
    pub tokenizer_path: String,
    pub config_path: String,
    pub quantized: bool,
    pub enable_timestamps: bool,
    pub heuristic: WordTimestampHeuristic,
}

impl WhisperEngine {
    pub fn new(config: WhisperEngineConfig) -> Result<Self> {
        let (model, tokenizer) = load_model(&config.model_path, &config.tokenizer_path, &config.config_path, config.quantized)?;
        let decoder = Decoder::new(model, tokenizer, &config.heuristic);
        Ok(Self { decoder, config })
    }

    pub fn transcribe(&mut self, pcm_audio: &[f32]) -> Result<Vec<TranscriptionResult>> {
        let mel = self.preprocess_audio(pcm_audio)?;
        let words = self.decoder.run(&mel)?;
        Ok(words)
    }

    fn preprocess_audio(&self, pcm_audio: &[f32]) -> Result<Tensor> {
        log_mel_spectrogram(pcm_audio, &Device::Cpu)
    }
}
