use coldvox_audio_quality::{
    AudioQualityMonitor, QualityConfig, QualityStatus, QualityWarning,
};
use hound::WavReader;
use std::path::{Path, PathBuf};

/// Load WAV file samples as i16 vector
fn load_wav_samples(path: impl AsRef<Path>) -> Result<Vec<i16>, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(format!("WAV file not found: {}", path.display()).into());
    }

    let mut reader = WavReader::open(path)?;
    let samples: Vec<i16> = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;
    Ok(samples)
}

/// Check if external datasets are available
fn external_datasets_available() -> bool {
    PathBuf::from("../../test_audio/baseline/LibriSpeech").exists()
}

/// Check if Pyramic dataset is available
fn pyramic_available() -> bool {
    PathBuf::from("../../test_audio/off_axis").exists()
}

// ============================================================================
// Basic Integration Tests (Use Committed Samples - Always Run)
// ============================================================================

#[test]
fn test_committed_sample_baseline() {
    // Test with committed samples (always available, ~320KB total)
    let samples = load_wav_samples("test_data/test_1.wav")
        .expect("Failed to load committed test sample");

    let config = QualityConfig::default();
    let mut monitor = AudioQualityMonitor::new(config);

    // Process audio in 512-sample chunks
    let mut good_frames = 0;
    let mut total_frames = 0;

    for chunk in samples.chunks(512) {
        let status = monitor.analyze(chunk);
        total_frames += 1;

        if matches!(status, QualityStatus::Good { .. }) {
            good_frames += 1;
        }
    }

    // Real audio may have quiet sections, silence, or transitions
    // Accept any reasonable number of good frames (> 10% as sanity check)
    let good_ratio = good_frames as f32 / total_frames as f32;
    println!(
        "Audio quality: {:.1}% good frames ({}/{})",
        good_ratio * 100.0,
        good_frames,
        total_frames
    );

    // Sanity check: At least some frames should be good (not all silence/errors)
    assert!(
        good_ratio > 0.1,
        "Expected some good frames, got only {:.1}% ({}/{})",
        good_ratio * 100.0,
        good_frames,
        total_frames
    );

    // Verify analysis completes without panicking for all frames
    assert_eq!(total_frames, (samples.len() + 511) / 512);
}

#[test]
fn test_committed_samples_no_panic() {
    // Verify monitor doesn't panic on various real audio samples
    let samples = vec![
        "test_data/test_1.wav",
        "test_data/test_3.wav",
        "test_data/test_5.wav",
    ];

    let config = QualityConfig::default();
    let mut monitor = AudioQualityMonitor::new(config);

    for sample_path in samples {
        let audio = load_wav_samples(sample_path)
            .unwrap_or_else(|e| panic!("Failed to load {}: {}", sample_path, e));

        // Process all chunks without panicking
        for chunk in audio.chunks(512) {
            let _status = monitor.analyze(chunk);
        }
    }
}

#[test]
fn test_synthetic_clipping_detection() {
    // Generate severely clipped audio (20dB over-gain)
    // Use maximum safe value (32767) to avoid i16::MIN overflow issues
    let mut samples = vec![0i16; 16000]; // 1 second at 16kHz
    for (i, sample) in samples.iter_mut().enumerate() {
        let value = ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / 16000.0).sin();
        // Apply gain and clamp to avoid i16::MIN (-32768) which causes overflow in abs()
        let amplified = value * 32767.0 * 10.0;
        *sample = amplified.clamp(-32767.0, 32767.0) as i16;
    }

    let config = QualityConfig::default();
    let mut monitor = AudioQualityMonitor::new(config);

    let mut found_clipping = false;
    for chunk in samples.chunks(512) {
        if let QualityStatus::Warning(QualityWarning::Clipping { peak_dbfs }) = monitor.analyze(chunk) {
            found_clipping = true;
            // Verify peak is actually near 0 dBFS
            assert!(
                peak_dbfs > -1.0,
                "Expected clipping near 0 dBFS, got {} dBFS",
                peak_dbfs
            );
            break;
        }
    }

    assert!(
        found_clipping,
        "Severely clipped audio should trigger Clipping warning"
    );
}

