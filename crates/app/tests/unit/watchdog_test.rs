#[cfg(test)]
mod tests {
    use coldvox_audio::watchdog::WatchdogTimer;
    use coldvox_foundation::clock::{self, Clock};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    /// Tests that regular feeding prevents watchdog timeout.
    ///
    /// This is core algorithm behavior: if we keep feeding the watchdog
    /// within the timeout window, it should never trigger.
    #[tokio::test]
    async fn test_watchdog_feed_prevents_timeout() {
        let test_clock = clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));
        let mut wd = WatchdogTimer::new_with_clock(Duration::from_millis(200), test_clock.clone());
        wd.start(running.clone());

        // Feed 5 times at 100ms intervals (all within 200ms timeout)
        for _ in 0..5 {
            test_clock.sleep(Duration::from_millis(100));
            wd.feed();
        }

        running.store(false, Ordering::SeqCst);
        wd.stop();
        assert!(!wd.is_triggered(), "Watchdog should not trigger when regularly fed");
    }

    /// Tests that watchdog triggers after timeout period without feeding.
    ///
    /// This is core algorithm behavior: if we don't feed the watchdog
    /// within the timeout window, it should trigger.
    #[tokio::test]
    async fn test_watchdog_timeout_triggers() {
        let test_clock = clock::test_clock();
        let running = Arc::new(AtomicBool::new(true));
        let mut wd = WatchdogTimer::new_with_clock(Duration::from_millis(150), test_clock.clone());
        wd.start(running.clone());

        // Wait longer than timeout without feeding
        test_clock.sleep(Duration::from_millis(200));
        assert!(wd.is_triggered(), "Watchdog should trigger after inactivity");

        running.store(false, Ordering::SeqCst);
        wd.stop();
    }
}
