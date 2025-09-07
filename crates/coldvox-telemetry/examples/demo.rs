//! Example demonstrating comprehensive STT performance metrics
//! 
//! This example shows how to use the enhanced STT telemetry system
//! to monitor transcription performance, latency, accuracy, and resource usage.

use coldvox_telemetry::{SttPerformanceMetrics, PerformanceThresholds, TimingMeasurement};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    println!("=== STT Performance Metrics Demo ===\n");

    // Create performance metrics instance
    let metrics = Arc::new(SttPerformanceMetrics::new());
    println!("✓ Created STT performance metrics system");

    // Simulate a transcription session
    simulate_transcription_session(&metrics);

    // Display comprehensive metrics report
    display_metrics_report(&metrics);

    // Check for performance alerts
    check_performance_alerts(&metrics);
}

fn simulate_transcription_session(metrics: &Arc<SttPerformanceMetrics>) {
    println!("\n--- Simulating Transcription Session ---");

    for i in 1..=5 {
        println!("Processing utterance {}...", i);
        
        // Simulate preprocessing
        let preprocessing = TimingMeasurement::start("preprocessing");
        std::thread::sleep(Duration::from_millis(5 + i * 2));
        preprocessing.end_and_record(|duration| {
            metrics.record_preprocessing_latency(duration);
        });

        // Simulate STT engine processing
        let engine = TimingMeasurement::start("engine_processing");
        std::thread::sleep(Duration::from_millis(100 + i * 20));
        engine.end_and_record(|duration| {
            metrics.record_engine_processing_time(duration);
        });

        // Simulate result delivery
        let delivery = TimingMeasurement::start("result_delivery");
        std::thread::sleep(Duration::from_millis(2));
        delivery.end_and_record(|duration| {
            metrics.record_result_delivery_latency(duration);
        });

        // Record end-to-end latency
        let total_latency = Duration::from_millis(107 + i * 22);
        metrics.record_end_to_end_latency(total_latency);

        // Record confidence score (simulating varying accuracy)
        let confidence = match i {
            1 => 0.95, // High confidence
            2 => 0.87, // Good confidence
            3 => 0.92, // High confidence
            4 => 0.73, // Lower confidence
            5 => 0.88, // Good confidence
            _ => 0.85, // Default confidence
        };
        metrics.record_confidence_score(confidence);

        // Record transcription outcome
        if confidence > 0.75 {
            metrics.record_transcription_success();
            metrics.record_final_transcription();
        } else {
            metrics.record_transcription_failure();
        }

        // Simulate memory usage
        let memory_usage = 1024 * 1024 * (20 + i * 5); // 20-45MB
        metrics.update_memory_usage(memory_usage);

        // Record processing
        metrics.increment_requests();
    }

    // Simulate a processing error
    println!("Simulating processing error...");
    metrics.record_transcription_failure();
    metrics.record_error();

    println!("✓ Completed transcription session simulation");
}

