//! Vosk STT plugin implementation
//!
//! This wraps the existing VoskTranscriber as a plugin

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

// This would import from coldvox-stt-vosk crate
// use coldvox_stt_vosk::VoskTranscriber;

/// Vosk-based STT plugin
#[derive(Debug)]
pub struct VoskPlugin {
    model_path: Option<PathBuf>,
    // transcriber: Option<VoskTranscriber>,  // Would be the actual Vosk instance
    initialized: bool,
}

impl VoskPlugin {
    pub fn new() -> Self {
        Self {
            model_path: None,
            initialized: false,
        }
    }

    pub fn with_model_path(path: PathBuf) -> Self {
        Self {
            model_path: Some(path),
            initialized: false,
        }
    }
}

impl Default for VoskPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SttPlugin for VoskPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "vosk".to_string(),
            name: "Vosk Speech Recognition".to_string(),
            description: "Offline speech recognition using Vosk models".to_string(),
            requires_network: false,
            is_local: true,
            is_available: check_vosk_available(),
            supported_languages: vec![
                "en".to_string(),
                "ru".to_string(),
                "de".to_string(),
                "es".to_string(),
                "fr".to_string(),
                "it".to_string(),
                "nl".to_string(),
                "pt".to_string(),
                "tr".to_string(),
                "cn".to_string(),
                "ja".to_string(),
                "hi".to_string(),
            ],
            memory_usage_mb: Some(500), // Typical Vosk model size
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: true,
            batch: true,
            word_timestamps: true,
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: false,
            custom_vocabulary: true,
        }
    }

    async fn is_available(&self) -> Result<bool, SttPluginError> {
        if !check_vosk_available() {
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
        // This would actually initialize VoskTranscriber
        // self.transcriber = Some(VoskTranscriber::new(config)?);

        self.initialized = true;
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

        // This would actually call the Vosk transcriber
        // if let Some(ref mut transcriber) = self.transcriber {
        //     return transcriber.process_audio(samples)
        //         .map_err(|e| SttPluginError::BackendError(Box::new(e)));
        // }

        // For now, return nothing (would be actual Vosk results)
        Ok(None)
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        // Would finalize Vosk transcription
        Ok(None)
    }

    async fn reset(&mut self) -> Result<(), SttPluginError> {
        // Would reset Vosk state
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
            self.model_path = Some(path.to_path_buf());
        }

        // Would actually load the Vosk model here
        Ok(())
    }
}

fn check_vosk_available() -> bool {
    // Check if libvosk is available on the system
    // This would use actual library detection logic

    #[cfg(target_os = "linux")]
    {
        // Check for libvosk.so
        std::path::Path::new("/usr/lib/libvosk.so").exists()
            || std::path::Path::new("/usr/local/lib/libvosk.so").exists()
    }

    #[cfg(target_os = "macos")]
    {
        // Check for libvosk.dylib
        std::path::Path::new("/usr/local/lib/libvosk.dylib").exists()
    }

    #[cfg(target_os = "windows")]
    {
        // Check for vosk.dll
        std::path::Path::new("C:\\Program Files\\Vosk\\vosk.dll").exists()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// Factory for creating VoskPlugin instances
pub struct VoskPluginFactory {
    model_path: Option<PathBuf>,
}

impl VoskPluginFactory {
    pub fn new() -> Self {
        Self {
            model_path: std::env::var("VOSK_MODEL_PATH")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    // Simple fallback model path resolution
                    let default_paths = [
                        PathBuf::from("models/vosk-model-small-en-us-0.15"),
                        PathBuf::from("vosk-model-small-en-us-0.15"),
                    ];

                    for path in &default_paths {
                        if path.exists() {
                            return Some(path.clone());
                        }
                    }

                    None
                }),
        }
    }

    pub fn with_model_path(path: PathBuf) -> Self {
        Self {
            model_path: Some(path),
        }
    }
}

impl Default for VoskPluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for VoskPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        let mut plugin = VoskPlugin::new();
        if let Some(ref path) = self.model_path {
            plugin.model_path = Some(path.clone());
        }
        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        VoskPlugin::new().info()
    }

    fn check_requirements(&self) -> Result<(), SttPluginError> {
        if !check_vosk_available() {
            return Err(SttPluginError::NotAvailable {
                reason: "libvosk not found on system".to_string(),
            });
        }

        if let Some(ref path) = self.model_path {
            if !path.exists() {
                return Err(SttPluginError::NotAvailable {
                    reason: format!("Model not found at {:?}", path),
                });
            }
        } else {
            return Err(SttPluginError::NotAvailable {
                reason: "No model path configured".to_string(),
            });
        }

        Ok(())
    }
}
