//! Whisper model loader built on top of hf-hub and Candle.
//!
//! This module is responsible for downloading (or pointing to) the Whisper model
//! artifacts, parsing the `config.json`, and instantiating a `tokenizers::Tokenizer`.
//! The actual model weights are left on disk and returned as a path so later phases
//! can mmap or stream them directly into Candle.

use std::fmt;
use std::path::{Path, PathBuf};

use candle_core::Device;
use candle_transformers::models::whisper::Config as WhisperConfig;
use hf_hub::{
    api::sync::{Api, ApiRepo},
    Repo, RepoType,
};
use tokenizers::Tokenizer;

const CONFIG_FILE: &str = "config.json";
const TOKENIZER_FILE: &str = "tokenizer.json";
const WEIGHTS_FILE: &str = "model.safetensors";

#[derive(Debug, Clone)]
pub struct ModelLoaderConfig {
    pub model_id: String,
    pub revision: String,
    pub local_path: Option<PathBuf>,
}

impl Default for ModelLoaderConfig {
    fn default() -> Self {
        Self {
            model_id: "openai/whisper-base.en".to_string(),
            revision: "main".to_string(),
            local_path: std::env::var("WHISPER_MODEL_PATH").ok().map(PathBuf::from),
        }
    }
}

impl ModelLoaderConfig {
    pub fn from_env() -> Self {
        Self::default()
    }
}

#[derive(Debug)]
pub struct ModelFiles {
    pub config_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub weights_path: PathBuf,
}

impl ModelFiles {
    fn ensure_exists(&self) -> Result<(), LoaderError> {
        for path in [&self.config_path, &self.tokenizer_path, &self.weights_path] {
            if !path.exists() {
                return Err(LoaderError::MissingFile { path: path.clone() });
            }
        }
        Ok(())
    }
}

pub struct ModelArtifacts {
    pub config: WhisperConfig,
    pub tokenizer: Tokenizer,
    pub weights_path: PathBuf,
}

impl fmt::Debug for ModelArtifacts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelArtifacts")
            .field("config", &self.config)
            .field("tokenizer", &"<tokenizer>")
            .field("weights_path", &self.weights_path)
            .finish()
    }
}

#[derive(Debug)]
enum ModelSource {
    Local { root: PathBuf },
    Hub { repo: ApiRepo },
}

#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("hf-hub API error: {0}")]
    Hub(#[from] hf_hub::api::sync::ApiError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse error: {0}")]
    ConfigParse(#[from] serde_json::Error),
    #[error("tokenizer error: {0}")]
    Tokenizer(#[from] tokenizers::Error),
    #[error("model file missing: {path:?}")]
    MissingFile { path: PathBuf },
}

#[derive(Debug)]
pub struct ModelLoader {
    source: ModelSource,
}

impl ModelLoader {
    pub fn new(cfg: ModelLoaderConfig) -> Result<Self, LoaderError> {
        let source = if let Some(root) = cfg.local_path.clone() {
            ModelSource::Local { root }
        } else {
            let api = Api::new()?;
            let repo = api.repo(Repo::with_revision(
                cfg.model_id.clone(),
                RepoType::Model,
                cfg.revision.clone(),
            ));
            ModelSource::Hub { repo }
        };

        Ok(Self { source })
    }

    fn fetch_files(&self) -> Result<ModelFiles, LoaderError> {
        match &self.source {
            ModelSource::Local { root } => {
                let files = ModelFiles {
                    config_path: root.join(CONFIG_FILE),
                    tokenizer_path: root.join(TOKENIZER_FILE),
                    weights_path: root.join(WEIGHTS_FILE),
                };
                files.ensure_exists()?;
                Ok(files)
            }
            ModelSource::Hub { repo } => {
                let config_path = repo.get(CONFIG_FILE)?;
                let tokenizer_path = repo.get(TOKENIZER_FILE)?;
                let weights_path = repo.get(WEIGHTS_FILE)?;
                Ok(ModelFiles {
                    config_path,
                    tokenizer_path,
                    weights_path,
                })
            }
        }
    }

    fn parse_config(path: &Path) -> Result<WhisperConfig, LoaderError> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents)?)
    }

    fn load_tokenizer(path: &Path) -> Result<Tokenizer, LoaderError> {
        Ok(Tokenizer::from_file(path)?)
    }

    /// Load Whisper artifacts for safetensor weights.
    pub fn load_safetensors(&self, device: &Device) -> Result<ModelArtifacts, LoaderError> {
        let _ = device; // Device is kept for parity with future GPU-aware loading.
        let files = self.fetch_files()?;
        let config = Self::parse_config(&files.config_path)?;
        let tokenizer = Self::load_tokenizer(&files.tokenizer_path)?;

        Ok(ModelArtifacts {
            config,
            tokenizer,
            weights_path: files.weights_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;
    use std::fs::File;
    use tempfile::tempdir;

    fn write_tokenizer(path: &Path) {
        let data = r#"{
            "version": "1.0",
            "truncation": null,
            "padding": null,
            "model": {
                "type": "WordLevel",
                "vocab": {"<unk>":0, "hello":1},
                "unk_token": "<unk>"
            }
        }"#;
        std::fs::write(path, data).unwrap();
    }

    fn write_config(path: &Path) {
        let data = r#"{
            "num_mel_bins": 80,
            "max_source_positions": 1500,
            "d_model": 512,
            "encoder_attention_heads": 8,
            "encoder_layers": 6,
            "vocab_size": 51865,
            "max_target_positions": 448,
            "decoder_attention_heads": 8,
            "decoder_layers": 6,
            "suppress_tokens": [1,2,3]
        }"#;
        std::fs::write(path, data).unwrap();
    }

    #[test]
    fn loads_local_artifacts() {
        let dir = tempdir().unwrap();
        write_config(&dir.path().join(CONFIG_FILE));
        write_tokenizer(&dir.path().join(TOKENIZER_FILE));
        File::create(dir.path().join(WEIGHTS_FILE)).unwrap();

        let cfg = ModelLoaderConfig {
            model_id: "local/test".into(),
            revision: "main".into(),
            local_path: Some(dir.path().to_path_buf()),
        };

        let loader = ModelLoader::new(cfg).expect("loader");
        let artifacts = loader.load_safetensors(&Device::Cpu).expect("artifacts");

        assert_eq!(artifacts.config.vocab_size, 51865);
        assert!(artifacts.weights_path.ends_with(WEIGHTS_FILE));
        assert_eq!(artifacts.tokenizer.get_vocab_size(false), 2);
    }

    #[test]
    fn errors_when_missing_files() {
        let dir = tempdir().unwrap();
        let cfg = ModelLoaderConfig {
            model_id: "local/test".into(),
            revision: "main".into(),
            local_path: Some(dir.path().to_path_buf()),
        };
        let loader = ModelLoader::new(cfg).expect("loader");
        let err = loader.load_safetensors(&Device::Cpu).unwrap_err();
        match err {
            LoaderError::MissingFile { path } => {
                assert!(path.ends_with(CONFIG_FILE));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
