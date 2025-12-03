//! Common test utilities for STT plugin testing

use anyhow::{anyhow, Result};
use std::path::PathBuf;

/// Get path to test audio file (16kHz mono WAV)
pub fn get_test_audio_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("app/test_audio_16k.wav")
}

/// Load test audio samples as i16 PCM, ensuring format is 16kHz mono.
pub fn load_test_audio() -> Result<Vec<i16>> {
    let path = get_test_audio_path();
    if !path.exists() {
        return Err(anyhow!("Test audio file not found: {}", path.display()));
    }

    let mut reader =
        hound::WavReader::open(&path).map_err(|e| anyhow!("Failed to open test audio: {}", e))?;

    let spec = reader.spec();
    if spec.sample_rate != 16000 {
        return Err(anyhow!(
            "Test audio must be 16kHz, but was {}Hz",
            spec.sample_rate
        ));
    }
    if spec.channels != 1 {
        return Err(anyhow!(
            "Test audio must be mono, but had {} channels",
            spec.channels
        ));
    }

    reader
        .samples::<i16>()
        .map(|s| s.map_err(|e| anyhow!("Failed to read sample: {}", e)))
        .collect()
}

/// Calculate audio duration in seconds
pub fn audio_duration_secs(samples: &[i16], sample_rate: u32) -> f32 {
    samples.len() as f32 / sample_rate as f32
}

/// Assert transcription is reasonable
pub fn assert_valid_transcription(text: &str) {
    assert!(!text.is_empty(), "Transcription is empty");
    assert!(text.len() > 5, "Transcription too short: '{}'", text);
    assert!(
        !text.contains("PLACEHOLDER"),
        "Transcription contains placeholder"
    );
}
