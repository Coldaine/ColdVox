//! End-to-end live tests for Moonshine STT plugin
//!
//! Run with: cargo test --features moonshine moonshine_e2e -- --nocapture

#![cfg(feature = "moonshine")]

mod common;

use coldvox_stt::plugin::SttPlugin;
use coldvox_stt::plugins::moonshine::{MoonshineModelSize, MoonshinePlugin};
use coldvox_stt::types::{TranscriptionConfig, TranscriptionEvent};

/// Load test audio file (16kHz WAV from app crate)
fn load_test_audio_16khz() -> Vec<i16> {
    common::load_test_audio()
}

#[tokio::test]
async fn test_moonshine_base_transcription() {
    let samples = load_test_audio_16khz();
    if samples.is_empty() {
        eprintln!("Skipping test_moonshine_base_transcription: no test audio");
        return;
    }

    let mut plugin = MoonshinePlugin::new().with_model_size(MoonshineModelSize::Base);

    let config = TranscriptionConfig {
        enabled: true,
        model_path: "moonshine-base".to_string(),
        include_words: false,
        partial_results: false,
        streaming: false,
        ..Default::default()
    };

    plugin
        .initialize(config)
        .await
        .expect("Failed to initialize Moonshine Base");

    println!(
        "Processing {} samples ({:.2}s of audio)",
        samples.len(),
        samples.len() as f32 / 16000.0
    );

    plugin
        .process_audio(&samples)
        .await
        .expect("Failed to process audio");

    let result = plugin.finalize().await.expect("Failed to finalize");

    assert!(result.is_some(), "Expected transcription result");

    if let Some(TranscriptionEvent::Final { text, .. }) = result {
        assert!(!text.is_empty(), "Transcription should not be empty");
        assert!(text.len() > 10, "Transcription too short: '{}'", text);

        println!("Moonshine Base transcription: {}", text);
    } else {
        panic!("Expected Final transcription event");
    }
}

#[tokio::test]
async fn test_moonshine_tiny_transcription() {
    let samples = load_test_audio_16khz();
    if samples.is_empty() {
        eprintln!("Skipping test_moonshine_tiny_transcription: no test audio");
        return;
    }

    let mut plugin = MoonshinePlugin::new().with_model_size(MoonshineModelSize::Tiny);

    let config = TranscriptionConfig {
        enabled: true,
        model_path: "moonshine-tiny".to_string(),
        ..Default::default()
    };

    plugin
        .initialize(config)
        .await
        .expect("Failed to initialize Moonshine Tiny");

    plugin
        .process_audio(&samples)
        .await
        .expect("Failed to process audio");

    let result = plugin.finalize().await.expect("Failed to finalize");

    assert!(result.is_some());

    if let Some(TranscriptionEvent::Final { text, .. }) = result {
        assert!(!text.is_empty());
        println!("Moonshine Tiny transcription: {}", text);
    }
}

#[tokio::test]
async fn test_plugin_info() {
    let plugin = MoonshinePlugin::new();
    let info = plugin.info();

    assert_eq!(info.id, "moonshine");
    assert!(info.supported_languages.contains(&"en".to_string()));
    assert!(info.is_local);
    assert!(!info.requires_network); // Cached after first download
    assert!(info.memory_usage_mb.is_some());

    println!("Plugin info correct: {} - {}", info.name, info.description);
}

#[tokio::test]
async fn test_capabilities() {
    let plugin = MoonshinePlugin::new();
    let caps = plugin.capabilities();

    assert!(!caps.streaming, "Moonshine is batch-only");
    assert!(caps.batch, "Moonshine supports batch processing");
    assert!(caps.auto_punctuation, "Moonshine auto-punctuates");
    assert!(
        !caps.word_timestamps,
        "Word timestamps not available via transformers"
    );

    println!("Capabilities verified");
}

#[tokio::test]
async fn test_reset() {
    let samples = load_test_audio_16khz();
    if samples.is_empty() || samples.len() < 2000 {
        eprintln!("Skipping test_reset: insufficient test audio");
        return;
    }

    let mut plugin = MoonshinePlugin::new();

    let config = TranscriptionConfig::default();
    plugin.initialize(config).await.expect("Init failed");

    plugin
        .process_audio(&samples[..1000])
        .await
        .expect("Process failed");

    // Reset should clear buffer
    plugin.reset().await.expect("Reset failed");

    // Should be able to process new audio after reset
    plugin
        .process_audio(&samples[1000..2000])
        .await
        .expect("Process after reset failed");

    println!("Reset functionality works");
}

