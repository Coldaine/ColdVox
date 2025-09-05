// Re-export from the new Vosk crate
pub use coldvox_stt_vosk::{VoskTranscriber, AsyncVoskTranscriber};

// For backward compatibility, also re-export the default model path function
pub use coldvox_stt_vosk::default_model_path;