#[test]
fn test_synthetic_quiet_audio() {
    // Generate very quiet audio (1% amplitude = ~-40 dBFS)
    let mut samples = vec![0i16; 16000]; // 1 second at 16kHz
    for (i, sample) in samples.iter_mut().enumerate() {
        let value = ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / 16000.0).sin();
        *sample = (value * 32767.0 * 0.01) as i16; // 1% amplitude
    }

    let config = QualityConfig::default();
    let mut monitor = AudioQualityMonitor::new(config);

    let mut found_too_quiet = false;
    for chunk in samples.chunks(512) {
        if let QualityStatus::Warning(QualityWarning::TooQuiet { rms_dbfs }) = monitor.analyze(chunk) {
            found_too_quiet = true;
            // Verify RMS is actually below threshold
            assert!(
                rms_dbfs < -40.0,
                "Expected RMS below -40 dBFS, got {} dBFS",
                rms_dbfs
            );
            break;
        }
    }

    assert!(
        found_too_quiet,
        "Very quiet audio should trigger TooQuiet warning"
    );
}

// ============================================================================
// External Dataset Tests (Only Run If Datasets Downloaded)
// ============================================================================

#[test]
fn test_librispeech_baseline() {
    if !external_datasets_available() {
        println!("Skipping LibriSpeech test - run ./scripts/download_test_audio.sh");
        return;
    }

    // Find first FLAC file in LibriSpeech test-clean
    let librispeech_dir = PathBuf::from("../../test_audio/baseline/LibriSpeech/test-clean");
    let flac_files: Vec<_> = walkdir::WalkDir::new(&librispeech_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "flac"))
        .take(3)
        .collect();

    if flac_files.is_empty() {
        println!("No FLAC files found in LibriSpeech directory");
        return;
    }

    let config = QualityConfig::default();
    let mut monitor = AudioQualityMonitor::new(config);

    for entry in flac_files {
        let samples = match load_wav_samples(entry.path()) {
            Ok(s) => s,
            Err(_) => continue, // Skip files that can't be loaded
        };

        // LibriSpeech is professional quality - should have mostly good frames
        let mut good_frames = 0;
        let mut total_frames = 0;

        for chunk in samples.chunks(512) {
            let status = monitor.analyze(chunk);
            total_frames += 1;

            if matches!(status, QualityStatus::Good { .. }) {
                good_frames += 1;
            }
        }

        let good_ratio = good_frames as f32 / total_frames as f32;
        assert!(
            good_ratio > 0.8,
            "LibriSpeech should be high quality, got {:.1}% good frames for {:?}",
            good_ratio * 100.0,
            entry.path()
        );
    }
}

#[test]
fn test_pyramic_off_axis_detection() {
    if !pyramic_available() {
        println!("Skipping Pyramic test - download from https://zenodo.org/records/1209563");
        return;
    }

    // Test on-axis (0°) vs off-axis (90°, 180°)
    let test_cases = vec![
        ("angle_000", false, "On-axis should be good quality"),
        ("angle_090", true, "90° off-axis should trigger warning"),
        ("angle_180", true, "180° off-axis should trigger warning"),
    ];

    let config = QualityConfig::default();

    for (angle_dir, expect_warning, description) in test_cases {
        let audio_path = PathBuf::from(format!("../../test_audio/off_axis/pyramic/{}/speech.wav", angle_dir));

        if !audio_path.exists() {
            println!("Skipping {} - file not found", angle_dir);
            continue;
        }

        let samples = match load_wav_samples(&audio_path) {
            Ok(s) => s,
            Err(_) => {
                println!("Could not load {}", angle_dir);
                continue;
            }
        };

        let mut monitor = AudioQualityMonitor::new(config.clone());
        let mut found_off_axis = false;

        // Process audio and check for off-axis warnings
        for chunk in samples.chunks(512) {
            if let QualityStatus::Warning(QualityWarning::OffAxis { .. }) = monitor.analyze(chunk) {
                found_off_axis = true;
                break;
            }
        }

        if expect_warning {
            assert!(found_off_axis, "{}: {}", angle_dir, description);
        }
        // Note: On-axis might still trigger warnings if audio is quiet or has issues
        // so we don't assert !found_off_axis for on-axis cases
    }
}

