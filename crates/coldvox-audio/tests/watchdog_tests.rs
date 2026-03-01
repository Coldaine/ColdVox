//! Watchdog timer tests
//!
//! Tests the audio watchdog timer that detects when audio data stops flowing.
//! Uses TestClock for deterministic testing without real time delays.

use coldvox_audio::WatchdogTimer;
use coldvox_foundation::clock::TestClock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn watchdog_not_triggered_initially() {
    let wd = WatchdogTimer::new(Duration::from_secs(5));
    assert!(!wd.is_triggered());
}

#[test]
fn watchdog_not_triggered_when_fed_regularly() {
    let clock = Arc::new(TestClock::new());
    let mut wd = WatchdogTimer::new_with_clock(Duration::from_secs(5), clock.clone());

    let running = Arc::new(AtomicBool::new(true));
    wd.start(running.clone());

    // Feed regularly
    for _ in 0..5 {
        wd.feed();
        clock.advance(Duration::from_secs(1));
        std::thread::sleep(Duration::from_millis(50)); // let watchdog thread run
    }

    assert!(!wd.is_triggered());
    running.store(false, Ordering::SeqCst);
}

#[test]
fn watchdog_triggers_when_starved() {
    let clock = Arc::new(TestClock::new());
    let mut wd = WatchdogTimer::new_with_clock(Duration::from_secs(2), clock.clone());

    let running = Arc::new(AtomicBool::new(true));
    wd.start(running.clone());

    // Advance time beyond timeout without feeding
    clock.advance(Duration::from_secs(5));
    // Give the watchdog thread time to notice
    std::thread::sleep(Duration::from_millis(200));

    assert!(wd.is_triggered(), "watchdog should trigger after timeout without feed");
    running.store(false, Ordering::SeqCst);
}

#[test]
fn watchdog_feed_resets_trigger() {
    let wd = WatchdogTimer::new(Duration::from_secs(5));
    wd.feed();
    assert!(!wd.is_triggered());
}
