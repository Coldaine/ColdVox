//! STT-specific performance metrics and monitoring
//!
//! This module provides comprehensive performance monitoring for Speech-to-Text (STT)
//! operations, including latency tracking, accuracy measurements, resource usage,
//! and operational metrics.

use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

const HISTORY_CAPACITY: usize = 100;

/// A clonable, thread-safe container for all STT performance metrics.
#[derive(Clone, Default)]
pub struct SttPerformanceMetrics {
    inner: Arc<RwLock<MetricsInner>>,
}

/// Private inner struct holding all the metric values.
#[derive(Default)]
struct MetricsInner {
    latency: LatencyMetrics,
    accuracy: AccuracyMetrics,
    resources: ResourceMetrics,
    operational: OperationalMetrics,
    latency_history: VecDeque<LatencySnapshot>,
    word_confidence_history: VecDeque<f64>,
}

#[derive(Default, Clone, Debug)]
pub struct LatencyMetrics {
    pub end_to_end_us: u64,
    pub engine_processing_us: u64,
    pub preprocessing_us: u64,
    pub result_delivery_us: u64,
    pub model_loading_ms: u64,
}

#[derive(Default, Clone, Debug)]
pub struct AccuracyMetrics {
    pub confidence_sum: f64,
    pub confidence_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub partial_count: u64,
    pub final_count: u64,
}

#[derive(Default, Clone, Debug)]
pub struct ResourceMetrics {
    pub memory_usage_bytes: u64,
    pub peak_memory_bytes: u64,
    pub buffer_utilization_pct: u64,
    pub active_threads: u64,
    pub allocated_buffers: u64,
}

#[derive(Default, Clone, Debug)]
pub struct OperationalMetrics {
    pub request_count: u64,
    pub error_count: u64,
    pub model_switches: u64,
    pub fallback_usage: u64,
    pub avg_queue_depth: u64,
    pub processing_fps: u64,
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
    pub max_latency_us: u64,
    pub min_confidence: f64,
    pub max_error_rate_per_1k: u64,
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
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceAlert {
    HighLatency { measured_us: u64, threshold_us: u64 },
    LowConfidence { measured: f64, threshold: f64 },
    HighErrorRate { measured_per_1k: u64, threshold_per_1k: u64 },
    HighMemoryUsage { measured_bytes: u64, threshold_bytes: u64 },
    ProcessingStalled { last_activity: Duration },
}

impl SttPerformanceMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_end_to_end_latency(&self, latency: Duration) {
        let latency_us = latency.as_micros() as u64;
        let mut inner = self.inner.write();
        inner.latency.end_to_end_us = latency_us;
        let snapshot = LatencySnapshot {
            timestamp: Instant::now(),
            end_to_end_us: latency_us,
            engine_processing_us: inner.latency.engine_processing_us,
            preprocessing_us: inner.latency.preprocessing_us,
            result_delivery_us: inner.latency.result_delivery_us,
        };
        inner.latency_history.push_back(snapshot);
        if inner.latency_history.len() > HISTORY_CAPACITY {
            inner.latency_history.pop_front();
        }
    }

    // ... all other recording methods will follow this pattern ...

    pub fn record_engine_processing_time(&self, duration: Duration) {
        self.inner.write().latency.engine_processing_us = duration.as_micros() as u64;
    }

    pub fn record_preprocessing_latency(&self, duration: Duration) {
        self.inner.write().latency.preprocessing_us = duration.as_micros() as u64;
    }

    pub fn record_result_delivery_latency(&self, duration: Duration) {
        self.inner.write().latency.result_delivery_us = duration.as_micros() as u64;
    }

    pub fn record_confidence_score(&self, confidence: f64) {
        let mut inner = self.inner.write();
        inner.accuracy.confidence_sum += confidence;
        inner.accuracy.confidence_count += 1;
        inner.word_confidence_history.push_back(confidence);
        if inner.word_confidence_history.len() > HISTORY_CAPACITY {
            inner.word_confidence_history.pop_front();
        }
    }

    pub fn record_transcription_success(&self) {
        self.inner.write().accuracy.success_count += 1;
    }