fn display_metrics_report(metrics: &Arc<SttPerformanceMetrics>) {
    println!("\n--- Performance Metrics Report ---");

    // Latency metrics
    let e2e_latency = metrics.latency.end_to_end_us.load(std::sync::atomic::Ordering::Relaxed);
    let engine_time = metrics.latency.engine_processing_us.load(std::sync::atomic::Ordering::Relaxed);
    let preprocessing = metrics.latency.preprocessing_us.load(std::sync::atomic::Ordering::Relaxed);
    let delivery = metrics.latency.result_delivery_us.load(std::sync::atomic::Ordering::Relaxed);
    
    println!("📊 Latency Metrics:");
    println!("  End-to-End:      {:.1}ms", e2e_latency as f64 / 1000.0);
    println!("  Engine Processing: {:.1}ms", engine_time as f64 / 1000.0);
    println!("  Preprocessing:   {:.1}ms", preprocessing as f64 / 1000.0);
    println!("  Result Delivery: {:.1}ms", delivery as f64 / 1000.0);

    // Accuracy metrics
    let avg_confidence = metrics.get_average_confidence();
    let success_rate = metrics.get_success_rate();
    let final_count = metrics.accuracy.final_count.load(std::sync::atomic::Ordering::Relaxed);
    let failure_count = metrics.accuracy.failure_count.load(std::sync::atomic::Ordering::Relaxed);
    
    println!("\n🎯 Accuracy Metrics:");
    println!("  Average Confidence: {:.1}%", avg_confidence * 100.0);
    println!("  Success Rate:       {:.1}%", success_rate * 100.0);
    println!("  Successful Finals:  {}", final_count);
    println!("  Failures:           {}", failure_count);

    // Resource metrics
    let memory_usage = metrics.resources.memory_usage_bytes.load(std::sync::atomic::Ordering::Relaxed);
    let peak_memory = metrics.resources.peak_memory_bytes.load(std::sync::atomic::Ordering::Relaxed);
    
    println!("\n💾 Resource Usage:");
    println!("  Current Memory: {:.1}MB", memory_usage as f64 / (1024.0 * 1024.0));
    println!("  Peak Memory:    {:.1}MB", peak_memory as f64 / (1024.0 * 1024.0));

    // Operational metrics
    let requests = metrics.operational.requests_per_second.load(std::sync::atomic::Ordering::Relaxed);
    let errors = metrics.operational.error_rate_per_1k.load(std::sync::atomic::Ordering::Relaxed);
    
    println!("\n⚙️  Operational Metrics:");
    println!("  Total Requests: {}", requests);
    println!("  Error Count:    {}", errors);
}

fn check_performance_alerts(metrics: &Arc<SttPerformanceMetrics>) {
    println!("\n--- Performance Alert Check ---");

    // Use stricter thresholds for demo
    let thresholds = PerformanceThresholds {
        max_latency_us: 150_000,  // 150ms
        min_confidence: 0.8,      // 80%
        max_error_rate_per_1k: 2, // 0.2%
        max_memory_bytes: 40 * 1024 * 1024, // 40MB
    };

    let alerts = metrics.check_alerts(&thresholds);
    
    if alerts.is_empty() {
        println!("✅ No performance alerts - all metrics within thresholds");
    } else {
        println!("⚠️  {} performance alert(s) detected:", alerts.len());
        
        for (i, alert) in alerts.iter().enumerate() {
            println!("  {}. {}", i + 1, format_alert(alert));
        }
    }

    // Demonstrate latency trend analysis
    if let Some(trend) = metrics.get_latency_trend() {
        println!("\n📈 Latency Trend Analysis:");
        if trend > 0.0 {
            println!("  ⚠️  Latency is increasing (slope: {:.2})", trend);
        } else if trend < 0.0 {
            println!("  ✅ Latency is decreasing (slope: {:.2})", trend);
        } else {
            println!("  ➡️  Latency is stable");
        }
    } else {
        println!("\n📈 Latency Trend: Insufficient data for analysis");
    }
}

fn format_alert(alert: &coldvox_telemetry::PerformanceAlert) -> String {
    match alert {
        coldvox_telemetry::PerformanceAlert::HighLatency { measured_us, threshold_us } => {
            format!("High Latency: {:.1}ms > {:.1}ms threshold", 
                   *measured_us as f64 / 1000.0, 
                   *threshold_us as f64 / 1000.0)
        }
        coldvox_telemetry::PerformanceAlert::LowConfidence { measured, threshold } => {
            format!("Low Confidence: {:.1}% < {:.1}% threshold", 
                   measured * 100.0, 
                   threshold * 100.0)
        }
        coldvox_telemetry::PerformanceAlert::HighErrorRate { measured_per_1k, threshold_per_1k } => {
            format!("High Error Rate: {}/1k > {}/1k threshold", 
                   measured_per_1k, 
                   threshold_per_1k)
        }
        coldvox_telemetry::PerformanceAlert::HighMemoryUsage { measured_bytes, threshold_bytes } => {
            format!("High Memory Usage: {:.1}MB > {:.1}MB threshold", 
                   *measured_bytes as f64 / (1024.0 * 1024.0),
                   *threshold_bytes as f64 / (1024.0 * 1024.0))
        }
        coldvox_telemetry::PerformanceAlert::ProcessingStalled { last_activity } => {
            format!("Processing Stalled: No activity for {:?}", last_activity)
        }
    }
}