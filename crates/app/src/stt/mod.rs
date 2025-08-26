// STT abstraction and optional engine implementations (feature-gated)

// Keep this module unreferenced until wired; compiling remains unaffected.

/// Minimal streaming transcription interface
pub trait Transcriber {
    /// Feed 16 kHz, mono, S16LE PCM samples.
    /// Returns Some(final_text_or_json) when an utterance completes, else None.
    fn accept_pcm16(&mut self, pcm: &[i16]) -> Result<Option<String>, String>;

    /// Signal end of input for the current utterance and get final result if any.
    fn finalize(&mut self) -> Result<Option<String>, String>;
}

#[cfg(feature = "vosk")]
pub mod vosk;

#[cfg(feature = "vosk")]
pub use vosk::VoskTranscriber;

#[cfg(feature = "vosk")]
pub mod processor;
