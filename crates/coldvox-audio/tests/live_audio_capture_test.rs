//! Live Audio Capture Test - Captures ACTUAL audio from microphone
//!
//! This test verifies the complete audio pipeline works end-to-end:
//! Device → Capture → Ring Buffer → Audio Frames
//!
//! Run with: cargo test -p coldvox-audio --features live-hardware-tests test_live_audio_capture -- --nocapture

#[cfg(all(test, windows, feature = "live-hardware-tests"))]
mod live_audio_tests {
    use coldvox_audio::capture::AudioCaptureThread;
    use coldvox_audio::ring_buffer::AudioRingBuffer;
    use coldvox_foundation::AudioConfig;
    use parking_lot::Mutex;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// Live test: Capture actual audio from microphone and verify samples flow
    #[test]
    fn test_live_audio_capture() {
        println!("\n========================================");
        println!("🎤 LIVE AUDIO CAPTURE TEST");
        println!("========================================");
        println!("This test will capture ACTUAL audio from your microphone.");
        println!("Please speak or make noise into the microphone...\n");

        // Configure audio capture
        let config = AudioConfig {
            silence_threshold: 100,
            capture_buffer_samples: 65536,
        };

        // Create ring buffer for audio data
        let ring_buffer = AudioRingBuffer::new(65536);
        let (producer, mut consumer) = ring_buffer.split();
        let producer = Arc::new(Mutex::new(producer));

        println!("📡 Starting audio capture thread...");

        // Spawn audio capture thread (no device monitor for simplicity)
        let capture_result = AudioCaptureThread::spawn(
            config, producer, None,  // Use default device
            false, // Disable device monitor
        );

        let (capture_thread, device_config, _config_rx, _device_event_rx) = match capture_result {
            Ok(result) => {
                println!("✅ Audio capture thread started successfully!");
                println!("   Sample rate: {} Hz", result.1.sample_rate);
                println!("   Channels: {}", result.1.channels);
                result
            }
            Err(e) => {
                panic!("❌ Failed to start audio capture: {}", e);
            }
        };

        // Let user know to make noise
        println!("\n🔴 Recording for 3 seconds... MAKE SOME NOISE! 🔴\n");

        // Capture audio for 3 seconds
        let capture_duration = Duration::from_secs(3);
        let start_time = Instant::now();
        let mut total_samples = 0u64;
        let mut max_amplitude: i16 = 0;
        let mut min_amplitude: i16 = 0;
        let mut frame_count = 0u64;
        let mut non_silent_frames = 0u64;
        const SILENCE_THRESHOLD: i16 = 100; // Samples below this are considered silence

        // Read audio data from ring buffer
        while start_time.elapsed() < capture_duration {
            // Try to read available samples
            let mut buffer = vec![0i16; 1024];
            let read_count = consumer.read(&mut buffer);
            if read_count > 0 {
                frame_count += 1;
                total_samples += read_count as u64;

                // Analyze samples
                let mut frame_has_sound = false;
                for &sample in &buffer[..read_count] {
                    max_amplitude = max_amplitude.max(sample);
                    min_amplitude = min_amplitude.min(sample);
                    if sample.abs() > SILENCE_THRESHOLD {
                        frame_has_sound = true;
                    }
                }
                if frame_has_sound {
                    non_silent_frames += 1;
                }

                if frame_count % 100 == 0 {
                    print!(
                        "\r📊 Captured: {} frames, {} samples",
                        frame_count, total_samples
                    );
                }
            } else {
                // Buffer empty, sleep briefly
                std::thread::sleep(Duration::from_millis(1));
            }
        }

        println!("\n\n⏹️  Stopping capture...");

        // Stop the capture thread
        capture_thread.stop();

        // Final analysis
        println!("\n========================================");
        println!("📈 CAPTURE RESULTS");
        println!("========================================");
        println!("Device Configuration:");
        println!("  Sample Rate: {} Hz", device_config.sample_rate);
        println!("  Channels: {}", device_config.channels);
        println!("\nCapture Statistics:");
        println!("  Total frames: {}", frame_count);
        println!("  Total samples: {}", total_samples);
        println!("  Duration: ~3 seconds");
        println!("  Effective sample rate: {} Hz", total_samples / 3);
        println!("\nAudio Analysis:");
        println!("  Max amplitude: {}", max_amplitude);
        println!("  Min amplitude: {}", min_amplitude);
        println!("  Dynamic range: {}", max_amplitude - min_amplitude);
        println!("  Non-silent frames: {}/{}", non_silent_frames, frame_count);

        // Assertions - verify we actually captured audio
        println!("\n🔍 Verification:");

        // Check we got some frames
        assert!(
            frame_count > 0,
            "❌ No audio frames captured! Check microphone."
        );
        println!("  ✅ Captured {} frames", frame_count);

        // Check we got reasonable sample count
        let expected_samples_min = device_config.sample_rate as u64 * 2; // At least 2 seconds
        assert!(
            total_samples >= expected_samples_min,
            "❌ Too few samples captured: {} (expected at least {})",
            total_samples,
            expected_samples_min
        );
        println!(
            "  ✅ Captured {} samples (≥{} expected)",
            total_samples, expected_samples_min
        );

        // Check we have some signal variation (not all zeros)
        let dynamic_range = max_amplitude - min_amplitude;
        assert!(
            dynamic_range > 0,
            "❌ Audio signal is completely flat (range={}). Microphone may not be working.",
            dynamic_range
        );
        println!("  ✅ Audio signal has variation (range={})", dynamic_range);

        // Check max amplitude is reasonable (not clipping at max)
        assert!(
            max_amplitude < 32700,
            "⚠️  Audio may be clipping (max={}). Consider lowering microphone volume.",
            max_amplitude
        );
        println!("  ✅ No clipping detected (max={} < 32700)", max_amplitude);

        // Check for non-silent frames (some actual sound was captured)
        // Note: This is informational only - low amplitude could mean quiet room or low mic sensitivity
        if non_silent_frames > 0 {
            println!("  ✅ Detected {} non-silent frames", non_silent_frames);
        } else {
            println!(
                "  ℹ️  All frames appear to be silent (room may be quiet or mic sensitivity low)"
            );
        }

        println!("\n✅ LIVE AUDIO CAPTURE TEST PASSED!");
        println!("========================================\n");
    }
}

#[cfg(all(test, not(windows)))]
mod non_windows_tests {
    #[test]
    fn test_skipped_on_non_windows() {
        println!("Live audio tests are Windows-only. Skipping.");
    }
}

#[cfg(all(test, windows, not(feature = "live-hardware-tests")))]
mod no_feature_tests {
    #[test]
    fn test_needs_feature_flag() {
        println!("Live audio tests require --features live-hardware-tests");
    }
}
