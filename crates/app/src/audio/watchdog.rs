use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct WatchdogTimer {
    timeout: Duration,
    start_epoch: Arc<RwLock<Option<Instant>>>,
    last_feed: Arc<AtomicU64>,
    triggered: Arc<AtomicBool>,
    handle: Option<Arc<JoinHandle<()>>>,
}

impl WatchdogTimer {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            start_epoch: Arc::new(RwLock::new(None)),
            last_feed: Arc::new(AtomicU64::new(0)),
            triggered: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }
    
    pub fn start(&mut self, running: Arc<AtomicBool>) {
        let timeout = self.timeout;
        let last_feed = Arc::clone(&self.last_feed);
        let triggered = Arc::clone(&self.triggered);
        let start_epoch = Arc::clone(&self.start_epoch);

        // Establish a common epoch and initialize feed time
        let epoch = Instant::now();
        *start_epoch.write() = Some(epoch);
        last_feed.store(0, Ordering::Relaxed);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            while running.load(Ordering::SeqCst) {
                interval.tick().await;

                let now_ms = {
                    let guard = start_epoch.read();
                    if let Some(epoch) = *guard { epoch.elapsed().as_millis() as u64 } else { 0 }
                };

                let last_ms = last_feed.load(Ordering::Relaxed);
                if last_ms > 0 && now_ms >= last_ms {
                    let elapsed = Duration::from_millis(now_ms - last_ms);
                    if elapsed > timeout && !triggered.load(Ordering::SeqCst) {
                        tracing::error!("Watchdog timeout! No audio data for {:?}", elapsed);
                        triggered.store(true, Ordering::SeqCst);
                    }
                }
            }
        });

        self.handle = Some(Arc::new(handle));
    }
    
    pub fn feed(&self) {
        // Use the same epoch as the watchdog loop
        let now_ms = {
            let guard = self.start_epoch.read();
            if let Some(epoch) = *guard { epoch.elapsed().as_millis() as u64 } else { 0 }
        };
        if now_ms > 0 {
            self.last_feed.store(now_ms, Ordering::Relaxed);
        }
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
        self.last_feed.store(0, Ordering::Relaxed);
        *self.start_epoch.write() = None;
    }
}