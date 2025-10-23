use async_trait::async_trait;
use thiserror::Error;
use log::{info, error};
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use std::mem;

use whisper_rs::{WhisperContext, FullParams, SamplingStrategy, WhisperContextParameters};

use coldvox_stt::plugin::{
    SttPlugin, SttPluginError, PluginInfo, PluginCapabilities, SttPluginFactory,
};
use coldvox_stt::types::{TranscriptionConfig, TranscriptionEvent};

const DEFAULT_MODEL_PATH: &str = "models/whisper/ggml-tiny.en.bin";

#[derive(Debug, Error)]
pub enum FasterWhisperError {
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
}

pub struct FasterWhisperPlugin {
    ctx: Option<WhisperContext>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    config: TranscriptionConfig,
}

impl std::fmt::Debug for FasterWhisperPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FasterWhisperPlugin")
            .field("audio_buffer", &self.audio_buffer)
            .field("config", &self.config)
            .finish()
    }
}

impl FasterWhisperPlugin {
    pub fn new() -> Self {
        Self {
            ctx: None,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            config: TranscriptionConfig::default(),
        }
    }
}

#[async_trait]
impl SttPlugin for FasterWhisperPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "faster-whisper".to_string(),
            name: "Faster Whisper".to_string(),
            description: "High-performance speech recognition using Faster Whisper".to_string(),
            requires_network: false,
            is_local: true,
            is_available: PathBuf::from(DEFAULT_MODEL_PATH).exists(),
            supported_languages: vec!["en".to_string()],
            memory_usage_mb: Some(1000),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false,
            batch: true,
            word_timestamps: false, // Disabled for now
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: true,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, SttPluginError> {
        Ok(PathBuf::from(DEFAULT_MODEL_PATH).exists())
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), SttPluginError> {
        info!("Initializing Faster Whisper plugin with whisper-rs");
        self.config = config;

        let model_path = self.config.model_path.clone();
        if !PathBuf::from(&model_path).exists() {
            let err_msg = format!("Model file not found at: {}", &model_path);
            error!("{}", err_msg);
            return Err(SttPluginError::ModelLoadFailed(err_msg));
        }

        let context = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
            .map_err(|e| SttPluginError::InitializationFailed(e.to_string()))?;

        self.ctx = Some(context);
        info!("whisper-rs context created successfully from {}", &self.config.model_path);
        Ok(())
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        if self.ctx.is_none() {
            return Err(SttPluginError::InitializationFailed("Plugin not initialized".to_string()));
        }

        let mut buffer = self.audio_buffer.lock();
        let mut float_samples = vec![0.0f32; samples.len()];
        whisper_rs::convert_integer_to_float_audio(samples, &mut float_samples).unwrap();
        buffer.extend_from_slice(&float_samples);

        Ok(None)
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, SttPluginError> {
        info!("Finalizing transcription with whisper-rs");

        let audio_data = {
            let mut buffer = self.audio_buffer.lock();
            if buffer.is_empty() {
                return Ok(None);
            }
            mem::take(&mut *buffer)
        };

        let ctx = self.ctx.as_ref().ok_or_else(|| {
            SttPluginError::InitializationFailed("Context not available".to_string())
        })?;

        let mut state = ctx.create_state()
            .map_err(|e| SttPluginError::InitializationFailed(e.to_string()))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(4);
        params.set_language(Some("en"));

        state.full(params, &audio_data)
            .map_err(|e| SttPluginError::TranscriptionFailed(e.to_string()))?;

        let num_segments = state.full_n_segments();

        let mut full_text = String::new();
        for i in 0..num_segments {
            let segment = state.get_segment(i).unwrap();
            full_text.push_str(segment.to_str().unwrap());
        }

        Ok(Some(TranscriptionEvent::Final {
            utterance_id: 1,
            text: full_text.trim().to_string(),
            words: None,
        }))
    }

    async fn reset(&mut self) -> Result<(), SttPluginError> {
        info!("Resetting Faster Whisper plugin state");
        self.audio_buffer.lock().clear();
        Ok(())
    }
}

pub struct FasterWhisperPluginFactory;

impl SttPluginFactory for FasterWhisperPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, SttPluginError> {
        Ok(Box::new(FasterWhisperPlugin::new()))
    }

    fn plugin_info(&self) -> PluginInfo {
        FasterWhisperPlugin::new().info()
    }

    fn check_requirements(&self) -> Result<(), SttPluginError> {
        // This check is simplified. A real implementation would be more robust.
        if !PathBuf::from(DEFAULT_MODEL_PATH).exists() {
            return Err(SttPluginError::NotAvailable {
                reason: format!("Faster Whisper model not found at {}", DEFAULT_MODEL_PATH),
            });
        }
        Ok(())
    }
}