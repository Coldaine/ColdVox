use std::sync::Arc;

use coldvox_audio::SharedAudioFrame;
use coldvox_telemetry::{FpsTracker, PipelineMetrics};
use coldvox_vad::{UnifiedVadConfig, VadEvent};
use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace};

use super::vad_adapter::VadAdapter;

pub struct VadProcessor {
    adapter: VadAdapter,
    audio_rx: broadcast::Receiver<SharedAudioFrame>,
    event_tx: Sender<VadEvent>,
    metrics: Option<Arc<PipelineMetrics>>,
    fps_tracker: FpsTracker,
    frames_processed: u64,
    events_generated: u64,
}

impl VadProcessor {
    pub fn new(
        config: UnifiedVadConfig,
    audio_rx: broadcast::Receiver<SharedAudioFrame>,
        event_tx: Sender<VadEvent>,
        metrics: Option<Arc<PipelineMetrics>>,
    ) -> Result<Self, String> {
        let adapter = VadAdapter::new(config)?;

        Ok(Self {
            adapter,
            audio_rx,
            event_tx,
            metrics,
            fps_tracker: FpsTracker::new(),
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

    async fn process_frame(&mut self, frame: SharedAudioFrame) {
        trace!(
            "VAD: Processing frame {:?} with {} samples",
            frame.timestamp,
            frame.samples.len()
        );

        if let Some(metrics) = &self.metrics {
            if let Some(fps) = self.fps_tracker.tick() {
                metrics.update_vad_fps(fps);
            }
        }

        // Process i16 samples directly (zero-copy from SharedAudioFrame)
        match self.adapter.process(&frame.samples) {
            Ok(Some(event)) => {
                self.events_generated += 1;

                // Log the specific VAD event
                match &event {
                    VadEvent::SpeechStart {
                        timestamp_ms,
                        energy_db,
                    } => {
                        debug!(
                            "VAD: Speech started at {}ms (energy: {:.2} dB)",
                            timestamp_ms, energy_db
                        );
                    }
                    VadEvent::SpeechEnd {
                        timestamp_ms,
                        duration_ms,
                        energy_db,
                    } => {
                        debug!(
                            "VAD: Speech ended at {}ms (duration: {}ms, energy: {:.2} dB)",
                            timestamp_ms, duration_ms, energy_db
                        );
                    }
                }

                debug!(
                    "VAD event: {:?} @ {}ms",
                    event,
                    match &event {
                        VadEvent::SpeechStart { timestamp_ms, .. } => *timestamp_ms,
                        VadEvent::SpeechEnd { timestamp_ms, .. } => *timestamp_ms,
                    }
                );

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

        if self.frames_processed.is_multiple_of(100) {
            tracing::debug!(
                "VAD: Received {} frames, processing active",
                self.frames_processed
            );
        }

        if self.frames_processed.is_multiple_of(1000) {
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
        audio_rx: broadcast::Receiver<SharedAudioFrame>,
        event_tx: Sender<VadEvent>,
        metrics: Option<Arc<PipelineMetrics>>,
    ) -> Result<JoinHandle<()>, String> {
        tracing::info!("VAD processor task spawning for mode: {:?}", config.mode);
        let processor = VadProcessor::new(config, audio_rx, event_tx, metrics)?;

        let handle = tokio::spawn(async move {
            processor.run().await;
        });

        Ok(handle)
    }
}
