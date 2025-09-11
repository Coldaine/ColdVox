// Audio Buffering Strategy:
// The STT processor buffers all audio frames during speech segments (SpeechStart → SpeechEnd)
// and processes the entire buffer at once when speech ends. This provides better context
// for the speech recognition model, leading to more accurate transcriptions.
// Text injection happens immediately (0ms timeout) after transcription completes.

use crate::stt::{TranscriptionConfig, TranscriptionEvent};
use coldvox_audio::chunker::AudioFrame;
use coldvox_vad::types::VadEvent;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

#[cfg(feature = "vosk")]
use crate::stt::VoskTranscriber;

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
    /// Performance metrics (streaming STT)
    pub perf: PerformanceMetrics,

    // Enhanced telemetry metrics
    /// Last end-to-end processing latency (microseconds)
    pub last_e2e_latency_us: u64,
    /// Last engine processing time (microseconds)
    pub last_engine_time_us: u64,
    /// Last preprocessing time (microseconds)
    pub last_preprocessing_us: u64,
    /// Average confidence score (0-1000 for precision)
    pub avg_confidence_x1000: u64,
    /// Total confidence measurements count
    pub confidence_measurements: u64,
    /// Memory usage estimate (bytes)
    pub memory_usage_bytes: u64,
    /// Buffer utilization percentage (0-100)
    pub buffer_utilization_pct: u64,
}

/// Detailed performance metrics for streaming STT
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Total processing time spent in STT operations
    pub total_processing_time_ns: u128,
    /// Average processing latency per frame
    pub avg_frame_latency_ns: u64,
    /// Peak memory usage in bytes
    pub peak_memory_usage_bytes: usize,
    /// Current memory usage in bytes
    pub current_memory_usage_bytes: usize,
    /// Buffer allocation count
    pub buffer_allocations: u64,
    /// Buffer reallocation count
    pub buffer_reallocations: u64,
    /// Total audio samples processed
    pub total_samples_processed: u64,
    /// Processing throughput (samples/second)
    pub throughput_samples_per_sec: f64,
    /// Time spent waiting for audio frames
    pub audio_wait_time_ns: u128,
    /// Time spent processing audio frames
    pub processing_time_ns: u128,
}

pub struct SttProcessor<T: coldvox_stt::StreamingStt> {
    /// Audio frame receiver (broadcast from pipeline)
    audio_rx: broadcast::Receiver<AudioFrame>,
    /// VAD event receiver
    vad_event_rx: mpsc::Receiver<VadEvent>,
    /// Transcription event sender
    event_tx: mpsc::Sender<TranscriptionEvent>,
    /// Generic STT implementation
    transcriber: T,
    /// Current utterance state
    state: UtteranceState,
    /// Metrics
    metrics: Arc<parking_lot::RwLock<SttMetrics>>,
    /// Configuration
    config: TranscriptionConfig,
    /// Performance metrics for comprehensive monitoring
    performance_metrics: Option<Arc<crate::telemetry::SttPerformanceMetrics>>,
}

impl<T: coldvox_stt::StreamingStt> SttProcessor<T> {
    /// Create a new STT processor with any StreamingStt implementation
    pub fn new(
        audio_rx: broadcast::Receiver<AudioFrame>,
        vad_event_rx: mpsc::Receiver<VadEvent>,
        event_tx: mpsc::Sender<TranscriptionEvent>,
        transcriber: T,
        config: TranscriptionConfig,
    ) -> Result<Self, String> {
        // Check if STT is enabled
        if !config.enabled {
            tracing::info!("STT processor disabled in configuration");
        }

        Ok(Self {
            audio_rx,
            vad_event_rx,
            event_tx,
            transcriber,
            state: UtteranceState::Idle,
            metrics: Arc::new(parking_lot::RwLock::new(SttMetrics::default())),
            config,
            performance_metrics: None,
        })
    }

