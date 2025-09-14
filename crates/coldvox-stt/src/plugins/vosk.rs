use std::path::PathBuf;

use crate::{
    plugin::{PluginCapabilities, PluginInfo, SttPluginFactory},
    SttPlugin, SttPluginError, TranscriptionConfig, TranscriptionEvent,
};

/// Vosk STT plugin - stub implementation
/// The actual Vosk implementation is in the coldvox-stt-vosk crate
/// to avoid circular dependencies.
#[derive(Debug)]
pub struct VoskPlugin {
    model_path: PathBuf,
}

impl VoskPlugin {
    pub fn new(model_path: PathBuf, _language: String) -> Self {
        Self { model_path }
    }
}

#[async_trait::async_trait]
impl SttPlugin for VoskPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "vosk".to_string(),
            name: "Vosk".to_string(),
            description: "Offline Vosk speech recognition".to_string(),
            requires_network: false,
            is_local: true,
            is_available: self.model_path.exists(),
            supported_languages: vec!["en".to_string()],
            memory_usage_mb: None,
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: true,
            batch: false,
            word_timestamps: true,
            confidence_scores: false,
            speaker_diarization: false,
            auto_punctuation: true,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, SttPluginError> {
        Ok(self.model_path.exists())
    }

    async fn initialize(&mut self, _config: TranscriptionConfig) -> Result<(), SttPluginError> {
        // Stub implementation - actual Vosk is in coldvox-stt-vosk crate
        Err(SttPluginError::NotAvailable {
            reason: "Vosk plugin is implemented in coldvox-stt-vosk crate".to_string(),
        })
    }

    async fn process_audio(
        &mut self,
        _samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        Err(SttPluginError::NotAvailable {
            reason: "Vosk plugin is implemented in coldvox-stt-vosk crate".to_string(),
        })
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        Err(SttPluginError::NotAvailable {
            reason: "Vosk plugin is implemented in coldvox-stt-vosk crate".to_string(),
        })
    }

    async fn reset(&mut self) -> Result<(), SttPluginError> {
        Err(SttPluginError::NotAvailable {
            reason: "Vosk plugin is implemented in coldvox-stt-vosk crate".to_string(),
        })
    }

    async fn load_model(
        &mut self,
        _model_path: Option<&std::path::Path>,
    ) -> Result<(), SttPluginError> {
        Err(SttPluginError::NotAvailable {
            reason: "Vosk plugin is implemented in coldvox-stt-vosk crate".to_string(),
        })
    }

    async fn unload(&mut self) -> Result<(), SttPluginError> {
        Err(SttPluginError::NotAvailable {
            reason: "Vosk plugin is implemented in coldvox-stt-vosk crate".to_string(),
        })
    }
}

/// Factory for creating Vosk plugins
pub struct VoskPluginFactory;

impl VoskPluginFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VoskPluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for VoskPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        let model_path = std::env::var("VOSK_MODEL_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("models/vosk-model-small-en-us-0.15"));
        let language = std::env::var("VOSK_LANGUAGE").unwrap_or_else(|_| "en".to_string());
        Ok(Box::new(VoskPlugin::new(model_path, language)))
    }

    fn plugin_info(&self) -> PluginInfo {
        VoskPlugin::new(
            PathBuf::from("models/vosk-model-small-en-us-0.15"),
            "en".into(),
        )
        .info()
    }

    fn check_requirements(&self) -> Result<(), SttPluginError> {
        let path = std::env::var("VOSK_MODEL_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("models/vosk-model-small-en-us-0.15"));
        if path.exists() {
            Err(SttPluginError::NotAvailable {
                reason: "Vosk plugin requires coldvox-stt-vosk crate".to_string(),
            })
        } else {
            Err(SttPluginError::NotAvailable {
                reason: format!(
                    "Vosk model missing at {} and requires coldvox-stt-vosk crate",
                    path.display()
                ),
            })
        }
    }
}
