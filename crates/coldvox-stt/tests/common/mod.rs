//! Common test utilities for STT plugin testing

use std::path::PathBuf;

/// Get path to test audio file (16kHz mono WAV)
pub fn get_test_audio_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("app/test_audio_16k.wav")
}

/// Load test audio samples as i16 PCM
/// Returns empty Vec if file doesn't exist (allows tests to skip gracefully)
/// Validates sample rate (16kHz) and channels (mono) for test correctness
pub fn load_test_audio() -> Vec<i16> {
    let path = get_test_audio_path();
    if !path.exists() {
        eprintln!(
            "Test audio file not found: {}. Skipping test.",
            path.display()
        );
        return Vec::new();
    }

    let mut reader = hound::WavReader::open(&path).expect("Failed to open test audio");

    let spec = reader.spec();
    assert_eq!(spec.sample_rate, 16000, "Test audio must be 16kHz");
    assert_eq!(spec.channels, 1, "Test audio must be mono");

    reader
        .samples::<i16>()
        .map(|s| s.expect("Failed to read sample"))
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
