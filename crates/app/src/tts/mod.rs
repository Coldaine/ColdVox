//! TTS integration module for ColdVox app

pub mod processor;

pub use processor::{TtsProcessor, TtsIntegrationConfig};

// Re-export TTS types when available
#[cfg(feature = "tts")]
pub use coldvox_tts::{TtsConfig, TtsEngine, SynthesisEvent, SynthesisOptions, VoiceInfo};

#[cfg(feature = "tts-espeak")]
pub use coldvox_tts_espeak::EspeakEngine;