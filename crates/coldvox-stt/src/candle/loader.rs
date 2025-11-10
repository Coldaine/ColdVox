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
///
/// # UNSAFE: Memory-mapped file loading
///
/// This function uses `unsafe` memory-mapped file loading via `from_mmaped_safetensors`.
///
/// ## Why memory mapping?
/// 1. **Performance**: Avoids loading entire model (1-3GB) into memory at once
/// 2. **OS optimization**: Let the kernel handle paging and caching
/// 3. **Startup time**: Much faster than reading the entire file into RAM
///
/// ## Safety considerations:
/// 1. The file must not be modified while mapped (immutable borrow contract)
/// 2. The file must be a valid SafeTensors file (validated by Candle)
/// 3. The memory layout must match SafeTensors format (validated at runtime)
///
/// ## Why it's safe in practice:
/// 1. Model files are read-only after download
/// 2. SafeTensors format includes checksums and validation
/// 3. Candle validates the file structure before using data
/// 4. The OS enforces memory protection (SIGSEGV if file deleted/truncated)
///
/// # DType Selection
///
/// Currently hardcoded to F32 (32-bit float) because:
/// 1. Most Whisper models are published in F32 format
/// 2. F16 (16-bit float) requires explicit conversion and GPU support
/// 3. Mixed precision (F16/F32) is not yet implemented
///
/// TODO: Support F16 models for lower memory usage on supported hardware
#[cfg(feature = "whisper")]
fn load_safetensors(
    model_path: &Path,
    config: Config,
    device: &Device,
) -> Result<Whisper> {
    // SAFETY: Model file is immutable, SafeTensors format is validated by Candle
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
///
/// # Async/Blocking Interaction
///
/// This function uses `tokio::task::block_in_place` to bridge async HuggingFace Hub API
/// with potentially synchronous calling contexts.
///
/// ## Why block_in_place?
/// 1. The HuggingFace Hub API is async-only
/// 2. Model loading often happens in sync contexts (e.g., `WhisperEngine::new`)
/// 3. `block_in_place` tells Tokio to move the blocking operation off the async worker thread
///
/// ## Deadlock prevention:
/// - `block_in_place` is **safe** when called from within a Tokio runtime
/// - It **will panic** if called outside a runtime
/// - It moves the current task to a blocking thread, preventing worker starvation
/// - The inner `block_on` runs the async download without blocking the async executor
///
/// ## When this could deadlock:
/// - If called from a single-threaded runtime (`current_thread` runtime) - WILL PANIC
/// - If called from outside any Tokio runtime - WILL PANIC
/// - If the runtime is shutting down - may hang
///
/// ## Usage note:
/// Prefer calling this from async contexts by using `repo.get().await` directly.
/// This function exists for compatibility with sync initialization code.
///
/// TODO: Consider requiring this to be called only from async contexts to avoid
/// the block_in_place complexity entirely.
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

    // Bridge async HuggingFace API with sync calling context
    // SAFETY: Only safe when called from within a multi-threaded Tokio runtime
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
    fn test_format_detection_safetensors() {
        let safetensors_path = Path::new("model.safetensors");
        let format = ModelFormat::from_path(safetensors_path);
        assert!(format.is_ok());
        assert_eq!(format.unwrap(), ModelFormat::SafeTensors);
    }

    #[test]
    fn test_format_detection_gguf() {
        let gguf_path = Path::new("model.gguf");
        let format = ModelFormat::from_path(gguf_path);
        assert!(format.is_ok());
        assert_eq!(format.unwrap(), ModelFormat::Gguf);
    }

    #[test]
    fn test_format_detection_invalid_extension() {
        let invalid_extensions = vec![
            "model.bin",
            "model.pt",
            "model.pth",
            "model.onnx",
            "model.h5",
            "model.txt",
            "model",
        ];

        for path_str in invalid_extensions {
            let path = Path::new(path_str);
            let result = ModelFormat::from_path(path);
            assert!(
                result.is_err(),
                "Expected error for path: {}, got: {:?}",
                path_str,
                result
            );
        }
    }

    #[test]
    fn test_format_detection_no_extension() {
        let no_ext_path = Path::new("model");
        let result = ModelFormat::from_path(no_ext_path);
        assert!(result.is_err(), "Should fail on files without extension");
    }

    #[test]
    fn test_format_detection_uppercase() {
        // Test case sensitivity
        let upper_safetensors = Path::new("MODEL.SAFETENSORS");
        let result = ModelFormat::from_path(upper_safetensors);
        // This will fail because extension check is case-sensitive
        assert!(result.is_err(), "Extension matching is case-sensitive");
    }

    #[test]
    fn test_format_detection_with_path() {
        let nested_safetensors = Path::new("/models/whisper/base/model.safetensors");
        let format = ModelFormat::from_path(nested_safetensors);
        assert!(format.is_ok());
        assert_eq!(format.unwrap(), ModelFormat::SafeTensors);

        let nested_gguf = Path::new("../../whisper-base.gguf");
        let format = ModelFormat::from_path(nested_gguf);
        assert!(format.is_ok());
        assert_eq!(format.unwrap(), ModelFormat::Gguf);
    }

    #[test]
    fn test_load_tokenizer_from_file() {
        // This test requires an actual tokenizer file
        // It's a placeholder for integration testing with real model files
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let tokenizer_path = dir.path().join("tokenizer.json");

        // Test missing file
        let result = load_tokenizer(&tokenizer_path);
        assert!(result.is_err(), "Should fail on missing tokenizer file");
    }

    #[test]
    fn test_load_tokenizer_from_directory() {
        // Test directory-based tokenizer loading
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let result = load_tokenizer(dir.path());
        // Should fail because tokenizer.json doesn't exist in temp dir
        assert!(result.is_err());
    }

    #[test]
    fn test_load_model_missing_file() {
        let device = Device::Cpu;
        let model_path = Path::new("/nonexistent/model.safetensors");
        let config_path = Path::new("/nonexistent/config.json");

        let result = load_model(model_path, config_path, &device);
        assert!(result.is_err(), "Should fail on missing model file");
    }

    #[test]
    fn test_load_model_invalid_format() {
        let device = Device::Cpu;
        let model_path = Path::new("/tmp/model.invalid");
        let config_path = Path::new("/tmp/config.json");

        let result = load_model(model_path, config_path, &device);
        assert!(result.is_err(), "Should fail on invalid format");
    }

    #[test]
    fn test_model_format_equality() {
        assert_eq!(ModelFormat::SafeTensors, ModelFormat::SafeTensors);
        assert_eq!(ModelFormat::Gguf, ModelFormat::Gguf);
        assert_ne!(ModelFormat::SafeTensors, ModelFormat::Gguf);
    }

    // Note: The following tests require actual model files and cannot run in CI
    // They are documented here for manual testing:
    //
    // #[test]
    // #[ignore] // Requires model files
    // fn test_load_safetensors_real_model() {
    //     // Test with actual model file from HuggingFace
    //     let model_path = Path::new("tests/fixtures/whisper-tiny/model.safetensors");
    //     let config_path = Path::new("tests/fixtures/whisper-tiny/config.json");
    //     let device = Device::Cpu;
    //     let result = load_model(model_path, config_path, &device);
    //     assert!(result.is_ok());
    // }
    //
    // #[test]
    // #[ignore] // Requires network access
    // fn test_download_model_from_hub() {
    //     // Test downloading from HuggingFace Hub
    //     // Requires Tokio runtime
    //     let rt = tokio::runtime::Runtime::new().unwrap();
    //     let result = rt.block_on(async {
    //         download_model_from_hub("openai/whisper-tiny", None)
    //     });
    //     assert!(result.is_ok());
    // }
}
