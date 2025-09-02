#[cfg(test)]
mod tests {
    use coldvox_audio::{AudioCapture, AudioConfig, AudioFrame};
    use coldvox_foundation::error::AudioError;
    use coldvox_vad::constants::FRAME_SIZE_SAMPLES;
    use std::time::Duration;
    use std::thread;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    #[test]
    #[cfg(feature = "live-hardware-tests")]
    fn test_end_to_end_capture_pipewire() {
        let config = AudioConfig {
            sample_rate: 16000,
            channels: 1,
            buffer_size: FRAME_SIZE_SAMPLES,
            silence_threshold: 100,
        };
        
        let mut capture = AudioCapture::new(config).expect("Failed to create capture");
        
        // Start capture with pipewire preference
        let result = tokio_test::block_on(capture.start(None));
        assert!(result.is_ok(), "Should start capture with default device");
        
        // Capture for 2 seconds
        thread::sleep(Duration::from_secs(2));
        
        // Check stats
        let stats = capture.get_stats();
        assert!(stats.frames_captured > 0, "Should have captured frames");
        assert_eq!(stats.disconnections, 0, "Should have no disconnections");
        
        capture.stop();
    }

    #[test]
    #[cfg(feature = "live-hardware-tests")]
    fn test_stats_reporting() {
        let config = AudioConfig::default();
        let mut capture = AudioCapture::new(config).expect("Failed to create capture");
        
        tokio_test::block_on(capture.start(None)).expect("Failed to start");
        
        let initial_stats = capture.get_stats();
        thread::sleep(Duration::from_secs(1));
        let after_stats = capture.get_stats();
        
        assert!(after_stats.frames_captured > initial_stats.frames_captured,
            "Frame count should increase");
        
        assert!(after_stats.active_frames > 0 || after_stats.silent_frames > 0,
            "Should classify frames as active or silent");
        
        capture.stop();
    }

    #[test]
    #[cfg(feature = "live-hardware-tests")]
    fn test_frame_flow() {
        let config = AudioConfig::default();
        let mut capture = AudioCapture::new(config).expect("Failed to create capture");
        
        tokio_test::block_on(capture.start(None)).expect("Failed to start");
        
        let mut frames_received = 0;
        let start = std::time::Instant::now();
        
        while start.elapsed() < Duration::from_secs(1) {
            if let Ok(frame) = capture.try_recv_timeout(Duration::from_millis(100)) {
                frames_received += 1;
                assert_eq!(frame.sample_rate, 16000, "Frame should have correct sample rate");
                assert_eq!(frame.channels, 1, "Frame should be mono");
                assert!(!frame.samples.is_empty(), "Frame should contain samples");
            }
        }
        
        assert!(frames_received > 0, "Should receive frames from capture");
        capture.stop();
    }

    #[test]
    #[cfg(feature = "live-hardware-tests")]
    fn test_clean_shutdown() {
        let config = AudioConfig::default();
        let capture = Arc::new(std::sync::Mutex::new(
            AudioCapture::new(config).expect("Failed to create capture")
        ));
        
        let capture_clone = capture.clone();
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_flag_clone = shutdown_flag.clone();
        
        // Set up Ctrl+C handler
        ctrlc::set_handler(move || {
            shutdown_flag_clone.store(true, Ordering::SeqCst);
        }).expect("Failed to set Ctrl+C handler");
        
        // Start capture
        tokio_test::block_on(
            capture.lock().unwrap().start(None)
        ).expect("Failed to start");
        
        // Simulate Ctrl+C after 1 second
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(1));
            shutdown_flag.store(true, Ordering::SeqCst);
        });
        
        // Wait for shutdown signal
        while !shutdown_flag.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(10));
        }
        
        // Clean shutdown
        capture_clone.lock().unwrap().stop();
        
        // Verify clean state
        let final_stats = capture_clone.lock().unwrap().get_stats();
        assert_eq!(final_stats.disconnections, 0, "Should have clean shutdown");
    }

    #[test]
    fn test_concurrent_operations() {
        use crossbeam_channel::bounded;
        
        let (tx, rx) = bounded::<AudioFrame>(100);
        let frame_counter = Arc::new(AtomicU64::new(0));
        
        // Simulate producer (audio capture)
        let tx_clone = tx.clone();
        let producer = thread::spawn(move || {
            for i in 0..1000 {
                let frame = AudioFrame {
                    samples: vec![i as i16; FRAME_SIZE_SAMPLES],
                    timestamp: std::time::Instant::now(),
                    sample_rate: 16000,
                    channels: 1,
                };
                
                if tx_clone.try_send(frame).is_err() {
                    // Buffer full, drop frame
                    break;
                }
                
                thread::sleep(Duration::from_micros(100));
            }
        });
        
        // Multiple consumers
        let consumers: Vec<_> = (0..3)
            .map(|id| {
                let rx_clone = rx.clone();
                let counter = frame_counter.clone();
                thread::spawn(move || {
                    while let Ok(frame) = rx_clone.recv_timeout(Duration::from_secs(1)) {
                        // Process frame
                        assert_eq!(frame.samples.len(), FRAME_SIZE_SAMPLES);
                        counter.fetch_add(1, Ordering::Relaxed);
                        thread::sleep(Duration::from_micros(50 * (id + 1) as u64));
                    }
                })
            })
            .collect();
        
        producer.join().unwrap();
        drop(tx); // Close channel
        
        for consumer in consumers {
            consumer.join().unwrap();
        }
        
        let total_processed = frame_counter.load(Ordering::Relaxed);
        assert!(total_processed > 0, "Should process frames concurrently");
    }

    #[test]
    fn test_buffer_pressure() {
        use crossbeam_channel::bounded;
        
        let (tx, rx) = bounded::<AudioFrame>(10); // Small buffer
        let mut dropped = 0;
        let mut sent = 0;
        
        // Try to send more frames than buffer can hold
        for i in 0..100 {
            let frame = AudioFrame {
                samples: vec![i as i16; FRAME_SIZE_SAMPLES],
                timestamp: std::time::Instant::now(),
                sample_rate: 16000,
                channels: 1,
            };
            
            match tx.try_send(frame) {
                Ok(_) => sent += 1,
                Err(_) => dropped += 1,
            }
        }
        
        assert!(sent <= 10, "Should not send more than buffer size");
        assert!(dropped > 0, "Should drop frames when buffer is full");
        
        // Drain buffer
        let mut received = 0;
        while rx.try_recv().is_ok() {
            received += 1;
        }
        
        assert_eq!(received, sent, "Should receive all sent frames");
    }

    #[test]
    #[cfg(feature = "live-hardware-tests")]
    fn test_device_specific_capture() {
        let config = AudioConfig::default();
        let mut capture = AudioCapture::new(config).expect("Failed to create capture");
        
        // Try to open a specific device (may not exist on all systems)
        let result = tokio_test::block_on(
            capture.start(Some("sysdefault:CARD=QuadCast"))
        );
        
        if result.is_ok() {
            thread::sleep(Duration::from_secs(1));
            let stats = capture.get_stats();
            assert!(stats.frames_captured > 0, "Should capture from specific device");
            capture.stop();
        } else {
            // Device not found is acceptable in test environment
            match result {
                Err(AudioError::DeviceNotFound { .. }) => {
                    println!("Specific device not found, test skipped");
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
                _ => unreachable!(),
            }
        }
    }
}
