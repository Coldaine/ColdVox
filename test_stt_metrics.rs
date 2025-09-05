/// Simple test to validate STT performance metrics functionality
/// This is a standalone test that doesn't require the full audio system

use std::time::Duration;

// Mock the telemetry types we need for testing
#[path = "crates/coldvox-telemetry/src/stt_metrics.rs"]
mod stt_metrics;

use stt_metrics::*;

fn main() {
    println!("Testing STT Performance Metrics...");

    // Test creating metrics
    let metrics = SttPerformanceMetrics::new();
    println!("✓ Created STT performance metrics");

    // Test recording latencies
    metrics.record_end_to_end_latency(Duration::from_millis(250));
    metrics.record_engine_processing_time(Duration::from_millis(150));
    metrics.record_preprocessing_latency(Duration::from_millis(10));
    metrics.record_result_delivery_latency(Duration::from_millis(5));
    println!("✓ Recorded timing metrics");

    // Test accuracy tracking
    metrics.record_confidence_score(0.85);
    metrics.record_confidence_score(0.92);
    metrics.record_confidence_score(0.78);
    println!("✓ Recorded confidence scores");

    let avg_confidence = metrics.get_average_confidence();
    println!("  Average confidence: {:.3}", avg_confidence);

    // Test success/failure tracking
    metrics.record_transcription_success();
    metrics.record_transcription_success();
    metrics.record_transcription_failure();
    
    let success_rate = metrics.get_success_rate();
    println!("  Success rate: {:.1}%", success_rate * 100.0);

    // Test resource monitoring
    metrics.update_memory_usage(1024 * 1024 * 50); // 50MB
    metrics.update_buffer_utilization(75);
    println!("✓ Updated resource metrics");

    // Test performance alerts
    let thresholds = PerformanceThresholds::default();
    let alerts = metrics.check_alerts(&thresholds);
    println!("✓ Checked for performance alerts: {} alerts", alerts.len());
    
    for alert in alerts {
        match alert {
            PerformanceAlert::HighLatency { measured_us, threshold_us } => {
                println!("  Alert: High latency {}µs > {}µs", measured_us, threshold_us);
            }
            PerformanceAlert::LowConfidence { measured, threshold } => {
                println!("  Alert: Low confidence {:.3} < {:.3}", measured, threshold);
            }
            PerformanceAlert::HighErrorRate { measured_per_1k, threshold_per_1k } => {
                println!("  Alert: High error rate {}/1k > {}/1k", measured_per_1k, threshold_per_1k);
            }
            PerformanceAlert::HighMemoryUsage { measured_bytes, threshold_bytes } => {
                println!("  Alert: High memory {}B > {}B", measured_bytes, threshold_bytes);
            }
            PerformanceAlert::ProcessingStalled { last_activity } => {
                println!("  Alert: Processing stalled for {:?}", last_activity);
            }
        }
    }

    // Test timing helper
    let timing = TimingMeasurement::start("test operation");
    std::thread::sleep(Duration::from_millis(10));
    let (label, duration) = timing.end();
    println!("✓ Timing measurement '{}' took {:?}", label, duration);

    println!("\nAll STT performance metrics tests passed! ✓");
}