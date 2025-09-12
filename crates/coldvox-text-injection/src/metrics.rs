//! # Metrics for the text injection crate.
//!
//! This module defines the data structures and traits for collecting
//! performance and reliability metrics during text injection. The goal is to
//! keep this lightweight and avoid heavy dependencies.

use crate::error::InjectionError;
use crate::probe::BackendId;
use heapless::Vec;
use std::collections::HashMap;

/// A fixed-capacity vector for storing the last N latency samples.
/// This allows for calculating approximate percentiles without a full histogram.
const LATENCY_SAMPLES_CAPACITY: usize = 64;

/// A collection of metrics for a single backend's performance.
#[derive(Default, Clone, Debug)]
pub struct BackendMetrics {
    pub attempts: u64,
    pub successes: u64,
    pub failures: u64,
    pub sum_latency_ms: u64,
    pub min_latency_ms: u32,
    pub max_latency_ms: u32,
    /// A small, fixed-capacity reservoir of the last N latency samples.
    pub samples_ms: Vec<u32, LATENCY_SAMPLES_CAPACITY>,
}

/// A comprehensive collection of metrics for the entire injection system.
#[derive(Default, Clone, Debug)]
pub struct InjectionMetrics {
    /// Total number of injection attempts initiated.
    pub total_attempts: u64,
    /// Total number of successful injections.
    pub total_successes: u64,
    /// Breakdown of failures by kind.
    pub failures_by_kind: HashMap<String, u64>,
    /// Metrics for each individual backend.
    pub backend_metrics: HashMap<BackendId, BackendMetrics>,
}

/// A trait for a sink that can receive and process injection metrics.
///
/// This allows the core logic to emit metrics without being tied to a specific
/// implementation.
pub trait MetricsSink: Send {
    /// Called when an attempt to use a backend starts.
    fn emit_start(&mut self, backend: BackendId);

    /// Called when a backend successfully completes an injection.
    fn emit_success(&mut self, backend: BackendId, latency_ms: u32);

    /// Called when a backend fails an injection attempt.
    fn emit_fail(&mut self, backend: BackendId, err: &InjectionError);
}

// Implement the sink for our main metrics struct.
impl MetricsSink for InjectionMetrics {
    fn emit_start(&mut self, backend: BackendId) {
        self.total_attempts += 1;
        self.backend_metrics
            .entry(backend)
            .or_default()
            .attempts += 1;
    }

    fn emit_success(&mut self, backend: BackendId, latency_ms: u32) {
        self.total_successes += 1;
        let bm = self.backend_metrics.entry(backend).or_default();
        bm.successes += 1;
        bm.sum_latency_ms += latency_ms as u64;

        if bm.min_latency_ms == 0 || latency_ms < bm.min_latency_ms {
            bm.min_latency_ms = latency_ms;
        }
        if latency_ms > bm.max_latency_ms {
            bm.max_latency_ms = latency_ms;
        }

        if bm.samples_ms.is_full() {
            // Simple reservoir sampling: replace a random element.
            // A more sophisticated approach could be used, but this is simple.
            // For now, we'll just overwrite the oldest element.
            // A circular buffer would be better. Let's just push and let the oldest drop.
            bm.samples_ms.remove(0);
        }
        let _ = bm.samples_ms.push(latency_ms);
    }

    fn emit_fail(&mut self, backend: BackendId, err: &InjectionError) {
        let bm = self.backend_metrics.entry(backend).or_default();
        bm.failures += 1;

        // Use the error's variant name as the key for simple aggregation.
        let error_kind = match err {
            InjectionError::Unavailable { .. } => "Unavailable",
            InjectionError::Timeout { .. } => "Timeout",
            InjectionError::PreconditionNotMet { .. } => "PreconditionNotMet",
            InjectionError::Transient { .. } => "Transient",
            InjectionError::ClipboardRestoreMismatch { .. } => "ClipboardRestoreMismatch",
            InjectionError::Io { .. } => "Io",
            InjectionError::Other { .. } => "Other",
        };

        *self.failures_by_kind.entry(error_kind.to_string()).or_default() += 1;
    }
}
