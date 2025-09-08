use coldvox_stt::{
    next_utterance_id, EventBasedTranscriber, Transcriber, TranscriptionConfig, TranscriptionEvent,
    WordInfo,
};
use tracing::warn;
use vosk::{CompleteResult, DecodingState, Model, PartialResult, Recognizer};

pub struct VoskTranscriber {
    recognizer: Recognizer,
    config: TranscriptionConfig,
    current_utterance_id: u64,
}

impl VoskTranscriber {
    /// Create a new VoskTranscriber with the given configuration
    pub fn new(config: TranscriptionConfig, sample_rate: f32) -> Result<Self, String> {
        // Validate sample rate - Vosk works best with 16kHz
        if (sample_rate - 16000.0).abs() > 0.1 {
            warn!(
                "VoskTranscriber: Sample rate {}Hz differs from expected 16000Hz. \
                This may affect transcription quality.",
                sample_rate
            );
        }

        // Use model path from config, or get default
        let model_path = if config.model_path.is_empty() {
            crate::default_model_path()
        } else {
            config.model_path.clone()
        };

        // Check if model path exists
        if !std::path::Path::new(&model_path).exists() {
            return Err(format!(
                "Vosk model not found at '{}'. This should not happen — model is vendored in repo. \
                 Please ensure 'vosk-model-small-en-us-0.15/' directory exists.",
                model_path
            ));
        }

        // Load the model
        let model = Model::new(&model_path)
            .ok_or_else(|| format!("Failed to load Vosk model from: {}", model_path))?;

        // Create recognizer with configuration
        let mut recognizer = Recognizer::new(&model, sample_rate).ok_or_else(|| {
            format!(
                "Failed to create Vosk recognizer with sample rate: {}",
                sample_rate
            )
        })?;

        // Configure recognizer based on config
        recognizer.set_max_alternatives(config.max_alternatives as u16);
        recognizer.set_words(config.include_words);
        recognizer.set_partial_words(config.partial_results && config.include_words);

        // Update the config to use the resolved model path
        let mut final_config = config;
        final_config.model_path = model_path;

        Ok(Self {
            recognizer,
            config: final_config,
            current_utterance_id: next_utterance_id(),
        })
    }

    /// Create a new VoskTranscriber with default model path (backward compatibility)
    pub fn new_with_default(model_path: &str, sample_rate: f32) -> Result<Self, String> {
        let config = TranscriptionConfig {
            enabled: true,
            model_path: model_path.to_string(),
            partial_results: true,
            max_alternatives: 1,
            include_words: false,
            buffer_size_ms: 512,
            streaming: false,
        };
        Self::new(config, sample_rate)
    }

    /// Update configuration (requires recreating recognizer)
    pub fn update_config(
        &mut self,
        config: TranscriptionConfig,
        sample_rate: f32,
    ) -> Result<(), String> {
        // Use model path from config, or get default
        let model_path = if config.model_path.is_empty() {
            crate::default_model_path()
        } else {
            config.model_path.clone()
        };

        // Recreate recognizer with new config
        let model = Model::new(&model_path)
            .ok_or_else(|| format!("Failed to load Vosk model from: {}", model_path))?;

        let mut recognizer = Recognizer::new(&model, sample_rate).ok_or_else(|| {
            format!(
                "Failed to create Vosk recognizer with sample rate: {}",
                sample_rate
            )
        })?;

        recognizer.set_max_alternatives(config.max_alternatives as u16);
        recognizer.set_words(config.include_words);
        recognizer.set_partial_words(config.partial_results && config.include_words);

        self.recognizer = recognizer;
        let mut final_config = config;
        final_config.model_path = model_path;
        self.config = final_config;
        Ok(())
    }

    // Private helper methods

