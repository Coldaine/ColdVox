use vosk::{Model, Recognizer, DecodingState, CompleteResult, PartialResult};

pub struct VoskTranscriber {
    recognizer: Recognizer,
}

impl VoskTranscriber {
    pub fn new(model_path: &str, sample_rate: f32) -> Result<Self, String> {
        let model = Model::new(model_path)
            .ok_or_else(|| format!("Failed to load Vosk model from: {}", model_path))?;
            
        let recognizer = Recognizer::new(&model, sample_rate)
            .ok_or_else(|| format!("Failed to create Vosk recognizer with sample rate: {}", sample_rate))?;
            
        Ok(Self { recognizer })
    }
}

impl super::Transcriber for VoskTranscriber {
    fn accept_pcm16(&mut self, pcm: &[i16]) -> Result<Option<String>, String> {
        // Pass the i16 samples directly - vosk expects i16, not bytes
        let state = self.recognizer.accept_waveform(pcm)
            .map_err(|e| format!("Vosk waveform acceptance failed: {:?}", e))?;
            
        match state {
            DecodingState::Finalized => {
                // Get final result when speech segment is complete
                let result = self.recognizer.result();
                Self::parse_complete_result(result)
            }
            DecodingState::Running => {
                // Get partial result for ongoing speech
                let partial = self.recognizer.partial_result();
                Self::parse_partial_result(partial)
            }
            DecodingState::Failed => {
                // Recognition failed for this chunk
                Err("Vosk recognition failed".to_string())
            }
        }
    }

    fn finalize(&mut self) -> Result<Option<String>, String> {
        let final_result = self.recognizer.final_result();
        Self::parse_complete_result(final_result)
    }
}

impl VoskTranscriber {
    fn parse_complete_result(result: CompleteResult) -> Result<Option<String>, String> {
        match result {
            CompleteResult::Single(single) => {
                let text = single.text;
                if text.trim().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(text.to_string()))
                }
            }
            CompleteResult::Multiple(multiple) => {
                // Take the first alternative if multiple are available
                if let Some(first) = multiple.alternatives.first() {
                    let text = first.text;
                    if text.trim().is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(text.to_string()))
                    }
                } else {
                    Ok(None)
                }
            }
        }
    }
    
    fn parse_partial_result(partial: PartialResult) -> Result<Option<String>, String> {
        let text = partial.partial;
        if text.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(format!("[partial] {}", text)))
        }
    }
}