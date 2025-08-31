use std::sync::Arc;
use crate::telemetry::pipeline_metrics::PipelineMetrics;
use crate::text_injection::manager::StrategyManager;
use crate::text_injection::types::{InjectionConfig, InjectionMetrics};
use crate::probes::common::{LiveTestResult, TestContext, TestError};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug)]
pub struct TextInjectionProbe;

impl TextInjectionProbe {
    pub async fn run(_ctx: &TestContext) -> Result<LiveTestResult, TestError> {
        let config = InjectionConfig::default();
        
        // Create shared metrics
        let _metrics = Arc::new(PipelineMetrics::default());
        let injection_metrics = Arc::new(std::sync::Mutex::new(InjectionMetrics::default()));
        
        // Create strategy manager
        let mut manager = StrategyManager::new(config, injection_metrics.clone());
        
        // Test basic injection
        let start_time = std::time::Instant::now();
        let result = manager.inject("Test injection").await;
        let duration = start_time.elapsed().as_millis() as u64;
        
        // Collect metrics
        let injection_metrics_guard = injection_metrics.lock().unwrap();
        let mut metrics_map = HashMap::new();
        metrics_map.insert("success".to_string(), json!(result.is_ok()));
        metrics_map.insert("duration_ms".to_string(), json!(duration));
        metrics_map.insert("attempts".to_string(), json!(injection_metrics_guard.attempts));
        metrics_map.insert("successes".to_string(), json!(injection_metrics_guard.successes));
        metrics_map.insert("failures".to_string(), json!(injection_metrics_guard.failures));
        
        // Evaluate results
        let (pass, notes) = evaluate_injection_result(&result, &metrics_map);
        
        Ok(LiveTestResult {
            test: "text_injection".to_string(),
            pass,
            metrics: metrics_map,
            notes: Some(notes),
            artifacts: vec![],
        })
    }
}

fn evaluate_injection_result(result: &Result<(), crate::text_injection::types::InjectionError>, 
                           metrics: &HashMap<String, serde_json::Value>) -> (bool, String) {
    let mut pass = true;
    let mut issues = Vec::new();
    
    match result {
        Ok(()) => {
            // Check if metrics are reasonable
            if let Some(successes) = metrics.get("successes").and_then(|v| v.as_u64()) {
                if successes != 1 {
                    pass = false;
                    issues.push(format!("Expected 1 success, got {}", successes));
                }
            }
            
            if let Some(attempts) = metrics.get("attempts").and_then(|v| v.as_u64()) {
                if attempts != 1 {
                    pass = false;
                    issues.push(format!("Expected 1 attempt, got {}", attempts));
                }
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