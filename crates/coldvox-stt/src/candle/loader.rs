//! Model loading utilities for Candle Whisper.
//!
//! This module handles loading Whisper models in various formats:
//! - SafeTensors (.safetensors) - standard format
//! - GGUF (.gguf) - quantized format
//!
//! It also manages tokenizer and configuration loading.

#[cfg(feature = "whisper")]
use anyhow::{Context, Result};
#[cfg(feature = "whisper")]
use candle_core::Device;
#[cfg(feature = "whisper")]
use candle_transformers::models::whisper::{Config, Whisper};
#[cfg(feature = "whisper")]
use std::path::Path;
#[cfg(feature = "whisper")]
use tokenizers::Tokenizer;

/// Model format detection
#[cfg(feature = "whisper")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    /// SafeTensors format (.safetensors)
    SafeTensors,
    /// GGUF quantized format (.gguf)
    Gguf,
}

#[cfg(feature = "whisper")]
impl ModelFormat {
    /// Detect format from file extension
    pub fn from_path(path: &Path) -> Result<Self> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .context("Failed to get file extension")?;

        match ext {
            "safetensors" => Ok(Self::SafeTensors),
            "gguf" => Ok(Self::Gguf),
            _ => anyhow::bail!("Unsupported model format: {}", ext),
        }
    }
}

/// Load the Whisper model from a file path
#[cfg(feature = "whisper")]
pub fn load_model(
    model_path: &Path,
    config_path: &Path,
    device: &Device,
) -> Result<Whisper> {
    // Load config
    let config_str = std::fs::read_to_string(config_path)
        .context("Failed to read model config")?;
    let config: Config = serde_json::from_str(&config_str)
        .context("Failed to parse model config")?;

    // Detect model format
    let format = ModelFormat::from_path(model_path)?;

    match format {
        ModelFormat::SafeTensors => load_safetensors(model_path, config, device),
        ModelFormat::Gguf => load_gguf(model_path, config, device),
    }
}

/// Load model from SafeTensors format
#[cfg(feature = "whisper")]
fn load_safetensors(
    model_path: &Path,
    config: Config,
    device: &Device,
) -> Result<Whisper> {
    let vb = unsafe {
        candle_nn::VarBuilder::from_mmaped_safetensors(
            &[model_path.to_path_buf()],
            candle_core::DType::F32,
            device,
        )?
    };

    Whisper::load(&vb, config).context("Failed to load Whisper model from SafeTensors")
}

/// Load model from GGUF quantized format
#[cfg(feature = "whisper")]
fn load_gguf(
    model_path: &Path,
    config: Config,
    device: &Device,
) -> Result<Whisper> {
    let mut file = std::fs::File::open(model_path)
        .context("Failed to open GGUF file")?;

    let gguf_file = candle_core::quantized::gguf_file::Content::read(&mut file)
        .context("Failed to read GGUF file")?;

    let vb = candle_core::quantized::VarBuilder::from_gguf(&gguf_file, device)?;

    Whisper::load(&vb, config).context("Failed to load Whisper model from GGUF")
}

/// Load tokenizer from a directory or file
#[cfg(feature = "whisper")]
pub fn load_tokenizer(tokenizer_path: &Path) -> Result<Tokenizer> {
    // Check if it's a directory (HF tokenizer format) or a file (tokenizer.json)
    let tokenizer_file = if tokenizer_path.is_dir() {
        tokenizer_path.join("tokenizer.json")
    } else {
        tokenizer_path.to_path_buf()
    };

    Tokenizer::from_file(&tokenizer_file)
        .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))
}

/// Helper to download model from HuggingFace Hub
#[cfg(feature = "whisper")]
pub fn download_model_from_hub(
    model_id: &str,
    revision: Option<&str>,
) -> Result<std::path::PathBuf> {
    use hf_hub::api::tokio::Api;

    let api = Api::new()?;
    let repo = api.model(model_id.to_string());

    let repo = if let Some(rev) = revision {
        repo.revision(rev.to_string())
    } else {
        repo
    };

    // Download model files
    let model_file = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            repo.get("model.safetensors").await
        })
    })?;

    Ok(model_file)
}

#[cfg(test)]
#[cfg(feature = "whisper")]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        let safetensors_path = Path::new("model.safetensors");
        assert_eq!(
            ModelFormat::from_path(safetensors_path).unwrap(),
            ModelFormat::SafeTensors
        );

        let gguf_path = Path::new("model.gguf");
        assert_eq!(
            ModelFormat::from_path(gguf_path).unwrap(),
            ModelFormat::Gguf
        );

        let invalid_path = Path::new("model.bin");
        assert!(ModelFormat::from_path(invalid_path).is_err());
    }
}
