use coldvox_stt::TranscriptionEvent;

/// Placeholder for pipeline metrics - to be provided by the main app
#[derive(Debug, Clone, Default)]
pub struct PipelineMetrics {
    pub processed_events: u64,
    pub injection_latency_ms: u64,
}
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Instant};
use tracing::{debug, error, info, warn};

use super::manager::StrategyManager;
use super::session::{InjectionSession, SessionConfig, SessionState};
use super::InjectionConfig;
use crate::types::InjectionMetrics;

/// Local metrics for the injection processor (UI/state), distinct from types::InjectionMetrics
#[derive(Debug, Clone, Default)]
pub struct ProcessorMetrics {
    /// Current session state
    pub session_state: SessionState,
    /// Number of transcriptions in current buffer
    pub buffer_size: usize,
    /// Total characters in buffer
    pub buffer_chars: usize,
    /// Time since last transcription (ms)
    pub time_since_last_transcription_ms: Option<u64>,
    /// Total successful injections
    pub successful_injections: u64,
    /// Total failed injections
    pub failed_injections: u64,
    /// Last injection timestamp
    pub last_injection_time: Option<Instant>,
}

impl ProcessorMetrics {
    /// Update metrics from current session state
    pub fn update_from_session(&mut self, session: &InjectionSession) {
        self.session_state = session.state();
        self.buffer_size = session.buffer_len();
        self.buffer_chars = session.total_chars();
        self.time_since_last_transcription_ms = session
            .time_since_last_transcription()
            .map(|d| d.as_millis() as u64);
    }
}

/// Processor that manages session-based text injection
pub struct InjectionProcessor {
    /// The injection session
    session: InjectionSession,
    /// Text injector for performing the actual injection
    injector: StrategyManager,
    /// Configuration
    config: InjectionConfig,
    /// Metrics for telemetry
    metrics: Arc<Mutex<ProcessorMetrics>>,
    /// Shared injection metrics for all components
    injection_metrics: Arc<Mutex<crate::types::InjectionMetrics>>,
    /// Pipeline metrics for integration
    _pipeline_metrics: Option<Arc<PipelineMetrics>>,
}

impl InjectionProcessor {
    /// Create a new injection processor
    pub async fn new(
        config: InjectionConfig,
        pipeline_metrics: Option<Arc<PipelineMetrics>>,
        injection_metrics: Arc<Mutex<InjectionMetrics>>,
    ) -> Self {
        // Create session with shared metrics
        let session_config = SessionConfig::default(); // TODO: Expose this if needed (config refinement)
        let session = InjectionSession::new(session_config, injection_metrics.clone());

        let injector = StrategyManager::new(config.clone(), injection_metrics.clone()).await;

        let metrics = Arc::new(Mutex::new(ProcessorMetrics {
            session_state: SessionState::Idle,
            ..Default::default()
        }));

        Self {
            session,
            injector,
            config,
            metrics,
            injection_metrics,
            _pipeline_metrics: pipeline_metrics,
        }
    }

    /// Prepare an injection by checking session state and extracting buffered text if ready.
    /// Returns Some(text) when there is content to inject, otherwise None.
    pub fn prepare_injection(&mut self) -> Option<String> {
        if self.session.should_inject() {
            let text = self.session.take_buffer();
            if !text.is_empty() {
                debug!("Injecting {} characters from session", text.len());
                return Some(text);
            }
        }
        None
    }

    /// Record the result of an injection attempt and refresh metrics.
    pub fn record_injection_result(&mut self, success: bool) {
        if success {
            self.metrics.lock().unwrap().successful_injections += 1;
            self.metrics.lock().unwrap().last_injection_time = Some(Instant::now());
        } else {
            self.metrics.lock().unwrap().failed_injections += 1;
        }
        self.update_metrics();
    }

    /// Get current metrics
    pub fn metrics(&self) -> ProcessorMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Handle a transcription event from the STT processor
    pub fn handle_transcription(&mut self, event: TranscriptionEvent) {
        match event {
            TranscriptionEvent::Partial {
                text, utterance_id, ..
            } => {
                debug!(
                    "Received partial transcription [{}]: {}",
                    utterance_id, text
                );
                self.update_metrics();
            }
            TranscriptionEvent::Final {
                text, utterance_id, ..
            } => {
                let text_len = text.len();
                info!("Received final transcription [{}]: {}", utterance_id, text);
                self.session.add_transcription(text);
                // Record the number of characters buffered
                if let Ok(mut metrics) = self.injection_metrics.lock() {
                    metrics.record_buffered_chars(text_len as u64);
                }
                self.update_metrics();
            }
            TranscriptionEvent::Error { code, message } => {
                warn!("Transcription error [{}]: {}", code, message);
            }
        }
    }

