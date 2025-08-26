
use tokio::sync::{broadcast, mpsc};
use crate::audio::vad_processor::AudioFrame;
use crate::stt::vosk::VoskTranscriber;
use crate::stt::Transcriber;
use crate::vad::types::VadEvent;

pub struct SttProcessor {
    audio_rx: broadcast::Receiver<AudioFrame>,
    vad_event_rx: mpsc::Receiver<VadEvent>,
    transcriber: VoskTranscriber,
    is_speaking: bool,
}

impl SttProcessor {
    pub fn new(
        audio_rx: broadcast::Receiver<AudioFrame>,
        vad_event_rx: mpsc::Receiver<VadEvent>,
    ) -> Result<Self, String> {
        // In the future, model_path and sample_rate will come from config
        // IMPORTANT: You must provide a valid path to a Vosk model directory.
        let transcriber = VoskTranscriber::new("vosk-model-en-us-0.22-lgraph", 16000.0)?;
        Ok(Self {
            audio_rx,
            vad_event_rx,
            transcriber,
            is_speaking: false,
        })
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                // Listen for a VAD event
                Some(event) = self.vad_event_rx.recv() => {
                    match event {
                        VadEvent::SpeechStart { .. } => {
                            tracing::debug!("STT processor received SpeechStart");
                            self.is_speaking = true;
                        }
                        VadEvent::SpeechEnd { .. } => {
                            tracing::debug!("STT processor received SpeechEnd");
                            self.is_speaking = false;
                            if let Ok(Some(text)) = self.transcriber.finalize() {
                                tracing::info!(target: "stt", "Final transcription: {}", text);
                            }
                        }
                    }
                }

                // Listen for an audio frame
                Ok(frame) = self.audio_rx.recv() => {
                    if self.is_speaking {
                        if let Ok(Some(partial_text)) = self.transcriber.accept_pcm16(&frame.data) {
                             tracing::info!(target: "stt", "Partial: {}", partial_text);
                        }
                    }
                }
                else => break, // Exit loop if a channel closes
            }
        }
        tracing::info!("STT processor task shutting down.");
    }
}
