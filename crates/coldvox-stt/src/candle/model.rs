//! Whisper model wrappers around candle-transformers implementations.

use std::path::PathBuf;

use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as whisper, Config as WhisperConfig};
use tokenizers::Tokenizer;

use super::loader::ModelArtifacts;

#[derive(Debug)]
pub enum WhisperModel {
    Normal(whisper::model::Whisper),
    Quantized(whisper::quantized_model::Whisper),
}

impl WhisperModel {
    pub fn config(&self) -> &WhisperConfig {
        match self {
            Self::Normal(model) => &model.config,
            Self::Quantized(model) => &model.config,
        }
    }

    pub fn encoder_forward(&mut self, x: &Tensor, flush: bool) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(model) => model.encoder.forward(x, flush),
            Self::Quantized(model) => model.encoder.forward(x, flush),
        }
    }

    pub fn decoder_forward(
        &mut self,
        x: &Tensor,
        xa: &Tensor,
        flush: bool,
    ) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(model) => model.decoder.forward(x, xa, flush),
            Self::Quantized(model) => model.decoder.forward(x, xa, flush),
        }
    }

    pub fn decoder_final_linear(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(model) => model.decoder.final_linear(x),
            Self::Quantized(model) => model.decoder.final_linear(x),
        }
    }
}

#[derive(Debug)]
pub struct WhisperComponents {
    pub model: WhisperModel,
    pub config: WhisperConfig,
    pub tokenizer: Tokenizer,
    pub weights_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelBuildError {
    #[error("candle error: {0}")]
    Candle(#[from] candle_core::Error),
}

pub fn build_from_artifacts(
    artifacts: ModelArtifacts,
    device: &Device,
) -> Result<WhisperComponents, ModelBuildError> {
    let ModelArtifacts {
        config,
        tokenizer,
        weights_path,
    } = artifacts;
    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[weights_path.clone()], whisper::DTYPE, device)?
    };
    let model = whisper::model::Whisper::load(&vb, config.clone())?;
    Ok(WhisperComponents {
        model: WhisperModel::Normal(model),
        config,
        tokenizer,
        weights_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;
    use tokenizers::Tokenizer;

    #[test]
    fn build_fails_without_weights() {
        let config = WhisperConfig {
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
        };
        let data = r#"{
            "version": "1.0",
            "truncation": null,
            "padding": null,
            "model": {
                "type": "WordLevel",
                "vocab": {"<unk>":0, "a":1},
                "unk_token": "<unk>"
            }
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let tokenizer_path = dir.path().join("tokenizer.json");
        std::fs::write(&tokenizer_path, data).unwrap();
        let tokenizer = Tokenizer::from_file(tokenizer_path).unwrap();
        let artifacts = ModelArtifacts {
            config,
            tokenizer,
            weights_path: dir.path().join("missing.safetensors"),
        };
        let err = build_from_artifacts(artifacts, &Device::Cpu).unwrap_err();
        match err {
            ModelBuildError::Candle(_) => {}
        }
    }
}
