use std::env;
use std::fmt;
use std::io;
use std::path::PathBuf;

use uuid::Uuid;
use zip::ZipArchive;

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
    RepoRoot, // kept for compatibility; same as ModelsDir with ancestor scanning
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
                write!(f, "Failed to auto-extract model: {}", msg)
            }
        }
    }
}

impl std::error::Error for ModelError {}

/// Locate the model directory.
pub fn locate_model(config_path: Option<&str>) -> Result<ModelInfo, ModelError> {
    // 1. Try environment variable first
    // HOW YOU GET HERE: User/CI explicitly set VOSK_MODEL_PATH environment variable
    if let Ok(p) = env::var("VOSK_MODEL_PATH") {
        tracing::debug!(
            target: "coldvox::stt::vosk",
            env_path = %p,
            "Trying VOSK_MODEL_PATH environment variable - REASON: Env var is set and takes highest priority"
        );
        let pb = PathBuf::from(&p);
        if pb.is_dir() {
            // SUCCESS CASE: The env var points to a valid directory
            // Most likely: CI runner set VOSK_MODEL_PATH correctly to cached model location
            tracing::info!(
                target: "coldvox::stt::vosk",
                path = %pb.display(),
                source = "environment",
                "Vosk model found via VOSK_MODEL_PATH - SUCCESS: Path exists and is a valid directory"
            );
            return Ok(ModelInfo {
                path: pb,
                source: ModelSource::Env,
            });
        } else {
            // FAILURE CASE: Env var is set but points to invalid/missing location
            // Most likely: CI script typo, model not extracted, or filesystem permission issue
            tracing::warn!(
                target: "coldvox::stt::vosk",
                env_path = %p,
                exists = pb.exists(),
                is_dir = pb.is_dir(),
                "VOSK_MODEL_PATH points to invalid location - REASON: Either file doesn't exist, is a file not directory, or unreadable"
            );
            return Err(ModelError::ExplicitPathMissing(p));
        }
    }

    // 2. Try config-provided path
    // HOW YOU GET HERE: VOSK_MODEL_PATH was NOT set, AND a non-empty model_path was provided in TranscriptionConfig
    if let Some(cp) = config_path.filter(|s| !s.is_empty()) {
        tracing::debug!(
            target: "coldvox::stt::vosk",
            config_path = %cp,
            "Trying config-provided model path - REASON: No env var, but config.model_path is set"
        );
        let pb = PathBuf::from(cp);
        if pb.is_dir() {
            // SUCCESS CASE: Config path points to valid directory
            // Most likely: User provided explicit --model-path CLI arg or set it in config file
            tracing::info!(
                target: "coldvox::stt::vosk",
                path = %pb.display(),
                source = "config",
                "Vosk model found via config path - SUCCESS: Config-provided path is valid directory"
            );
            return Ok(ModelInfo {
                path: pb,
                source: ModelSource::Config,
            });
        } else {
            // FAILURE CASE: Config path set but invalid
            // Most likely: User provided wrong path in config/CLI, or model was deleted
            tracing::warn!(
                target: "coldvox::stt::vosk",
                config_path = %cp,
                exists = pb.exists(),
                is_dir = pb.is_dir(),
                "Config-provided model path is invalid - REASON: Path in config is wrong, doesn't exist, or not a directory"
            );
            return Err(ModelError::ExplicitPathMissing(cp.to_string()));
        }
    }

    // 3. Try auto-discovery
    // HOW YOU GET HERE: No VOSK_MODEL_PATH env var AND no config.model_path provided (or both failed)
    // This is the NORMAL path for most users who just extract model to models/ directory
    tracing::debug!(
        target: "coldvox::stt::vosk",
        "Starting auto-discovery for Vosk model - REASON: No explicit path provided, searching standard locations"
    );
    let candidates = find_model_candidates();
    if !candidates.is_empty() {
        // PARTIAL SUCCESS: Found one or more vosk-model-* directories
        // Most likely: User extracted model to models/, or working from dev environment
        tracing::debug!(
            target: "coldvox::stt::vosk",
            count = candidates.len(),
            candidates = ?candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "Vosk model discovery candidates found - REASON: Found vosk-model-* directories in scanned paths"
        );
    } else {
        // FAILURE: No models found anywhere
        // Most likely: Fresh clone/install, model never extracted, or running from wrong directory
        tracing::debug!(
            target: "coldvox::stt::vosk",
            "No model candidates found during auto-discovery - REASON: No vosk-model-* directories exist in any scanned location"
        );
    }

    if let Some(best_candidate) = pick_best_candidate(candidates) {
        // SUCCESS CASE: Found and selected best model from candidates
        // Most likely: Normal operation, model in models/ directory, picked best (small en-us with highest version)
        tracing::info!(
            target: "coldvox::stt::vosk",
            path = %best_candidate.display(),
            source = "auto-discovery",
            "Vosk model found via auto-discovery - SUCCESS: Selected best candidate from discovered models"
        );
        return Ok(ModelInfo {
            path: best_candidate,
            source: ModelSource::ModelsDir,
        });
    }

    // 4. Build detailed error message with what was tried
    // HOW YOU GET HERE: ALL previous attempts failed - this is the FINAL FAILURE path
    // Most likely scenarios:
    //   - CI: Model setup script didn't run or failed silently
    //   - Local dev: User forgot to extract model or is in wrong directory
    //   - Production: Model directory was deleted or permissions changed
    let mut checked_paths = Vec::new();

    // Add environment variable attempt (if it was tried)
    if let Ok(env_path) = env::var("VOSK_MODEL_PATH") {
        checked_paths.push(format!("VOSK_MODEL_PATH={}", env_path));
    }

    // Add config path attempt (if it was provided)
    if let Some(cp) = config_path.filter(|s| !s.is_empty()) {
        checked_paths.push(format!("config_path={}", cp));
    }

    // Add auto-discovery attempts (always happens if we got here)
    if let Ok(cwd) = env::current_dir() {
        for i in 0..=3 {
            let mut path = cwd.clone();
            for _ in 0..i {
                path.pop();
            }
            let models_path = path.join(MODELS_DIR);
            checked_paths.push(format!("auto_discovery={}", models_path.display()));
        }
    }

    let checked_str = checked_paths.join(", ");
    tracing::error!(
        target: "coldvox::stt::vosk",
        checked_paths = %checked_str,
        env_var_set = env::var("VOSK_MODEL_PATH").is_ok(),
        config_path_provided = config_path.is_some(),
        cwd = ?env::current_dir().ok(),
        "Vosk model not found after exhaustive search - COMPLETE FAILURE: Model unavailable via any method"
    );

    // This error will propagate up through plugin initialization and cause STT to be unavailable
    Err(ModelError::NotFound {
        checked: checked_str,
        guidance: "Set VOSK_MODEL_PATH environment variable, provide model_path in config, or place model in models/ directory. Run with --auto-extract-model to extract from ZIP files.".to_string(),
    })
}