    /// Create with default configuration (backward compatibility)
    #[deprecated(note = "Use new() with a proper StreamingStt implementation")]
    pub fn new_with_default(
        audio_rx: broadcast::Receiver<AudioFrame>,
        vad_event_rx: mpsc::Receiver<VadEvent>,
    ) -> Result<Self, String> {
        // This is now a stub for backward compatibility
        // In practice, runtime.rs should use the plugin manager
        return Err("new_with_default is deprecated. Use plugin manager instead.".to_string());
    }

    /// Get current metrics
    pub fn metrics(&self) -> SttMetrics {
        self.metrics.read().clone()
    }

    /// Set performance metrics for comprehensive monitoring
    pub fn set_performance_metrics(
        &mut self,
        performance_metrics: Arc<crate::telemetry::SttPerformanceMetrics>,
    ) {
        self.performance_metrics = Some(performance_metrics);
    }

    /// Get performance metrics reference
    pub fn performance_metrics(&self) -> Option<&Arc<crate::telemetry::SttPerformanceMetrics>> {
        self.performance_metrics.as_ref()
    }

    /// Run the STT processor loop
    pub async fn run(mut self) {
        // Exit early if STT is disabled
        if !self.config.enabled {
            tracing::info!(
                target: "stt",
                "STT processor disabled - exiting immediately"
            );
            return;
        }

        tracing::info!(
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
                        VadEvent::SpeechStart { timestamp_ms, .. } => {
                            self.handle_speech_start(timestamp_ms).await;
                        }
                        VadEvent::SpeechEnd { timestamp_ms, duration_ms, .. } => {
                            self.handle_speech_end(timestamp_ms, Some(duration_ms)).await;
                        }
                    }
                }

                // Listen for audio frames
                Ok(frame) = self.audio_rx.recv() => {
                    self.handle_audio_frame(frame).await;
                }

