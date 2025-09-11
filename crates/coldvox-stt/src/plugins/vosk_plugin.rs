//! Vosk STT plugin implementation
//!
//! This wraps the existing VoskTranscriber as a plugin

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[cfg(feature = "vosk")]
use coldvox_stt_vosk::VoskTranscriber;

/// Vosk-based STT plugin
#[derive(Debug)]
pub struct VoskPlugin {
    model_path: Option<PathBuf>,
    #[cfg(feature = "vosk")]
    transcriber: Option<VoskTranscriber>,
    initialized: bool,
    config: Option<TranscriptionConfig>,
}

impl VoskPlugin {
    pub fn new() -> Self {
        Self {
            model_path: None,
            #[cfg(feature = "vosk")]
            transcriber: None,
            initialized: false,
            config: None,
        }
    }

    pub fn with_model_path(path: PathBuf) -> Self {
        Self {
            model_path: Some(path),
            #[cfg(feature = "vosk")]
            transcriber: None,
            initialized: false,
            config: None,
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

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), SttPluginError> {
        #[cfg(feature = "vosk")]
        {
            // Determine model path
            let model_path = if let Some(p) = &self.model_path {
                p.display().to_string()
            } else {
                config.model_path.clone()
            };

            // Validate model path exists
            if !std::path::Path::new(&model_path).exists() {
                return Err(SttPluginError::ModelNotFound { path: model_path });
            }

            // Create VoskTranscriber
            let transcriber_config = TranscriptionConfig {
                model_path,
                ..config.clone()
            };

            match VoskTranscriber::new(transcriber_config, 16000.0) {
                Ok(transcriber) => {
                    self.transcriber = Some(transcriber);
                    self.config = Some(config);
                    self.initialized = true;
                    tracing::info!("VoskPlugin initialized successfully");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to create VoskTranscriber: {}", e);
                    Err(SttPluginError::InitializationFailed(e))
                }
            }
        }
        
        #[cfg(not(feature = "vosk"))]
        {
            Err(SttPluginError::NotAvailable {
                reason: "Vosk feature not enabled".to_string(),
            })
        }
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        if !self.initialized {
            return Err(SttPluginError::InitializationFailed(
                "Plugin not initialized".to_string(),
            ));
        }

        #[cfg(feature = "vosk")]
        {
            if let Some(ref mut transcriber) = self.transcriber {
                // Use the EventBasedTranscriber interface
                match crate::EventBasedTranscriber::accept_frame(transcriber, samples) {
                    Ok(event) => Ok(event),
                    Err(e) => {
                        tracing::error!("Vosk transcriber error: {}", e);
                        Err(SttPluginError::TranscriptionFailed(e))
                    }
                }
            } else {
                Err(SttPluginError::InitializationFailed(
                    "Transcriber not initialized".to_string(),
                ))
            }
        }
        
        #[cfg(not(feature = "vosk"))]
        {
            Err(SttPluginError::NotAvailable {
                reason: "Vosk feature not enabled".to_string(),
            })
        }
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        #[cfg(feature = "vosk")]
        {
            if let Some(ref mut transcriber) = self.transcriber {
                match crate::EventBasedTranscriber::finalize_utterance(transcriber) {
                    Ok(event) => Ok(event),
                    Err(e) => {
                        tracing::error!("Vosk transcriber finalization error: {}", e);
                        Err(SttPluginError::TranscriptionFailed(e))
                    }
                }
            } else {
                Ok(None)
            }
        }
        
        #[cfg(not(feature = "vosk"))]
        Ok(None)
    }

    async fn reset(&mut self) -> Result<(), SttPluginError> {
        #[cfg(feature = "vosk")]
        {
            if let Some(ref mut transcriber) = self.transcriber {
                match crate::EventBasedTranscriber::reset(transcriber) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        tracing::warn!("Vosk transcriber reset error: {}", e);
                        // Don't fail on reset errors, just log
                        Ok(())
                    }
                }
            } else {
                Ok(())
            }
        }
        
        #[cfg(not(feature = "vosk"))]
        Ok(())
    }

    async fn load_model(&mut self, model_path: Option<&Path>) -> Result<(), SttPluginError> {
        if let Some(path) = model_path {
            if !path.exists() {
                return Err(SttPluginError::ModelNotFound {
                    path: path.display().to_string(),
                });
            }
            self.model_path = Some(path.to_path_buf());
        }

        // If we have a config, reinitialize with the new model
        if let Some(config) = self.config.clone() {
            self.initialized = false;
            self.initialize(config).await?;
        }

        Ok(())
    }

    async fn unload_model(&mut self) -> Result<(), SttPluginError> {
        #[cfg(feature = "vosk")]
        {
            self.transcriber = None;
        }
        self.initialized = false;
        tracing::info!("VoskPlugin model unloaded");
        Ok(())
    }

    fn memory_usage_bytes(&self) -> Option<u64> {
        // Estimate: typical small Vosk model is ~40MB, medium ~500MB
        if self.is_model_loaded() {
            Some(500 * 1024 * 1024) // 500MB estimate
        } else {
            Some(1024 * 1024) // 1MB for plugin overhead
        }
    }

    fn is_model_loaded(&self) -> bool {
        #[cfg(feature = "vosk")]
        {
            self.transcriber.is_some()
        }
        
        #[cfg(not(feature = "vosk"))]
        false
    }
}

fn check_vosk_available() -> bool {
    // Check if the vosk feature is enabled at compile time
    #[cfg(feature = "vosk")]
    {
        // Additional runtime checks could be added here
        // For now, assume it's available if compiled with the feature
        true
    }
    
    #[cfg(not(feature = "vosk"))]
    false
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
                    // Use default model path
                    let default = PathBuf::from("models/vosk-model-small-en-us-0.15");
                    if default.exists() {
                        Some(default)
                    } else {
                        None
                    }
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
                reason: "Vosk feature not enabled".to_string(),
            });
        }

        if let Some(ref path) = self.model_path {
            if !path.exists() {
                return Err(SttPluginError::ModelNotFound {
                    path: path.display().to_string(),
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
