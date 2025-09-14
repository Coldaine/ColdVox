//! Core types for text-to-speech functionality

use serde::{Deserialize, Serialize};

/// TTS synthesis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// Enable/disable TTS synthesis
    pub enabled: bool,
    /// Default voice to use
    pub default_voice: Option<String>,
    /// Speaking rate (words per minute, typically 100-300)
    pub speech_rate: Option<u32>,
    /// Voice pitch (0.0-2.0, 1.0 is normal)
    pub pitch: Option<f32>,
    /// Volume (0.0-1.0)
    pub volume: Option<f32>,
    /// Audio output format
    pub output_format: AudioFormat,
    /// Engine-specific options
    pub engine_options: std::collections::HashMap<String, String>,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_voice: None,
            speech_rate: Some(180), // Reasonable default speaking rate
            pitch: Some(1.0),
            volume: Some(0.8),
            output_format: AudioFormat::Wav16bit,
            engine_options: std::collections::HashMap::new(),
        }
    }
}

/// Audio output format for synthesis
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AudioFormat {
    /// WAV format, 16-bit PCM
    Wav16bit,
    /// WAV format, 8-bit PCM
    Wav8bit,
    /// Raw PCM data, 16-bit
    Raw16bit,
    /// Raw PCM data, 8-bit
    Raw8bit,
}

/// Voice information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    /// Unique voice identifier
    pub id: String,
    /// Human-readable voice name
    pub name: String,
    /// Language code (e.g., "en-US", "fr-FR")
    pub language: String,
    /// Gender (if available)
    pub gender: Option<VoiceGender>,
    /// Age category (if available)
    pub age: Option<VoiceAge>,
    /// Engine-specific properties
    pub properties: std::collections::HashMap<String, String>,
}

/// Voice gender categories
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VoiceGender {
    Male,
    Female,
    Neutral,
    Unknown,
}

/// Voice age categories
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VoiceAge {
    Child,
    Adult,
    Senior,
    Unknown,
}

/// Options for individual synthesis requests
#[derive(Debug, Clone)]
pub struct SynthesisOptions {
    /// Override voice for this synthesis
    pub voice: Option<String>,
    /// Override speech rate for this synthesis
    pub speech_rate: Option<u32>,
    /// Override pitch for this synthesis
    pub pitch: Option<f32>,
    /// Override volume for this synthesis
    pub volume: Option<f32>,
    /// Mark as high priority (interrupt current synthesis)
    pub high_priority: bool,
}

impl Default for SynthesisOptions {
    fn default() -> Self {
        Self {
            voice: None,
            speech_rate: None,
            pitch: None,
            volume: None,
            high_priority: false,
        }
    }
}