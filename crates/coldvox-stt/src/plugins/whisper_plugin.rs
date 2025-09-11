//! OpenAI Whisper STT plugin implementation
//!
//! This is a stub implementation for the Whisper speech recognition engine.
//! Future work will integrate with whisper.cpp or whisper-rs for actual transcription.

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent, WordInfo};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Whisper-based STT plugin
#[derive(Debug)]
pub struct WhisperPlugin {
    model_path: Option<PathBuf>,
    model_size: WhisperModelSize,
    initialized: bool,
    language: Option<String>,
    _temperature: f32,
    _beam_size: usize,
    _best_of: usize,
}

/// Available Whisper model sizes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WhisperModelSize {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
    LargeV2,
    LargeV3,
}

impl WhisperModelSize {
    fn memory_usage_mb(&self) -> u32 {
        match self {
            Self::Tiny => 100,
            Self::Base => 200,
            Self::Small => 500,
            Self::Medium => 1500,
            Self::Large | Self::LargeV2 | Self::LargeV3 => 3000,
        }
    }

    fn model_name(&self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Base => "base",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::LargeV2 => "large-v2",
            Self::LargeV3 => "large-v3",
        }
    }
}

impl Default for WhisperModelSize {
    fn default() -> Self {
        Self::Base
    }
}

impl WhisperPlugin {
    pub fn new() -> Self {
        Self {
            model_path: None,
            model_size: WhisperModelSize::default(),
            initialized: false,
            language: None,
            _temperature: 0.0,
            _beam_size: 5,
            _best_of: 1,
        }
    }

    pub fn with_model_size(mut self, size: WhisperModelSize) -> Self {
        self.model_size = size;
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }
}

