use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct WatchdogTimer {
    timeout: Duration,
    last_feed: Arc<RwLock<Option<Instant>>>,
    triggered: Arc<AtomicBool>,
    handle: Option<Arc<JoinHandle<()>>>,
}

impl WatchdogTimer {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            last_feed: Arc::new(RwLock::new(None)),
            triggered: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    pub fn start(&mut self, running: Arc<AtomicBool>) {
        let timeout = self.timeout;
        let last_feed = Arc::clone(&self.last_feed);
        let triggered = Arc::clone(&self.triggered);

        // Initialize the last feed time
        *last_feed.write() = Some(Instant::now());

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            while running.load(Ordering::SeqCst) {
                interval.tick().await;

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
                        guard.map(|last_time| now.duration_since(last_time))
                            .unwrap_or(Duration::ZERO)
                    };
                    tracing::error!("Watchdog timeout! No audio data for {:?}", elapsed);
                    triggered.store(true, Ordering::SeqCst);
                }
            }
        });

        self.handle = Some(Arc::new(handle));
    }

    pub fn feed(&self) {
        *self.last_feed.write() = Some(Instant::now());
        self.triggered.store(false, Ordering::SeqCst);
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }

    pub fn stop(&mut self) {
        // Allow external loop condition (running flag) to stop naturally; also abort task if present
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        self.triggered.store(false, Ordering::SeqCst);
        *self.last_feed.write() = None;
    }
}
