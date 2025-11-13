//! Candle Whisper plugin implementation
//!
//! This plugin wraps the Candle-based WhisperEngine and adapts it to the SttPlugin interface.

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::candle::{TranscribeOptions, WhisperDevice, WhisperEngine, WhisperEngineInit};
use crate::plugin::{SttPlugin, SttPluginConfig};
use crate::plugin_types::{AudioBuffer, PluginResult, TranscriptionResult};
use crate::types::{TranscriptionEvent, WordInfo};

/// Configuration for the Candle Whisper plugin
#[derive(Debug, Clone)]
pub struct CandleWhisperConfig {
    /// Path to model or HuggingFace model ID
    pub model_id: String,
    /// Whether to use quantized model
    pub quantized: bool,
    /// Device to use (CPU or CUDA)
    pub device: WhisperDevice,
    /// Language code (e.g., "en", "es"). None for auto-detection.
    pub language: Option<String>,
    /// Temperature for sampling (0.0 = greedy)
    pub temperature: f32,
    /// Enable word-level timestamps
    pub enable_timestamps: bool,
}

impl Default for CandleWhisperConfig {
    fn default() -> Self {
        Self {
            model_id: "openai/whisper-base".to_string(),
            quantized: false,
            device: WhisperDevice::Cpu,
            language: Some("en".to_string()),
            temperature: 0.0,
            enable_timestamps: true,
        }
    }
}

/// Candle Whisper plugin
pub struct CandleWhisperPlugin {
    engine: Arc<WhisperEngine>,
    config: CandleWhisperConfig,
    audio_buffer: Mutex<Vec<i16>>,
}

impl CandleWhisperPlugin {
    /// Create a new Candle Whisper plugin
    pub fn new(config: CandleWhisperConfig) -> PluginResult<Self> {
        tracing::info!("Initializing Candle Whisper plugin with model: {}", config.model_id);

        // Initialize engine from model ID
        let engine = WhisperEngine::from_model_id(
            &config.model_id,
            config.quantized,
            config.device.clone(),
        )
        .map_err(|e| format!("Failed to initialize Whisper engine: {}", e))?;

        Ok(Self {
            engine: Arc::new(engine),
            config,
            audio_buffer: Mutex::new(Vec::new()),
        })
    }

    /// Create from explicit engine initialization
    pub fn from_init(init: WhisperEngineInit, config: CandleWhisperConfig) -> PluginResult<Self> {
        tracing::info!("Initializing Candle Whisper plugin from explicit config");

        let engine = WhisperEngine::new(init)
            .map_err(|e| format!("Failed to initialize Whisper engine: {}", e))?;

        Ok(Self {
            engine: Arc::new(engine),
            config,
            audio_buffer: Mutex::new(Vec::new()),
        })
    }

    /// Get transcription options from config
    fn get_transcribe_options(&self) -> TranscribeOptions {
        TranscribeOptions {
            language: self.config.language.clone(),
            task: crate::candle::WhisperTask::Transcribe,
            temperature: self.config.temperature,
            enable_timestamps: self.config.enable_timestamps,
        }
    }
}

#[async_trait]
impl SttPlugin for CandleWhisperPlugin {
    fn name(&self) -> &str {
        "candle-whisper"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    async fn initialize(&mut self, _config: SttPluginConfig) -> PluginResult<()> {
        // Engine is already initialized in new()
        Ok(())
    }

    async fn process_audio(&mut self, audio: AudioBuffer) -> PluginResult<Option<TranscriptionResult>> {
        // Buffer audio for later transcription
        let mut buffer = self.audio_buffer.lock();
        buffer.extend_from_slice(&audio.samples);

        // Return None - we process on finalize
        Ok(None)
    }

    async fn finalize(&mut self) -> PluginResult<Option<TranscriptionResult>> {
        let mut buffer = self.audio_buffer.lock();

        if buffer.is_empty() {
            return Ok(None);
        }

        tracing::debug!("Finalizing transcription with {} samples", buffer.len());

        // Get transcription options
        let opts = self.get_transcribe_options();

        // Transcribe buffered audio
        let transcript = self
            .engine
            .transcribe_pcm16(&buffer, &opts)
            .map_err(|e| format!("Transcription failed: {}", e))?;

        // Clear buffer
        buffer.clear();

        // Convert to TranscriptionResult
        if transcript.text.is_empty() {
            return Ok(None);
        }

        // Convert segments to words (simplified - each segment becomes one "word")
        let words: Vec<WordInfo> = transcript
            .segments
            .iter()
            .map(|seg| WordInfo {
                word: seg.text.clone(),
                start: seg.start_seconds,
                end: seg.end_seconds,
                probability: seg.avg_logprob.exp(), // Convert log prob to probability
            })
            .collect();

        let result = TranscriptionResult {
            text: transcript.text.clone(),
            is_final: true,
            language: transcript.language.clone(),
            words: if self.config.enable_timestamps {
                Some(words)
            } else {
                None
            },
            confidence: None, // Could compute from avg_logprob if needed
        };

        Ok(Some(result))
    }

    async fn reset(&mut self) -> PluginResult<()> {
        let mut buffer = self.audio_buffer.lock();
        buffer.clear();
        Ok(())
    }

    async fn shutdown(&mut self) -> PluginResult<()> {
        tracing::info!("Shutting down Candle Whisper plugin");
        self.reset().await?;
        Ok(())
    }

    fn supports_streaming(&self) -> bool {
        // Whisper is batch-based, not truly streaming
        false
    }

    fn supports_partials(&self) -> bool {
        // No partial results in current implementation
        false
    }
}

/// Convert TranscriptionResult to TranscriptionEvent
///
/// # Utterance ID Generation
///
/// This implementation generates a new utterance ID on each conversion using
/// `next_utterance_id()`. This is safe because:
///
/// 1. Each `TranscriptionResult` is converted exactly once in the `finalize()` method
/// 2. The conversion happens immediately before returning the result
/// 3. No intermediate storage or caching of `TranscriptionResult` occurs
///
/// The ID is generated during conversion rather than at `TranscriptionResult` creation
/// because `TranscriptionResult` is a generic type shared across plugins, while utterance
/// IDs are specific to the event stream semantics of `TranscriptionEvent`.
impl From<TranscriptionResult> for TranscriptionEvent {
    fn from(result: TranscriptionResult) -> Self {
        // Generate ID at conversion time - safe because each result is converted exactly once
        let utterance_id = crate::next_utterance_id();

        if result.is_final {
            TranscriptionEvent::Final {
                text: result.text,
                utterance_id,
                words: result.words,
            }
        } else {
            TranscriptionEvent::Partial {
                text: result.text,
                utterance_id,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CandleWhisperConfig::default();
        assert_eq!(config.model_id, "openai/whisper-base");
        assert!(!config.quantized);
        assert_eq!(config.temperature, 0.0);
    }
}
