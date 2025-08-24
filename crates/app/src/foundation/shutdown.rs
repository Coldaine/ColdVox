use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;
use tokio::sync::Notify;

pub struct ShutdownHandler {
    shutdown_requested: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
}

impl ShutdownHandler {
    pub fn new() -> Self {
        Self {
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }
    
    pub async fn install(self) -> ShutdownGuard {
        let shutdown_requested = Arc::clone(&self.shutdown_requested);
        let shutdown_notify = Arc::clone(&self.shutdown_notify);
        
        tokio::spawn(async move {
            // Wait for Ctrl-C
            signal::ctrl_c().await.expect("Failed to install Ctrl-C handler");
            
            tracing::info!("Shutdown requested via Ctrl-C");
            shutdown_requested.store(true, Ordering::SeqCst);
            shutdown_notify.notify_waiters();
        });
        
        // Install panic handler
        let original_panic = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            tracing::error!("PANIC: {}", panic_info);
            eprintln!("Application panicked: {}", panic_info);
            original_panic(panic_info);
        }));
        
        ShutdownGuard {
            shutdown_requested: self.shutdown_requested,
            shutdown_notify: self.shutdown_notify,
        }
    }
}

pub struct ShutdownGuard {
    shutdown_requested: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
}

impl ShutdownGuard {
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }
    
    pub async fn wait(&self) {
        self.shutdown_notify.notified().await;
    }
    
    pub fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
    }
}