fn find_model_candidates() -> Vec<PathBuf> {
    // Called during auto-discovery when no explicit path was provided
    let mut candidates = Vec::new();
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            // RARE: Only fails if process has no working directory (very unusual)
            // Most likely: Running in restricted container, deleted directory, or permission issue
            tracing::warn!(
                target: "coldvox::stt::vosk",
                error = %e,
                "Failed to get current directory for model discovery - REASON: Process CWD is inaccessible or deleted"
            );
            return candidates;
        }
    };

    tracing::debug!(
        target: "coldvox::stt::vosk",
        cwd = %cwd.display(),
        "Starting model discovery from current directory - REASON: Scanning up to 3 ancestor levels for models/ directories"
    );

    for i in 0..=3 {
        let mut path = cwd.clone();
        for _ in 0..i {
            path.pop();
        }
        let models_path = path.join(MODELS_DIR);
        let models_path_str = models_path.display().to_string();

        // Check each ancestor level (0=cwd, 1=parent, 2=grandparent, 3=great-grandparent)
        tracing::trace!(
            target: "coldvox::stt::vosk",
            search_path = %models_path_str,
            ancestor_level = i,
            exists = models_path.exists(),
            is_dir = models_path.is_dir(),
            "Checking models directory - SCANNING: Looking for vosk-model-* subdirectories"
        );

        if models_path.is_dir() {
            // HOW YOU GET HERE: Found a models/ directory at this ancestor level
            // Most likely: In dev environment with models/ in project root
            match std::fs::read_dir(models_path) {
                Ok(entries) => {
                    let mut found_candidates = 0;
                    for entry in entries.filter_map(Result::ok) {
                        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                            continue;
                        }

                        let file_name = entry.file_name();
                        let file_name_str = file_name.to_string_lossy();

                        if file_name_str.starts_with("vosk-model-") {
                            // FOUND: A directory matching vosk-model-* pattern
                            // Most likely: User extracted model correctly, or CI setup worked
                            let candidate_path = entry.path();
                            tracing::debug!(
                                target: "coldvox::stt::vosk",
                                candidate = %candidate_path.display(),
                                ancestor_level = i,
                                "Found potential Vosk model directory - REASON: Directory name matches vosk-model-* pattern"
                            );
                            candidates.push(candidate_path);
                            found_candidates += 1;
                        }
                    }
                    tracing::debug!(
                        target: "coldvox::stt::vosk",
                        search_path = %models_path_str,
                        candidates_found = found_candidates,
                        "Completed scanning models directory - RESULT: Listed all vosk-model-* directories found"
                    );
                }
                Err(e) => {
                    // RARE: Directory exists but can't be read
                    // Most likely: Permission denied, or filesystem error
                    tracing::warn!(
                        target: "coldvox::stt::vosk",
                        search_path = %models_path_str,
                        error = %e,
                        "Failed to read models directory - REASON: Permission denied or filesystem error"
                    );
                }
            }
        }
        // Note: If models_path.is_dir() is false, we silently skip (no directory at this level)
    }

    tracing::debug!(
        target: "coldvox::stt::vosk",
        total_candidates = candidates.len(),
        "Model discovery completed"
    );

    candidates
}

