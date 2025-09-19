use crate::model;
use crate::vosk_transcriber::VoskTranscriber;
use async_trait::async_trait;
use coldvox_stt::plugin::{
    PluginCapabilities, PluginInfo, SttPlugin, SttPluginError, SttPluginFactory,
};
use coldvox_stt::{EventBasedTranscriber, TranscriptionConfig, TranscriptionEvent};
use std::fmt;
use std::path::{Path, PathBuf};

pub struct VoskPlugin {
    transcriber: Option<VoskTranscriber>,
    config: TranscriptionConfig,
    sample_rate: f32,
    model_path: PathBuf,
}

impl fmt::Debug for VoskPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoskPlugin")
            .field(
                "transcriber",
                &self.transcriber.as_ref().map(|_| "Some(VoskTranscriber)"),
            )
            .field("config", &self.config)
            .field("sample_rate", &self.sample_rate)
            .field("model_path", &self.model_path)
            .finish()
    }
}

impl Default for VoskPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VoskPlugin {
    pub fn new() -> Self {
        let model_info = model::locate_model(None).ok();
        let model_path = model_info.map_or_else(model::default_model_path, |info| info.path);

        Self {
            transcriber: None,
            config: TranscriptionConfig::default(),
            sample_rate: 16000.0, // Vosk preferred sample rate
            model_path,
        }
    }
}

#[async_trait]
impl SttPlugin for VoskPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "vosk".to_string(),
            name: "Vosk".to_string(),
            description: "Offline Vosk speech recognition".to_string(),
            requires_network: false,
            is_local: true,
            is_available: self.model_path.exists(),
            supported_languages: vec!["en-us".to_string()], // example, can be improved
            memory_usage_mb: None,                          // Could be estimated
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: true,
            batch: true, // VoskTranscriber has finalize which can be used for batch
            word_timestamps: true,
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: true,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, SttPluginError> {
        Ok(self.model_path.exists())
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), SttPluginError> {
        self.config = config;
        let transcriber = VoskTranscriber::new(self.config.clone(), self.sample_rate)
            .map_err(SttPluginError::InitializationFailed)?;
        self.transcriber = Some(transcriber);
        Ok(())
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        if let Some(ref mut transcriber) = self.transcriber {
            transcriber
                .accept_frame(samples)
                .map_err(SttPluginError::ProcessingError)
        } else {
            Err(SttPluginError::NotAvailable {
                reason: "Plugin not initialized".to_string(),
            })
        }
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        if let Some(ref mut transcriber) = self.transcriber {
            transcriber
                .finalize_utterance()
                .map_err(SttPluginError::ProcessingError)
        } else {
            Err(SttPluginError::NotAvailable {
                reason: "Plugin not initialized".to_string(),
            })
        }
    }

    async fn reset(&mut self) -> Result<(), SttPluginError> {
        if let Some(ref mut transcriber) = self.transcriber {
            transcriber.reset().map_err(SttPluginError::ProcessingError)
        } else {
            Err(SttPluginError::NotAvailable {
                reason: "Plugin not initialized".to_string(),
            })
        }
    }

    async fn load_model(&mut self, model_path: Option<&Path>) -> Result<(), SttPluginError> {
        let path_to_load = model_path.map_or_else(|| self.model_path.clone(), |p| p.to_path_buf());

        let mut config = self.config.clone();
        config.model_path = path_to_load.to_string_lossy().into_owned();

        let transcriber = VoskTranscriber::new(config, self.sample_rate)
            .map_err(SttPluginError::ModelLoadFailed)?;

        self.transcriber = Some(transcriber);
        self.model_path = path_to_load;
        Ok(())
    }

    async fn unload(&mut self) -> Result<(), SttPluginError> {
        if self.transcriber.is_some() {
            self.transcriber = None;
            Ok(())
        } else {
            Err(SttPluginError::AlreadyUnloaded("Vosk".to_string()))
        }
    }
}

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
        Ok(Box::new(VoskPlugin::new()))
    }

    fn plugin_info(&self) -> PluginInfo {
        VoskPlugin::new().info()
    }

    fn check_requirements(&self) -> Result<(), SttPluginError> {
        // Use the centralized model location logic
        match model::locate_model(None) {
            Ok(_) => Ok(()),
            Err(e) => Err(SttPluginError::NotAvailable {
                reason: format!("Vosk model not found: {}", e),
            }),
        }
    }
}
