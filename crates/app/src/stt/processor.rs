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
}

#[cfg(feature = "vosk")]
pub struct SttProcessor {
    /// Audio frame receiver (broadcast from pipeline)
    audio_rx: broadcast::Receiver<AudioFrame>,
    /// VAD event receiver
    vad_event_rx: mpsc::Receiver<VadEvent>,
    /// Transcription event sender
    event_tx: mpsc::Sender<TranscriptionEvent>,
    /// Vosk transcriber instance
    transcriber: VoskTranscriber,
    /// Current utterance state
    state: UtteranceState,
    /// Metrics
    metrics: Arc<parking_lot::RwLock<SttMetrics>>,
    /// Configuration
    config: TranscriptionConfig,
}

#[cfg(feature = "vosk")]
impl SttProcessor {
    /// Create a new STT processor
    pub fn new(
        audio_rx: broadcast::Receiver<AudioFrame>,
        vad_event_rx: mpsc::Receiver<VadEvent>,
        event_tx: mpsc::Sender<TranscriptionEvent>,
        config: TranscriptionConfig,
    ) -> Result<Self, String> {
        // Check if STT is enabled
        if !config.enabled {
            tracing::info!("STT processor disabled in configuration");
        }

        // Create Vosk transcriber with configuration
        let transcriber = VoskTranscriber::new(config.clone(), 16000.0)?;

        Ok(Self {
            audio_rx,
            vad_event_rx,
            event_tx,
            transcriber,
            state: UtteranceState::Idle,
            metrics: Arc::new(parking_lot::RwLock::new(SttMetrics::default())),
            config,
        })
    }

    /// Create with default configuration (backward compatibility)
    pub fn new_with_default(
        audio_rx: broadcast::Receiver<AudioFrame>,
        vad_event_rx: mpsc::Receiver<VadEvent>,
    ) -> Result<Self, String> {
        // Create a simple event channel for compatibility
        let (event_tx, _event_rx) = mpsc::channel(100);

        // Use default config with the default model path
        let config = TranscriptionConfig {
            enabled: true,
            model_path: crate::stt::vosk::default_model_path(),
            partial_results: true,
            max_alternatives: 1,
            include_words: false,
            buffer_size_ms: 512,
        };

        Self::new(audio_rx, vad_event_rx, event_tx, config)
    }

    /// Get current metrics
    pub fn metrics(&self) -> SttMetrics {
        self.metrics.read().clone()
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
        tracing::debug!(target: "stt", "STT processor received SpeechStart at {}ms", timestamp_ms);

        // Store the start time as Instant for duration calculations
        let start_instant = Instant::now();

        self.state = UtteranceState::SpeechActive {
            started_at: start_instant,
            audio_buffer: Vec::with_capacity(16000 * 10), // Pre-allocate for up to 10 seconds
            frames_buffered: 0,
        };

        // Reset transcriber for new utterance
        if let Err(e) = coldvox_stt::EventBasedTranscriber::reset(&mut self.transcriber) {
            tracing::warn!(target: "stt", "Failed to reset transcriber: {}", e);
        }

        tracing::info!(target: "stt", "Started buffering audio for new utterance");
    }

    /// Handle speech end event
    async fn handle_speech_end(&mut self, timestamp_ms: u64, duration_ms: Option<u64>) {
        tracing::debug!(
            target: "stt",
            "STT processor received SpeechEnd at {}ms (duration: {:?}ms)",
            timestamp_ms,
            duration_ms
        );

        // Process the buffered audio all at once
        if let UtteranceState::SpeechActive {
            audio_buffer,
            frames_buffered,
            ..
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

            if !audio_buffer.is_empty() {
                // Send the entire buffer to the transcriber at once
                match coldvox_stt::EventBasedTranscriber::accept_frame(
                    &mut self.transcriber,
                    &audio_buffer,
                ) {
                    Ok(Some(event)) => {
                        self.send_event(event).await;

                        // Update metrics
                        let mut metrics = self.metrics.write();
                        metrics.frames_out += frames_buffered;
                        metrics.last_event_time = Some(Instant::now());
                    }
                    Ok(None) => {
                        tracing::debug!(target: "stt", "No transcription from buffered audio");
                    }
                    Err(e) => {
                        tracing::error!(target: "stt", "Failed to process buffered audio: {}", e);

                        // Send error event
                        let error_event = TranscriptionEvent::Error {
                            code: "BUFFER_PROCESS_ERROR".to_string(),
                            message: e,
                        };
                        self.send_event(error_event).await;

                        // Update metrics
                        self.metrics.write().error_count += 1;
                    }
                }
            }

            // Finalize to get any remaining transcription
            match coldvox_stt::EventBasedTranscriber::finalize_utterance(&mut self.transcriber) {
                Ok(Some(event)) => {
                    self.send_event(event).await;

                    // Update metrics
                    let mut metrics = self.metrics.write();
                    metrics.final_count += 1;
                    metrics.last_event_time = Some(Instant::now());
                }
                Ok(None) => {
                    tracing::debug!(target: "stt", "No final transcription available");
                }
                Err(e) => {
                    tracing::error!(target: "stt", "Failed to finalize transcription: {}", e);

                    // Send error event
                    let error_event = TranscriptionEvent::Error {
                        code: "FINALIZE_ERROR".to_string(),
                        message: e,
                    };
                    self.send_event(error_event).await;

                    // Update metrics
                    self.metrics.write().error_count += 1;
                }
            }
        }

        self.state = UtteranceState::Idle;
    }

    /// Handle incoming audio frame
    async fn handle_audio_frame(&mut self, frame: AudioFrame) {
        // Update metrics
        self.metrics.write().frames_in += 1;

        // Only buffer if speech is active
        if let UtteranceState::SpeechActive {
            ref mut audio_buffer,
            ref mut frames_buffered,
            ..
        } = &mut self.state
        {
            // Convert f32 samples back to i16
            let i16_samples: Vec<i16> = frame
                .samples
                .iter()
                .map(|&s| (s * i16::MAX as f32) as i16)
                .collect();

            // Buffer the audio frame
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
}

#[cfg(not(feature = "vosk"))]
pub struct SttProcessor;

#[cfg(not(feature = "vosk"))]
impl SttProcessor {
    /// Create a stub STT processor when Vosk feature is disabled
    pub fn new(
        _audio_rx: broadcast::Receiver<AudioFrame>,
        _vad_event_rx: mpsc::Receiver<VadEvent>,
        _event_tx: mpsc::Sender<TranscriptionEvent>,
        _config: TranscriptionConfig,
    ) -> Result<Self, String> {
        tracing::info!("STT processor disabled - Vosk feature not enabled");
        Ok(Self)
    }

    /// Stub method for backward compatibility
    pub fn new_with_default(
        _audio_rx: broadcast::Receiver<AudioFrame>,
        _vad_event_rx: mpsc::Receiver<VadEvent>,
    ) -> Result<Self, String> {
        Self::new(
            _audio_rx,
            _vad_event_rx,
            mpsc::channel(1).0,
            TranscriptionConfig::default(),
        )
    }

    /// Get stub metrics
    pub fn metrics(&self) -> SttMetrics {
        SttMetrics::default()
    }

    /// Run stub processor
    pub async fn run(self) {
        tracing::info!("STT processor stub running - no actual processing (Vosk feature disabled)");
        // Just sleep forever since there's nothing to do
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }
}