impl Default for WhisperPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SttPlugin for WhisperPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "whisper".to_string(),
            name: "OpenAI Whisper".to_string(),
            description: "Local speech recognition using OpenAI Whisper models".to_string(),
            requires_network: false, // Local inference
            is_local: true,
            is_available: check_whisper_available(),
            supported_languages: vec![
                // Whisper supports 99 languages, listing major ones
                "en".to_string(),
                "zh".to_string(),
                "de".to_string(),
                "es".to_string(),
                "ru".to_string(),
                "ko".to_string(),
                "fr".to_string(),
                "ja".to_string(),
                "pt".to_string(),
                "tr".to_string(),
                "pl".to_string(),
                "ca".to_string(),
                "nl".to_string(),
                "ar".to_string(),
                "sv".to_string(),
                "it".to_string(),
                "id".to_string(),
                "hi".to_string(),
                "fi".to_string(),
                "vi".to_string(),
                "he".to_string(),
                "uk".to_string(),
                "el".to_string(),
                "ms".to_string(),
                "cs".to_string(),
                "ro".to_string(),
                "da".to_string(),
                "hu".to_string(),
                "ta".to_string(),
                "no".to_string(),
                "th".to_string(),
                "ur".to_string(),
                "hr".to_string(),
                "bg".to_string(),
                "lt".to_string(),
                "la".to_string(),
                "mi".to_string(),
                "ml".to_string(),
                "cy".to_string(),
                "sk".to_string(),
                "te".to_string(),
                "fa".to_string(),
                "lv".to_string(),
                "bn".to_string(),
                "sr".to_string(),
                "az".to_string(),
                "sl".to_string(),
                "kn".to_string(),
                "et".to_string(),
                "mk".to_string(),
                "br".to_string(),
                "eu".to_string(),
                "is".to_string(),
                "hy".to_string(),
                "ne".to_string(),
                "mn".to_string(),
                "bs".to_string(),
                "kk".to_string(),
                "sq".to_string(),
                "sw".to_string(),
                "gl".to_string(),
                "mr".to_string(),
                "pa".to_string(),
                "si".to_string(),
                "km".to_string(),
                "sn".to_string(),
                "yo".to_string(),
                "so".to_string(),
                "af".to_string(),
                "oc".to_string(),
                "ka".to_string(),
                "be".to_string(),
                "tg".to_string(),
                "sd".to_string(),
                "gu".to_string(),
                "am".to_string(),
                "yi".to_string(),
                "lo".to_string(),
                "uz".to_string(),
                "fo".to_string(),
                "ht".to_string(),
                "ps".to_string(),
                "tk".to_string(),
                "nn".to_string(),
                "mt".to_string(),
                "sa".to_string(),
                "lb".to_string(),
                "my".to_string(),
                "bo".to_string(),
                "tl".to_string(),
                "mg".to_string(),
                "as".to_string(),
                "tt".to_string(),
                "haw".to_string(),
                "ln".to_string(),
                "ha".to_string(),
                "ba".to_string(),
                "jw".to_string(),
                "su".to_string(),
            ],
            memory_usage_mb: Some(self.model_size.memory_usage_mb()),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false, // Whisper processes complete audio segments
            batch: true,
            word_timestamps: true,
            confidence_scores: true,
            speaker_diarization: false, // Not natively supported
            auto_punctuation: true,
            custom_vocabulary: false, // Limited support via prompt
        }
    }

    async fn is_available(&self) -> Result<bool, SttPluginError> {
        if !check_whisper_available() {
            return Ok(false);
        }

        // Check if model exists
        if let Some(ref path) = self.model_path {
            if !path.exists() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn initialize(&mut self, _config: TranscriptionConfig) -> Result<(), SttPluginError> {
        // In a real implementation, this would:
        // 1. Load the Whisper model (ONNX or GGML format)
        // 2. Initialize the inference runtime
        // 3. Set up audio processing pipeline

        if self.model_path.is_none() {
            // Try to find a default model
            let default_path = find_default_whisper_model(self.model_size);
            if let Some(path) = default_path {
                self.model_path = Some(path);
            } else {
                return Err(SttPluginError::ModelLoadFailed(format!(
                    "No Whisper {} model found",
                    self.model_size.model_name()
                )));
            }
        }

        // Validate model path
        if let Some(ref path) = self.model_path {
            if !path.exists() {
                return Err(SttPluginError::ModelLoadFailed(format!(
                    "Model not found at {:?}",
                    path
                )));
            }
        }

        self.initialized = true;
        tracing::info!(
            "Whisper plugin initialized with {} model",
            self.model_size.model_name()
        );

        Ok(())
    }

    async fn process_audio(
        &mut self,
        _samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        if !self.initialized {
            return Err(SttPluginError::InitializationFailed(
                "Plugin not initialized".to_string(),
            ));
        }

        // Stub implementation - in reality would:
        // 1. Accumulate audio samples into appropriate segment length (typically 30s max)
        // 2. Convert i16 samples to f32 for Whisper
        // 3. Run inference
        // 4. Return transcription with timestamps

        // For now, return nothing (would be actual Whisper results)
        Ok(None)
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        if !self.initialized {
            return Ok(None);
        }

        // In a real implementation, this would:
        // 1. Process any remaining audio in the buffer
        // 2. Run final inference
        // 3. Return the complete transcription

        // Stub: return a mock final transcription for testing
        Ok(Some(TranscriptionEvent::Final {
            utterance_id: 1,
            text: "[Whisper stub: transcription would appear here]".to_string(),
            words: Some(vec![
                WordInfo {
                    text: "[Whisper".to_string(),
                    start: 0.0,
                    end: 0.5,
                    conf: 0.95,
                },
                WordInfo {
                    text: "stub:]".to_string(),
                    start: 0.5,
                    end: 1.0,
                    conf: 0.95,
                },
            ]),
        }))
    }

    async fn reset(&mut self) -> Result<(), SttPluginError> {
        // Reset internal state for new transcription session
        // In a real implementation, would clear audio buffers and reset model state
        Ok(())
    }

    async fn load_model(&mut self, model_path: Option<&Path>) -> Result<(), SttPluginError> {
        if let Some(path) = model_path {
            if !path.exists() {
                return Err(SttPluginError::ModelLoadFailed(format!(
                    "Model not found at {:?}",
                    path
                )));
            }

            // Determine model size from filename if possible
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.contains("tiny") {
                    self.model_size = WhisperModelSize::Tiny;
                } else if file_name.contains("base") {
                    self.model_size = WhisperModelSize::Base;
                } else if file_name.contains("small") {
                    self.model_size = WhisperModelSize::Small;
                } else if file_name.contains("medium") {
                    self.model_size = WhisperModelSize::Medium;
                } else if file_name.contains("large-v3") {
                    self.model_size = WhisperModelSize::LargeV3;
                } else if file_name.contains("large-v2") {
                    self.model_size = WhisperModelSize::LargeV2;
                } else if file_name.contains("large") {
                    self.model_size = WhisperModelSize::Large;
                }
            }

            self.model_path = Some(path.to_path_buf());
        }

        // In a real implementation, would actually load the model here
        Ok(())
    }
}

