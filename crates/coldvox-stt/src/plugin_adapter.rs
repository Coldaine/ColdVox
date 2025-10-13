//! Plugin adapter that implements StreamingStt trait
//!
//! This module bridges STT plugins implementing the internal SttPlugin interface
//! to the public StreamingStt abstraction used by the async STT processor.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::plugin::{SttPlugin, SttPluginError};
use crate::types::TranscriptionEvent;
use crate::{next_utterance_id, StreamingStt, TranscriptionConfig};

/// Adapter that wraps an SttPlugin and implements StreamingStt
pub struct PluginAdapter {
    plugin: Arc<RwLock<Box<dyn SttPlugin>>>,
    current_utterance_id: u64,
}

impl PluginAdapter {
    /// Create a new adapter wrapping the given plugin
    pub fn new(plugin: Box<dyn SttPlugin>) -> Self {
        Self {
            plugin: Arc::new(RwLock::new(plugin)),
            current_utterance_id: next_utterance_id(),
        }
    }

    /// Initialize the plugin with configuration
    pub async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), SttPluginError> {
        let mut plugin = self.plugin.write().await;
        plugin.initialize(config).await
    }

    /// Get plugin info
    pub async fn plugin_info(&self) -> crate::plugin::PluginInfo {
        let plugin = self.plugin.read().await;
        plugin.info()
    }
}

#[async_trait]
impl StreamingStt for PluginAdapter {
    /// Process audio frame during active speech
    async fn on_speech_frame(&mut self, samples: &[i16]) -> Option<TranscriptionEvent> {
        let mut plugin = self.plugin.write().await;
        match plugin.process_audio(samples).await {
            Ok(event) => event.map(|e| match e {
                TranscriptionEvent::Partial {
                    utterance_id: _,
                    text,
                    t0,
                    t1,
                } => TranscriptionEvent::Partial {
                    utterance_id: self.current_utterance_id,
                    text,
                    t0,
                    t1,
                },
                TranscriptionEvent::Final {
                    utterance_id: _,
                    text,
                    words,
                } => TranscriptionEvent::Final {
                    utterance_id: self.current_utterance_id,
                    text,
                    words,
                },
                TranscriptionEvent::Error { code, message } => {
                    TranscriptionEvent::Error { code, message }
                }
            }),
            Err(e) => {
                tracing::error!(target: "stt", "STT plugin error during frame processing: {}", e);
                Some(TranscriptionEvent::Error {
                    code: "PLUGIN_PROCESS_ERROR".to_string(),
                    message: e.to_string(),
                })
            }
        }
    }

    /// Finalize transcription at end of speech segment
    async fn on_speech_end(&mut self) -> Option<TranscriptionEvent> {
        let mut plugin = self.plugin.write().await;
        match plugin.finalize().await {
            Ok(event) => {
                let mapped = event.map(|e| match e {
                    TranscriptionEvent::Partial {
                        utterance_id: _,
                        text,
                        t0,
                        t1,
                    } => TranscriptionEvent::Partial {
                        utterance_id: self.current_utterance_id,
                        text,
                        t0,
                        t1,
                    },
                    TranscriptionEvent::Final {
                        utterance_id: _,
                        text,
                        words,
                    } => TranscriptionEvent::Final {
                        utterance_id: self.current_utterance_id,
                        text,
                        words,
                    },
                    TranscriptionEvent::Error { code, message } => {
                        TranscriptionEvent::Error { code, message }
                    }
                });
                if mapped.is_some() {
                    // Start new utterance for next speech segment
                    self.current_utterance_id = next_utterance_id();
                }
                mapped
            }
            Err(e) => {
                tracing::error!(target: "stt", "STT plugin error during finalization: {}", e);
                Some(TranscriptionEvent::Error {
                    code: "PLUGIN_FINALIZE_ERROR".to_string(),
                    message: e.to_string(),
                })
            }
        }
    }

    /// Reset state for new utterance
    async fn reset(&mut self) {
        let mut plugin = self.plugin.write().await;
        self.current_utterance_id = next_utterance_id();
        if let Err(e) = plugin.reset().await {
            tracing::warn!(target: "stt", "Failed to reset STT plugin: {}", e);
        }
    }
}
