//! Text-to-speech abstraction layer for ColdVox
//!
//! This crate provides the foundational types and traits for text-to-speech functionality,
//! including synthesis events, configuration, and the base TTS trait.

use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};

pub mod types;
pub mod engine;
pub mod error;

pub use engine::{TtsEngine, SynthesisEvent};
pub use error::{TtsError, TtsResult};
pub use types::{TtsConfig, VoiceInfo, SynthesisOptions, VoiceGender, VoiceAge, AudioFormat};

/// Generates unique synthesis IDs
static SYNTHESIS_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique synthesis ID
pub fn next_synthesis_id() -> u64 {
    SYNTHESIS_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Text-to-speech synthesis interface
/// 
/// This trait defines the core interface for TTS engines.
/// Implementations should handle text input and produce audio output.
#[async_trait]
pub trait TextToSpeechSynthesizer: Send + Sync {
    /// Synthesize text to audio
    /// Returns synthesis events including audio data or errors
    async fn synthesize(&mut self, text: &str, options: Option<SynthesisOptions>) -> TtsResult<SynthesisEvent>;
    
    /// Get available voices for this engine
    async fn available_voices(&self) -> TtsResult<Vec<VoiceInfo>>;
    
    /// Set the current voice
    async fn set_voice(&mut self, voice_id: &str) -> TtsResult<()>;
    
    /// Get current configuration
    fn config(&self) -> &TtsConfig;
    
    /// Check if engine is ready for synthesis
    async fn is_ready(&self) -> bool;
    
    /// Stop any ongoing synthesis
    async fn stop(&mut self) -> TtsResult<()>;
}