fn check_whisper_available() -> bool {
    // Check if whisper runtime is available
    // This would check for:
    // 1. whisper.cpp library (libwhisper.so/dylib/dll)
    // 2. OR whisper-rs Rust bindings
    // 3. OR ONNX runtime for ONNX models

    // For now, return false since this is a stub
    // In production, would check for actual whisper dependencies
    false
}

fn find_default_whisper_model(size: WhisperModelSize) -> Option<PathBuf> {
    // Look for Whisper models in standard locations
    let model_name = format!("ggml-{}.bin", size.model_name());

    // Check common model directories
    let search_paths = vec![
        PathBuf::from("models/whisper"),
        PathBuf::from("models"),
        PathBuf::from("/usr/share/whisper/models"),
        PathBuf::from("/usr/local/share/whisper/models"),
        dirs::home_dir()
            .map(|h| h.join(".whisper/models"))
            .unwrap_or_default(),
        dirs::home_dir()
            .map(|h| h.join(".cache/whisper"))
            .unwrap_or_default(),
    ];

    for base_path in search_paths {
        let model_path = base_path.join(&model_name);
        if model_path.exists() {
            return Some(model_path);
        }

        // Also check for ONNX models
        let onnx_name = format!("{}.onnx", size.model_name());
        let onnx_path = base_path.join(&onnx_name);
        if onnx_path.exists() {
            return Some(onnx_path);
        }
    }

    // Check environment variable
    if let Ok(whisper_model_path) = std::env::var("WHISPER_MODEL_PATH") {
        let path = PathBuf::from(whisper_model_path);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Factory for creating WhisperPlugin instances
pub struct WhisperPluginFactory {
    model_path: Option<PathBuf>,
    model_size: WhisperModelSize,
    language: Option<String>,
}

impl WhisperPluginFactory {
    pub fn new() -> Self {
        Self {
            model_path: std::env::var("WHISPER_MODEL_PATH").ok().map(PathBuf::from),
            model_size: WhisperModelSize::Base,
            language: None,
        }
    }

    pub fn with_model_size(mut self, size: WhisperModelSize) -> Self {
        self.model_size = size;
        self
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }
}

impl Default for WhisperPluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for WhisperPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        let mut plugin = WhisperPlugin::new().with_model_size(self.model_size);

        if let Some(ref path) = self.model_path {
            plugin = plugin.with_model_path(path.clone());
        }

        if let Some(ref lang) = self.language {
            plugin = plugin.with_language(lang.clone());
        }

        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        WhisperPlugin::new().with_model_size(self.model_size).info()
    }

    fn check_requirements(&self) -> Result<(), SttPluginError> {
        if !check_whisper_available() {
            return Err(SttPluginError::NotAvailable {
                reason: "Whisper runtime not found on system. Install whisper.cpp or ensure ONNX runtime is available.".to_string(),
            });
        }

        // If a specific model path is configured, verify it exists
        if let Some(ref path) = self.model_path {
            if !path.exists() {
                return Err(SttPluginError::NotAvailable {
                    reason: format!("Model not found at {:?}", path),
                });
            }
        } else {
            // Try to find a default model
            if find_default_whisper_model(self.model_size).is_none() {
                return Err(SttPluginError::NotAvailable {
                    reason: format!(
                        "No Whisper {} model found. Download from https://huggingface.co/ggerganov/whisper.cpp",
                        self.model_size.model_name()
                    ),
                });
            }
        }

        Ok(())
    }
}

// Helper to get home directory for model search
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_size_memory() {
        assert_eq!(WhisperModelSize::Tiny.memory_usage_mb(), 100);
        assert_eq!(WhisperModelSize::Base.memory_usage_mb(), 200);
        assert_eq!(WhisperModelSize::Small.memory_usage_mb(), 500);
        assert_eq!(WhisperModelSize::Medium.memory_usage_mb(), 1500);
        assert_eq!(WhisperModelSize::Large.memory_usage_mb(), 3000);
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = WhisperPlugin::new();
        let info = plugin.info();
        assert_eq!(info.id, "whisper");
        assert!(!info.requires_network);
        assert!(info.is_local);
    }

    #[test]
    fn test_factory_creation() {
        let factory = WhisperPluginFactory::new();
        let info = factory.plugin_info();
        assert_eq!(info.id, "whisper");

        // Creating should work even if requirements aren't met (stub)
        let result = factory.create();
        assert!(result.is_ok());
    }
}