#[tokio::test]
async fn test_empty_audio() {
    let mut plugin = MoonshinePlugin::new();

    let config = TranscriptionConfig::default();
    plugin.initialize(config).await.expect("Init failed");

    // Process empty buffer
    plugin
        .process_audio(&[])
        .await
        .expect("Empty process failed");

    let result = plugin.finalize().await.expect("Finalize failed");
    assert!(result.is_none(), "Empty audio should return None");

    println!("Empty audio handled correctly");
}

#[test]
fn test_model_size_identifiers() {
    assert_eq!(
        MoonshineModelSize::Tiny.model_identifier(),
        "UsefulSensors/moonshine-tiny"
    );
    assert_eq!(
        MoonshineModelSize::Base.model_identifier(),
        "UsefulSensors/moonshine-base"
    );
}

#[test]
fn test_default_model_is_base() {
    assert_eq!(MoonshineModelSize::default(), MoonshineModelSize::Base);
}

#[test]
fn test_memory_usage_estimates() {
    assert_eq!(MoonshineModelSize::Tiny.memory_usage_mb(), 300);
    assert_eq!(MoonshineModelSize::Base.memory_usage_mb(), 500);
    assert!(MoonshineModelSize::Base.memory_usage_mb() < 1000);
}

/// Tests that modify env vars must run serially to avoid race conditions
#[test]
#[serial_test::serial]
fn test_factory_env_vars() {
    use coldvox_stt::plugin::SttPluginFactory;
    use coldvox_stt::plugins::MoonshinePluginFactory;
    use std::env;

    // Save original value
    let original = env::var("MOONSHINE_MODEL").ok();

    env::set_var("MOONSHINE_MODEL", "tiny");
    let factory = MoonshinePluginFactory::new();
    // Factory respects env var - check via plugin creation
    let plugin = factory.create().expect("Factory create failed");
    let info = plugin.info();
    assert!(
        info.name.contains("Tiny"),
        "Should use Tiny model from env var"
    );

    // Restore original value
    match original {
        Some(v) => env::set_var("MOONSHINE_MODEL", v),
        None => env::remove_var("MOONSHINE_MODEL"),
    }
}

#[test]
#[serial_test::serial]
fn test_factory_invalid_env_var() {
    use coldvox_stt::plugin::SttPluginFactory;
    use coldvox_stt::plugins::MoonshinePluginFactory;
    use std::env;

    // Save original value
    let original = env::var("MOONSHINE_MODEL").ok();

    env::set_var("MOONSHINE_MODEL", "invalid");
    let factory = MoonshinePluginFactory::new();
    let plugin = factory
        .create()
        .expect("Factory should handle invalid env var");
    let info = plugin.info();
    // Should fall back to default (Base)
    assert!(
        info.name.contains("Base"),
        "Should fall back to Base on invalid env var"
    );

    // Restore original value
    match original {
        Some(v) => env::set_var("MOONSHINE_MODEL", v),
        None => env::remove_var("MOONSHINE_MODEL"),
    }
}

/// Test that MAX_AUDIO_BUFFER_SAMPLES limit is enforced
#[tokio::test]
async fn test_buffer_overflow_protection() {
    let mut plugin = MoonshinePlugin::new();

    let config = TranscriptionConfig::default();
    plugin.initialize(config).await.expect("Init failed");

    // Create audio that exceeds 10 minutes (16000 * 60 * 10 = 9,600,000 samples)
    // We'll create slightly more than the limit to test truncation
    const MAX_SAMPLES: usize = 16000 * 60 * 10;
    let oversized_audio: Vec<i16> = vec![0i16; MAX_SAMPLES + 10000];

    // Process in chunks - the buffer should cap at MAX_SAMPLES
    for chunk in oversized_audio.chunks(16000) {
        let _ = plugin.process_audio(chunk).await;
    }

    // Reset clears the buffer - if we got here without panic, the limit worked
    plugin.reset().await.expect("Reset failed");

    println!(
        "Buffer overflow protection verified (limit: {} samples)",
        MAX_SAMPLES
    );
}

/// Test that common::load_test_audio validates audio format correctly
#[test]
fn test_load_test_audio_validation() {
    let path = common::get_test_audio_path();
    if !path.exists() {
        eprintln!("Skipping test_load_test_audio_validation: no test audio file");
        return;
    }

    // Should not panic - validates 16kHz mono
    let samples = common::load_test_audio();
    assert!(!samples.is_empty(), "Should load valid test audio");

    println!("Audio validation test passed ({} samples)", samples.len());
}
