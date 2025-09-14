//! Async STT processor using the StreamingStt trait.
//!
//! This is an incremental introduction alongside the existing synchronous
//! EventBasedTranscriber-based processor. Once stabilized, this can replace
//! the older implementation.

use crate::{StreamingStt, TranscriptionConfig, TranscriptionEvent};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info};

/// Minimal audio frame used by the streaming processor
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub data: Vec<i16>,
    pub timestamp_ms: u64,
    pub sample_rate: u32,
}

/// VAD events (duplicated to avoid cross-crate dependency churn during migration)
#[derive(Debug, Clone, Copy)]
pub enum VadEvent {
    SpeechStart { timestamp_ms: u64 },
    SpeechEnd { timestamp_ms: u64, duration_ms: u64 },
}

#[derive(Debug, Clone)]
pub enum UtteranceState {
    Idle,
    SpeechActive {
        started_at: Instant,
        frames_buffered: u64,
    },
}

#[derive(Debug, Clone, Default)]
pub struct StreamingMetrics {
    pub frames_in: u64,
    pub frames_forwarded: u64,
    pub frames_dropped: u64,
    pub partial_count: u64,
    pub final_count: u64,
    pub error_count: u64,
}

pub struct StreamingSttProcessor<T: StreamingStt> {
    audio_rx: broadcast::Receiver<AudioFrame>,
    vad_event_rx: mpsc::Receiver<VadEvent>,
    event_tx: mpsc::Sender<TranscriptionEvent>,
    engine: T,
    state: UtteranceState,
    metrics: Arc<parking_lot::RwLock<StreamingMetrics>>,
    config: TranscriptionConfig,
}

impl<T: StreamingStt> StreamingSttProcessor<T> {
    pub fn new(
        audio_rx: broadcast::Receiver<AudioFrame>,
        vad_event_rx: mpsc::Receiver<VadEvent>,
        event_tx: mpsc::Sender<TranscriptionEvent>,
        engine: T,
        config: TranscriptionConfig,
    ) -> Self {
        if !config.enabled {
            info!(target: "stt", "Streaming STT processor disabled in configuration");
        }
        Self {
            audio_rx,
            vad_event_rx,
            event_tx,
            engine,
            state: UtteranceState::Idle,
            metrics: Arc::new(parking_lot::RwLock::new(StreamingMetrics::default())),
            config,
        }
    }

    pub fn metrics(&self) -> StreamingMetrics {
        self.metrics.read().clone()
    }

    pub async fn run(mut self) {
        if !self.config.enabled {
            return;
        }
        info!(target: "stt", "Streaming STT processor starting (partials: {}, words: {})", self.config.partial_results, self.config.include_words);
        loop {
            tokio::select! {
                Some(vad) = self.vad_event_rx.recv() => {
                    match vad {
                        VadEvent::SpeechStart { timestamp_ms } => self.on_speech_start(timestamp_ms).await,
                        VadEvent::SpeechEnd { timestamp_ms, duration_ms } => self.on_speech_end(timestamp_ms, duration_ms).await,
                    }
                }
                Ok(frame) = self.audio_rx.recv() => {
                    self.on_audio_frame(frame).await;
                }
                else => { info!(target: "stt", "Streaming STT processor shutting down (channels closed)"); break; }
            }
        }
        let m = self.metrics.read();
        info!(target: "stt", "Streaming STT final stats frames_in={} forwarded={} dropped={} partials={} finals={} errors={}", m.frames_in, m.frames_forwarded, m.frames_dropped, m.partial_count, m.final_count, m.error_count);
    }

    async fn on_speech_start(&mut self, _timestamp_ms: u64) {
        debug!(target: "stt", "SpeechStart received");
        self.state = UtteranceState::SpeechActive {
            started_at: Instant::now(),
            frames_buffered: 0,
        };
        self.engine.reset().await;
    }

    async fn on_speech_end(&mut self, _timestamp_ms: u64, _duration_ms: u64) {
        debug!(target: "stt", "SpeechEnd received");
        if let Some(event) = self.engine.on_speech_end().await {
            self.forward_event(event).await;
        }
        self.state = UtteranceState::Idle;
    }

    async fn on_audio_frame(&mut self, frame: AudioFrame) {
        self.metrics.write().frames_in += 1;
        match self.state {
            UtteranceState::SpeechActive {
                ref mut frames_buffered,
                ..
            } => {
                *frames_buffered += 1;
                if let Some(evt) = self.engine.on_speech_frame(&frame.data).await {
                    self.forward_event(evt).await;
                }
            }
            UtteranceState::Idle => { /* discard frames until speech active */ }
        }
    }

    async fn forward_event(&self, event: TranscriptionEvent) {
        match &event {
            TranscriptionEvent::Partial { .. } => self.metrics.write().partial_count += 1,
            TranscriptionEvent::Final { .. } => self.metrics.write().final_count += 1,
            TranscriptionEvent::Error { .. } => self.metrics.write().error_count += 1,
        }
        if let Err(e) = self.event_tx.send(event).await {
            debug!(target: "stt", "Failed sending event (channel closed): {}", e);
        }
    }
}
