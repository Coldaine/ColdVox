//! TTS engine abstraction and synthesis events

use crate::types::{TtsConfig, VoiceInfo, SynthesisOptions};
use crate::error::{TtsError, TtsResult};
use async_trait::async_trait;

/// Synthesis event types
#[derive(Debug, Clone)]
pub enum SynthesisEvent {
    /// Synthesis started for the given text
    Started {
        synthesis_id: u64,
        text: String,
        voice_id: String,
    },
    /// Audio data chunk available
    AudioData {
        synthesis_id: u64,
        data: Vec<u8>,
        sample_rate: u32,
        channels: u16,
    },
    /// Synthesis completed successfully
    Completed {
        synthesis_id: u64,
        total_duration_ms: u64,
    },
    /// Synthesis failed with error
    Failed {
        synthesis_id: u64,
        error: String,
    },
    /// Synthesis was cancelled/stopped
    Cancelled {
        synthesis_id: u64,
    },
}

/// Core TTS engine interface
/// 
/// Implementations provide specific TTS functionality (espeak, festival, etc.)
#[async_trait]
pub trait TtsEngine: Send + Sync {
    /// Get engine name/identifier
    fn name(&self) -> &str;
    
    /// Get engine version
    fn version(&self) -> &str;
    
    /// Initialize the engine with configuration
    async fn initialize(&mut self, config: TtsConfig) -> TtsResult<()>;
    
    /// Check if the engine is available on this system
    async fn is_available(&self) -> bool;
    
    /// Synthesize text to speech
    async fn synthesize(&mut self, text: &str, options: Option<SynthesisOptions>) -> TtsResult<SynthesisEvent>;
    
    /// Get available voices
    async fn list_voices(&self) -> TtsResult<Vec<VoiceInfo>>;
    
    /// Set current voice
    async fn set_voice(&mut self, voice_id: &str) -> TtsResult<()>;
    
    /// Stop current synthesis
    async fn stop_synthesis(&mut self) -> TtsResult<()>;
    
    /// Get current configuration
    fn config(&self) -> &TtsConfig;
    
    /// Shutdown the engine and cleanup resources
    async fn shutdown(&mut self) -> TtsResult<()>;
}