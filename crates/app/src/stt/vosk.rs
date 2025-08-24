#[allow(unused)]
pub struct VoskTranscriber {
    // Placeholder for model/recognizer handles once the vosk crate is added.
}

impl VoskTranscriber {
    pub fn new(_model_path: &str, _sample_rate: f32) -> Result<Self, String> {
        // In a follow-up, load model and create recognizer.
        Ok(Self {})
    }
}

impl super::Transcriber for VoskTranscriber {
    fn accept_pcm16(&mut self, _pcm: &[i16]) -> Result<Option<String>, String> {
        // On next step: call recognizer.AcceptWaveform() and PartialResult/Result
        Ok(None)
    }

    fn finalize(&mut self) -> Result<Option<String>, String> {
        // On next step: return final result JSON/text
        Ok(None)
    }
}
