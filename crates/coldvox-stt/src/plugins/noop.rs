//! No-operation STT plugin for testing and fallback

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use async_trait::async_trait;

use coldvox_foundation::error::ColdVoxError;

/// A no-op STT plugin that never transcribes anything
/// Useful for testing the pipeline without STT dependencies
#[derive(Debug, Clone)]
pub struct NoOpPlugin {
    initialized: bool,
}

impl NoOpPlugin {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for NoOpPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SttPlugin for NoOpPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "noop".to_string(),
            name: "No-Op STT".to_string(),
            description: "A null STT plugin that produces no transcriptions".to_string(),
            requires_network: false,
            is_local: true,
            is_available: true,
            supported_languages: vec!["*".to_string()], // Supports "all" languages by doing nothing
            memory_usage_mb: Some(0),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: true,
            batch: true,
            word_timestamps: false,
            confidence_scores: false,
            speaker_diarization: false,
            auto_punctuation: false,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(true) // Always available
    }

    async fn initialize(&mut self, _config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        self.initialized = true;
        Ok(())
    }

    async fn process_audio(
        &mut self,
        _samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        // Never produces transcriptions
        Ok(None)
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        Ok(None)
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        Ok(())
    }
}

/// Factory for creating NoOpPlugin instances
pub struct NoOpPluginFactory;

impl SttPluginFactory for NoOpPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        Ok(Box::new(NoOpPlugin::new()))
    }

    fn plugin_info(&self) -> PluginInfo {
        NoOpPlugin::new().info()
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        Ok(()) // No requirements
    }
}
