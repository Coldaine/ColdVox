//! Common helpers for STT processing to reduce boilerplate and centralize logic.

use crate::constants::*;
use crate::types::{SttMetrics, TranscriptionEvent};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use coldvox_telemetry::{pipeline_metrics::PipelineMetrics, stt_metrics::SttPerformanceMetrics};

/// Manages audio buffering for a single utterance.
pub struct AudioBufferManager {
    audio_buffer: Vec<i16>,
    frames_buffered: u64,
    started_at: Instant,
}

impl AudioBufferManager {
    pub fn new(started_at: Instant) -> Self {
        Self {
            audio_buffer: Vec::with_capacity(SAMPLE_RATE_HZ as usize * DEFAULT_BUFFER_DURATION_SECONDS),
            frames_buffered: 0,
            started_at,
        }
    }

    pub fn add_frame(&mut self, frame_data: &[i16]) {
        self.audio_buffer.extend_from_slice(frame_data);
        self.frames_buffered += 1;
        if self.frames_buffered % LOGGING_INTERVAL_FRAMES == 0 {
            self.log_buffering_progress();
        }
    }

    pub fn buffer_size(&self) -> usize {
        self.audio_buffer.len()
    }

    pub fn frames_buffered(&self) -> u64 {
        self.frames_buffered
    }

    pub fn chunks(&self, chunk_size: usize) -> impl Iterator<Item = &[i16]> {
        self.audio_buffer.chunks(chunk_size)
    }

    pub fn clear(&mut self) {
        self.audio_buffer.clear();
        self.frames_buffered = 0;
    }

    fn log_buffering_progress(&self) {
        debug!(
            target: "stt",
            "Buffering audio: {} frames, {} samples ({:.2}s)",
            self.frames_buffered,
            self.audio_buffer.len(),
            self.audio_buffer.len() as f32 / SAMPLE_RATE_HZ as f32
        );
    }

    pub fn log_processing_info(&self) {
        info!(
            target: "stt",
            "Processing buffered audio: {} samples ({:.2}s), {} frames",
            self.buffer_size(),
            self.buffer_size() as f32 / SAMPLE_RATE_HZ as f32,
            self.frames_buffered()
        );
    }
}

/// Handles sending transcription events and updating metrics.
pub struct EventEmitter {
    event_tx: mpsc::Sender<TranscriptionEvent>,
    metrics: Arc<parking_lot::RwLock<SttMetrics>>,
    stt_metrics: Arc<SttPerformanceMetrics>,
    pipeline_metrics: Arc<PipelineMetrics>,
}

impl EventEmitter {
    pub fn new(
        event_tx: mpsc::Sender<TranscriptionEvent>,
        metrics: Arc<parking_lot::RwLock<SttMetrics>>,
        stt_metrics: Arc<SttPerformanceMetrics>,
        pipeline_metrics: Arc<PipelineMetrics>,
    ) -> Self {
        Self { event_tx, metrics, stt_metrics, pipeline_metrics }
    }

    pub async fn emit(&self, event: TranscriptionEvent) -> Result<(), mpsc::error::SendError<TranscriptionEvent>> {
        self.update_metrics(&event);
        self.log_event(&event);

        match tokio::time::timeout(
            std::time::Duration::from_secs(SEND_TIMEOUT_SECONDS),
            self.event_tx.send(event),
        )
        .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                debug!(target: "stt", "Event channel closed");
                Err(e)
            }
            Err(_) => {
                warn!(target: "stt", "Event channel send timed out after 5s - consumer too slow");
                self.metrics.write().frames_dropped += 1;
                // Consider what to do on timeout, maybe return an error
                Ok(())
            }
        }
    }

    fn update_metrics(&self, event: &TranscriptionEvent) {
        let mut metrics = self.metrics.write();
        match event {
            TranscriptionEvent::Partial { .. } => metrics.partial_count += 1,
            TranscriptionEvent::Final { .. } => metrics.final_count += 1,
            TranscriptionEvent::Error { .. } => metrics.error_count += 1,
        }
    }

    fn log_event(&self, event: &TranscriptionEvent) {
        match event {
            TranscriptionEvent::Partial { text, .. } => info!(target: "stt", "Partial: {}", text),
            TranscriptionEvent::Final { text, words, .. } => {
                let word_count = words.as_ref().map_or(0, |w| w.len());
                info!(target: "stt", "Final: {} (words: {})", text, word_count);
            }
            TranscriptionEvent::Error { code, message } => {
                error!(target: "stt", "Error [{}]: {}", code, message);
            }
        }
    }
}
