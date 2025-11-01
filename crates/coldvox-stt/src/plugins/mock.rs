//! Mock STT plugin for testing

use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent, WordInfo};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use tracing::info;

use coldvox_foundation::error::{ColdVoxError, SttError};

/// Configuration for mock transcriptions
#[derive(Debug, Clone)]
pub struct MockConfig {
    /// Text to return after N audio chunks
    pub transcription_after_chunks: Option<(usize, String)>,

    /// Immediate transcription to return
    pub immediate_transcription: Option<String>,

    /// Simulate processing delay in ms
    pub processing_delay_ms: u64,

    /// Simulate failure after N calls
    pub fail_after_calls: Option<usize>,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            transcription_after_chunks: Some((1, "mock test transcription".to_string())),
            immediate_transcription: None,
            processing_delay_ms: 0,
            fail_after_calls: None,
        }
    }
}

/// Mock STT plugin for testing the pipeline
#[derive(Debug)]
pub struct MockPlugin {
    config: MockConfig,
    state: Arc<Mutex<MockState>>,
}

#[derive(Debug)]
struct MockState {
    initialized: bool,
    chunks_processed: usize,
    calls_made: usize,
    has_session_audio: bool,
}

impl MockPlugin {
    pub fn new(config: MockConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(MockState {
                initialized: false,
                chunks_processed: 0,
                calls_made: 0,
                has_session_audio: false,
            })),
        }
    }

    pub fn with_transcription(text: String) -> Self {
        Self::new(MockConfig {
            immediate_transcription: Some(text),
            ..Default::default()
        })
    }

    pub fn with_delayed_transcription(chunks: usize, text: String) -> Self {
        Self::new(MockConfig {
            transcription_after_chunks: Some((chunks, text)),
            immediate_transcription: None,
            ..Default::default()
        })
    }
}

impl Default for MockPlugin {
    fn default() -> Self {
        Self::new(MockConfig::default())
    }
}

#[async_trait]
impl SttPlugin for MockPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "mock".to_string(),
            name: "Mock STT".to_string(),
            description: "Configurable mock STT for testing".to_string(),
            requires_network: false,
            is_local: true,
            is_available: true,
            supported_languages: vec!["en".to_string()],
            memory_usage_mb: Some(1),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: true,
            batch: true,
            word_timestamps: true,
            confidence_scores: true,
            speaker_diarization: false,
            auto_punctuation: true,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(true)
    }

    async fn initialize(&mut self, _config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        let mut state = self.state.lock().unwrap();
        state.initialized = true;
        Ok(())
    }

    async fn process_audio(
        &mut self,
        _samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        // Get state and check failure conditions
        let should_fail = {
            let mut state = self.state.lock().unwrap();
            state.calls_made += 1;

            if let Some(fail_after) = self.config.fail_after_calls {
                state.calls_made > fail_after
            } else {
                false
            }
        };

        if should_fail {
            return Err(SttError::TranscriptionFailed("Simulated failure".to_string()).into());
        }

        // Simulate processing delay
        if self.config.processing_delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.processing_delay_ms,
            ))
            .await;
        }

        // Check for immediate transcription
        if let Some(ref text) = self.config.immediate_transcription {
            return Ok(Some(create_mock_event(text.clone())));
        }

        // Check for delayed transcription
        let should_transcribe = {
            let mut state = self.state.lock().unwrap();
            state.chunks_processed += 1;
            state.has_session_audio = true;

            if let Some((chunks, _)) = self.config.transcription_after_chunks {
                if state.chunks_processed >= chunks {
                    info!(
                        "MockPlugin: producing transcription after {} chunks",
                        state.chunks_processed
                    );
                    state.chunks_processed = 0; // Reset for next transcription
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if should_transcribe {
            if let Some((_, ref text)) = self.config.transcription_after_chunks {
                return Ok(Some(create_mock_event(text.clone())));
            }
        }

        Ok(None)
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        info!("MockPlugin::finalize called");
        let state = self.state.lock().unwrap();
        info!(
            "MockPlugin: finalize - chunks_processed = {}",
            state.chunks_processed
        );

        // Return final transcription on finalize if chunks have been processed
        if state.chunks_processed > 0 {
            info!(
                "MockPlugin: producing Final event on finalize after {} chunks processed",
                state.chunks_processed
            );
            let text = "mock test transcription".to_string();
            return Ok(Some(create_mock_event(text)));
        }

        info!("MockPlugin: no Final event on finalize (no chunks processed)");
        Ok(None)
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        let mut state = self.state.lock().unwrap();
        state.chunks_processed = 0;
        state.calls_made = 0;
        state.has_session_audio = false;
        Ok(())
    }
}

fn create_mock_event(text: String) -> TranscriptionEvent {
    let words: Vec<WordInfo> = text
        .split_whitespace()
        .enumerate()
        .map(|(i, word)| WordInfo {
            text: word.to_string(),
            start: i as f32 * 0.5,
            end: (i as f32 + 1.0) * 0.5,
            conf: 0.95,
        })
        .collect();

    TranscriptionEvent::Final {
        utterance_id: crate::next_utterance_id(),
        text,
        words: Some(words),
    }
}

/// Factory for creating MockPlugin instances
pub struct MockPluginFactory {
    config: MockConfig,
}

impl MockPluginFactory {
    pub fn new(config: MockConfig) -> Self {
        Self { config }
    }
}

impl Default for MockPluginFactory {
    fn default() -> Self {
        Self::new(MockConfig::default())
    }
}

impl SttPluginFactory for MockPluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        Ok(Box::new(MockPlugin::new(self.config.clone())))
    }

    fn plugin_info(&self) -> PluginInfo {
        MockPlugin::default().info()
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        Ok(())
    }
}
