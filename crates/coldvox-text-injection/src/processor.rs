//! # Text Injection Processor
//!
//! This module acts as a high-level interface for the text injection system.
//! It is responsible for receiving transcription events and coordinating with
//! the `StrategyManager` to perform the injection.

use crate::manager::StrategyManager;
use crate::metrics::{InjectionMetrics, MetricsSink};
use crate::types::InjectionConfig;
use coldvox_stt::TranscriptionEvent;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// A simplified processor that directly injects final transcriptions.
///
/// The complex buffering and session logic has been removed in favor of a
/// more direct approach. This processor listens for `Final` transcription
/// events and immediately attempts to inject them using the `StrategyManager`.
pub struct AsyncInjectionProcessor {
    /// The manager that handles the injection strategy.
    manager: StrategyManager,
    /// Receiver for transcription events from the STT engine.
    transcription_rx: mpsc::Receiver<TranscriptionEvent>,
    /// Receiver for the shutdown signal.
    shutdown_rx: mpsc::Receiver<()>,
    /// Shared metrics for the injection system.
    metrics: Arc<Mutex<InjectionMetrics>>,
}

impl AsyncInjectionProcessor {
    /// Creates a new async injection processor.
    pub fn new(
        config: InjectionConfig,
        transcription_rx: mpsc::Receiver<TranscriptionEvent>,
        shutdown_rx: mpsc::Receiver<()>,
    ) -> Self {
        let manager = StrategyManager::new(config);
        let metrics = Arc::new(Mutex::new(InjectionMetrics::default()));

        Self {
            manager,
            transcription_rx,
            shutdown_rx,
            metrics,
        }
    }

    /// Runs the main loop of the injection processor.
    ///
    /// It listens for transcription events and shutdown signals, and triggers
    /// injection for final transcriptions.
    pub async fn run(mut self) {
        info!("Injection processor started.");
        loop {
            tokio::select! {
                Some(event) = self.transcription_rx.recv() => {
                    if let TranscriptionEvent::Final { text, .. } = event {
                        if text.is_empty() {
                            continue;
                        }
                        info!("Received final transcription, attempting injection...");
                        // Don't hold the lock across the await
                        let result = self.manager.inject_with_fail_fast(&text, &mut InjectionMetrics::default()).await;
                        // Now update metrics with the result
                        let mut metrics_guard = self.metrics.lock().unwrap();
                        match result {
                            Ok(outcome) => {
                                info!("Injection successful: {:?}", outcome);
                                // Manually emit success metrics
                                <InjectionMetrics as MetricsSink>::emit_success(&mut *metrics_guard, crate::probe::BackendId::Atspi, outcome.latency_ms);
                            }
                            Err(e) => {
                                error!("Injection failed: {}", e);
                                // Manually emit failure metrics
                                <InjectionMetrics as MetricsSink>::emit_fail(&mut *metrics_guard, crate::probe::BackendId::Atspi, &e);
                            }
                        }
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    info!("Shutdown signal received. Exiting injection processor.");
                    break;
                }
            }
        }
    }

    /// Returns a clone of the metrics Arc for external monitoring.
    pub fn get_metrics(&self) -> Arc<Mutex<InjectionMetrics>> {
        self.metrics.clone()
    }
}