fn pick_best_candidate(mut candidates: Vec<PathBuf>) -> Option<PathBuf> {
    fn extract_trailing_version_nums(name: &str) -> Vec<u32> {
        // Grab trailing run of digits/dots, e.g., "0.22" from "vosk-model-small-en-us-0.22"
        let mut end = name.len();
        for (idx, ch) in name.char_indices().rev() {
            if ch.is_ascii_digit() || ch == '.' {
                end = end.min(idx + ch.len_utf8());
                continue;
            }
            if end < name.len() {
                let start = idx + ch.len_utf8();
                let slice = &name[start..end];
                return slice
                    .split('.')
                    .filter_map(|part| part.parse::<u32>().ok())
                    .collect();
            }
        }
        let slice = &name[..end];
        slice
            .split('.')
            .filter_map(|part| part.parse::<u32>().ok())
            .collect()
    }

    candidates.sort_by(|a, b| {
        let a_name = a
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        let b_name = b
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        let a_small = a_name.contains("small");
        let b_small = b_name.contains("small");
        let a_en = a_name.contains("en-us");
        let b_en = b_name.contains("en-us");
        let a_ver = extract_trailing_version_nums(&a_name);
        let b_ver = extract_trailing_version_nums(&b_name);

        // Order: small (true first), en-us (true first), version (descending), name (asc)
        b_small
            .cmp(&a_small)
            .then_with(|| b_en.cmp(&a_en))
            .then_with(|| b_ver.cmp(&a_ver))
            .then_with(|| a_name.cmp(&b_name))
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
                            if entry.file_type().is_ok_and(|ft| ft.is_file()) {
                                let file_name = entry.file_name();
                                let file_name_str = file_name.to_string_lossy();
                                if file_name_str.starts_with("vosk-model-")
                                    && file_name_str.ends_with(".zip")
                                {
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
    tracing::debug!(
        target: "coldvox::stt::vosk",
        auto_extract_enabled = auto_extract,
        "Checking if Vosk model is available"
    );

    // If a model already exists, we're good.
    if let Ok(info) = locate_model(None) {
        tracing::debug!(
            target: "coldvox::stt::vosk",
            model_path = %info.path.display(),
            "Existing model found, no extraction needed"
        );
        return Ok(Some(info));
    }

    if !auto_extract {
        tracing::debug!(
            target: "coldvox::stt::vosk",
            "Auto-extraction disabled and no existing model found"
        );
        return Ok(None);
    }

    tracing::info!(
        target: "coldvox::stt::vosk",
        "No existing model found, attempting auto-extraction"
    );

    let zip_candidates = find_zip_candidates();
    if !zip_candidates.is_empty() {
        tracing::debug!(
            target: "coldvox::stt::vosk",
            count = zip_candidates.len(),
            zips = ?zip_candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "Vosk zip candidates found for auto-extract"
        );
    } else {
        tracing::debug!(
            target: "coldvox::stt::vosk",
            "No zip files found for auto-extraction"
        );
    }

    if let Some(best_zip) = pick_best_candidate(zip_candidates) {
        tracing::info!(
            target: "coldvox::stt::vosk",
            zip_path = %best_zip.display(),
            "Attempting to extract model from zip"
        );
        match extract_model(&best_zip) {
            Ok(info) => {
                tracing::info!(
                    target: "coldvox::stt::vosk",
                    extracted_path = %info.path.display(),
                    "Successfully extracted model from zip"
                );
                Ok(Some(info))
            }
            Err(e) => {
                tracing::error!(
                    target: "coldvox::stt::vosk",
                    zip_path = %best_zip.display(),
                    error = %e,
                    "Failed to extract model from zip"
                );
                Err(e)
            }
        }
    } else {
        tracing::warn!(
            target: "coldvox::stt::vosk",
            "No suitable zip file found for auto-extraction"
        );
        Ok(None)
    }
}

fn extract_model(zip_path: &std::path::Path) -> Result<ModelInfo, ModelError> {
    let models_dir = std::path::Path::new(MODELS_DIR);
    std::fs::create_dir_all(models_dir).map_err(|e| ModelError::ExtractionFailed(e.to_string()))?;

    let lock_path = models_dir.join(".extract.lock");
    if lock_path.exists() {
        // Check if lock file is stale (older than 30 minutes)
        if let Ok(metadata) = std::fs::metadata(&lock_path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = std::time::SystemTime::now().duration_since(modified) {
                    if duration.as_secs() > 1800 {
                        tracing::warn!(
                            "Removing stale extraction lock file (age: {} seconds)",
                            duration.as_secs()
                        );
                        let _ = std::fs::remove_file(&lock_path);
                    } else {
                        return Err(ModelError::ExtractionFailed(format!(
                            "Extraction in progress (lock age: {} seconds)",
                            duration.as_secs()
                        )));
                    }
                }
            }
        }
        // If we couldn't get lock file age, assume it's stale and remove it
        if lock_path.exists() {
            let _ = std::fs::remove_file(&lock_path);
        }
    }
    std::fs::File::create(&lock_path).map_err(|e| ModelError::ExtractionFailed(e.to_string()))?;

    let temp_dir = models_dir.join(format!(".tmp-{}", Uuid::new_v4()));
    let extraction_result = (|| -> Result<ModelInfo, ModelError> {
        std::fs::create_dir_all(&temp_dir)?;
        let file = std::fs::File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = temp_dir.join(file.enclosed_name().ok_or("Invalid file path in zip")?);

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }

        let extracted_dir = std::fs::read_dir(&temp_dir)?
            .filter_map(Result::ok)
            .find(|e| e.file_type().is_ok_and(|ft| ft.is_dir()))
            .map(|e| e.path())
            .ok_or("No directory found in zip")?;

        let final_path = models_dir.join(extracted_dir.file_name().unwrap());
        std::fs::rename(&extracted_dir, &final_path)?;

        Ok(ModelInfo {
            path: final_path,
            source: ModelSource::Extracted,
        })
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
    locate_model(None)
        .map(|info| info.path)
        .unwrap_or_else(|_| PathBuf::from("models/vosk-model-small-en-us-0.15"))
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
        target: "coldvox::stt::vosk",
        path = %info.path.display(),
        source = source_desc,
        canonical_path = ?info.path.canonicalize().ok(),
        exists = info.path.exists(),
        is_dir = info.path.is_dir(),
        "Vosk model resolved successfully"
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

    #[test]
    fn pick_best_prefers_small_en_and_version() {
        let base = std::env::temp_dir().join(format!("cvx-test-{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(base.join("models"));

        let dirs = vec![
            "vosk-model-en-us-0.15",
            "vosk-model-small-en-us-0.9",
            "vosk-model-small-en-us-0.22",
            "vosk-model-small-de-0.30",
        ];
        for d in &dirs {
            let _ = std::fs::create_dir_all(base.join("models").join(d));
        }

        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&base).unwrap();
        let found = super::find_model_candidates();
        let best = super::pick_best_candidate(found).expect("a best candidate");
        std::env::set_current_dir(cwd).unwrap();

        assert!(best
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("vosk-model-small-en-us-0.22"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn scan_includes_ancestors_up_to_three() {
        // Ensure environment does not short-circuit discovery
        let prev_env = std::env::var("VOSK_MODEL_PATH").ok();
        std::env::remove_var("VOSK_MODEL_PATH");

        let root = std::env::temp_dir().join(format!("cvx-scan-{}", Uuid::new_v4()));
        let a = root.join("a");
        let b = a.join("b");
        let c = b.join("c");
        let d = c.join("d"); // current dir
        for p in [&root, &a, &b, &c, &d] {
            let _ = std::fs::create_dir_all(p);
        }
        // Place model at a/models (exactly 3 ancestors from d: d->c->b->a)
        let model_dir = a.join("models").join("vosk-model-small-en-us-0.15");
        let _ = std::fs::create_dir_all(&model_dir);

        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&d).unwrap();
        let info = locate_model(None).expect("should find ancestor model");
        std::env::set_current_dir(cwd).unwrap();

        assert_eq!(info.path, model_dir);
        let _ = std::fs::remove_dir_all(root);

        // Restore environment
        if let Some(v) = prev_env {
            std::env::set_var("VOSK_MODEL_PATH", v);
        } else {
            std::env::remove_var("VOSK_MODEL_PATH");
        }
    }
}
