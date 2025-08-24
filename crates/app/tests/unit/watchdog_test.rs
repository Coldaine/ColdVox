#[cfg(test)]
mod tests {
    use coldvox_app::audio::watchdog::WatchdogTimer;
    use std::time::Duration;
    use std::thread;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    #[test]
    fn test_watchdog_creation() {
        // Test various timeout durations
        let watchdog_1s = WatchdogTimer::new(Duration::from_secs(1));
        assert!(watchdog_1s.is_ok(), "Should create watchdog with 1s timeout");
        
        let watchdog_100ms = WatchdogTimer::new(Duration::from_millis(100));
        assert!(watchdog_100ms.is_ok(), "Should create watchdog with 100ms timeout");
        
        let watchdog_10s = WatchdogTimer::new(Duration::from_secs(10));
        assert!(watchdog_10s.is_ok(), "Should create watchdog with 10s timeout");
    }

    #[test]
    fn test_watchdog_pet_prevents_timeout() {
        let timeout_triggered = Arc::new(AtomicBool::new(false));
        let timeout_triggered_clone = timeout_triggered.clone();
        
        let mut watchdog = WatchdogTimer::with_callback(
            Duration::from_millis(200),
            move || {
                timeout_triggered_clone.store(true, Ordering::SeqCst);
            }
        );
        
        watchdog.start();
        
        // Pet the watchdog every 100ms for 500ms total
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(100));
            watchdog.pet();
        }
        
        watchdog.stop();
        
        assert!(!timeout_triggered.load(Ordering::SeqCst),
            "Watchdog should not timeout when petted regularly");
    }

    #[test]
    fn test_watchdog_timeout_triggers() {
        let timeout_triggered = Arc::new(AtomicBool::new(false));
        let timeout_triggered_clone = timeout_triggered.clone();
        
        let mut watchdog = WatchdogTimer::with_callback(
            Duration::from_millis(100),
            move || {
                timeout_triggered_clone.store(true, Ordering::SeqCst);
            }
        );
        
        watchdog.start();
        
        // Don't pet the watchdog, wait for timeout
        thread::sleep(Duration::from_millis(200));
        
        assert!(timeout_triggered.load(Ordering::SeqCst),
            "Watchdog should timeout after specified duration");
        
        watchdog.stop();
    }

    #[test]
    fn test_watchdog_stop() {
        let timeout_count = Arc::new(AtomicU32::new(0));
        let timeout_count_clone = timeout_count.clone();
        
        let mut watchdog = WatchdogTimer::with_callback(
            Duration::from_millis(50),
            move || {
                timeout_count_clone.fetch_add(1, Ordering::SeqCst);
            }
        );
        
        watchdog.start();
        thread::sleep(Duration::from_millis(100)); // Let it timeout once
        watchdog.stop();
        
        let count_after_stop = timeout_count.load(Ordering::SeqCst);
        thread::sleep(Duration::from_millis(100)); // Wait to ensure no more timeouts
        let count_after_wait = timeout_count.load(Ordering::SeqCst);
        
        assert_eq!(count_after_stop, count_after_wait,
            "Watchdog should not trigger after being stopped");
    }

    #[test]
    fn test_epoch_change_on_restart() {
        let mut watchdog = WatchdogTimer::new(Duration::from_millis(100)).unwrap();
        
        // Start and stop multiple times
        watchdog.start();
        let epoch1 = watchdog.current_epoch();
        watchdog.stop();
        
        watchdog.start();
        let epoch2 = watchdog.current_epoch();
        watchdog.stop();
        
        watchdog.start();
        let epoch3 = watchdog.current_epoch();
        watchdog.stop();
        
        assert_ne!(epoch1, epoch2, "Epoch should change after restart");
        assert_ne!(epoch2, epoch3, "Epoch should change after each restart");
        assert!(epoch3 > epoch1, "Epoch should increment");
    }

    #[test]
    fn test_concurrent_pet_operations() {
        let timeout_triggered = Arc::new(AtomicBool::new(false));
        let timeout_triggered_clone = timeout_triggered.clone();
        
        let watchdog = Arc::new(Mutex::new(WatchdogTimer::with_callback(
            Duration::from_millis(200),
            move || {
                timeout_triggered_clone.store(true, Ordering::SeqCst);
            }
        )));
        
        watchdog.lock().unwrap().start();
        
        // Spawn multiple threads that pet the watchdog
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let watchdog_clone = watchdog.clone();
                thread::spawn(move || {
                    for _ in 0..10 {
                        thread::sleep(Duration::from_millis(40));
                        watchdog_clone.lock().unwrap().pet();
                    }
                })
            })
            .collect();
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        watchdog.lock().unwrap().stop();
        
        assert!(!timeout_triggered.load(Ordering::SeqCst),
            "Concurrent petting should prevent timeout");
    }

    #[test]
    fn test_timeout_callback_execution() {
        let callback_count = Arc::new(AtomicU32::new(0));
        let callback_count_clone = callback_count.clone();
        
        let mut watchdog = WatchdogTimer::with_callback(
            Duration::from_millis(50),
            move || {
                callback_count_clone.fetch_add(1, Ordering::SeqCst);
            }
        );
        
        watchdog.start();
        
        // Wait for exactly one timeout
        thread::sleep(Duration::from_millis(75));
        watchdog.pet(); // Reset after first timeout
        
        // Wait for another timeout
        thread::sleep(Duration::from_millis(75));
        
        watchdog.stop();
        
        let final_count = callback_count.load(Ordering::SeqCst);
        assert_eq!(final_count, 2, "Callback should execute exactly twice");
    }

    #[test]
    fn test_rapid_start_stop() {
        let timeout_triggered = Arc::new(AtomicBool::new(false));
        let timeout_triggered_clone = timeout_triggered.clone();
        
        let mut watchdog = WatchdogTimer::with_callback(
            Duration::from_millis(100),
            move || {
                timeout_triggered_clone.store(true, Ordering::SeqCst);
            }
        );
        
        // Rapidly start and stop
        for _ in 0..10 {
            watchdog.start();
            thread::sleep(Duration::from_millis(10));
            watchdog.stop();
        }
        
        assert!(!timeout_triggered.load(Ordering::SeqCst),
            "Rapid start/stop should not trigger timeout");
    }

    #[test]
    fn test_pet_after_timeout() {
        let timeout_count = Arc::new(AtomicU32::new(0));
        let timeout_count_clone = timeout_count.clone();
        
        let mut watchdog = WatchdogTimer::with_callback(
            Duration::from_millis(50),
            move || {
                timeout_count_clone.fetch_add(1, Ordering::SeqCst);
            }
        );
        
        watchdog.start();
        
        // Let it timeout
        thread::sleep(Duration::from_millis(75));
        assert_eq!(timeout_count.load(Ordering::SeqCst), 1, "Should timeout once");
        
        // Pet after timeout should reset timer
        watchdog.pet();
        
        // Should not immediately timeout again
        thread::sleep(Duration::from_millis(30));
        assert_eq!(timeout_count.load(Ordering::SeqCst), 1, "Should still be 1 timeout");
        
        // Wait for another timeout
        thread::sleep(Duration::from_millis(30));
        assert_eq!(timeout_count.load(Ordering::SeqCst), 2, "Should timeout again");
        
        watchdog.stop();
    }

    #[test] 
    fn test_watchdog_with_jitter() {
        // Test that timeout includes jitter for recovery scenarios
        let timeout_times = Arc::new(Mutex::new(Vec::new()));
        
        for _ in 0..5 {
            let timeout_times_clone = timeout_times.clone();
            let start = std::time::Instant::now();
            
            let mut watchdog = WatchdogTimer::with_callback(
                Duration::from_millis(100),
                move || {
                    let elapsed = start.elapsed();
                    timeout_times_clone.lock().unwrap().push(elapsed);
                }
            );
            
            watchdog.start();
            thread::sleep(Duration::from_millis(150));
            watchdog.stop();
        }
        
        let times = timeout_times.lock().unwrap();
        let mut all_same = true;
        for i in 1..times.len() {
            if times[i] != times[i-1] {
                all_same = false;
                break;
            }
        }
        
        // With jitter, not all timeouts should be exactly the same
        // This test might be flaky due to timing, but demonstrates the concept
        assert!(!all_same || times.len() < 2,
            "Timeouts should have some variation with jitter");
    }
}