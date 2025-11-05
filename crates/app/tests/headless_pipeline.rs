//! Headless, end-to-end integration test for the ColdVox pipeline.
//!
//! This test validates the full application pipeline, from audio input to text
//! injection, in a deterministic, CI-friendly way. It is enabled by the
//! `headless-test` feature flag.
//!
//! When this feature is enabled:
//! 1. The application's `runtime` is compiled to exclude all GUI components.
//! 2. The real text injection backends (like AT-SPI) are replaced with a
//!    `MockInjectionSink` that captures injected text for assertions.
//! 3. The STT model is forced to a small, CPU-friendly version for consistency.

use coldvox_app::runtime::{ActivationMode, AppRuntimeOptions, InjectionOptions};
use coldvox_app::audio::wav_file_loader::WavFileLoader;
use coldvox_audio::DeviceConfig;
use coldvox_stt::plugin::{FailoverConfig, GcPolicy, PluginSelectionConfig};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Creates the runtime options specifically for the headless end-to-end test.
fn headless_test_opts(sender: mpsc::Sender<String>) -> AppRuntimeOptions {
    AppRuntimeOptions {
        device: None,
        resampler_quality: coldvox_audio::ResamplerQuality::Balanced,
        activation_mode: ActivationMode::Vad,
        stt_selection: Some(PluginSelectionConfig {
            preferred_plugin: Some("whisper".to_string()),
            fallback_plugins: vec![],
            require_local: true,
            max_memory_mb: None,
            required_language: Some("en".to_string()),
            failover: Some(FailoverConfig {
                failover_threshold: 3,
                failover_cooldown_secs: 1,
            }),
            gc_policy: Some(GcPolicy {
                model_ttl_secs: 30,
                enabled: false, // Disable GC for test
            }),
            metrics: None,
            auto_extract_model: true,
        }),
        #[cfg(feature = "text-injection")]
        injection: Some(InjectionOptions {
            enable: true,
            inject_on_unknown_focus: true,
            ..Default::default()
        }),
        enable_device_monitor: false,
        capture_buffer_samples: 65_536,
        #[cfg(feature = "headless-test")]
        test_device_config: None,
        #[cfg(feature = "headless-test")]
        test_capture_to_dummy: true,
        #[cfg(feature = "headless-test")]
        mock_injection_sender: Some(sender),
    }
}

/// Normalizes a transcript for WER calculation by converting it to lowercase,
/// removing punctuation, and trimming whitespace.
fn normalize_transcript(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

#[cfg(feature = "headless-test")]
#[tokio::test]
async fn test_headless_pipeline_end_to_end() {
    // 1. Initialize test infrastructure
    coldvox_app::test_utils::init_test_infrastructure();
    // Use a small model for CI-friendliness. This can be overridden by environment variables.
    std::env::set_var("WHISPER_MODEL_SIZE", "tiny.en");

    // 2. Set up the mock injection sink
    let (injection_tx, mut injection_rx) = mpsc::channel::<String>(100);

    // 3. Configure and start the application runtime in headless mode
    let wav_path = "test_data/test_2.wav";
    let mut wav_loader = WavFileLoader::new(wav_path).expect("Failed to load test WAV file");
    let mut opts = headless_test_opts(injection_tx);
    opts.test_device_config = Some(DeviceConfig {
        sample_rate: wav_loader.sample_rate(),
        channels: wav_loader.channels(),
    });

    let app =
        coldvox_app::runtime::start(opts).await.expect("Failed to start headless app runtime");

    // 4. Stream the WAV file into the pipeline
    let audio_producer = app.audio_producer.clone();
    let stream_handle = tokio::spawn(async move {
        wav_loader
            .stream_to_ring_buffer_locked(audio_producer)
            .await
            .unwrap();
    });

    // 5. Collect the output from the injection sink
    let collector_handle = tokio::spawn(async move {
        let mut all_injected_text = String::new();
        while let Some(text) = injection_rx.recv().await {
            all_injected_text.push_str(&text);
        }
        all_injected_text
    });

    // 6. Wait for processing to complete and shut down
    stream_handle.await.expect("WAV streaming failed");
    // Add a small delay to ensure the pipeline has time to process the last audio chunks
    tokio::time::sleep(Duration::from_secs(2)).await;
    Arc::new(app).shutdown().await;

    let final_text = collector_handle.await.expect("Text collection failed");

    // 7. Perform WER assertion
    let expected_transcript = "None of";

    let normalized_expected = normalize_transcript(&expected_transcript);
    let normalized_actual = normalize_transcript(&final_text);

    let wer = coldvox_app::test_utils::wer::calculate_wer(&normalized_expected, &normalized_actual);
    let wer_threshold = 0.35; // Generous threshold for the tiny.en model

    assert!(
        wer <= wer_threshold,
        "Word Error Rate exceeded threshold: {:.2}% > {:.2}%. Expected: '{}', Got: '{}'",
        wer * 100.0,
        wer_threshold * 100.0,
        normalized_expected,
        normalized_actual
    );

    println!(
        "Headless pipeline test passed with WER: {:.2}%",
        wer * 100.0
    );
}
