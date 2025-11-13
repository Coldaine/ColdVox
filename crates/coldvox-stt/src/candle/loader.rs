//! Model and tokenizer loading for Whisper
//!
//! This module handles loading Whisper models in various formats (safetensors, GGUF)
//! along with their associated tokenizers and configurations.

use anyhow::{Context, Result};
use candle_core::Device;
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, Config};
use std::path::Path;

/// Loaded model components
pub struct LoadedModel {
    pub model: m::model::Whisper,
    pub config: Config,
    pub tokenizer: tokenizers::Tokenizer,
}

/// Load a Whisper model from disk
///
/// # Arguments
/// * `model_path` - Path to the model file (safetensors or GGUF)
/// * `config_path` - Path to the config.json file
/// * `tokenizer_path` - Path to the tokenizer.json file
/// * `quantized` - Whether the model is in GGUF quantized format
/// * `device` - Device to load the model on
pub fn load_model(
    model_path: &Path,
    config_path: &Path,
    tokenizer_path: &Path,
    quantized: bool,
    device: &Device,
) -> Result<LoadedModel> {
    // Load config
    let config_content = std::fs::read_to_string(config_path)
        .context("Failed to read model config")?;
    let config: Config = serde_json::from_str(&config_content)
        .context("Failed to parse model config")?;

    // Load tokenizer
    let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
        .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

    // Load model weights
    let vb = if quantized {
        // Load GGUF quantized model
        let gguf_file = std::fs::File::open(model_path)
            .context("Failed to open GGUF model file")?;
        let mut reader = std::io::BufReader::new(gguf_file);

        let content = candle_core::quantized::gguf_file::Content::read(&mut reader)
            .context("Failed to read GGUF file")?;

        VarBuilder::from_gguf(content, device)
            .context("Failed to create VarBuilder from GGUF")?
    } else {
        // Load standard safetensors model
        unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_path], m::DTYPE, device)
                .context("Failed to load safetensors model")?
        }
    };

    // Build the model
    let model = m::model::Whisper::load(&vb, config.clone())
        .context("Failed to build Whisper model")?;

    Ok(LoadedModel {
        model,
        config,
        tokenizer,
    })
}

/// Attempt to download a model from HuggingFace Hub if it doesn't exist locally
///
/// # Arguments
/// * `model_id` - HuggingFace model identifier (e.g., "openai/whisper-base")
/// * `revision` - Git revision/branch (usually "main")
/// * `quantized` - Whether to download quantized version
///
/// # Returns
/// Paths to the downloaded model files (model, config, tokenizer)
pub fn download_model(
    model_id: &str,
    revision: &str,
    quantized: bool,
) -> Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let api = hf_hub::api::sync::Api::new()
        .context("Failed to create HuggingFace API client")?;

    let repo = api.repo(hf_hub::Repo::model(model_id.to_string()));

    // Determine which files to download
    let model_file = if quantized {
        "model-q8_0.gguf" // Default to Q8 quantization
    } else {
        "model.safetensors"
    };

    let model_path = repo
        .get(model_file)
        .context(format!("Failed to download model file: {}", model_file))?;

    let config_path = repo
        .get("config.json")
        .context("Failed to download config.json")?;

    let tokenizer_path = repo
        .get("tokenizer.json")
        .context("Failed to download tokenizer.json")?;

    Ok((model_path, config_path, tokenizer_path))
}

/// Resolve model paths - use local if available, download if not
///
/// # Arguments
/// * `model_path_or_id` - Either a local path or HuggingFace model ID
/// * `quantized` - Whether to use quantized version
///
/// # Returns
/// Resolved paths to model files
pub fn resolve_model_paths(
    model_path_or_id: &str,
    quantized: bool,
) -> Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let path = Path::new(model_path_or_id);

    // Check if it's a local directory
    if path.is_dir() {
        let model_file = if quantized {
            path.join("model-q8_0.gguf")
        } else {
            path.join("model.safetensors")
        };

        let config_file = path.join("config.json");
        let tokenizer_file = path.join("tokenizer.json");

        if model_file.exists() && config_file.exists() && tokenizer_file.exists() {
            return Ok((model_file, config_file, tokenizer_file));
        }
    }

    // Check if it's a single model file
    if path.is_file() && (path.extension().and_then(|s| s.to_str()) == Some("safetensors")
        || path.extension().and_then(|s| s.to_str()) == Some("gguf")) {

        let parent = path.parent()
            .context("Model file has no parent directory")?;

        let config_file = parent.join("config.json");
        let tokenizer_file = parent.join("tokenizer.json");

        if config_file.exists() && tokenizer_file.exists() {
            return Ok((path.to_path_buf(), config_file, tokenizer_file));
        }
    }

    // Treat as HuggingFace model ID and try to download
    tracing::info!("Model not found locally, attempting download from HuggingFace Hub: {}", model_path_or_id);
    download_model(model_path_or_id, "main", quantized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_local_directory() {
        // This test would need actual model files to work
        // Keeping as a placeholder for integration testing
    }
}
