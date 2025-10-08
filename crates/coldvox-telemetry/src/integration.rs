//! Integration helpers for STT performance metrics
//!
//! This module provides utilities to easily integrate STT performance metrics
//! into existing applications with minimal code changes.

use crate::stt_metrics::{PerformanceAlert, PerformanceThresholds, SttPerformanceMetrics};
use std::sync::Arc;
use std::time::Duration;

/// Builder for easy STT performance metrics setup
pub struct SttMetricsBuilder {
    thresholds: PerformanceThresholds,
    enable_alerts: bool,
    enable_trending: bool,
}

impl SttMetricsBuilder {
    /// Create a new metrics builder with default settings
    pub fn new() -> Self {
        Self {
            thresholds: PerformanceThresholds::default(),
            enable_alerts: true,
            enable_trending: true,
        }
    }

    /// Set custom performance thresholds
    pub fn with_thresholds(mut self, thresholds: PerformanceThresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// Set maximum acceptable latency threshold
    pub fn with_max_latency(mut self, latency_ms: u64) -> Self {
        self.thresholds.max_latency_us = latency_ms * 1000;
        self
    }

    /// Set minimum confidence threshold
    pub fn with_min_confidence(mut self, confidence: f64) -> Self {
        self.thresholds.min_confidence = confidence;
        self
    }

    /// Set maximum error rate threshold (per 1000 operations)
    pub fn with_max_error_rate(mut self, error_rate_per_1k: u64) -> Self {
        self.thresholds.max_error_rate_per_1k = error_rate_per_1k;
        self
    }

    /// Set maximum memory usage threshold (in MB)
    pub fn with_max_memory_mb(mut self, memory_mb: u64) -> Self {
        self.thresholds.max_memory_bytes = memory_mb * 1024 * 1024;
        self
    }

    /// Enable or disable performance alerts
    pub fn with_alerts(mut self, enable: bool) -> Self {
        self.enable_alerts = enable;
        self
    }

    /// Enable or disable latency trending analysis
    pub fn with_trending(mut self, enable: bool) -> Self {
        self.enable_trending = enable;
        self
    }

    /// Build the metrics system
    pub fn build(self) -> SttMetricsManager {
        SttMetricsManager {
            metrics: Arc::new(SttPerformanceMetrics::new()),
            thresholds: self.thresholds,
            enable_alerts: self.enable_alerts,
            enable_trending: self.enable_trending,
        }
    }
}

impl Default for SttMetricsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for STT performance metrics with built-in alert handling
pub struct SttMetricsManager {
    metrics: Arc<SttPerformanceMetrics>,
    thresholds: PerformanceThresholds,
    enable_alerts: bool,
    enable_trending: bool,
}

impl SttMetricsManager {
    /// Get reference to the metrics for integration with STT processor
    pub fn metrics(&self) -> Arc<SttPerformanceMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Record a successful transcription with timing and confidence
    pub fn record_successful_transcription(
        &self,
        end_to_end_latency: Duration,
        engine_processing_time: Duration,
        confidence_score: Option<f64>,
    ) {
        self.metrics.record_end_to_end_latency(end_to_end_latency);
        self.metrics
            .record_engine_processing_time(engine_processing_time);
        self.metrics.record_transcription_success();
        self.metrics.record_final_transcription();
        self.metrics.increment_total_requests();

        if let Some(confidence) = confidence_score {
            self.metrics.record_confidence_score(confidence);
        }
    }

    /// Record a failed transcription
    pub fn record_failed_transcription(&self, error_latency: Option<Duration>) {
        self.metrics.record_transcription_failure();
        self.metrics.record_error();
        self.metrics.increment_total_requests();

        if let Some(latency) = error_latency {
            self.metrics.record_end_to_end_latency(latency);
        }
    }