                else => {
                    tracing::info!(target: "stt", "STT processor shutting down: all channels closed");
                    break;
                }
            }
        }

        // Log final metrics
        let metrics = self.metrics.read();
        tracing::info!(
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
        tracing::info!(target: "stt", "STT processor received SpeechStart at {}ms", timestamp_ms);

        // Store the start time as Instant for duration calculations
        let start_instant = Instant::now();

        self.state = UtteranceState::SpeechActive {
            started_at: start_instant,
            audio_buffer: Vec::with_capacity(16000 * 10), // Pre-allocate for up to 10 seconds
            frames_buffered: 0,
        };

        // Reset transcriber for new utterance
        self.transcriber.reset().await;

        tracing::info!(target: "stt", "Started buffering audio for new utterance");
    }

    /// Handle speech end event
    async fn handle_speech_end(&mut self, timestamp_ms: u64, duration_ms: Option<u64>) {
        tracing::info!(
            target: "stt",
            "STT processor received SpeechEnd at {}ms (duration: {:?}ms)",
            timestamp_ms,
            duration_ms
        );

        // Start timing the entire end-to-end process
        let e2e_start = Instant::now();

        // Process the buffered audio all at once
        if let UtteranceState::SpeechActive {
            audio_buffer,
            frames_buffered,
            started_at: _,
        } = &self.state
        {
            let buffer_size = audio_buffer.len();
            tracing::info!(
                target: "stt",
                "Processing buffered audio: {} samples ({:.2}s), {} frames",
                buffer_size,
                buffer_size as f32 / 16000.0,
                frames_buffered
            );

            // Record memory usage estimate
            let estimated_memory = buffer_size * std::mem::size_of::<i16>() + 1024; // Buffer + overhead
            if let Some(perf_metrics) = &self.performance_metrics {
                perf_metrics.update_memory_usage(estimated_memory as u64);
            }

            if !audio_buffer.is_empty() {
                // Time the preprocessing phase
                let preprocessing_start = Instant::now();

                // Calculate buffer utilization (assuming max 10 seconds)
                let max_samples = 16000 * 10;
                let _utilization = ((buffer_size * 100) / max_samples).min(100);

                let preprocessing_time = preprocessing_start.elapsed();

                // Retry logic with telemetry tracking - simplified for StreamingStt
                // Time the STT engine processing
                let engine_start = Instant::now();

                if let Some(event) = self.transcriber.on_speech_frame(audio_buffer).await {
                    let engine_time = engine_start.elapsed();
                    let delivery_start = Instant::now();

                    self.send_event(event.clone()).await;

                    let delivery_time = delivery_start.elapsed();
                    let e2e_time = e2e_start.elapsed();

                    // Update comprehensive metrics
                    self.update_timing_metrics(
                        e2e_time,
                        engine_time,
                        preprocessing_time,
                        delivery_time,
                    );

                    // Extract confidence if available and update accuracy metrics
                    self.update_accuracy_metrics(&event, true);

                    // Update basic metrics
                    let mut metrics = self.metrics.write();
                    metrics.frames_out += frames_buffered;
                    metrics.last_event_time = Some(Instant::now());
                } else {
                    tracing::debug!(target: "stt", "No transcription from buffered audio");
                }
            }

            // Finalize to get any remaining transcription
            {
                if let Some(event) = self.transcriber.on_speech_end().await {
                    self.send_event(event.clone()).await;

                    // Update accuracy metrics
                    self.update_accuracy_metrics(&event, true);

                    // Update metrics
                    let mut metrics = self.metrics.write();
                    metrics.final_count += 1;
                    metrics.last_event_time = Some(Instant::now());
                } else {
                    tracing::debug!(target: "stt", "No final transcription available");
                }
            }
        }

        self.state = UtteranceState::Idle;
    }

    /// Handle incoming audio frame
    async fn handle_audio_frame(&mut self, frame: AudioFrame) {
        // Update metrics
        self.metrics.write().frames_in += 1;

        // Check if we're in streaming mode and speech is active
        let is_streaming_and_active =
            self.config.streaming && matches!(self.state, UtteranceState::SpeechActive { .. });

        if is_streaming_and_active {
            // Streaming mode: Process audio chunks incrementally
            // Convert f32 samples to i16 (PCM)
            let i16_samples: Vec<i16> = frame
                .samples
                .iter()
                .map(|&sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .collect();

            if let Some(event) = self.transcriber.on_speech_frame(&i16_samples).await {
                self.send_event(event).await;
                let mut metrics = self.metrics.write();
                metrics.frames_out += 1;
                metrics.last_event_time = Some(Instant::now());
            }

            // Update frame count for streaming mode
            if let UtteranceState::SpeechActive {
                ref mut frames_buffered,
                ..
            } = &mut self.state
            {
                *frames_buffered += 1;

                // Log periodically
                if *frames_buffered % 100 == 0 {
                    tracing::debug!(
                        target: "stt",
                        "Streaming audio: {} frames processed",
                        frames_buffered
                    );
                }
            }
        } else if let UtteranceState::SpeechActive {
            ref mut audio_buffer,
            ref mut frames_buffered,
            ..
        } = &mut self.state
        {
            // Batch mode: Buffer the audio frame
            // Convert f32 samples to i16 (PCM)
            let i16_samples: Vec<i16> = frame
                .samples
                .iter()
                .map(|&sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .collect();

            audio_buffer.extend_from_slice(&i16_samples);
            *frames_buffered += 1;

            // Log periodically to show we're buffering
            if *frames_buffered % 100 == 0 {
                tracing::debug!(
                    target: "stt",
                    "Buffering audio: {} frames, {} samples ({:.2}s)",
                    frames_buffered,
                    audio_buffer.len(),
                    audio_buffer.len() as f32 / 16000.0
                );
            }
        }
    }

    /// Send transcription event
    async fn send_event(&self, event: TranscriptionEvent) {
        // Log the event
        match &event {
            TranscriptionEvent::Partial { text, .. } => {
                tracing::info!(target: "stt", "Partial: {}", text);
            }
            TranscriptionEvent::Final { text, words, .. } => {
                let word_count = words.as_ref().map(|w| w.len()).unwrap_or(0);
                tracing::info!(target: "stt", "Final: {} (words: {})", text, word_count);
            }
            TranscriptionEvent::Error { code, message } => {
                tracing::error!(target: "stt", "Error [{}]: {}", code, message);
            }
        }

        // Send to channel with backpressure - wait if channel is full
        // Use timeout to prevent indefinite blocking
        match tokio::time::timeout(std::time::Duration::from_secs(5), self.event_tx.send(event))
            .await
        {
            Ok(Ok(())) => {
                // Successfully sent
            }
            Ok(Err(_)) => {
                // Channel closed
                tracing::debug!(target: "stt", "Event channel closed");
            }
            Err(_) => {
                // Timeout - consumer is too slow
                tracing::warn!(target: "stt", "Event channel send timed out after 5s - consumer too slow");
                self.metrics.write().frames_dropped += 1;
            }
        }
    }

    /// Update timing metrics from processing measurements
    fn update_timing_metrics(
        &self,
        e2e_time: std::time::Duration,
        engine_time: std::time::Duration,
        preprocessing_time: std::time::Duration,
        delivery_time: std::time::Duration,
    ) {
        // Update basic metrics with latest timings
        {
            let mut metrics = self.metrics.write();
            metrics.last_e2e_latency_us = e2e_time.as_micros() as u64;
            metrics.last_engine_time_us = engine_time.as_micros() as u64;
            metrics.last_preprocessing_us = preprocessing_time.as_micros() as u64;
        }

        // Update comprehensive performance metrics if available
        if let Some(perf_metrics) = &self.performance_metrics {
            perf_metrics.record_end_to_end_latency(e2e_time);
            perf_metrics.record_engine_processing_time(engine_time);
            perf_metrics.record_preprocessing_latency(preprocessing_time);
            perf_metrics.record_result_delivery_latency(delivery_time);
            perf_metrics.increment_requests();
        }
    }

    /// Update accuracy metrics from transcription events
    fn update_accuracy_metrics(&self, event: &TranscriptionEvent, success: bool) {
        if let Some(perf_metrics) = &self.performance_metrics {
            if success {
                perf_metrics.record_transcription_success();

                // Extract confidence from transcription events
                match event {
                    TranscriptionEvent::Final { words, .. } => {
                        // Calculate average confidence from word-level data if available
                        if let Some(word_list) = words {
                            if !word_list.is_empty() {
                                let avg_confidence: f64 =
                                    word_list.iter().map(|w| w.conf as f64).sum::<f64>()
                                        / word_list.len() as f64;
                                perf_metrics.record_confidence_score(avg_confidence);
                            }
                        }
                        perf_metrics.record_final_transcription();
                    }
                    TranscriptionEvent::Partial { .. } => {
                        perf_metrics.record_partial_transcription();
                    }
                    TranscriptionEvent::Error { .. } => {
                        perf_metrics.record_transcription_failure();
                        perf_metrics.record_error();
                    }
                }
            } else {
                perf_metrics.record_transcription_failure();
                perf_metrics.record_error();
            }
        }

        // Update basic metrics
        {
            let mut metrics = self.metrics.write();
            match event {
                TranscriptionEvent::Final { words, .. } => {
                    metrics.final_count += 1;

                    // Update confidence if available
                    if let Some(word_list) = words {
                        if !word_list.is_empty() {
                            let avg_confidence: f64 =
                                word_list.iter().map(|w| w.conf as f64).sum::<f64>()
                                    / word_list.len() as f64;

                            // Update running average (stored as x1000 for precision)
                            let confidence_x1000 = (avg_confidence * 1000.0) as u64;
                            let current_sum =
                                metrics.avg_confidence_x1000 * metrics.confidence_measurements;
                            metrics.confidence_measurements += 1;
                            metrics.avg_confidence_x1000 =
                                (current_sum + confidence_x1000) / metrics.confidence_measurements;
                        }
                    }
                }
                TranscriptionEvent::Partial { .. } => {
                    metrics.partial_count += 1;
                }
                TranscriptionEvent::Error { .. } => {
                    metrics.error_count += 1;
                }
            }
        }
    }
}

// No longer need separate stub implementation since we're generic over StreamingStt
