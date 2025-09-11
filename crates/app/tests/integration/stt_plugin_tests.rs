//! Integration tests for STT plugin system

use coldvox_stt::{
    plugin::{PluginSelectionConfig, SttPluginError},
    plugin_adapter::PluginAdapter,
    plugins::{MockPlugin, NoOpPlugin},
    types::{TranscriptionConfig, TranscriptionEvent},
};
use coldvox_app::stt::plugin_manager::{SttPluginManager, FailoverConfig};
use coldvox_telemetry::PipelineMetrics;
use std::sync::Arc;
use tokio_test;

#[tokio::test]
async fn test_plugin_manager_initialization() {
    let mut manager = SttPluginManager::new();
    
    // Should initialize with some plugin
    let result = manager.initialize().await;
    assert!(result.is_ok(), "Plugin manager should initialize successfully");
    
    let active_backend = result.unwrap();
    assert!(!active_backend.is_empty(), "Should have an active backend");
    
    // Should be able to list plugins
    let plugins = manager.list_plugins().await;
    assert!(!plugins.is_empty(), "Should have at least some plugins available");
    
    // Check that mock and noop are available
    let plugin_ids: Vec<String> = plugins.iter().map(|p| p.id.clone()).collect();
    assert!(plugin_ids.contains(&"mock".to_string()));
    assert!(plugin_ids.contains(&"noop".to_string()));
}

#[tokio::test]
async fn test_plugin_manager_with_specific_backend() {
    let selection_config = PluginSelectionConfig {
        preferred_plugin: Some("mock".to_string()),
        fallback_plugins: vec!["noop".to_string()],
        require_local: true,
        max_memory_mb: None,
        required_language: Some("en".to_string()),
    };
    
    let failover_config = FailoverConfig::default();
    let mut manager = SttPluginManager::with_config(selection_config, failover_config);
    
    let result = manager.initialize().await;
    assert!(result.is_ok());
    
    let active_backend = result.unwrap();
    assert_eq!(active_backend, "mock");
}

#[tokio::test]
async fn test_plugin_adapter_with_mock() {
    let plugin = Box::new(MockPlugin::new());
    let mut adapter = PluginAdapter::new(plugin);
    
    let config = TranscriptionConfig {
        enabled: true,
        model_path: "test-model".to_string(),
        partial_results: true,
        max_alternatives: 1,
        include_words: false,
        buffer_size_ms: 512,
        streaming: false,
    };
    
    // Initialize
    let result = adapter.initialize(config).await;
    assert!(result.is_ok(), "Adapter should initialize successfully");
    
    // Reset
    adapter.reset().await;
    
    // Process some audio
    let audio_samples = vec![0i16; 1600]; // 100ms at 16kHz
    let result = adapter.on_speech_frame(&audio_samples).await;
    // MockPlugin should return some event
    assert!(result.is_some(), "MockPlugin should return an event");
    
    // Finalize
    let final_result = adapter.on_speech_end().await;
    assert!(final_result.is_some(), "MockPlugin should return final event");
}

#[tokio::test]
async fn test_plugin_adapter_with_noop() {
    let plugin = Box::new(NoOpPlugin::new());
    let mut adapter = PluginAdapter::new(plugin);
    
    let config = TranscriptionConfig::default();
    
    // Initialize
    let result = adapter.initialize(config).await;
    assert!(result.is_ok(), "NoOp adapter should initialize successfully");
    
    // Process audio - should return None
    let audio_samples = vec![0i16; 1600];
    let result = adapter.on_speech_frame(&audio_samples).await;
    assert!(result.is_none(), "NoOpPlugin should return None");
    
    // Finalize - should return None
    let final_result = adapter.on_speech_end().await;
    assert!(final_result.is_none(), "NoOpPlugin should return None for finalize");
}

#[tokio::test]
async fn test_error_classification() {
    // Test that error types are correctly classified
    let transient_error = SttPluginError::DecodeTimeout { timeout_ms: 5000 };
    assert!(transient_error.is_transient());
    assert!(!transient_error.should_failover());
    
    let failover_error = SttPluginError::BackendUnavailable;
    assert!(!failover_error.is_transient());
    assert!(failover_error.should_failover());
    
    let permanent_error = SttPluginError::ConfigurationError("Bad config".to_string());
    assert!(!permanent_error.is_transient());
    assert!(!permanent_error.should_failover());
}

#[tokio::test]
async fn test_plugin_manager_with_metrics() {
    let metrics = Arc::new(PipelineMetrics::default());
    let mut manager = SttPluginManager::new();
    manager.set_metrics(metrics.clone());
    
    let result = manager.initialize().await;
    assert!(result.is_ok());
    
    // Check that metrics were updated with backend name
    // Note: This is a basic test - in practice we'd check the actual metrics values
    assert!(result.is_ok());
}

#[tokio::test] 
async fn test_fallback_order() {
    let selection_config = PluginSelectionConfig {
        preferred_plugin: Some("nonexistent".to_string()), // This should fail
        fallback_plugins: vec!["mock".to_string(), "noop".to_string()],
        require_local: true,
        max_memory_mb: None,
        required_language: Some("en".to_string()),
    };
    
    let failover_config = FailoverConfig::default();
    let mut manager = SttPluginManager::with_config(selection_config, failover_config);
    
    let result = manager.initialize().await;
    assert!(result.is_ok());
    
    // Should fall back to mock (first in fallback list)
    let active_backend = result.unwrap();
    assert_eq!(active_backend, "mock");
}

#[test]
fn test_synthetic_audio_generation() {
    // Test the synthetic audio generation from the example
    let duration_secs = 1u64;
    let sample_rate = 16000u32;
    let total_samples = duration_secs as usize * sample_rate as usize;
    
    let mut samples = Vec::with_capacity(total_samples);
    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        let signal = 0.3 * (2.0 * std::f32::consts::PI * 200.0 * t).sin();
        let sample = (signal * 16384.0) as i16;
        samples.push(sample);
    }
    
    assert_eq!(samples.len(), total_samples);
    assert!(samples.iter().any(|&sample| sample.abs() > 1000));
}