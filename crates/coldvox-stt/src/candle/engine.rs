//! High-level Whisper engine facade, wiring loader + model + decoder.

use candle_core::Device;

use super::decode::{Decoder, DecoderSettings};
use super::loader::{LoaderError, ModelLoader, ModelLoaderConfig};
use super::model::{build_from_artifacts, ModelBuildError, WhisperComponents};
use super::types::Transcript;

#[derive(Debug, Clone)]
pub struct WhisperEngineInit {
    pub model_id: String,
    pub revision: String,
    pub local_path: Option<std::path::PathBuf>,
    pub decoder_settings: DecoderSettings,
    pub device: Device,
}

impl Default for WhisperEngineInit {
    fn default() -> Self {
        Self {
            model_id: "openai/whisper-base.en".to_string(),
            revision: "main".to_string(),
            local_path: std::env::var("WHISPER_MODEL_PATH").ok().map(Into::into),
            decoder_settings: DecoderSettings::default(),
            device: Device::Cpu,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WhisperEngineError {
    #[error(transparent)]
    Loader(#[from] LoaderError),
    #[error(transparent)]
    Model(#[from] ModelBuildError),
    #[error("transcription not yet implemented")]
    NotImplemented,
}

pub struct WhisperEngine {
    device: Device,
    components: WhisperComponents,
    decoder: Decoder,
}

impl WhisperEngine {
    pub fn new(init: WhisperEngineInit) -> Result<Self, WhisperEngineError> {
        let loader_cfg = ModelLoaderConfig {
            model_id: init.model_id.clone(),
            revision: init.revision.clone(),
            local_path: init.local_path.clone(),
        };
        let loader = ModelLoader::new(loader_cfg)?;
        let artifacts = loader.load_safetensors(&init.device)?;
        let components = build_from_artifacts(artifacts, &init.device)?;
        let decoder = Decoder::new(components.tokenizer.clone(), init.decoder_settings);

        Ok(Self {
            device: init.device,
            components,
            decoder,
        })
    }

    pub fn decoder(&self) -> &Decoder {
        &self.decoder
    }

    /// Placeholder transcription entry point. Future implementation will feed audio
    /// through Candle, then decode/timestamp the resulting tokens.
    pub fn transcribe(&mut self, _audio: &[f32]) -> Result<Transcript, WhisperEngineError> {
        let _ = &self.device;
        let _ = &self.components;
        let _ = &self.decoder;
        Err(WhisperEngineError::NotImplemented)
    }
}