    /// Check if injection should be performed and execute if needed
    pub async fn check_and_inject(&mut self) -> anyhow::Result<()> {
        if self.session.should_inject() {
            // Mode decision is now centralized in StrategyManager
            // which receives the config and makes the paste vs keystroke decision
            self.perform_injection().await?;
        }
        Ok(())
    }

    /// Force injection of current buffer (for manual triggers)
    pub async fn force_inject(&mut self) -> anyhow::Result<()> {
        if self.session.has_content() {
            // Mode decision is now centralized in StrategyManager
            self.session.force_inject();
            self.perform_injection().await?;
        }
        Ok(())
    }

    /// Clear current session buffer
    pub fn clear_session(&mut self) {
        self.session.clear();
        self.update_metrics();
        info!("Session cleared manually");
    }

    /// Perform the actual text injection
    async fn perform_injection(&mut self) -> anyhow::Result<()> {
        let text = self.session.take_buffer();
        if text.is_empty() {
            return Ok(());
        }

        // Record the time from final transcription to injection
        let latency = self
            .session
            .time_since_last_transcription()
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        info!(
            "Injecting {} characters from session (latency: {}ms)",
            text.len(),
            latency
        );

        // Record the latency in metrics
        if let Ok(mut metrics) = self.injection_metrics.lock() {
            metrics.record_latency_from_final(latency);
            metrics.update_last_injection();
        }

        match self.injector.inject(&text).await {
            Ok(()) => {
                info!("Successfully injected text");
                self.metrics.lock().unwrap().successful_injections += 1;
                self.metrics.lock().unwrap().last_injection_time = Some(Instant::now());
            }
            Err(e) => {
                error!("Failed to inject text: {}", e);
                self.metrics.lock().unwrap().failed_injections += 1;
                return Err(e.into());
            }
        }

        self.update_metrics();
        Ok(())
    }

    /// Update internal metrics from session state
    fn update_metrics(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        let previous_state = metrics.session_state;
        let previous_buffer = metrics.buffer_size;
        metrics.update_from_session(&self.session);
        if metrics.session_state != previous_state || metrics.buffer_size != previous_buffer {
            debug!(
                prev_state = %previous_state,
                new_state = %metrics.session_state,
                buffer_items = metrics.buffer_size,
                buffer_chars = metrics.buffer_chars,
                "Session metrics updated"
            );
        }
    }

    /// Get current session state
    pub fn session_state(&self) -> SessionState {
        self.session.state()
    }

    /// Get buffer content preview (for debugging/UI)
    pub fn buffer_preview(&self) -> String {
        let text = self.session.buffer_preview();
        let preview = if text.len() > 100 {
            format!("{}...", &text[..100])
        } else {
            text
        };
        debug!("Buffer preview: {}", preview);
        preview
    }

    /// Get the last partial transcription text (for real-time feedback)
    pub fn last_partial_text(&self) -> Option<String> {
        None
    }
}

/// Async wrapper for the injection processor that runs in a dedicated task
pub struct AsyncInjectionProcessor {
    processor: Arc<tokio::sync::Mutex<InjectionProcessor>>,
    transcription_rx: mpsc::Receiver<TranscriptionEvent>,
    shutdown_rx: mpsc::Receiver<()>,
    // dedicated injector to avoid awaiting while holding the processor lock
    injector: StrategyManager,
}

impl AsyncInjectionProcessor {
    /// Create a new async injection processor
    pub async fn new(
        config: InjectionConfig,
        transcription_rx: mpsc::Receiver<TranscriptionEvent>,
        shutdown_rx: mpsc::Receiver<()>,
        pipeline_metrics: Option<Arc<PipelineMetrics>>,
    ) -> Self {
        // Create shared injection metrics
        let injection_metrics = Arc::new(Mutex::new(crate::types::InjectionMetrics::default()));

        // Create processor with shared metrics
        let processor = Arc::new(tokio::sync::Mutex::new(
            InjectionProcessor::new(config.clone(), pipeline_metrics, injection_metrics.clone())
                .await,
        ));

        // Create injector with shared metrics
        let injector = StrategyManager::new(config, injection_metrics.clone()).await;

        Self {
            processor,
            transcription_rx,
            shutdown_rx,
            injector,
        }
    }

