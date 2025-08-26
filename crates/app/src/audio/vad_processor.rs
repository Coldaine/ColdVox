use crate::vad::config::UnifiedVadConfig;
use crate::vad::types::VadEvent;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{Sender};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use super::vad_adapter::VadAdapter;

#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub data: Vec<i16>,
    pub timestamp_ms: u64,
}

pub struct VadProcessor {
    adapter: VadAdapter,
    audio_rx: broadcast::Receiver<AudioFrame>,
    event_tx: Sender<VadEvent>,
    frames_processed: u64,
    events_generated: u64,
}

impl VadProcessor {
    pub fn new(
        config: UnifiedVadConfig,
        audio_rx: broadcast::Receiver<AudioFrame>,
        event_tx: Sender<VadEvent>,
    ) -> Result<Self, String> {
        let adapter = VadAdapter::new(config)?;

        Ok(Self {
            adapter,
            audio_rx,
            event_tx,
            frames_processed: 0,
            events_generated: 0,
        })
    }

    pub async fn run(mut self) {
        info!("VAD processor task started");

        // This loop will automatically exit when the sender side of the broadcast channel is dropped.
        while let Ok(frame) = self.audio_rx.recv().await {
            self.process_frame(frame).await;
        }

        info!(
            "VAD processor task shutting down. Frames processed: {}, Events generated: {}",
            self.frames_processed, self.events_generated
        );
    }

    async fn process_frame(&mut self, frame: AudioFrame) {
        match self.adapter.process(&frame.data) {
            Ok(Some(event)) => {
                self.events_generated += 1;

                if let Err(e) = self.event_tx.send(event).await {
                    error!("Failed to send VAD event: {}", e);
                }
            }
            Ok(None) => {
                // No event generated
            }
            Err(e) => {
                error!("VAD processing error: {}", e);
            }
        }

        self.frames_processed += 1;

        if self.frames_processed % 1000 == 0 {
            debug!(
                "VAD processor: {} frames processed, {} events generated, current state: {:?}",
                self.frames_processed,
                self.events_generated,
                self.adapter.current_state()
            );
        }
    }

    pub fn spawn(
        config: UnifiedVadConfig,
        audio_rx: broadcast::Receiver<AudioFrame>,
        event_tx: Sender<VadEvent>,
    ) -> Result<JoinHandle<()>, String> {
        let processor = VadProcessor::new(config, audio_rx, event_tx)?;

        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        Ok(handle)
    }
}