    fn parse_complete_result_static(
        result: CompleteResult,
        utterance_id: u64,
        include_words: bool,
    ) -> Option<TranscriptionEvent> {
        match result {
            CompleteResult::Single(single) => {
                let text = single.text;
                if text.trim().is_empty() {
                    None
                } else {
                    let words = if include_words && !single.result.is_empty() {
                        Some(
                            single
                                .result
                                .into_iter()
                                .map(|w| WordInfo {
                                    text: w.word.to_string(),
                                    start: w.start,
                                    end: w.end,
                                    conf: w.conf,
                                })
                                .collect(),
                        )
                    } else {
                        None
                    };

                    Some(TranscriptionEvent::Final {
                        utterance_id,
                        text: text.to_string(),
                        words,
                    })
                }
            }
            CompleteResult::Multiple(multiple) => {
                // Take the first alternative if multiple are available
                if let Some(first) = multiple.alternatives.first() {
                    let text = first.text;
                    if text.trim().is_empty() {
                        None
                    } else {
                        let words = if include_words && !first.result.is_empty() {
                            Some(
                                first
                                    .result
                                    .iter()
                                    .map(|w| WordInfo {
                                        text: w.word.to_string(),
                                        start: w.start,
                                        end: w.end,
                                        conf: 0.5, // Default confidence when not available from Vosk API
                                    })
                                    .collect(),
                            )
                        } else {
                            None
                        };

                        Some(TranscriptionEvent::Final {
                            utterance_id,
                            text: text.to_string(),
                            words,
                        })
                    }
                } else {
                    None
                }
            }
        }
    }

    fn parse_partial_result_static(
        partial: PartialResult,
        utterance_id: u64,
    ) -> Option<TranscriptionEvent> {
        let text = partial.partial;
        if text.trim().is_empty() {
            None
        } else {
            // Partial results don't typically have timing info in vosk
            Some(TranscriptionEvent::Partial {
                utterance_id,
                text: text.to_string(),
                t0: None,
                t1: None,
            })
        }
    }
}

impl EventBasedTranscriber for VoskTranscriber {
    /// Accept PCM16 audio and return transcription events
    fn accept_frame(&mut self, pcm: &[i16]) -> Result<Option<TranscriptionEvent>, String> {
        // Skip if transcription is disabled
        if !self.config.enabled {
            return Ok(None);
        }

        // Pass the i16 samples directly - vosk expects i16
        let state = self
            .recognizer
            .accept_waveform(pcm)
            .map_err(|e| format!("Vosk waveform acceptance failed: {:?}", e))?;

        match state {
            DecodingState::Finalized => {
                // Get final result when speech segment is complete
                let result = self.recognizer.result();
                let event = Self::parse_complete_result_static(
                    result,
                    self.current_utterance_id,
                    self.config.include_words,
                );
                Ok(event)
            }
            DecodingState::Running => {
                // Get partial result for ongoing speech if enabled
                if self.config.partial_results {
                    let partial = self.recognizer.partial_result();
                    let event =
                        Self::parse_partial_result_static(partial, self.current_utterance_id);
                    Ok(event)
                } else {
                    Ok(None)
                }
            }
            DecodingState::Failed => {
                // Recognition failed for this chunk
                Ok(Some(TranscriptionEvent::Error {
                    code: "VOSK_DECODE_FAILED".to_string(),
                    message: "Vosk recognition failed for current chunk".to_string(),
                }))
            }
        }
    }

    /// Finalize current utterance and return final result
    fn finalize_utterance(&mut self) -> Result<Option<TranscriptionEvent>, String> {
        let final_result = self.recognizer.final_result();
        let event = Self::parse_complete_result_static(
            final_result,
            self.current_utterance_id,
            self.config.include_words,
        );

        // Start new utterance for next speech segment
        self.current_utterance_id = next_utterance_id();

        Ok(event)
    }

    /// Reset recognizer state for new utterance
    fn reset(&mut self) -> Result<(), String> {
        // Vosk doesn't have an explicit reset, but finalizing clears state
        let _ = self.recognizer.final_result();
        self.current_utterance_id = next_utterance_id();
        Ok(())
    }

    /// Get current configuration
    fn config(&self) -> &TranscriptionConfig {
        &self.config
    }
}

// Implement the legacy Transcriber trait for backward compatibility
impl Transcriber for VoskTranscriber {
    fn accept_pcm16(&mut self, pcm: &[i16]) -> Result<Option<String>, String> {
        match self.accept_frame(pcm)? {
            Some(TranscriptionEvent::Final { text, .. }) => Ok(Some(text)),
            Some(TranscriptionEvent::Partial { text, .. }) => {
                Ok(Some(format!("[partial] {}", text)))
            }
            Some(TranscriptionEvent::Error { message, .. }) => Err(message),
            None => Ok(None),
        }
    }

    fn finalize(&mut self) -> Result<Option<String>, String> {
        match self.finalize_utterance()? {
            Some(TranscriptionEvent::Final { text, .. }) => Ok(Some(text)),
            Some(TranscriptionEvent::Partial { text, .. }) => Ok(Some(text)),
            Some(TranscriptionEvent::Error { message, .. }) => Err(message),
            None => Ok(None),
        }
    }
}