    /// Run the injection processor loop
    pub async fn run(mut self) -> anyhow::Result<()> {
        let check_interval = Duration::from_millis(100); // TODO: Make configurable (config refinement)
        let mut interval = time::interval(check_interval);

        info!("Injection processor started");

        loop {
            tokio::select! {
                // Handle transcription events
                Some(event) = self.transcription_rx.recv() => {
                    let mut processor = self.processor.lock().await;
                    processor.handle_transcription(event);
                }

                // Periodic check for silence timeout
                _ = interval.tick() => {
                    // Prepare any pending injection without holding the lock across await
                    let maybe_text = {
                        let mut processor = self.processor.lock().await;
                        // Extract text to inject if session criteria are met
                        processor.prepare_injection()
                    };

                    if let Some(text) = maybe_text {
                        // Perform the async injection outside the lock
                        info!("Attempting injection of {} characters", text.len());
                        let result = self.injector.inject(&text).await;
                        let success = result.is_ok();

                        // Record result back into the processor state/metrics
                        let mut processor = self.processor.lock().await;
                        processor.record_injection_result(success);
                        if let Err(e) = result {
                            error!("Injection failed: {}", e);
                        } else {
                            info!("Injection completed successfully");
                        }
                    }
                }

                // Shutdown signal
                _ = self.shutdown_rx.recv() => {
                    info!("Received shutdown signal, graceful exit initiated");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Get current metrics
    pub async fn metrics(&self) -> ProcessorMetrics {
        self.processor.lock().await.metrics()
    }

    /// Force injection (for manual triggers)
    pub async fn force_inject(&self) -> anyhow::Result<()> {
        self.processor.lock().await.force_inject().await
    }

    /// Clear session (for cancellation)
    pub async fn clear_session(&self) {
        self.processor.lock().await.clear_session();
    }

    /// Get the last partial transcription text (for real-time feedback)
    pub async fn last_partial_text(&self) -> Option<String> {
        self.processor.lock().await.last_partial_text()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn test_injection_processor_basic_flow() {
        let config = InjectionConfig::default();

        let injection_metrics = Arc::new(Mutex::new(crate::types::InjectionMetrics::default()));
        let mut processor = InjectionProcessor::new(config, None, injection_metrics).await;

        // Start with idle state
        assert_eq!(processor.session_state(), SessionState::Idle);

        // Add a transcription
        processor.handle_transcription(TranscriptionEvent::Final {
            utterance_id: 1,
            text: "Hello world".to_string(),
            words: None,
        });

        assert_eq!(processor.session_state(), SessionState::Buffering);

        // Wait for silence timeout
        thread::sleep(Duration::from_millis(300));

        // Check for silence transition (this would normally be called periodically)
        processor.session.check_for_silence_transition();

        // Should be in WaitingForSilence state now
        assert_eq!(processor.session_state(), SessionState::WaitingForSilence);

        // This should trigger injection check
        let should_inject = processor.session.should_inject();
        assert!(should_inject, "Session should be ready to inject");

        // Instead of actually injecting (which requires ydotool),
        // we'll manually clear the buffer to simulate successful injection
        let buffer_content = processor.session.take_buffer();
        assert_eq!(buffer_content, "Hello world");

        // Should be back to idle after taking the buffer
        assert_eq!(processor.session_state(), SessionState::Idle);
    }

    #[tokio::test]
    async fn test_metrics_update() {
        let config = InjectionConfig::default();
        let injection_metrics = Arc::new(Mutex::new(crate::types::InjectionMetrics::default()));
        let mut processor = InjectionProcessor::new(config, None, injection_metrics).await;

        // Add transcription
        processor.handle_transcription(TranscriptionEvent::Final {
            utterance_id: 1,
            text: "Test transcription".to_string(),
            words: None,
        });

        let metrics = processor.metrics();
        assert_eq!(metrics.session_state, SessionState::Buffering);
        assert_eq!(metrics.buffer_size, 1);
        assert!(metrics.buffer_chars > 0);
    }

    #[tokio::test]
    async fn test_partial_transcription_handling() {
        let config = InjectionConfig::default();
        let injection_metrics = Arc::new(Mutex::new(crate::types::InjectionMetrics::default()));
        let mut processor = InjectionProcessor::new(config, None, injection_metrics).await;

        // Start with idle state
        assert_eq!(processor.session_state(), SessionState::Idle);

        // Handle partial transcription
        processor.handle_transcription(TranscriptionEvent::Partial {
            utterance_id: 1,
            text: "Hello".to_string(),
            t0: None,
            t1: None,
        });

        // Should still be idle since partial events don't change session state
        assert_eq!(processor.session_state(), SessionState::Idle);

        // Handle final transcription
        processor.handle_transcription(TranscriptionEvent::Final {
            utterance_id: 1,
            text: "Hello world".to_string(),
            words: None,
        });

        // Now should be buffering
        assert_eq!(processor.session_state(), SessionState::Buffering);
        assert_eq!(processor.session.buffer_len(), 1);
    }
}