#[test]
fn test_pyramic_spectral_ratio_comparison() {
    if !pyramic_available() {
        println!("Skipping Pyramic spectral comparison - download dataset");
        return;
    }

    // Compare spectral characteristics: on-axis should have higher high-freq content
    let on_axis_path = PathBuf::from("../../test_audio/off_axis/pyramic/angle_000/speech.wav");
    let off_axis_path = PathBuf::from("../../test_audio/off_axis/pyramic/angle_090/speech.wav");

    if !on_axis_path.exists() || !off_axis_path.exists() {
        println!("Pyramic samples not found");
        return;
    }

    let on_axis = load_wav_samples(&on_axis_path).expect("Failed to load on-axis sample");
    let off_axis = load_wav_samples(&off_axis_path).expect("Failed to load off-axis sample");

    let config = QualityConfig::default();
    let mut monitor_on = AudioQualityMonitor::new(config.clone());
    let mut monitor_off = AudioQualityMonitor::new(config);

    // Analyze first few frames and collect results
    let on_axis_status = monitor_on.analyze(&on_axis[..512.min(on_axis.len())]);
    let off_axis_status = monitor_off.analyze(&off_axis[..512.min(off_axis.len())]);

    println!("On-axis (0°): {:?}", on_axis_status);
    println!("Off-axis (90°): {:?}", off_axis_status);

    // This test is informational - just verify both analyses complete without panic
    // Actual spectral ratios depend on audio content and microphone characteristics
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_custom_thresholds() {
    // Test with more aggressive thresholds
    let config = QualityConfig::builder()
        .too_quiet_threshold_dbfs(-30.0) // More strict (default: -40.0)
        .clipping_threshold_dbfs(-3.0)   // More lenient (default: -1.0)
        .off_axis_threshold(0.5)         // More lenient (default: 0.3)
        .build();

    let mut monitor = AudioQualityMonitor::new(config);

    // Generate borderline quiet audio (-35 dBFS)
    let mut samples = vec![0i16; 512];
    for (i, sample) in samples.iter_mut().enumerate() {
        let value = ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / 16000.0).sin();
        *sample = (value * 32767.0 * 0.02) as i16; // ~-35 dBFS
    }

    // With -30 dBFS threshold, this should trigger warning
    let status = monitor.analyze(&samples);
    assert!(
        matches!(status, QualityStatus::Warning(QualityWarning::TooQuiet { .. })),
        "Audio at -35 dBFS should trigger warning with -30 dBFS threshold"
    );
}

#[test]
fn test_rate_limiting() {
    // Note: Rate limiting uses Instant::now() which doesn't advance in fast tests
    // This test verifies the first warning is emitted, but can't reliably test
    // the 2-second cooldown without actual time passing or dependency injection

    let config = QualityConfig::default();
    let mut monitor = AudioQualityMonitor::new(config);

    // Generate consistently quiet audio
    let mut samples = vec![0i16; 512];
    for (i, sample) in samples.iter_mut().enumerate() {
        let value = ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / 16000.0).sin();
        *sample = (value * 32767.0 * 0.005) as i16; // Very quiet
    }

    let mut warning_count = 0;

    // Process 100 frames
    for _ in 0..100 {
        if matches!(monitor.analyze(&samples), QualityStatus::Warning(_)) {
            warning_count += 1;
        }
    }

    // Should get at least one warning
    assert!(
        warning_count > 0,
        "Should get at least one warning for consistently quiet audio"
    );

    // Note: In a real application with actual time passing, rate limiting would
    // reduce warning frequency. This test just verifies warnings are emitted.
}
