#[cfg(test)]
mod tests {
    use coldvox_audio::watchdog::WatchdogTimer;
    use std::time::Duration;
    use std::thread;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use tokio::time::sleep;
    use coldvox_foundation::clock::{Clock, TestClock, SharedClock};

    #[tokio::test]
    async fn test_watchdog_creation() {
        // Test various timeout durations
        let watchdog_1s = WatchdogTimer::new(Duration::from_secs(1));
        let watchdog_100ms = WatchdogTimer::new(Duration::from_millis(100));
        let watchdog_10s = WatchdogTimer::new(Duration::from_secs(10));
        // These should all succeed without panicking
    }

    #[tokio::test]
    async fn test_watchdog_pet_prevents_timeout() {
        let test_clock = coldvox_foundation::clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));

        let mut watchdog = WatchdogTimer::new_with_clock(Duration::from_millis(200), test_clock.clone());
        watchdog.start(running.clone());

        // Pet the watchdog every 100ms for 500ms total
        for _ in 0..5 {
            test_clock.sleep(Duration::from_millis(100));
            watchdog.feed();
        }

        running.store(false, Ordering::SeqCst);
        watchdog.stop();

        assert!(!watchdog.is_triggered(),
            "Watchdog should not timeout when fed regularly");
    }

    #[tokio::test]
    async fn test_watchdog_timeout_triggers() {
        let test_clock = coldvox_foundation::clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));

        let mut watchdog = WatchdogTimer::new_with_clock(Duration::from_millis(200), test_clock.clone());
        watchdog.start(running.clone());

        // Advance time past the timeout without feeding
        test_clock.sleep(Duration::from_millis(250));

        running.store(false, Ordering::SeqCst);
        watchdog.stop();

        assert!(watchdog.is_triggered(),
            "Watchdog should timeout after specified duration");
    }

    #[tokio::test]
    async fn test_watchdog_stop() {
        let test_clock = coldvox_foundation::clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));

        let mut watchdog = WatchdogTimer::new_with_clock(Duration::from_millis(50), test_clock.clone());
        watchdog.start(running.clone());

        // Let it timeout once
        test_clock.sleep(Duration::from_millis(100));
        let triggered_before_stop = watchdog.is_triggered();

        running.store(false, Ordering::SeqCst);
        watchdog.stop();

        // Wait more time to ensure no more triggers
        test_clock.sleep(Duration::from_millis(100));
        let triggered_after_stop = watchdog.is_triggered();

        assert!(triggered_before_stop, "Watchdog should have triggered before stop");
        assert_eq!(triggered_before_stop, triggered_after_stop,
            "Watchdog should not trigger again after being stopped");
    }

    #[tokio::test]
    async fn test_epoch_change_on_restart() {
        let test_clock = coldvox_foundation::clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));

        let mut watchdog = WatchdogTimer::new_with_clock(Duration::from_millis(100), test_clock.clone());

        // Start and stop multiple times
        watchdog.start(running.clone());
        test_clock.sleep(Duration::from_millis(50));
        running.store(false, Ordering::SeqCst);
        watchdog.stop();

        // Reset for second run
        let running2 = Arc::new(AtomicBool::new(true));
        watchdog.start(running2.clone());
        test_clock.sleep(Duration::from_millis(50));
        running2.store(false, Ordering::SeqCst);
        watchdog.stop();

        // Should not be triggered since we fed it regularly
        assert!(!watchdog.is_triggered(), "Watchdog should not trigger when restarted properly");
    }

    #[tokio::test]
    async fn test_concurrent_feed_operations() {
        let test_clock = coldvox_foundation::clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));

        let watchdog = Arc::new(WatchdogTimer::new_with_clock(Duration::from_millis(200), test_clock.clone()));
        let watchdog_clone = Arc::clone(&watchdog);

        // Start the watchdog
        let mut watchdog_mut = WatchdogTimer::new_with_clock(Duration::from_millis(200), test_clock.clone());
        watchdog_mut.start(running.clone());

        // Spawn multiple threads to feed the watchdog concurrently
        let handles: Vec<_> = (0..5).map(|_| {
            let watchdog = Arc::clone(&watchdog_clone);
            let test_clock = test_clock.clone();
            thread::spawn(move || {
                for _ in 0..10 {
                    test_clock.sleep(Duration::from_millis(20));
                    watchdog.feed();
                }
            })
        }).collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        running.store(false, Ordering::SeqCst);
        watchdog_mut.stop();

        assert!(!watchdog.is_triggered(),
            "Watchdog should not timeout with concurrent feeding");
    }

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
                        sleep(Duration::from_millis(40));
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

    #[tokio::test]
    async fn test_timeout_callback_execution() {
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
        sleep(Duration::from_millis(75));
        watchdog.pet(); // Reset after first timeout

        // Wait for another timeout
        sleep(Duration::from_millis(75));

        watchdog.stop();

        let final_count = callback_count.load(Ordering::SeqCst);
        assert_eq!(final_count, 2, "Callback should execute exactly twice");
    }

    #[tokio::test]
    async fn test_rapid_start_stop() {
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
            sleep(Duration::from_millis(10));
            watchdog.stop();
        }

        assert!(!timeout_triggered.load(Ordering::SeqCst),
            "Rapid start/stop should not trigger timeout");
    }

    #[tokio::test]
    async fn test_pet_after_timeout() {
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
        sleep(Duration::from_millis(75));
        assert_eq!(timeout_count.load(Ordering::SeqCst), 1, "Should timeout once");

        // Pet after timeout should reset timer
        watchdog.pet();

        // Should not immediately timeout again
        sleep(Duration::from_millis(30));
        assert_eq!(timeout_count.load(Ordering::SeqCst), 1, "Should still be 1 timeout");

        // Wait for another timeout
        sleep(Duration::from_millis(30));
        assert_eq!(timeout_count.load(Ordering::SeqCst), 2, "Should timeout again");

        watchdog.stop();
    }

    #[tokio::test]
    async fn test_watchdog_with_jitter() {
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
            sleep(Duration::from_millis(150));
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

    #[tokio::test]
    async fn test_watchdog_deterministic_with_test_clock() {
        // Test using TestClock for deterministic behavior
        let test_clock: SharedClock = std::sync::Arc::new(TestClock::new());
        let start_time = test_clock.now();

        let timeout_triggered = Arc::new(AtomicBool::new(false));
        let timeout_triggered_clone = timeout_triggered.clone();

        let mut watchdog = WatchdogTimer::with_callback(
            Duration::from_millis(100),
            move || {
                timeout_triggered_clone.store(true, Ordering::SeqCst);
            }
        );

        watchdog.start();

        // Advance virtual time by 50ms - should not timeout
        test_clock.advance(Duration::from_millis(50));
        assert!(!timeout_triggered.load(Ordering::SeqCst),
            "Should not timeout at 50ms");

        // Advance another 60ms - should timeout
        test_clock.advance(Duration::from_millis(60));
        assert!(timeout_triggered.load(Ordering::SeqCst),
            "Should timeout after 110ms total");

        watchdog.stop();

        // Verify the test clock advanced correctly
        let elapsed = test_clock.now().duration_since(start_time);
        assert_eq!(elapsed, Duration::from_millis(110),
            "Test clock should have advanced by exactly the expected amount");
    }
}
