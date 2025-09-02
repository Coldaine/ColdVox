use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct WatchdogTimer {
    timeout: Duration,
    last_feed: Arc<RwLock<Option<Instant>>>,
    triggered: Arc<AtomicBool>,
    handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl WatchdogTimer {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            last_feed: Arc::new(RwLock::new(None)),
            triggered: Arc::new(AtomicBool::new(false)),
            handle: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(&mut self, running: Arc<AtomicBool>) {
        let timeout = self.timeout;
        let last_feed = Arc::clone(&self.last_feed);
        let triggered = Arc::clone(&self.triggered);

        // Initialize the last feed time
        *last_feed.write() = Some(Instant::now());

        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(1));

                let now = Instant::now();
                let should_trigger = {
                    let guard = last_feed.read();
                    if let Some(last_time) = *guard {
                        let elapsed = now.duration_since(last_time);
                        elapsed > timeout && !triggered.load(Ordering::SeqCst)
                    } else {
                        false
                    }
                };

                if should_trigger {
                    let elapsed = {
                        let guard = last_feed.read();
                        guard
                            .map(|last_time| now.duration_since(last_time))
                            .unwrap_or(Duration::ZERO)
                    };
                    tracing::error!("Watchdog timeout! No audio data for {:?}", elapsed);
                    triggered.store(true, Ordering::SeqCst);
                }
            }
        });

        *self.handle.write() = Some(handle);
    }

    pub fn feed(&self) {
        *self.last_feed.write() = Some(Instant::now());
        self.triggered.store(false, Ordering::SeqCst);
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }

    pub fn stop(&mut self) {
        // Allow external loop condition (running flag) to stop naturally; join thread if present
        if let Some(handle) = self.handle.write().take() {
            let _ = handle.join();
        }
        self.triggered.store(false, Ordering::SeqCst);
        *self.last_feed.write() = None;
    }
}
