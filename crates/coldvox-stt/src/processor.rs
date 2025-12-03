//! STT processor gated by VAD events
//!
//! This module provides a generic STT processor that buffers audio during speech
//! segments and processes transcription when speech ends. The processor is designed
//! to work with any VAD system and any STT implementation.

use crate::constants::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use crate::StreamingStt;
use coldvox_telemetry::SttPerformanceMetrics;
/// Minimal audio frame type (i16 PCM) used by the generic STT processor
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub data: Vec<i16>,
    pub timestamp_ms: u64,
    pub sample_rate: u32,
}

/// Minimal VAD event type mirrored here to avoid cross-crate deps
#[derive(Debug, Clone, Copy)]
pub enum VadEvent {
    SpeechStart { timestamp_ms: u64 },
    SpeechEnd { timestamp_ms: u64, duration_ms: u64 },
}
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info};

/// STT processor state
#[derive(Debug, Clone)]
pub enum UtteranceState {
    /// No speech detected
    Idle,
    /// Speech is active, buffering audio
    SpeechActive {
        /// Timestamp when speech started
        started_at: Instant,
        /// Buffered audio frames for this utterance
        audio_buffer: Vec<i16>,
        /// Number of frames buffered
        frames_buffered: u64,
    },
}

/// Generic STT processor that works with any streaming STT implementation
pub struct SttProcessor<T: StreamingStt> {
    /// Audio frame receiver (broadcast from pipeline)
    audio_rx: broadcast::Receiver<AudioFrame>,
    /// VAD event receiver
    vad_event_rx: mpsc::Receiver<VadEvent>,
    /// Transcription event sender
    event_tx: mpsc::Sender<TranscriptionEvent>,
    /// Streaming STT implementation
    stt_engine: T,
    /// Current utterance state
    state: UtteranceState,
    /// Metrics
    metrics: SttPerformanceMetrics,
    /// Configuration
    config: TranscriptionConfig,
}

impl<T: StreamingStt + Send> SttProcessor<T> {
    /// Create a new STT processor
    pub fn new(
        audio_rx: broadcast::Receiver<AudioFrame>,
        vad_event_rx: mpsc::Receiver<VadEvent>,
        event_tx: mpsc::Sender<TranscriptionEvent>,
        stt_engine: T,
        config: TranscriptionConfig,
    ) -> Self {
        // Check if STT is enabled
        if !config.enabled {
            info!("STT processor disabled in configuration");
        }

        Self {
            audio_rx,
            vad_event_rx,
            event_tx,
            stt_engine,
            state: UtteranceState::Idle,
            metrics: SttPerformanceMetrics::new(),
            config,
        }
    }

    /// Get current metrics
    pub fn metrics(&self) -> SttPerformanceMetrics {
        self.metrics.clone()
    }

    /// Run the STT processor loop
    pub async fn run(mut self) {
        // Exit early if STT is disabled
        if !self.config.enabled {
            info!(
                target: "stt",
                "STT processor disabled - exiting immediately"
            );
            return;
        }

        info!(
            target: "stt",
            "STT processor starting (model: {}, partials: {}, words: {})",
            self.config.model_path,
            self.config.partial_results,
            self.config.include_words
        );

        loop {
            tokio::select! {
                // Listen for VAD events
                Some(event) = self.vad_event_rx.recv() => {
                    self.metrics.increment_requests();
                    match event {
                        VadEvent::SpeechStart { timestamp_ms } => {
                            debug!(target: "stt", "Received SpeechStart event @ {}ms", timestamp_ms);
                            self.handle_speech_start(timestamp_ms).await;
                        }
                        VadEvent::SpeechEnd { timestamp_ms, duration_ms } => {
                            debug!(target: "stt", "Received SpeechEnd event @ {}ms (duration={}ms)", timestamp_ms, duration_ms);
                            self.handle_speech_end(timestamp_ms, Some(duration_ms)).await;
                        }
                    }
                }

                // Listen for audio frames
                Ok(frame) = self.audio_rx.recv() => {
                    self.metrics.increment_requests();
                    self.handle_audio_frame(frame);
                }

                else => {
                    info!(target: "stt", "STT processor shutting down: all channels closed");
                    break;
                }
            }
        }
        // Log final metrics
        let (_, accuracy, _, operational) = self.metrics.snapshot();
        info!(
            target: "stt",
            "STT processor final stats - requests: {}, partials: {}, finals: {}, errors: {}",
            operational.request_count,
            accuracy.partial_count,
            accuracy.final_count,
            operational.error_count
        );
    }

    /// Handle speech start event
    async fn handle_speech_start(&mut self, timestamp_ms: u64) {
        debug!(target: "stt", "STT processor received SpeechStart at {}ms", timestamp_ms);

        // Store the start time as Instant for duration calculations
        let start_instant = Instant::now();

        self.state = UtteranceState::SpeechActive {
            started_at: start_instant,
            audio_buffer: Vec::with_capacity(
                SAMPLE_RATE_HZ as usize * DEFAULT_BUFFER_DURATION_SECONDS,
            ),
            frames_buffered: 0,
        };

        // Reset STT engine for new utterance
        self.stt_engine.reset().await;

        info!(target: "stt", "Started buffering audio for new utterance");
    }

    /// Handle speech end event
    async fn handle_speech_end(&mut self, _timestamp_ms: u64, _duration_ms: Option<u64>) {
        debug!(target: "stt", "Starting handle_speech_end()");
        let _guard = coldvox_telemetry::TimingGuard::new(&self.metrics, |m, d| {
            m.record_end_to_end_latency(d)
        });

        if let UtteranceState::SpeechActive { audio_buffer, .. } = &self.state {
            if !audio_buffer.is_empty() {
                for chunk in audio_buffer.chunks(DEFAULT_CHUNK_SIZE_SAMPLES) {
                    if let Some(event) = self.stt_engine.on_speech_frame(chunk).await {
                        self.send_event(event).await;
                    }
                }
            }

            match self.stt_engine.on_speech_end().await {
                Some(event) => {
                    self.metrics.record_transcription_success();
                    self.send_event(event).await;
                }
                None => {
                    self.metrics.record_transcription_failure();
                    debug!(target: "stt", "STT engine returned None on speech end");
                }
            }
        }

        self.state = UtteranceState::Idle;
    }

    /// Handle incoming audio frame
    fn handle_audio_frame(&mut self, frame: AudioFrame) {
        if let UtteranceState::SpeechActive { audio_buffer, .. } = &mut self.state {
            audio_buffer.extend_from_slice(&frame.data);
            let utilization = (audio_buffer.len() * 100) / audio_buffer.capacity();
            self.metrics.update_buffer_utilization(utilization as u64);
        }
    }

    /// Send transcription event
    async fn send_event(&self, event: TranscriptionEvent) {
        match &event {
            TranscriptionEvent::Partial { .. } => self.metrics.record_partial_transcription(),
            TranscriptionEvent::Final { .. } => self.metrics.record_final_transcription(),
            TranscriptionEvent::Error { .. } => self.metrics.record_error(),
        }

        if tokio::time::timeout(std::time::Duration::from_secs(5), self.event_tx.send(event))
            .await
            .is_err()
        {
            self.metrics.record_error();
            debug!(target: "stt", "Event channel closed or send timed out");
        }
    }
}
