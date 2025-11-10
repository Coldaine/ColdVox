//! Pure Rust Whisper STT Plugin Stub
//!
//! This plugin provides a placeholder implementation for Whisper-based speech-to-text.
//! The previous Python-dependent implementation (faster-whisper-rs + PyO3) has been removed
//! in favor of a future pure-Rust backend. This stub returns unavailable status until
//! a native Rust implementation is available.
//!
//! ## Migration to Pure Rust
//!
//! This stub will be replaced with a native Rust implementation using a pure-Rust
//! Whisper inference library. The migration eliminates Python/PyO3 coupling and
//! runtime dependency complexity, providing predictable build isolation.

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use async_trait::async_trait;
use coldvox_foundation::error::{ColdVoxError, SttError};

/// Whisper-based STT plugin stub (returns unavailable until pure-Rust implementation is ready)
#[derive(Debug)]
pub struct WhisperPlugin {
    #[allow(dead_code)]
    initialized: bool,
}

impl WhisperPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
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
            name: "Whisper (Pure Rust - Coming Soon)".to_string(),
            description: "Local transcription via pure-Rust Whisper implementation (not yet available)".to_string(),
            requires_network: false,
            is_local: true,
            is_available: false, // Always unavailable until pure-Rust implementation is ready
            supported_languages: vec!["auto".to_string(), "en".to_string()],
            memory_usage_mb: None,
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false,
            batch: false,
            word_timestamps: false,
            confidence_scores: false,
            speaker_diarization: false,
            auto_punctuation: false,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(false)
    }

    async fn initialize(&mut self, _config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        Err(SttError::NotAvailable {
            plugin: "whisper".to_string(),
            reason: "Pure Rust Whisper implementation not yet available. The previous Python-based implementation has been removed.".to_string(),
        }
        .into())
    }

    async fn process_audio(
        &mut self,
        _samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        Err(SttError::NotAvailable {
            plugin: "whisper".to_string(),
            reason: "Pure Rust Whisper implementation not yet available".to_string(),
        }
        .into())
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        Err(SttError::NotAvailable {
            plugin: "whisper".to_string(),
            reason: "Pure Rust Whisper implementation not yet available".to_string(),
        }
        .into())
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        Ok(())
    }

    async fn load_model(&mut self, _model_path: Option<&std::path::Path>) -> Result<(), ColdVoxError> {
        Err(SttError::NotAvailable {
            plugin: "whisper".to_string(),
            reason: "Pure Rust Whisper implementation not yet available".to_string(),
        }
        .into())
    }

    async fn unload(&mut self) -> Result<(), ColdVoxError> {
        Ok(())
    }
}

/// Factory for creating WhisperPlugin stub instances.
pub struct WhisperPluginFactory {
    // No configuration needed for the stub
}

impl WhisperPluginFactory {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for WhisperPluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for WhisperPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        Ok(Box::new(WhisperPlugin::new()))
    }

    fn plugin_info(&self) -> PluginInfo {
        WhisperPlugin::new().info()
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        Err(SttError::NotAvailable {
            plugin: "whisper".to_string(),
            reason: "Pure Rust Whisper implementation not yet available. The previous Python-based implementation has been removed.".to_string(),
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_whisper_plugin_stub_unavailable() {
        let plugin = WhisperPlugin::new();
        
        // Plugin should report as unavailable
        assert!(!plugin.info().is_available);
        assert!(!plugin.is_available().await.unwrap());
        
        // All operations should fail with NotAvailable error
        assert!(plugin.initialize(TranscriptionConfig::default()).await.is_err());
        assert!(plugin.process_audio(&[0]).await.is_err());
        assert!(plugin.finalize().await.is_err());
        assert!(plugin.load_model(None).await.is_err());
    }

    #[test]
    fn test_whisper_factory_stub_unavailable() {
        let factory = WhisperPluginFactory::new();
        
        // Should fail requirements check
        assert!(factory.check_requirements().is_err());
        
        // Can still create plugin instance (for interface compatibility)
        assert!(factory.create().is_ok());
    }
}
