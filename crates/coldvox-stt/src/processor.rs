//! STT processor gated by VAD events
//!
//! This module provides a generic STT processor that buffers audio during speech
//! segments and processes transcription when speech ends. The processor is designed
//! to work with any VAD system and any STT implementation.

use crate::constants::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use crate::StreamingStt;
use crate::helpers::*;
use coldvox_telemetry::{stt_metrics::SttPerformanceMetrics, pipeline_metrics::PipelineMetrics};
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
use std::sync::atomic::Ordering;
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
    },
}

/// STT processor metrics
#[derive(Debug, Clone, Default)]
pub struct SttMetrics {
    /// Total frames received
    pub frames_in: u64,
    /// Total frames processed
    pub frames_out: u64,
    /// Total frames dropped due to overflow
    pub frames_dropped: u64,
    /// Number of partial transcriptions
    pub partial_count: u64,
    /// Number of final transcriptions
    pub final_count: u64,
    /// Number of errors
    pub error_count: u64,
    /// Current queue depth
    pub queue_depth: usize,
    /// Time since last STT event
    pub last_event_time: Option<Instant>,
    /// Cumulative total latency in microseconds
    pub total_latency_us: u64,
}

impl SttMetrics {
    /// Get total latency in microseconds
    pub fn total_latency_us(&self) -> u64 {
        self.total_latency_us
    }
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
    /// STT performance metrics
    stt_metrics: Arc<SttPerformanceMetrics>,
    /// Pipeline metrics
    pipeline_metrics: Arc<PipelineMetrics>,
    /// Configuration
    config: TranscriptionConfig,
    /// Audio buffer manager
    buffer_mgr: Option<AudioBufferManager>,
    /// Event emitter
    emitter: EventEmitter,
}

