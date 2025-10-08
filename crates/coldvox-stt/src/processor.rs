//! STT processor gated by VAD events
//!
//! This module provides a generic STT processor that buffers audio during speech
//! segments and processes transcription when speech ends. The processor is designed
//! to work with any VAD system and any STT implementation.

use crate::constants::*;
use crate::types::{SttMetrics, TranscriptionConfig, TranscriptionEvent};
use crate::StreamingStt;
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
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

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
    metrics: Arc<parking_lot::RwLock<SttMetrics>>,
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
            metrics: Arc::new(parking_lot::RwLock::new(SttMetrics::default())),
            config,
        }
    }

    /// Get current metrics
    pub fn metrics(&self) -> SttMetrics {
        self.metrics.read().clone()
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
                    self.handle_audio_frame(frame);
                }

                else => {
                    info!(target: "stt", "STT processor shutting down: all channels closed");
                    break;
                }
            }
        }

        // Log final metrics
        let metrics = self.metrics.read();
        info!(
            target: "stt",
            "STT processor final stats - frames in: {}, out: {}, dropped: {}, partials: {}, finals: {}, errors: {}",
            metrics.frames_in,
            metrics.frames_out,
            metrics.frames_dropped,
            metrics.partial_count,
            metrics.final_count,
            metrics.error_count
        );
    }

    /// Handle speech start event
    async fn handle_speech_start(&mut self, timestamp_ms: u64) {
        debug!(target: "stt", "STT processor received SpeechStart at {}ms", timestamp_ms);

        // Store the start time as Instant for duration calculations
        let start_instant = Instant::now();

        self.state = UtteranceState::SpeechActive {
            started_at: start_instant,
            audio_buffer: Vec::with_capacity(SAMPLE_RATE_HZ as usize * DEFAULT_BUFFER_DURATION_SECONDS),
            frames_buffered: 0,
        };

        // Reset STT engine for new utterance
        self.stt_engine.reset().await;

        info!(target: "stt", "Started buffering audio for new utterance");
    }

    /// Handle speech end event
    async fn handle_speech_end(&mut self, _timestamp_ms: u64, _duration_ms: Option<u64>) {
        debug!(target: "stt", "Starting handle_speech_end()");

        // Process the buffered audio all at once
        if let UtteranceState::SpeechActive {
            audio_buffer,
            frames_buffered,
            ..
        } = &self.state
        {
            let buffer_size = audio_buffer.len();
            info!(
                target: "stt",
                "Processing buffered audio: {} samples ({:.2}s), {} frames",
                buffer_size,
                buffer_size as f32 / SAMPLE_RATE_HZ as f32,
                frames_buffered
            );

            if !audio_buffer.is_empty() {
                // Send the entire buffer to the STT engine
                // Stream model expects per-frame feeding; here we feed the whole buffered audio
                // in chunks to preserve event semantics.
                for chunk in audio_buffer.chunks(DEFAULT_CHUNK_SIZE_SAMPLES) {
                    // Process in 1-second chunks
                    if let Some(event) = self.stt_engine.on_speech_frame(chunk).await {
                        self.send_event(event).await;
                    }
                }
                debug!(target: "stt", "Finished streaming frames to STT engine");
                let mut metrics = self.metrics.write();
                metrics.frames_out += frames_buffered;
                metrics.last_event_time = Some(Instant::now());
            }

            // Finalize to get any remaining transcription
            let result = self.stt_engine.on_speech_end().await;
            match result {
                Some(event) => {
                    debug!(target: "stt", "STT engine returned Final event: {:?}", event);
                    self.send_event(event).await;
                    let mut metrics = self.metrics.write();
                    metrics.final_count += 1;
                    metrics.last_event_time = Some(Instant::now());
                }
                None => {
                    debug!(target: "stt", "STT engine returned None on speech end");
                }
            }
        }

        self.state = UtteranceState::Idle;
    }

    /// Handle incoming audio frame
    fn handle_audio_frame(&mut self, frame: AudioFrame) {
        // Update metrics
        self.metrics.write().frames_in += 1;

        // Only buffer if speech is active
        if let UtteranceState::SpeechActive {
            ref mut audio_buffer,
            ref mut frames_buffered,
            ..
        } = &mut self.state
        {
            // Buffer the audio frame (already i16 PCM)
            audio_buffer.extend_from_slice(&frame.data);
            *frames_buffered += 1;

            // Log periodically to show we're buffering
            if *frames_buffered % LOGGING_INTERVAL_FRAMES == 0 {
                debug!(
                    target: "stt",
                    "Buffering audio: {} frames, {} samples ({:.2}s)",
                    frames_buffered,
                    audio_buffer.len(),
                    audio_buffer.len() as f32 / SAMPLE_RATE_HZ as f32
                );
            }
        }
    }

    /// Send transcription event
    async fn send_event(&self, event: TranscriptionEvent) {
        // Log the event
        match &event {
            TranscriptionEvent::Partial { text, .. } => {
                info!(target: "stt", "Partial: {}", text);
                self.metrics.write().partial_count += 1;
            }
            TranscriptionEvent::Final { text, words, .. } => {
                let word_count = words.as_ref().map(|w| w.len()).unwrap_or(0);
                info!(target: "stt", "Final: {} (words: {})", text, word_count);
                self.metrics.write().final_count += 1;
            }
            TranscriptionEvent::Error { code, message } => {
                error!(target: "stt", "Error [{}]: {}", code, message);
                self.metrics.write().error_count += 1;
            }
        }

        // Send to channel with backpressure - wait if channel is full
        // Use timeout to prevent indefinite blocking
        match tokio::time::timeout(std::time::Duration::from_secs(SEND_TIMEOUT_SECONDS), self.event_tx.send(event))
            .await
        {
            Ok(Ok(())) => {
                // Successfully sent
            }
            Ok(Err(_)) => {
                // Channel closed
                debug!(target: "stt", "Event channel closed");
            }
            Err(_) => {
                // Timeout - consumer is too slow
                warn!(target: "stt", "Event channel send timed out after 5s - consumer too slow");
                self.metrics.write().frames_dropped += 1;
            }
        }
    }
}
