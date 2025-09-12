//! Model management utilities for Vosk STT.
//!
//! Centralized logic for locating and validating the Vosk model directory so
//! other parts of the codebase (tests, examples, CI scripts) do not re‑implement
//! path probing. This also future‑proofs expansion to multiple models.
use std::fmt;
use std::path::{Path, PathBuf};

/// Canonical model directory name (small English model).
pub const MODEL_DIR_NAME: &str = "vosk-model-small-en-us-0.15";
/// Preferred parent directory when models are organized semantically.
pub const MODELS_DIR: &str = "models";

/// Result of a successful model location.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub path: PathBuf,
    pub source: ModelSource,
}

#[derive(Debug, Clone, Copy)]
pub enum ModelSource {
    /// Explicit path provided via `VOSK_MODEL_PATH` environment variable.
    Env,
    /// Path provided in `TranscriptionConfig`.
    Config,
    /// Located under `models/<MODEL_DIR_NAME>`.
    ModelsDir,
    /// Located at repository root `<MODEL_DIR_NAME>` (legacy layout).
    RepoRoot,
}

/// Error describing why a model could not be located.
#[derive(Debug)]
pub enum ModelError {
    ExplicitPathMissing(String),
    NotFound { checked: String, guidance: String },
    ExtractionFailed(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::ExplicitPathMissing(p) => {
                write!(
                    f,
                    "Vosk model path '{}' does not exist or is not a directory",
                    p
                )
            }
            ModelError::NotFound { checked, guidance } => {
                write!(
                    f,
                    "Vosk model not found. Checked: {}. Guidance: {}",
                    checked, guidance
                )
            }
            ModelError::ExtractionFailed(msg) => {
                write!(
                    f,
                    "Failed to auto-extract model from vendor bundle: {}",
                    msg
                )
            }
        }
    }
}

impl std::error::Error for ModelError {}

/// Locate the model directory using (in order):
/// 1. Environment variable `VOSK_MODEL_PATH`
/// 2. Explicit config path (if provided)
/// 3. `models/<MODEL_DIR_NAME>` (new preferred layout)
/// 4. `<MODEL_DIR_NAME>` at repository root (legacy layout)
pub fn locate_model(config_path: Option<&str>) -> Result<ModelInfo, ModelError> {
    // 1. Environment override
    if let Ok(p) = std::env::var("VOSK_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.is_dir() {
            return Ok(ModelInfo {
                path: pb,
                source: ModelSource::Env,
            });
        } else {
            return Err(ModelError::ExplicitPathMissing(p));
        }
    }

    // 2. Config path
    if let Some(cp) = config_path.filter(|s| !s.is_empty()) {
        let pb = PathBuf::from(cp);
        if pb.is_dir() {
            return Ok(ModelInfo {
                path: pb,
                source: ModelSource::Config,
            });
        } else {
            return Err(ModelError::ExplicitPathMissing(cp.to_string()));
        }
    }

    // 3. models/<MODEL_DIR_NAME>
    let models_dir_candidate = Path::new(MODELS_DIR).join(MODEL_DIR_NAME);
    if models_dir_candidate.is_dir() {
        return Ok(ModelInfo {
            path: models_dir_candidate,
            source: ModelSource::ModelsDir,
        });
    }

    // 4. <MODEL_DIR_NAME> at repo root (legacy layout)
    let root_candidate = Path::new(MODEL_DIR_NAME);
    if root_candidate.is_dir() {
        tracing::warn!(
            "Using legacy model layout at repo root '{}'. This will be removed after deprecation window. \
             Please move to 'models/{}'" ,
            MODEL_DIR_NAME, MODEL_DIR_NAME
        );
        return Ok(ModelInfo {
            path: root_candidate.to_path_buf(),
            source: ModelSource::RepoRoot,
        });
    }

    // Construct helpful message
    let checked = vec![
        format!("$VOSK_MODEL_PATH (env)"),
        config_path.unwrap_or("").to_string(),
        format!("{}/{}", MODELS_DIR, MODEL_DIR_NAME),
        MODEL_DIR_NAME.to_string(),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(", ");

    let guidance = format!(
        "Ensure the model directory is committed under '{}/{}' (preferred) or place it at repo root '{}'. \
         You can also set VOSK_MODEL_PATH to an absolute path.",
        MODELS_DIR, MODEL_DIR_NAME, MODEL_DIR_NAME
    );

    Err(ModelError::NotFound { checked, guidance })
}

/// Log model resolution information in a standardized format.
/// Call this once during application startup where STT initializes.
pub fn log_model_resolution(info: &ModelInfo) {
    let source_desc = match info.source {
        ModelSource::Env => "environment variable (VOSK_MODEL_PATH)",
        ModelSource::Config => "configuration",
        ModelSource::ModelsDir => "ModelsDir (preferred layout)",
        ModelSource::RepoRoot => "RepoRoot (legacy layout)",
    };
    tracing::info!(
        "Vosk model resolved: path={} source={}",
        info.path.display(),
        source_desc
    );
}

