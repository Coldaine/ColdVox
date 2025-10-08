//! STT-specific performance metrics and monitoring
//!
//! This module provides comprehensive performance monitoring for Speech-to-Text (STT)
//! operations, including latency tracking, accuracy measurements, resource usage,
//! and operational metrics.

use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Comprehensive STT performance metrics
#[derive(Clone, Default)]
pub struct SttPerformanceMetrics {
    /// Latency tracking
    pub latency: Arc<LatencyMetrics>,
    /// Accuracy tracking
    pub accuracy: Arc<AccuracyMetrics>,
    /// Resource usage monitoring
    pub resources: Arc<ResourceMetrics>,
    /// Operational metrics
    pub operational: Arc<OperationalMetrics>,
}

/// Latency measurement metrics
#[derive(Default)]
pub struct LatencyMetrics {
    /// End-to-end transcription latency (microseconds)
    pub end_to_end_us: Arc<AtomicU64>,
    /// STT engine processing time (microseconds)
    pub engine_processing_us: Arc<AtomicU64>,
    /// Audio preprocessing latency (microseconds)
    pub preprocessing_us: Arc<AtomicU64>,
    /// Result delivery latency (microseconds)
    pub result_delivery_us: Arc<AtomicU64>,
    /// Model loading time (milliseconds)
    pub model_loading_ms: Arc<AtomicU64>,
    /// Latency history for trend analysis
    pub latency_history: Arc<RwLock<VecDeque<LatencySnapshot>>>,
}

/// Accuracy measurement metrics
#[derive(Default)]
pub struct AccuracyMetrics {
    /// Total confidence score sum (for averaging)
    pub confidence_sum: Arc<AtomicU64>,
    /// Number of confidence measurements
    pub confidence_count: Arc<AtomicU64>,
    /// Transcription success count
    pub success_count: Arc<AtomicU64>,
    /// Transcription failure count
    pub failure_count: Arc<AtomicU64>,
    /// Partial transcription count
    pub partial_count: Arc<AtomicU64>,
    /// Final transcription count
    pub final_count: Arc<AtomicU64>,
    /// Average word confidence over time
    pub word_confidence_history: Arc<RwLock<VecDeque<f64>>>,
}

/// Resource usage monitoring metrics
#[derive(Default)]
pub struct ResourceMetrics {
    /// Current memory usage in bytes
    pub memory_usage_bytes: Arc<AtomicU64>,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: Arc<AtomicU64>,
    /// Audio buffer utilization percentage (0-100)
    pub buffer_utilization_pct: Arc<AtomicU64>,
    /// Active processing threads count
    pub active_threads: Arc<AtomicU64>,
    /// Total allocated buffers count
    pub allocated_buffers: Arc<AtomicU64>,
}

/// Operational metrics for monitoring
#[derive(Default)]
pub struct OperationalMetrics {
    /// Transcription requests per second
    pub requests_per_second: Arc<AtomicU64>,
    /// Error rate per component (per 1000 operations)
    pub error_rate_per_1k: Arc<AtomicU64>,
    /// Model switch count
    pub model_switches: Arc<AtomicU64>,
    /// Fallback mechanism usage count
    pub fallback_usage: Arc<AtomicU64>,
    /// Average queue depth
    pub avg_queue_depth: Arc<AtomicU64>,
    /// Processing rate (frames per second)
    pub processing_fps: Arc<AtomicU64>,
}

/// Snapshot of latency measurements for historical tracking
#[derive(Debug, Clone)]
pub struct LatencySnapshot {
    pub timestamp: Instant,
    pub end_to_end_us: u64,
    pub engine_processing_us: u64,
    pub preprocessing_us: u64,
    pub result_delivery_us: u64,
}

/// Performance alert thresholds
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    /// Maximum acceptable end-to-end latency (microseconds)
    pub max_latency_us: u64,
    /// Minimum acceptable confidence score (0.0-1.0)
    pub min_confidence: f64,
    /// Maximum acceptable error rate (per 1000 operations)
    pub max_error_rate_per_1k: u64,
    /// Maximum acceptable memory usage (bytes)
    pub max_memory_bytes: u64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_latency_us: 500_000,              // 500ms
            min_confidence: 0.7,                  // 70%
            max_error_rate_per_1k: 50,            // 5%
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// Performance alert types
#[derive(Debug, Clone)]
pub enum PerformanceAlert {
    HighLatency {
        measured_us: u64,
        threshold_us: u64,
    },
    LowConfidence {
        measured: f64,
        threshold: f64,
    },
    HighErrorRate {
        measured_per_1k: u64,
        threshold_per_1k: u64,
    },
    HighMemoryUsage {
        measured_bytes: u64,
        threshold_bytes: u64,
    },
    ProcessingStalled {
        last_activity: Duration,
    },
}

