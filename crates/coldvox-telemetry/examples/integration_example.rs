//! Simple integration example showing how to use STT performance metrics
//!
//! This example demonstrates the easy integration API for adding
//! comprehensive STT performance monitoring to existing applications.

use coldvox_telemetry::{LatencyTrend, SttMetricsBuilder, SttMetricsManager};
use std::time::Duration;

fn main() {
    println!("=== STT Metrics Integration Example ===\n");

    // Example 1: Quick setup with preset configuration
    println!("1. Setting up metrics with production preset...");
    let metrics_manager = SttMetricsBuilder::production().build();
    println!("✓ Production metrics configured with 500ms latency threshold\n");

    // Example 2: Custom configuration
    println!("2. Setting up custom configuration...");
    let _custom_manager = SttMetricsBuilder::new()
        .with_max_latency(200) // 200ms max latency
        .with_min_confidence(0.85) // 85% min confidence
        .with_max_memory_mb(256) // 256MB max memory
        .build();
    println!("✓ Custom metrics configured\n");

    // Example 3: Simulate some transcription activity
    println!("3. Simulating STT activity...");
    simulate_stt_activity(&metrics_manager);

    // Example 4: Check performance and alerts
    println!("\n4. Performance monitoring...");
    monitor_performance(&metrics_manager);

    // Example 5: Integration with existing STT processor
    println!("\n5. Integration example...");
    show_processor_integration();
}

fn simulate_stt_activity(manager: &SttMetricsManager) {
    // Simulate successful transcriptions
    for i in 1..=3 {
        let latency = Duration::from_millis(150 + i * 50);
        let processing_time = Duration::from_millis(120 + i * 30);
        let confidence = 0.9 - (i as f64 * 0.05);

        manager.record_successful_transcription(latency, processing_time, Some(confidence));
        println!(
            "  ✓ Recorded successful transcription {} ({}ms, {:.1}% confidence)",
            i,
            latency.as_millis(),
            confidence * 100.0
        );
    }

    // Simulate a failure
    manager.record_failed_transcription(Some(Duration::from_millis(500)));
    println!("  ⚠️  Recorded failed transcription (500ms)");
}

fn monitor_performance(manager: &SttMetricsManager) {
    // Get performance summary
    let summary = manager.get_performance_summary();
    println!("Performance Summary:");
    println!("  • Average Latency: {:.1}ms", summary.avg_latency_ms);
    println!(
        "  • Average Confidence: {:.1}%",
        summary.avg_confidence * 100.0
    );
    println!("  • Success Rate: {:.1}%", summary.success_rate * 100.0);
    println!("  • Total Requests: {}", summary.total_requests);

    // Check for alerts
    let alerts = manager.check_alerts();
    if alerts.is_empty() {
        println!("  ✅ No performance alerts");
    } else {
        println!("  ⚠️  {} alert(s) detected", alerts.len());
    }

    // Check latency trend
    match manager.get_latency_trend() {
        Some(LatencyTrend::Increasing) => println!("  📈 Latency trend: Increasing"),
        Some(LatencyTrend::Decreasing) => println!("  📉 Latency trend: Decreasing"),
        Some(LatencyTrend::Stable) => println!("  ➡️  Latency trend: Stable"),
        None => println!("  📊 Latency trend: Insufficient data"),
    }

    // Get formatted report
    println!("\nFormatted Report:");
    println!("{}", manager.format_metrics_report());
}

fn show_processor_integration() {
    println!("Example integration with STT processor:");
    println!();
    println!("```rust");
    println!("// In your STT processor initialization:");
    println!("let metrics_manager = SttMetricsBuilder::production().build();");
    println!("let performance_metrics = metrics_manager.metrics();");
    println!();
    println!("// Set the metrics on your STT processor:");
    println!("stt_processor.set_performance_metrics(performance_metrics);");
    println!();
    println!("// Periodic monitoring (in your application loop):");
    println!("let alerts = metrics_manager.check_alerts();");
    println!("if !alerts.is_empty() {{");
    println!("    warn!(\"STT performance alerts: {{:?}}\", alerts);");
    println!("}}");
    println!();
    println!("// Log performance report periodically:");
    println!("info!(\"{{}}\", metrics_manager.format_metrics_report());");
    println!("```");
    println!();
    println!("✓ The metrics are automatically collected during transcription");
    println!("✓ Minimal code changes required for existing applications");
    println!("✓ Comprehensive monitoring with configurable alerts");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_example() {
        // Test that the integration example works without panics
        let manager = SttMetricsBuilder::testing().build();
        simulate_stt_activity(&manager);
        monitor_performance(&manager);

        let summary = manager.get_performance_summary();
        assert!(summary.total_requests > 0);
        assert!(summary.total_errors > 0);
    }
}