    /// Get current performance summary
    pub fn get_performance_summary(&self) -> PerformanceSummary {
        let latency_us = self
            .metrics
            .latency
            .end_to_end_us
            .load(std::sync::atomic::Ordering::Relaxed);
        let avg_confidence = self.metrics.get_average_confidence();
        let success_rate = self.metrics.get_success_rate();
        let memory_usage = self
            .metrics
            .resources
            .memory_usage_bytes
            .load(std::sync::atomic::Ordering::Relaxed);

        PerformanceSummary {
            avg_latency_ms: latency_us as f64 / 1000.0,
            avg_confidence,
            success_rate,
            memory_usage_mb: memory_usage as f64 / (1024.0 * 1024.0),
            total_requests: self
                .metrics
                .operational
                .requests_per_second
                .load(std::sync::atomic::Ordering::Relaxed),
            total_errors: self
                .metrics
                .operational
                .error_rate_per_1k
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Check for performance alerts and return them
    pub fn check_alerts(&self) -> Vec<PerformanceAlert> {
        if !self.enable_alerts {
            return Vec::new();
        }

        self.metrics.check_alerts(&self.thresholds)
    }

    /// Get latency trend if trending is enabled
    pub fn get_latency_trend(&self) -> Option<LatencyTrend> {
        if !self.enable_trending {
            return None;
        }

        self.metrics.get_latency_trend().map(|slope| {
            if slope > 1000.0 {
                LatencyTrend::Increasing
            } else if slope < -1000.0 {
                LatencyTrend::Decreasing
            } else {
                LatencyTrend::Stable
            }
        })
    }

    /// Get a formatted metrics report for logging
    pub fn format_metrics_report(&self) -> String {
        let summary = self.get_performance_summary();
        let alerts = self.check_alerts();

        let mut report = format!(
            "STT Performance Report:\n\
             • Latency: {:.1}ms avg\n\
             • Confidence: {:.1}%\n\
             • Success Rate: {:.1}%\n\
             • Memory: {:.1}MB\n\
             • Requests: {} total, {} errors",
            summary.avg_latency_ms,
            summary.avg_confidence * 100.0,
            summary.success_rate * 100.0,
            summary.memory_usage_mb,
            summary.total_requests,
            summary.total_errors
        );

        if !alerts.is_empty() {
            report.push_str(&format!("\n⚠️  {} alert(s) active", alerts.len()));
        }

        if let Some(trend) = self.get_latency_trend() {
            let trend_str = match trend {
                LatencyTrend::Increasing => "📈 Increasing",
                LatencyTrend::Decreasing => "📉 Decreasing",
                LatencyTrend::Stable => "➡️ Stable",
            };
            report.push_str(&format!("\n• Latency Trend: {}", trend_str));
        }

        report
    }
}

/// Performance summary snapshot
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub avg_latency_ms: f64,
    pub avg_confidence: f64,
    pub success_rate: f64,
    pub memory_usage_mb: f64,
    pub total_requests: u64,
    pub total_errors: u64,
}

/// Latency trend analysis result
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LatencyTrend {
    Increasing,
    Decreasing,
    Stable,
}

/// Easy-to-use presets for common configurations
impl SttMetricsBuilder {
    /// Production configuration with moderate thresholds
    pub fn production() -> Self {
        Self::new()
            .with_max_latency(500) // 500ms
            .with_min_confidence(0.75) // 75%
            .with_max_error_rate(50) // 5%
            .with_max_memory_mb(512) // 512MB
    }

    /// Development configuration with relaxed thresholds
    pub fn development() -> Self {
        Self::new()
            .with_max_latency(1000) // 1s
            .with_min_confidence(0.6) // 60%
            .with_max_error_rate(100) // 10%
            .with_max_memory_mb(1024) // 1GB
    }

    /// High-performance configuration with strict thresholds
    pub fn high_performance() -> Self {
        Self::new()
            .with_max_latency(100) // 100ms
            .with_min_confidence(0.9) // 90%
            .with_max_error_rate(10) // 1%
            .with_max_memory_mb(256) // 256MB
    }

    /// Testing configuration with very relaxed thresholds
    pub fn testing() -> Self {
        Self::new().with_alerts(false).with_trending(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_builder() {
        let manager = SttMetricsBuilder::new()
            .with_max_latency(200)
            .with_min_confidence(0.8)
            .build();

        assert_eq!(manager.thresholds.max_latency_us, 200_000);
        assert_eq!(manager.thresholds.min_confidence, 0.8);
    }

    #[test]
    fn test_preset_configurations() {
        let prod = SttMetricsBuilder::production().build();
        assert_eq!(prod.thresholds.max_latency_us, 500_000);

        let dev = SttMetricsBuilder::development().build();
        assert_eq!(dev.thresholds.max_latency_us, 1_000_000);

        let hp = SttMetricsBuilder::high_performance().build();
        assert_eq!(hp.thresholds.max_latency_us, 100_000);
    }

    #[test]
    fn test_performance_summary() {
        let manager = SttMetricsBuilder::testing().build();

        manager.record_successful_transcription(
            Duration::from_millis(150),
            Duration::from_millis(100),
            Some(0.85),
        );

        let summary = manager.get_performance_summary();
        assert!(summary.avg_latency_ms > 0.0);
        assert!(summary.avg_confidence > 0.0);
    }

    #[test]
    fn test_metrics_report_formatting() {
        let manager = SttMetricsBuilder::testing().build();

        manager.record_successful_transcription(
            Duration::from_millis(100),
            Duration::from_millis(80),
            Some(0.9),
        );

        let report = manager.format_metrics_report();
        assert!(report.contains("STT Performance Report"));
        assert!(report.contains("Latency:"));
        assert!(report.contains("Confidence:"));
    }
}
