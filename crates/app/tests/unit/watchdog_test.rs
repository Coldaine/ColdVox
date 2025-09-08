#[cfg(test)]
mod tests {
    use coldvox_audio::watchdog::WatchdogTimer;
    use coldvox_foundation::clock::{self, SharedClock, Clock};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn test_watchdog_creation() {
        let _wd1 = WatchdogTimer::new(Duration::from_secs(1));
        let _wd2 = WatchdogTimer::new(Duration::from_millis(250));
        let _wd3 = WatchdogTimer::new(Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_watchdog_feed_prevents_timeout() {
        let test_clock = clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));
        let mut wd = WatchdogTimer::new_with_clock(Duration::from_millis(200), test_clock.clone());
        wd.start(running.clone());

        for _ in 0..5 { // 5 * 100ms < total timeout window with feeds
            test_clock.sleep(Duration::from_millis(100));
            wd.feed();
        }

        running.store(false, Ordering::SeqCst);
        wd.stop();
        assert!(!wd.is_triggered(), "Watchdog should not trigger when regularly fed");
    }

    #[tokio::test]
    async fn test_watchdog_timeout_triggers() {
        let test_clock = clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));
        let mut wd = WatchdogTimer::new_with_clock(Duration::from_millis(150), test_clock.clone());
        wd.start(running.clone());

        test_clock.sleep(Duration::from_millis(200)); // exceed timeout
        assert!(wd.is_triggered(), "Watchdog should trigger after inactivity");

        running.store(false, Ordering::SeqCst);
        wd.stop();
    }

    #[tokio::test]
    async fn test_watchdog_stop_resets_trigger() {
        let test_clock = clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));
        let mut wd = WatchdogTimer::new_with_clock(Duration::from_millis(80), test_clock.clone());
        wd.start(running.clone());
        test_clock.sleep(Duration::from_millis(120));
        assert!(wd.is_triggered(), "Should be triggered before stop");
        running.store(false, Ordering::SeqCst);
        wd.stop();
        assert!(!wd.is_triggered(), "Stop should clear trigger state");
    }

    #[tokio::test]
    async fn test_restart_does_not_carry_trigger_state() {
        let test_clock = clock::test_clock();
        let running1 = Arc::new(AtomicBool::new(true));
        let mut wd = WatchdogTimer::new_with_clock(Duration::from_millis(100), test_clock.clone());
        wd.start(running1.clone());
        test_clock.sleep(Duration::from_millis(120));
        assert!(wd.is_triggered());
        running1.store(false, Ordering::SeqCst);
        wd.stop();

        // Restart
        let running2 = Arc::new(AtomicBool::new(true));
        wd.start(running2.clone());
        wd.feed(); // immediate feed after restart
        test_clock.sleep(Duration::from_millis(50));
        assert!(!wd.is_triggered(), "Trigger state should not persist across restart");
        running2.store(false, Ordering::SeqCst);
        wd.stop();
    }

    #[tokio::test]
    async fn test_concurrent_feed_operations() {
        // Use real clock to avoid virtual time contention in threads
        let running = Arc::new(AtomicBool::new(true));
        let mut wd = WatchdogTimer::new(Duration::from_millis(500));
        wd.start(running.clone());
        let shared = Arc::new(wd.clone());

        let threads: Vec<_> = (0..4).map(|_| {
            let w = shared.clone();
            thread::spawn(move || {
                for _ in 0..10 {
                    w.feed();
                    std::thread::sleep(Duration::from_millis(20));
                }
            })
        }).collect();

        for t in threads { t.join().unwrap(); }
        running.store(false, Ordering::SeqCst);
        // We cloned earlier before start; stopping original clone now
        // Create a mutable instance to stop (original mut wd consumed by clone, so rebuild)
        // Simpler: nothing further; triggered should be false.
        assert!(!shared.is_triggered(), "Concurrent feeding should prevent trigger");
    }
}