    pub fn record_transcription_failure(&self) {
        self.inner.write().accuracy.failure_count += 1;
    }

    pub fn record_partial_transcription(&self) {
        self.inner.write().accuracy.partial_count += 1;
    }

    pub fn record_final_transcription(&self) {
        self.inner.write().accuracy.final_count += 1;
    }

    pub fn update_memory_usage(&self, bytes: u64) {
        let mut inner = self.inner.write();
        inner.resources.memory_usage_bytes = bytes;
        if bytes > inner.resources.peak_memory_bytes {
            inner.resources.peak_memory_bytes = bytes;
        }
    }

    pub fn update_buffer_utilization(&self, utilization_pct: u64) {
        self.inner.write().resources.buffer_utilization_pct = utilization_pct.min(100);
    }

    pub fn increment_requests(&self) {
        self.inner.write().operational.request_count += 1;
    }

    pub fn record_error(&self) {
        self.inner.write().operational.error_count += 1;
    }

    pub fn get_average_confidence(&self) -> f64 {
        let inner = self.inner.read();
        if inner.accuracy.confidence_count > 0 {
            inner.accuracy.confidence_sum / inner.accuracy.confidence_count as f64
        } else {
            0.0
        }
    }

    pub fn get_success_rate(&self) -> f64 {
        let inner = self.inner.read();
        let total = inner.accuracy.success_count + inner.accuracy.failure_count;
        if total > 0 {
            inner.accuracy.success_count as f64 / total as f64
        } else {
            0.0
        }
    }

    pub fn check_alerts(&self, thresholds: &PerformanceThresholds) -> Vec<PerformanceAlert> {
        let mut alerts = Vec::new();
        let inner = self.inner.read();

        if inner.latency.end_to_end_us > thresholds.max_latency_us {
            alerts.push(PerformanceAlert::HighLatency {
                measured_us: inner.latency.end_to_end_us,
                threshold_us: thresholds.max_latency_us,
            });
        }

        let avg_confidence = self.get_average_confidence();
        if avg_confidence > 0.0 && avg_confidence < thresholds.min_confidence {
            alerts.push(PerformanceAlert::LowConfidence {
                measured: avg_confidence,
                threshold: thresholds.min_confidence,
            });
        }

        let total_ops = inner.operational.request_count;
        if total_ops > 1000 { // Only check if significant number of operations
             let error_rate_per_1k = (inner.operational.error_count * 1000) / total_ops;
             if error_rate_per_1k > thresholds.max_error_rate_per_1k {
                alerts.push(PerformanceAlert::HighErrorRate {
                    measured_per_1k: error_rate_per_1k,
                    threshold_per_1k: thresholds.max_error_rate_per_1k,
                });
            }
        }

        if inner.resources.memory_usage_bytes > thresholds.max_memory_bytes {
            alerts.push(PerformanceAlert::HighMemoryUsage {
                measured_bytes: inner.resources.memory_usage_bytes,
                threshold_bytes: thresholds.max_memory_bytes,
            });
        }

        alerts
    }