impl SttPerformanceMetrics {
    /// Create new STT performance metrics instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Record end-to-end latency measurement
    pub fn record_end_to_end_latency(&self, latency: Duration) {
        let latency_us = latency.as_micros() as u64;
        self.latency
            .end_to_end_us
            .store(latency_us, Ordering::Relaxed);

        // Add to history for trend analysis
        self.add_latency_snapshot(LatencySnapshot {
            timestamp: Instant::now(),
            end_to_end_us: latency_us,
            engine_processing_us: self.latency.engine_processing_us.load(Ordering::Relaxed),
            preprocessing_us: self.latency.preprocessing_us.load(Ordering::Relaxed),
            result_delivery_us: self.latency.result_delivery_us.load(Ordering::Relaxed),
        });
    }

    /// Record engine processing time
    pub fn record_engine_processing_time(&self, duration: Duration) {
        self.latency
            .engine_processing_us
            .store(duration.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record preprocessing latency
    pub fn record_preprocessing_latency(&self, duration: Duration) {
        self.latency
            .preprocessing_us
            .store(duration.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record result delivery latency
    pub fn record_result_delivery_latency(&self, duration: Duration) {
        self.latency
            .result_delivery_us
            .store(duration.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record confidence score
    pub fn record_confidence_score(&self, confidence: f64) {
        // Convert to integer (0-1000) for atomic storage
        let confidence_int = (confidence * 1000.0) as u64;

        // Update running average
        self.accuracy
            .confidence_sum
            .fetch_add(confidence_int, Ordering::Relaxed);
        self.accuracy
            .confidence_count
            .fetch_add(1, Ordering::Relaxed);

        // Store in history
        let mut history = self.accuracy.word_confidence_history.write();
        history.push_back(confidence);

        // Keep only last 100 measurements
        if history.len() > 100 {
            history.pop_front();
        }
    }

    /// Record transcription success
    pub fn record_transcription_success(&self) {
        self.accuracy.success_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record transcription failure
    pub fn record_transcription_failure(&self) {
        self.accuracy.failure_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record partial transcription
    pub fn record_partial_transcription(&self) {
        self.accuracy.partial_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record final transcription
    pub fn record_final_transcription(&self) {
        self.accuracy.final_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Update memory usage
    pub fn update_memory_usage(&self, bytes: u64) {
        self.resources
            .memory_usage_bytes
            .store(bytes, Ordering::Relaxed);

        // Update peak if higher
        let current_peak = self.resources.peak_memory_bytes.load(Ordering::Relaxed);
        if bytes > current_peak {
            self.resources
                .peak_memory_bytes
                .store(bytes, Ordering::Relaxed);
        }
    }

    /// Update buffer utilization percentage
    pub fn update_buffer_utilization(&self, utilization_pct: u64) {
        self.resources
            .buffer_utilization_pct
            .store(utilization_pct.min(100), Ordering::Relaxed);
    }

    /// Increment processing requests
    pub fn increment_requests(&self) {
        // This would be used with a rate calculator for requests per second
        // For now, just increment the operational counter
        self.operational
            .requests_per_second
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_total_requests(&self) {
        self.operational
            .requests_per_second
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record processing error
    pub fn record_error(&self) {
        self.operational
            .error_rate_per_1k
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get current average confidence score
    pub fn get_average_confidence(&self) -> f64 {
        let sum = self.accuracy.confidence_sum.load(Ordering::Relaxed);
        let count = self.accuracy.confidence_count.load(Ordering::Relaxed);

        if count > 0 {
            (sum as f64) / (count as f64) / 1000.0
        } else {
            0.0
        }
    }

    /// Get current success rate
    pub fn get_success_rate(&self) -> f64 {
        let success = self.accuracy.success_count.load(Ordering::Relaxed) as f64;
        let failure = self.accuracy.failure_count.load(Ordering::Relaxed) as f64;
        let total = success + failure;

        if total > 0.0 {
            success / total
        } else {
            0.0
        }
    }

    /// Check for performance alerts based on thresholds
    pub fn check_alerts(&self, thresholds: &PerformanceThresholds) -> Vec<PerformanceAlert> {
        let mut alerts = Vec::new();

        // Check latency
        let latency_us = self.latency.end_to_end_us.load(Ordering::Relaxed);
        if latency_us > thresholds.max_latency_us {
            alerts.push(PerformanceAlert::HighLatency {
                measured_us: latency_us,
                threshold_us: thresholds.max_latency_us,
            });
        }

        // Check confidence
        let avg_confidence = self.get_average_confidence();
        if avg_confidence < thresholds.min_confidence && avg_confidence > 0.0 {
            alerts.push(PerformanceAlert::LowConfidence {
                measured: avg_confidence,
                threshold: thresholds.min_confidence,
            });
        }

        // Check error rate
        let error_rate = self.operational.error_rate_per_1k.load(Ordering::Relaxed);
        if error_rate > thresholds.max_error_rate_per_1k {
            alerts.push(PerformanceAlert::HighErrorRate {
                measured_per_1k: error_rate,
                threshold_per_1k: thresholds.max_error_rate_per_1k,
            });
        }

        // Check memory usage
        let memory_usage = self.resources.memory_usage_bytes.load(Ordering::Relaxed);
        if memory_usage > thresholds.max_memory_bytes {
            alerts.push(PerformanceAlert::HighMemoryUsage {
                measured_bytes: memory_usage,
                threshold_bytes: thresholds.max_memory_bytes,
            });
        }

        alerts
    }

    /// Add latency snapshot to history
    fn add_latency_snapshot(&self, snapshot: LatencySnapshot) {
        let mut history = self.latency.latency_history.write();
        history.push_back(snapshot);

        // Keep only last 100 measurements
        if history.len() > 100 {
            history.pop_front();
        }
    }

    /// Get latency trend (returns slope indicating improvement/degradation)
    pub fn get_latency_trend(&self) -> Option<f64> {
        let history = self.latency.latency_history.read();

        if history.len() < 10 {
            return None;
        }

        // Simple linear regression on recent latency measurements
        let recent: Vec<_> = history.iter().rev().take(10).collect();
        let n = recent.len() as f64;

        let sum_x: f64 = (0..recent.len()).map(|i| i as f64).sum();
        let sum_y: f64 = recent.iter().map(|s| s.end_to_end_us as f64).sum();
        let sum_xy: f64 = recent
            .iter()
            .enumerate()
            .map(|(i, s)| i as f64 * s.end_to_end_us as f64)
            .sum();
        let sum_x2: f64 = (0..recent.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        Some(slope)
    }
}

/// Timing helper for measuring operation durations
#[derive(Debug)]
pub struct TimingMeasurement {
    start: Instant,
    label: String,
}

impl TimingMeasurement {
    /// Start timing an operation
    pub fn start(label: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            label: label.into(),
        }
    }

    /// End timing and return duration
    pub fn end(self) -> (String, Duration) {
        (self.label, self.start.elapsed())
    }

    /// End timing and record to metrics
    pub fn end_and_record<F>(self, record_fn: F)
    where
        F: FnOnce(Duration),
    {
        let duration = self.start.elapsed();
        record_fn(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_stt_performance_metrics_creation() {
        let metrics = SttPerformanceMetrics::new();

        // Verify initial state
        assert_eq!(metrics.get_average_confidence(), 0.0);
        assert_eq!(metrics.get_success_rate(), 0.0);
    }

    #[test]
    fn test_latency_recording() {
        let metrics = SttPerformanceMetrics::new();

        let latency = Duration::from_millis(250);
        metrics.record_end_to_end_latency(latency);

        let recorded = metrics
            .latency
            .end_to_end_us
            .load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(recorded, 250_000); // 250ms in microseconds
    }

    #[test]
    fn test_confidence_tracking() {
        let metrics = SttPerformanceMetrics::new();

        metrics.record_confidence_score(0.8);
        metrics.record_confidence_score(0.9);
        metrics.record_confidence_score(0.7);

        let avg = metrics.get_average_confidence();
        assert!((avg - 0.8).abs() < 0.01, "Expected ~0.8, got {}", avg);
    }

    #[test]
    fn test_success_rate_calculation() {
        let metrics = SttPerformanceMetrics::new();

        metrics.record_transcription_success();
        metrics.record_transcription_success();
        metrics.record_transcription_failure();

        let rate = metrics.get_success_rate();
        assert!((rate - 0.666).abs() < 0.01, "Expected ~0.666, got {}", rate);
    }

    #[test]
    fn test_performance_alerts() {
        let metrics = SttPerformanceMetrics::new();
        let thresholds = PerformanceThresholds {
            max_latency_us: 100_000, // 100ms
            min_confidence: 0.8,
            max_error_rate_per_1k: 10,
            max_memory_bytes: 1024,
        };

        // Trigger high latency alert
        metrics.record_end_to_end_latency(Duration::from_millis(200));

        // Trigger low confidence alert
        metrics.record_confidence_score(0.5);

        let alerts = metrics.check_alerts(&thresholds);
        assert!(
            alerts.len() >= 2,
            "Expected at least 2 alerts, got {}",
            alerts.len()
        );

        // Check for high latency alert
        let has_latency_alert = alerts
            .iter()
            .any(|alert| matches!(alert, PerformanceAlert::HighLatency { .. }));
        assert!(has_latency_alert, "Expected high latency alert");
    }

    #[test]
    fn test_timing_measurement() {
        let timing = TimingMeasurement::start("test");
        std::thread::sleep(Duration::from_millis(10));
        let (label, duration) = timing.end();

        assert_eq!(label, "test");
        assert!(
            duration >= Duration::from_millis(9),
            "Duration too short: {:?}",
            duration
        );
        assert!(
            duration <= Duration::from_millis(50),
            "Duration too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_memory_usage_tracking() {
        let metrics = SttPerformanceMetrics::new();

        metrics.update_memory_usage(1024);
        assert_eq!(
            metrics
                .resources
                .memory_usage_bytes
                .load(std::sync::atomic::Ordering::Relaxed),
            1024
        );

        // Test peak tracking
        metrics.update_memory_usage(2048);
        assert_eq!(
            metrics
                .resources
                .peak_memory_bytes
                .load(std::sync::atomic::Ordering::Relaxed),
            2048
        );

        // Lower usage shouldn't affect peak
        metrics.update_memory_usage(512);
        assert_eq!(
            metrics
                .resources
                .peak_memory_bytes
                .load(std::sync::atomic::Ordering::Relaxed),
            2048
        );
    }
}
