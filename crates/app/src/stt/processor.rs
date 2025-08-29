use tokio::sync::{broadcast, mpsc};
use crate::audio::vad_processor::AudioFrame;
use crate::stt::{VoskTranscriber, TranscriptionEvent, TranscriptionConfig};
use crate::vad::types::VadEvent;
use std::sync::Arc;
use std::time::Instant;

/// STT processor state
#[derive(Debug, Clone)]
pub enum UtteranceState {
    /// No speech detected
    Idle,
    /// Speech is active
    SpeechActive {
        /// Timestamp when speech started
        started_at: Instant,
        /// Timestamp of last partial result
        last_partial_at: Option<Instant>,
        /// Number of frames processed
        frames_processed: u64,
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
        
        // Use default config with a placeholder model path
        let config = TranscriptionConfig {
            enabled: true,
            model_path: "vosk-model-en-us-0.22-lgraph".to_string(),
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
            last_partial_at: None,
            frames_processed: 0,
        };
        
        // Reset transcriber for new utterance
        if let Err(e) = self.transcriber.reset() {
            tracing::warn!(target: "stt", "Failed to reset transcriber: {}", e);
        }
    }
    
    /// Handle speech end event
    async fn handle_speech_end(&mut self, timestamp_ms: u64, duration_ms: Option<u64>) {
        tracing::debug!(
            target: "stt",
            "STT processor received SpeechEnd at {}ms (duration: {:?}ms)",
            timestamp_ms,
            duration_ms
        );
        
        // Finalize current utterance
        match self.transcriber.finalize_utterance() {
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
        
        self.state = UtteranceState::Idle;
    }
    
    /// Handle incoming audio frame
    async fn handle_audio_frame(&mut self, frame: AudioFrame) {
        // Update metrics
        self.metrics.write().frames_in += 1;
        
        // Only process if speech is active
        let should_process = matches!(self.state, UtteranceState::SpeechActive { .. });
        
        if !should_process {
            return;
        }
        
        // Process frame through transcriber
        match self.transcriber.accept_frame(&frame.data) {
            Ok(Some(event)) => {
                self.send_event(event.clone()).await;
                
                // Update metrics and state
                let mut metrics = self.metrics.write();
                metrics.frames_out += 1;
                metrics.last_event_time = Some(Instant::now());
                
                match event {
                    TranscriptionEvent::Partial { .. } => {
                        metrics.partial_count += 1;
                        
                        // Update state
                        if let UtteranceState::SpeechActive {
                            started_at,
                            frames_processed,
                            ..
                        } = self.state
                        {
                            self.state = UtteranceState::SpeechActive {
                                started_at,
                                last_partial_at: Some(Instant::now()),
                                frames_processed: frames_processed + 1,
                            };
                        }
                    }
                    TranscriptionEvent::Final { .. } => {
                        metrics.final_count += 1;
                    }
                    TranscriptionEvent::Error { .. } => {
                        metrics.error_count += 1;
                    }
                }
            }
            Ok(None) => {
                // No transcription for this frame
                if let UtteranceState::SpeechActive {
                    started_at,
                    last_partial_at,
                    frames_processed,
                } = self.state
                {
                    self.state = UtteranceState::SpeechActive {
                        started_at,
                        last_partial_at,
                        frames_processed: frames_processed + 1,
                    };
                }
            }
            Err(e) => {
                tracing::error!(target: "stt", "Transcription error: {}", e);
                
                // Send error event
                let error_event = TranscriptionEvent::Error {
                    code: "TRANSCRIPTION_ERROR".to_string(),
                    message: e,
                };
                self.send_event(error_event).await;
                
                // Update metrics
                self.metrics.write().error_count += 1;
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
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.event_tx.send(event)
        ).await {
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