impl<T: StreamingStt + Send> SttProcessor<T> {
    /// Create a new STT processor
    pub fn new(
        audio_rx: broadcast::Receiver<AudioFrame>,
        vad_event_rx: mpsc::Receiver<VadEvent>,
        event_tx: mpsc::Sender<TranscriptionEvent>,
        stt_engine: T,
        config: TranscriptionConfig,
        stt_metrics: Arc<SttPerformanceMetrics>,
        pipeline_metrics: Arc<PipelineMetrics>,
    ) -> Self {
        // Check if STT is enabled
        if !config.enabled {
            info!("STT processor disabled in configuration");
        }

        let metrics = Arc::new(parking_lot::RwLock::new(SttMetrics::default()));
        Self {
            audio_rx,
            vad_event_rx,
            event_tx: event_tx.clone(),
            stt_engine,
            state: UtteranceState::Idle,
            metrics: metrics.clone(),
            stt_metrics,
            pipeline_metrics,
            config,
            buffer_mgr: None,
            emitter: EventEmitter::new(event_tx, metrics, stt_metrics.clone(), pipeline_metrics.clone()),
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
                    self.handle_audio_frame(frame).await;
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

        self.pipeline_metrics.speech_segments_count.fetch_add(1, Ordering::Relaxed);

        // Store the start time as Instant for duration calculations
        let start_instant = Instant::now();

        self.buffer_mgr = Some(AudioBufferManager::new(start_instant));

        self.state = UtteranceState::SpeechActive {
            started_at: start_instant,
        };

        // Reset STT engine for new utterance
        self.stt_engine.reset().await;

        info!(target: "stt", "Started buffering audio for new utterance");
    }

    /// Handle speech end event
    async fn handle_speech_end(&mut self, _timestamp_ms: u64, _duration_ms: Option<u64>) {
        debug!(target: "stt", "Starting handle_speech_end()");

        // Process the buffered audio all at once
        if let Some(mgr) = &mut self.buffer_mgr {
            mgr.log_processing_info();

            if mgr.buffer_size() > 0 {
                // Process chunks and emit events
                for chunk in mgr.chunks(SAMPLE_RATE_HZ as usize) {
                    if let Some(event) = self.stt_engine.on_speech_frame(chunk).await {
                        self.emitter.emit(event).await.ok();
                    }
                }
                debug!(target: "stt", "Finished streaming frames to STT engine");
                let mut metrics = self.metrics.write();
                metrics.frames_out += mgr.frames_buffered();
                metrics.last_event_time = Some(Instant::now());
                mgr.clear();
            }

            // Finalize to get any remaining transcription
            let result = self.stt_engine.on_speech_end().await;
            match result {
                Some(event) => {
                    debug!(target: "stt", "STT engine returned Final event: {:?}", event);
                    self.emitter.emit(event).await.ok();
                }
                None => {
                    debug!(target: "stt", "STT engine returned None on speech end");
                }
            }

            // Clear the buffer manager
            self.buffer_mgr = None;
        }

        self.state = UtteranceState::Idle;
    }

    /// Handle the result from finalizing the STT engine
    async fn handle_finalization_result(&self, result: Option<TranscriptionEvent>) {
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

    /// Handle incoming audio frame
    async fn handle_audio_frame(&mut self, frame: AudioFrame) {
        // Update metrics
        self.metrics.write().frames_in += 1;
        self.pipeline_metrics.capture_frames.fetch_add(1, Ordering::Relaxed);

        // Only buffer if speech is active
        self.buffer_audio_frame_if_speech_active(frame);
    }

    /// Buffer an audio frame if speech is active and log progress periodically
    fn buffer_audio_frame_if_speech_active(&mut self, frame: AudioFrame) {
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
        match tokio::time::timeout(
            std::time::Duration::from_secs(SEND_TIMEOUT_SECONDS),
            self.event_tx.send(event),
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StreamingStt, TranscriptionConfig};
    use std::sync::Arc;
    use std::sync::Mutex;
    use tokio::sync::{mpsc};
    use std::time::Instant;

    // Helper to get text from event for assertions
    fn get_text(event: &TranscriptionEvent) -> Option<&str> {
        match event {
            TranscriptionEvent::Partial { text, .. } => Some(text),
            TranscriptionEvent::Final { text, .. } => Some(text),
            TranscriptionEvent::Error { .. } => None,
        }
    }

    // Mock STT implementation for testing
    struct MockStt {
        utterance_count: Arc<Mutex<u64>>,
    }

    impl MockStt {
        fn new() -> Self {
            Self {
                utterance_count: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait::async_trait]
    impl StreamingStt for MockStt {
        async fn on_speech_frame(&mut self, _samples: &[i16]) -> Option<TranscriptionEvent> {
            let count = *self.utterance_count.lock().unwrap();
            Some(TranscriptionEvent::Partial {
                utterance_id: count,
                text: "partial mock".to_string(),
                t0: Some(0.0),
                t1: Some(1.0),
            })
        }

        async fn on_speech_end(&mut self) -> Option<TranscriptionEvent> {
            let mut count = self.utterance_count.lock().unwrap();
            let id = *count;
            *count += 1;
            Some(TranscriptionEvent::Final {
                utterance_id: id,
                text: "final mock".to_string(),
                words: None,
            })
        }

        async fn reset(&mut self) {
            let mut count = self.utterance_count.lock().unwrap();
            *count = 0;
        }
    }


    #[tokio::test]
    async fn test_processor_basic_flow() {
        // Setup event channel for testing
        let (event_tx, mut event_rx) = mpsc::channel(10);

        // Create mock STT
        let mut mock_stt = MockStt::new();

        // Config
        let config = TranscriptionConfig {
            enabled: true,
            model_path: "mock".to_string(),
            partial_results: true,
            include_words: false,
            ..Default::default()
        };

        // Create dummy receivers (we'll call methods directly)
        let (_audio_tx, audio_rx) = tokio::sync::broadcast::channel(10);
        let (_vad_tx, vad_rx) = mpsc::channel(10);

        // Create processor
        let stt_metrics = Arc::new(SttPerformanceMetrics::default());
        let pipeline_metrics = Arc::new(PipelineMetrics::default());
        let mut processor = SttProcessor::new(
            audio_rx,
            vad_rx,
            event_tx,
            mock_stt,
            config,
            stt_metrics,
            pipeline_metrics,
        );

        // Test SpeechStart - should initialize buffer and reset STT
        let speech_start_timestamp = 100u64;
        processor.handle_speech_start(speech_start_timestamp).await;
        
        assert!(processor.buffer_mgr.is_some(), "Buffer manager should be initialized");
        assert!(matches!(processor.state, UtteranceState::SpeechActive { .. }), "State should be SpeechActive");

        // Test audio frame - should buffer if active
        let frame = AudioFrame {
            data: vec![42i16; 160], // 10ms of audio
            timestamp_ms: 150,
            sample_rate: 16000,
        };
        processor.handle_audio_frame(frame.clone()).await;
        
        let mgr = processor.buffer_mgr.as_ref().unwrap();
        assert_eq!(mgr.buffer_size(), 160, "Frame should be buffered");
        assert_eq!(mgr.frames_buffered(), 1, "One frame should be counted");

        // Verify metrics - frames_in incremented
        let metrics = processor.metrics();
        assert_eq!(metrics.frames_in, 1, "Frame input should be counted");

        // Test SpeechEnd - should process buffer, emit events, clear state
        processor.handle_speech_end(200, Some(100)).await;

        // Verify events emitted
        let mut events = vec![];
        if let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }
        if let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }

        assert_eq!(events.len(), 2, "Should receive partial and final events");
        assert!(events.iter().any(|e| get_text(e) == Some("partial mock")), "Should have partial event");
        assert!(events.iter().any(|e| get_text(e) == Some("final mock")), "Should have final event");

        // Verify buffer cleared and state reset
        assert!(processor.buffer_mgr.is_none(), "Buffer should be cleared");
        assert!(matches!(processor.state, UtteranceState::Idle), "State should be Idle");

        // Verify metrics updated
        let metrics = processor.metrics();
        assert_eq!(metrics.frames_in, 1, "Input frames counted");
        assert_eq!(metrics.frames_out, 1, "Output frames processed");
        assert_eq!(metrics.final_count, 1, "Final event counted");
        assert!(metrics.last_event_time.is_some(), "Last event time set");
    }
}