    // ... other getter methods if needed ...
    pub fn get_latency_trend(&self) -> Option<f64> {
        let inner = self.inner.read();
        if inner.latency_history.len() < 10 { return None; }

        let recent: Vec<_> = inner.latency_history.iter().rev().take(10).collect();
        let n = recent.len() as f64;
        let sum_x: f64 = (0..recent.len()).map(|i| i as f64).sum();
        let sum_y: f64 = recent.iter().map(|s| s.end_to_end_us as f64).sum();
        let sum_xy: f64 = recent.iter().enumerate().map(|(i, s)| i as f64 * s.end_to_end_us as f64).sum();
        let sum_x2: f64 = (0..recent.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        Some(slope)
    }

    // This gives a snapshot of the current metrics state
    pub fn snapshot(&self) -> (LatencyMetrics, AccuracyMetrics, ResourceMetrics, OperationalMetrics) {
        let inner = self.inner.read();
        (
            inner.latency.clone(),
            inner.accuracy.clone(),
            inner.resources.clone(),
            inner.operational.clone()
        )
    }
}

/// A simple RAII timer that records a metric when it goes out of scope.
pub struct TimingGuard<'a, F>
where
    F: FnOnce(&'a SttPerformanceMetrics, Duration),
{
    start: Instant,
    metrics: &'a SttPerformanceMetrics,
    record_fn: Option<F>,
}

impl<'a, F> TimingGuard<'a, F>
where
    F: FnOnce(&'a SttPerformanceMetrics, Duration),
{
    pub fn new(metrics: &'a SttPerformanceMetrics, record_fn: F) -> Self {
        Self {
            start: Instant::now(),
            metrics,
            record_fn: Some(record_fn),
        }
    }
}

impl<'a, F> Drop for TimingGuard<'a, F>
where
    F: FnOnce(&'a SttPerformanceMetrics, Duration),
{
    fn drop(&mut self) {
        if let Some(record_fn) = self.record_fn.take() {
            (record_fn)(self.metrics, self.start.elapsed());
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stt_performance_metrics_creation() {
        let metrics = SttPerformanceMetrics::new();
        assert_eq!(metrics.get_average_confidence(), 0.0);
        assert_eq!(metrics.get_success_rate(), 0.0);
    }

    #[test]
    fn test_latency_recording() {
        let metrics = SttPerformanceMetrics::new();
        let latency = Duration::from_millis(250);
        metrics.record_end_to_end_latency(latency);
        let (latency_metrics, _, _, _) = metrics.snapshot();
        assert_eq!(latency_metrics.end_to_end_us, 250_000);
    }

    #[test]
    fn test_confidence_tracking() {
        let metrics = SttPerformanceMetrics::new();
        metrics.record_confidence_score(0.8);
        metrics.record_confidence_score(0.9);
        metrics.record_confidence_score(0.7);
        let avg = metrics.get_average_confidence();
        assert!((avg - 0.8).abs() < 0.001, "Expected ~0.8, got {}", avg);
    }

    #[test]
    fn test_success_rate_calculation() {
        let metrics = SttPerformanceMetrics::new();
        metrics.record_transcription_success();
        metrics.record_transcription_success();
        metrics.record_transcription_failure();
        let rate = metrics.get_success_rate();
        assert!((rate - 2.0 / 3.0).abs() < 0.001, "Expected ~0.666, got {}", rate);
    }

    #[test]
    fn test_performance_alerts() {
        let metrics = SttPerformanceMetrics::new();
        let thresholds = PerformanceThresholds {
            max_latency_us: 100_000,
            min_confidence: 0.8,
            max_error_rate_per_1k: 10,
            max_memory_bytes: 1024,
        };

        metrics.record_end_to_end_latency(Duration::from_millis(200));
        metrics.record_confidence_score(0.5);

        let alerts = metrics.check_alerts(&thresholds);
        assert_eq!(alerts.len(), 2);
        assert!(alerts.contains(&PerformanceAlert::HighLatency { measured_us: 200_000, threshold_us: 100_000 }));
        assert!(alerts.contains(&PerformanceAlert::LowConfidence { measured: 0.5, threshold: 0.8 }));
    }

    #[test]
    fn test_timing_guard() {
        let metrics = SttPerformanceMetrics::new();
        {
            let _guard = TimingGuard::new(&metrics, |m, d| m.record_end_to_end_latency(d));
            std::thread::sleep(Duration::from_millis(10));
        }
        let (latency, _, _, _) = metrics.snapshot();
        assert!(latency.end_to_end_us >= 10_000);
        assert!(latency.end_to_end_us < 50_000);
    }

    #[test]
    fn test_memory_usage_tracking() {
        let metrics = SttPerformanceMetrics::new();
        metrics.update_memory_usage(1024);
        let (_, _, resources, _) = metrics.snapshot();
        assert_eq!(resources.memory_usage_bytes, 1024);
        assert_eq!(resources.peak_memory_bytes, 1024);

        metrics.update_memory_usage(2048);
        let (_, _, resources, _) = metrics.snapshot();
        assert_eq!(resources.peak_memory_bytes, 2048);

        metrics.update_memory_usage(512);
        let (_, _, resources, _) = metrics.snapshot();
        assert_eq!(resources.peak_memory_bytes, 2048);
        assert_eq!(resources.memory_usage_bytes, 512);
    }
}
