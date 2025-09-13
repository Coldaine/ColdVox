use std::path::PathBuf;
use std::sync::Arc;

use coldvox_stt_vosk::VoskTranscriber;
use crate::{SttPlugin, SttPluginError, plugin::PluginInfo, TranscriptionEvent, WordInfo};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

pub struct VoskPlugin {
    transcriber: Arc<RwLock<Option<VoskTranscriber>>>,
    model_path: PathBuf,
    language: String,
    initialized: bool,
}

impl VoskPlugin {
    pub fn new(model_path: PathBuf, language: String) -> Self {
        Self {
            transcriber: Arc::new(RwLock::new(None)),
            model_path,
            language,
            initialized: false,
        }
    }

    async fn initialize_transcriber(&mut self) -> Result<(), SttPluginError> {
        let model_path = self.model_path.clone();
        if !model_path.exists() {
            return Err(SttPluginError::InitializationFailed(format!(
                "Vosk model not found at {}",
                model_path.display()
            )));
        }
        let transcriber = VoskTranscriber::new(model_path, self.language.clone()).await
            .map_err(|e| SttPluginError::InitializationFailed(e.to_string()))?;
        let mut guard = self.transcriber.write().await;
        *guard = Some(transcriber);
        self.initialized = true;
        Ok(())
    }
}

#[async_trait::async_trait]
impl SttPlugin for VoskPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "vosk".to_string(),
            name: "Vosk STT".to_string(),
            description: "Offline speech recognition using Vosk API".to_string(),
            is_available: self.model_path.exists(),
            requires_model: true,
            supports_streaming: true,
            supported_languages: vec!["en".to_string(), "fr".to_string()], // Add more as needed
        }
    }

    async fn initialize(&mut self) -> Result<(), SttPluginError> {
        if !self.initialized {
            self.initialize_transcriber().await?;
            info!("Vosk plugin initialized with model at {}", self.model_path.display());
        }
        Ok(())
    }

    async fn process_audio(&mut self, samples: &[i16]) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        if !self.initialized {
            return Err(SttPluginError::NotInitialized);
        }
        let guard = self.transcriber.read().await;
        if let Some(transcriber) = &*guard {
            match transcriber.process_audio(samples).await {
                Ok(event) => Ok(event.map(|text| TranscriptionEvent::Partial {
                    utterance_id: 0,
                    text,
                    words: None,
                })),
                Err(e) => Err(SttPluginError::ProcessingFailed(e.to_string())),
            }
        } else {
            Err(SttPluginError::NotInitialized)
        }
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        if !self.initialized {
            return Ok(None);
        }
        let guard = self.transcriber.read().await;
        if let Some(transcriber) = &*guard {
            match transcriber.finalize().await {
                Ok((text, words)) => Ok(Some(TranscriptionEvent::Final {
                    utterance_id: 0,
                    text,
                    words: Some(words.into_iter().map(|w| WordInfo {
                        text: w.word,
                        start: w.start,
                        end: w.end,
                        conf: w.conf,
                    }).collect()),
                })),
                Err(e) => Err(SttPluginError::ProcessingFailed(e.to_string())),
            }
        } else {
            Ok(None)
        }
    }

    async fn unload(&mut self) -> Result<(), SttPluginError> {
        let mut guard = self.transcriber.write().await;
        *guard = None;
        self.initialized = false;
        info!("Vosk plugin unloaded");
        Ok(())
    }

    async fn reset(&mut self) -> Result<(), SttPluginError> {
        if self.initialized {
            let guard = self.transcriber.read().await;
            if let Some(transcriber) = &*guard {
                transcriber.reset().await.map_err(|e| SttPluginError::ProcessingFailed(e.to_string()))?;
            }
        }
        Ok(())
    }
}

pub struct VoskPluginFactory;

impl crate::PluginFactory for VoskPluginFactory {
    fn create_plugin(&self) -> Result<Box<dyn SttPlugin + Send + Sync>, SttPluginError> {
        let model_path = std::env::var("VOSK_MODEL_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("models/vosk-model-small-en-us-0.15"));
        let language = std::env::var("VOSK_LANGUAGE").unwrap_or_else(|_| "en-us".to_string());
        Ok(Box::new(VoskPlugin::new(model_path, language)))
    }
}