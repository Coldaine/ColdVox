use crate::probes::common::{LiveTestResult, TestContext, TestError};
use crate::text_injection::manager::StrategyManager;
use crate::text_injection::{InjectionConfig, InjectionMetrics};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct TextInjectionProbe;

impl TextInjectionProbe {
    pub async fn run(_ctx: &TestContext) -> Result<LiveTestResult, TestError> {
        let config = InjectionConfig::default();

        let injection_metrics = Arc::new(std::sync::Mutex::new(InjectionMetrics::default()));

        // Create strategy manager
        let manager = StrategyManager::new(config);

        // Test basic injection
        let start_time = std::time::Instant::now();
        let mut metrics = crate::text_injection::InjectionMetrics::default();
        let result = manager.inject_with_fail_fast("Test injection", &mut metrics).await;
        let duration = start_time.elapsed().as_millis() as u64;

        // Collect metrics
        let injection_metrics_guard = injection_metrics.lock().unwrap();
        let mut metrics_map = HashMap::new();
        metrics_map.insert("success".to_string(), json!(result.is_ok()));
        metrics_map.insert("duration_ms".to_string(), json!(duration));
        metrics_map.insert(
            "attempts".to_string(),
            json!(injection_metrics_guard.total_attempts),
        );
        metrics_map.insert(
            "successes".to_string(),
            json!(injection_metrics_guard.total_successes),
        );
        metrics_map.insert(
            "failures".to_string(),
            json!(injection_metrics_guard.failures_by_kind.len()),
        );

        // Evaluate results
        let (pass, notes) = evaluate_injection_result(&result);

        Ok(LiveTestResult {
            test: "text_injection".to_string(),
            pass,
            metrics: metrics_map,
            notes: Some(notes),
            artifacts: vec![],
        })
    }
}

fn evaluate_injection_result(
    result: &Result<crate::text_injection::InjectionOutcome, crate::text_injection::InjectionError>,
) -> (bool, String) {
    let mut pass = true;
    let mut issues = Vec::new();

    match result {
        Ok(outcome) => {
            // Check if the injection was successful
            if outcome.latency_ms == 0 {
                pass = false;
                issues.push("Injection reported 0ms latency, which seems incorrect".to_string());
            }
        }
        Err(e) => {
            pass = false;
            issues.push(format!("Injection failed: {}", e));
        }
    }

    let notes = if issues.is_empty() {
        "Text injection test completed successfully".to_string()
    } else {
        format!("Issues found: {}", issues.join("; "))
    };

    (pass, notes)
}
