use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct WatchdogTimer {
    timeout: Duration,
    last_feed: Arc<AtomicU64>,
    triggered: Arc<AtomicBool>,
    handle: Option<Arc<JoinHandle<()>>>,
}

impl WatchdogTimer {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            last_feed: Arc::new(AtomicU64::new(0)),
            triggered: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }
    
    pub fn start(&mut self, running: Arc<AtomicBool>) {
        let timeout = self.timeout;
        let last_feed = Arc::clone(&self.last_feed);
        let triggered = Arc::clone(&self.triggered);
        
        // Set initial feed time using Instant
        let now = Instant::now();
        self.last_feed.store(now.elapsed().as_millis() as u64, Ordering::Relaxed);
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let start_time = Instant::now();
            
            while running.load(Ordering::SeqCst) {
                interval.tick().await;
                
                // Calculate elapsed time since last feed
                let last_ms = last_feed.load(Ordering::Relaxed);
                let now_ms = start_time.elapsed().as_millis() as u64;
                
                // Check if we have a valid last feed time and calculate elapsed
                if last_ms > 0 && now_ms >= last_ms {
                    let elapsed = Duration::from_millis(now_ms - last_ms);
                    
                    if elapsed > timeout && !triggered.load(Ordering::SeqCst) {
                        tracing::error!("Watchdog timeout! No audio data for {:?}", elapsed);
                        triggered.store(true, Ordering::SeqCst);
                        // Trigger recovery mechanism
                    }
                }
            }
        });
        
        self.handle = Some(Arc::new(handle));
    }
    
    pub fn feed(&self) {
        let now = Instant::now().elapsed().as_millis() as u64;
        self.last_feed.store(now, Ordering::Relaxed);
        self.triggered.store(false, Ordering::SeqCst);
    }
    
    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }
}