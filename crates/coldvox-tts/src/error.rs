//! Error types for TTS functionality

use thiserror::Error;

/// TTS error types
#[derive(Error, Debug)]
pub enum TtsError {
    /// Engine is not available or not installed
    #[error("TTS engine not available: {0}")]
    EngineNotAvailable(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// Voice not found or not supported
    #[error("Voice not found: {0}")]
    VoiceNotFound(String),
    
    /// Synthesis failed
    #[error("Synthesis failed: {0}")]
    SynthesisError(String),
    
    /// Audio output error
    #[error("Audio output error: {0}")]
    AudioError(String),
    
    /// IO error (file operations, process spawning, etc.)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Engine initialization error
    #[error("Engine initialization failed: {0}")]
    InitializationError(String),
    
    /// Engine is busy (another synthesis in progress)
    #[error("Engine is busy")]
    EngineBusy,
    
    /// Invalid text input
    #[error("Invalid text input: {0}")]
    InvalidInput(String),
    
    /// Engine-specific error
    #[error("Engine error ({engine}): {message}")]
    EngineSpecific {
        engine: String,
        message: String,
    },
}

/// Result type for TTS operations
pub type TtsResult<T> = Result<T, TtsError>;