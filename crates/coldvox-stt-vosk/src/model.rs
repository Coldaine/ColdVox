use std::env;
use std::fmt;
use std::io;
use std::path::PathBuf;

use zip::ZipArchive;
use uuid::Uuid;

/// Preferred parent directory when models are organized semantically.
pub const MODELS_DIR: &str = "models";

/// Result of a successful model location.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub path: PathBuf,
    pub source: ModelSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSource {
    Env,
    Config,
    ModelsDir,
    RepoRoot, // This is now effectively the same as ModelsDir with ancestor scanning
    Extracted,
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
                write!(f, "Vosk model path '{}' does not exist or is not a directory", p)
            }
            ModelError::NotFound { checked, guidance } => {
                write!(f, "Vosk model not found. Checked: {}. Guidance: {}", checked, guidance)
            }
            ModelError::ExtractionFailed(msg) => {
                write!(f, "Failed to auto-extract model: {}", msg)
            }
        }
    }
}

impl std::error::Error for ModelError {}

/// Locate the model directory.
pub fn locate_model(config_path: Option<&str>) -> Result<ModelInfo, ModelError> {
    if let Ok(p) = env::var("VOSK_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.is_dir() {
            return Ok(ModelInfo { path: pb, source: ModelSource::Env });
        }
    }

    if let Some(cp) = config_path.filter(|s| !s.is_empty()) {
        let pb = PathBuf::from(cp);
        if pb.is_dir() {
            return Ok(ModelInfo { path: pb, source: ModelSource::Config });
        }
    }

    let candidates = find_model_candidates();
    if let Some(best_candidate) = pick_best_candidate(candidates) {
        return Ok(ModelInfo { path: best_candidate, source: ModelSource::ModelsDir });
    }

    Err(ModelError::NotFound {
        checked: "standard locations".to_string(),
        guidance: "No vosk-model-* directory found.".to_string(),
    })
}

fn find_model_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(current_dir) = env::current_dir() {
        for i in 0..=3 {
            let mut path = current_dir.clone();
            for _ in 0..i {
                path.pop();
            }
            let models_path = path.join(MODELS_DIR);
            if models_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(models_path) {
                    for entry in entries.filter_map(Result::ok) {
                        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                            if entry.file_name().to_string_lossy().starts_with("vosk-model-") {
                                candidates.push(entry.path());
                            }
                        }
                    }
                }
            }
        }
    }
    candidates
}

fn pick_best_candidate(mut candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.sort_by(|a, b| {
        let a_name = a.file_name().unwrap_or_default().to_string_lossy();
        let b_name = b.file_name().unwrap_or_default().to_string_lossy();
        // A simple heuristic: prefer smaller models, then english, then sort alphabetically.
        let a_score = (a_name.contains("small"), a_name.contains("en-us"));
        let b_score = (b_name.contains("small"), b_name.contains("en-us"));
        b_score.cmp(&a_score).then_with(|| a_name.cmp(&b_name))
    });
    candidates.into_iter().next()
}

fn find_zip_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(current_dir) = env::current_dir() {
        for i in 0..=3 {
            let mut path = current_dir.clone();
            for _ in 0..i {
                path.pop();
            }
            // Check current directory and models directory for zip files
            for search_dir in [&path, &path.join(MODELS_DIR)] {
                if search_dir.is_dir() {
                    if let Ok(entries) = std::fs::read_dir(search_dir) {
                        for entry in entries.filter_map(Result::ok) {
                            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                                let file_name = entry.file_name();
                                let file_name_str = file_name.to_string_lossy();
                                if file_name_str.starts_with("vosk-model-") && file_name_str.ends_with(".zip") {
                                    candidates.push(entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    candidates
}

pub fn ensure_model_available(auto_extract: bool) -> Result<Option<ModelInfo>, ModelError> {
    // If a model already exists, we're good.
    if let Ok(info) = locate_model(None) {
        return Ok(Some(info));
    }

    if !auto_extract {
        return Ok(None);
    }

    let zip_candidates = find_zip_candidates();
    if let Some(best_zip) = pick_best_candidate(zip_candidates) {
        return extract_model(&best_zip).map(Some);
    }

    Ok(None)
}

fn extract_model(zip_path: &std::path::Path) -> Result<ModelInfo, ModelError> {
    let models_dir = std::path::Path::new(MODELS_DIR);
    std::fs::create_dir_all(models_dir).map_err(|e| ModelError::ExtractionFailed(e.to_string()))?;

    let lock_path = models_dir.join(".extract.lock");
    if lock_path.exists() {
        return Err(ModelError::ExtractionFailed("Extraction lock file exists".to_string()));
    }
    std::fs::File::create(&lock_path).map_err(|e| ModelError::ExtractionFailed(e.to_string()))?;

    let temp_dir = models_dir.join(format!(".tmp-{}", Uuid::new_v4()));
    let extraction_result = (|| -> Result<ModelInfo, ModelError> {
        std::fs::create_dir_all(&temp_dir)?;
        let file = std::fs::File::open(&zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = temp_dir.join(file.enclosed_name().ok_or_else(|| "Invalid file path in zip")?);

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }

        let extracted_dir = std::fs::read_dir(&temp_dir)?
            .filter_map(Result::ok)
            .find(|e| e.file_type().map_or(false, |ft| ft.is_dir()))
            .map(|e| e.path())
            .ok_or_else(|| "No directory found in zip")?;

        let final_path = models_dir.join(extracted_dir.file_name().unwrap());
        std::fs::rename(&extracted_dir, &final_path)?;

        Ok(ModelInfo { path: final_path, source: ModelSource::Extracted })
    })();

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::remove_file(&lock_path);

    extraction_result
}


impl From<io::Error> for ModelError {
    fn from(err: io::Error) -> Self {
        ModelError::ExtractionFailed(err.to_string())
    }
}

impl From<zip::result::ZipError> for ModelError {
    fn from(err: zip::result::ZipError) -> Self {
        ModelError::ExtractionFailed(err.to_string())
    }
}

impl From<&str> for ModelError {
    fn from(err: &str) -> Self {
        ModelError::ExtractionFailed(err.to_string())
    }
}

pub fn default_model_path() -> PathBuf {
    locate_model(None).map(|info| info.path).unwrap_or_else(|_| PathBuf::from("models/vosk-model-small-en-us-0.15"))
}


pub fn log_model_resolution(info: &ModelInfo) {
    let source_desc = match info.source {
        ModelSource::Env => "environment variable (VOSK_MODEL_PATH)",
        ModelSource::Config => "configuration",
        ModelSource::ModelsDir => "autodetected",
        ModelSource::RepoRoot => "autodetected",
        ModelSource::Extracted => "auto-extracted",
    };
    tracing::info!(
        "Vosk model resolved: path={} source={}",
        info.path.display(),
        source_desc
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_is_deterministic() {
        let p = default_model_path();
        assert!(p.to_string_lossy().contains("vosk-model"));
    }
}
