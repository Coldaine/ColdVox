use async_trait::async_trait;
use std::sync::Arc;

use coldvox_stt::{StreamingStt, TranscriptionEvent, next_utterance_id};

/// Adapter that exposes SttPluginManager as a StreamingStt engine
pub struct ManagerStreamingAdapter {
    manager: Arc<tokio::sync::RwLock<crate::stt::plugin_manager::SttPluginManager>>,
    current_utterance_id: u64,
}

impl ManagerStreamingAdapter {
    pub fn new(manager: Arc<tokio::sync::RwLock<crate::stt::plugin_manager::SttPluginManager>>) -> Self {
        Self { manager, current_utterance_id: next_utterance_id() }
    }
}

#[async_trait]
impl StreamingStt for ManagerStreamingAdapter {
    async fn on_speech_frame(&mut self, samples: &[i16]) -> Option<TranscriptionEvent> {
        let mut mgr = self.manager.write().await;
        match mgr.process_audio(samples).await {
            Ok(Some(e)) => Some(match e {
                TranscriptionEvent::Partial { utterance_id: _, text, t0, t1 } =>
                    TranscriptionEvent::Partial { utterance_id: self.current_utterance_id, text, t0, t1 },
                TranscriptionEvent::Final { utterance_id: _, text, words } =>
                    TranscriptionEvent::Final { utterance_id: self.current_utterance_id, text, words },
                TranscriptionEvent::Error { code, message } => TranscriptionEvent::Error { code, message },
            }),
            Ok(None) => None,
            Err(err) => Some(TranscriptionEvent::Error { code: "PLUGIN_PROCESS_ERROR".to_string(), message: err }),
        }
    }

    async fn on_speech_end(&mut self) -> Option<TranscriptionEvent> {
        let mut mgr = self.manager.write().await;
        match mgr.finalize().await {
            Ok(Some(e)) => {
                let mapped = match e {
                    TranscriptionEvent::Partial { utterance_id: _, text, t0, t1 } =>
                        TranscriptionEvent::Partial { utterance_id: self.current_utterance_id, text, t0, t1 },
                    TranscriptionEvent::Final { utterance_id: _, text, words } =>
                        TranscriptionEvent::Final { utterance_id: self.current_utterance_id, text, words },
                    TranscriptionEvent::Error { code, message } => TranscriptionEvent::Error { code, message },
                };
                // Advance utterance for next segment
                self.current_utterance_id = next_utterance_id();
                Some(mapped)
            }
            Ok(None) => None,
            Err(err) => Some(TranscriptionEvent::Error { code: "PLUGIN_FINALIZE_ERROR".to_string(), message: err }),
        }
    }

    async fn reset(&mut self) {
        self.current_utterance_id = next_utterance_id();
        let mut mgr = self.manager.write().await;
        let _ = mgr.reset().await; // best effort
    }
}

