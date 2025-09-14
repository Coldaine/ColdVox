//! TTS processor integration for ColdVox app

#[cfg(feature = "tts")]
use coldvox_tts::{TtsConfig, TtsEngine, SynthesisEvent, SynthesisOptions};
#[cfg(feature = "tts-espeak")]
use coldvox_tts_espeak::EspeakEngine;

use coldvox_stt::TranscriptionEvent;
use tokio::sync::mpsc;
use tracing::{debug, info, warn, error};
use std::sync::Arc;

/// TTS processor that can synthesize transcription events
#[cfg(feature = "tts")]
pub struct TtsProcessor {
    engine: Box<dyn TtsEngine>,
    config: TtsConfig,
    transcription_rx: mpsc::Receiver<TranscriptionEvent>,
    is_enabled: bool,
}

#[cfg(feature = "tts")]
impl TtsProcessor {
    /// Create a new TTS processor with eSpeak engine
    #[cfg(feature = "tts-espeak")]
    pub async fn new_with_espeak(
        config: TtsConfig,
        transcription_rx: mpsc::Receiver<TranscriptionEvent>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut engine = Box::new(EspeakEngine::new()) as Box<dyn TtsEngine>;
        
        if !engine.is_available().await {
            warn!("eSpeak TTS engine not available - TTS will be disabled");
            return Ok(Self {
                engine,
                config,
                transcription_rx,
                is_enabled: false,
            });
        }
        
        engine.initialize(config.clone()).await?;
        info!("TTS processor initialized with eSpeak engine");
        
        Ok(Self {
            engine,
            config,
            transcription_rx,
            is_enabled: config.enabled,
        })
    }
    
    /// Start processing transcription events
    pub async fn run(mut self) {
        info!("Starting TTS processor");
        
        while let Some(event) = self.transcription_rx.recv().await {
            if !self.is_enabled {
                continue;
            }
            
            match event {
                TranscriptionEvent::Final { text, utterance_id, .. } => {
                    debug!("Processing final transcription [{}]: {}", utterance_id, text);
                    
                    if let Err(e) = self.synthesize_text(&text).await {
                        error!("Failed to synthesize text: {}", e);
                    }
                }
                TranscriptionEvent::Error { code, message } => {
                    // Optionally synthesize error notifications
                    if self.config.engine_options.get("announce_errors").map(|v| v == "true").unwrap_or(false) {
                        let error_msg = format!("Transcription error: {}", message);
                        if let Err(e) = self.synthesize_text(&error_msg).await {
                            error!("Failed to synthesize error message: {}", e);
                        }
                    } else {
                        debug!("Transcription error [{}]: {}", code, message);
                    }
                }
                TranscriptionEvent::Partial { .. } => {
                    // Skip partial results by default
                    continue;
                }
            }
        }
        
        info!("TTS processor stopped");
    }
    
    /// Synthesize text using the configured engine
    async fn synthesize_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if text.trim().is_empty() {
            return Ok(());
        }
        
        // Use lower volume for background announcements
        let options = SynthesisOptions {
            volume: Some(0.6),
            ..Default::default()
        };
        
        match self.engine.synthesize(text, Some(options)).await? {
            SynthesisEvent::AudioData { synthesis_id, data, sample_rate, channels } => {
                info!(
                    "Synthesized text [{}]: {} bytes at {} Hz, {} channels",
                    synthesis_id, data.len(), sample_rate, channels
                );
                
                // In a full implementation, this would play the audio
                // For now, we could save to a temp file or send to audio system
                self.handle_audio_output(data).await?;
            }
            SynthesisEvent::Failed { synthesis_id, error } => {
                warn!("Synthesis failed [{}]: {}", synthesis_id, error);
            }
            _ => {
                debug!("Unexpected synthesis event");
            }
        }
        
        Ok(())
    }
    
    /// Handle synthesized audio output
    async fn handle_audio_output(&self, audio_data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        // In a production system, this would:
        // 1. Send audio to the system audio output
        // 2. Queue for playback through CPAL or similar
        // 3. Save to temporary files for immediate playback
        
        // For this implementation, we'll just log the audio data size
        debug!("Generated {} bytes of TTS audio", audio_data.len());
        
        // Optionally save to temp file for testing
        if self.config.engine_options.get("save_audio").map(|v| v == "true").unwrap_or(false) {
            let filename = format!("/tmp/tts_output_{}.wav", chrono::Utc::now().timestamp_millis());
            tokio::fs::write(&filename, &audio_data).await?;
            debug!("Saved TTS audio to: {}", filename);
        }
        
        Ok(())
    }
    
    /// Update TTS configuration
    pub async fn update_config(&mut self, config: TtsConfig) -> Result<(), Box<dyn std::error::Error>> {
        self.config = config.clone();
        self.is_enabled = config.enabled;
        
        if self.is_enabled {
            self.engine.initialize(config).await?;
            info!("TTS configuration updated");
        }
        
        Ok(())
    }
    
    /// Get current configuration
    pub fn config(&self) -> &TtsConfig {
        &self.config
    }
    
    /// Check if TTS is enabled
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }
}

/// Stub implementation when TTS features are disabled
#[cfg(not(feature = "tts"))]
pub struct TtsProcessor;

#[cfg(not(feature = "tts"))]
impl TtsProcessor {
    pub async fn new_with_espeak(
        _config: (),
        _transcription_rx: mpsc::Receiver<TranscriptionEvent>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self)
    }
    
    pub async fn run(self) {
        // No-op when TTS is disabled
    }
}

/// Configuration for TTS integration
#[derive(Debug, Clone)]
pub struct TtsIntegrationConfig {
    #[cfg(feature = "tts")]
    pub tts_config: TtsConfig,
    pub announce_errors: bool,
    pub announce_final_transcriptions: bool,
    pub save_audio_files: bool,
}

impl Default for TtsIntegrationConfig {
    fn default() -> Self {
        #[cfg(feature = "tts")]
        let mut tts_config = TtsConfig::default();
        #[cfg(feature = "tts")]
        {
            tts_config.volume = Some(0.6); // Lower volume for background announcements
        }
        
        Self {
            #[cfg(feature = "tts")]
            tts_config,
            announce_errors: false,
            announce_final_transcriptions: true,
            save_audio_files: false,
        }
    }
}