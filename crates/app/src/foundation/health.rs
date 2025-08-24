use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub name: String,
    pub healthy: bool,
    pub last_check: Instant,
    pub last_error: Option<String>,
    pub check_count: u64,
    pub failure_count: u64,
}

pub trait HealthCheck: Send + Sync {
    fn check(&self) -> Result<(), String>;
    fn name(&self) -> &str;
}

pub struct HealthMonitor {
    components: Arc<RwLock<HashMap<String, ComponentHealth>>>,
    checks: Arc<RwLock<Vec<Box<dyn HealthCheck>>>>,
    check_interval: Duration,
    handle: Option<JoinHandle<()>>,
}

impl HealthMonitor {
    pub fn new(check_interval: Duration) -> Self {
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
            checks: Arc::new(RwLock::new(Vec::new())),
            check_interval,
            handle: None,
        }
    }
    
    pub fn register(&self, component: Box<dyn HealthCheck>) {
        let name = component.name().to_string();
        let mut components = self.components.write();
        components.insert(name.clone(), ComponentHealth {
            name,
            healthy: true,
            last_check: Instant::now(),
            last_error: None,
            check_count: 0,
            failure_count: 0,
        });
        self.checks.write().push(component);
    }
    
    pub fn start(mut self) -> Self {
        let components = Arc::clone(&self.components);
        let checks = Arc::clone(&self.checks);
        let interval = self.check_interval;
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                // Run registered health checks and update status
                let now = Instant::now();
                let mut map = components.write();
                for hc in checks.read().iter() {
                    let name = hc.name().to_string();
                    let entry = map.entry(name.clone()).or_insert(ComponentHealth {
                        name: name.clone(),
                        healthy: true,
                        last_check: now,
                        last_error: None,
                        check_count: 0,
                        failure_count: 0,
                    });

                    entry.check_count += 1;
                    entry.last_check = now;
                    match hc.check() {
                        Ok(_) => {
                            if !entry.healthy {
                                tracing::info!(component = %name, "Component recovered");
                            }
                            entry.healthy = true;
                            entry.last_error = None;
                        }
                        Err(err) => {
                            entry.healthy = false;
                            entry.failure_count += 1;
                            entry.last_error = Some(err.clone());
                            tracing::warn!(component = %name, failure_count = entry.failure_count, "Health check failed: {}", err);
                        }
                    }
                }
            }
        });
        
        self.handle = Some(handle);
        self
    }
    
    pub fn get_status(&self) -> HashMap<String, ComponentHealth> {
        self.components.read().clone()
    }
    
    pub fn all_healthy(&self) -> bool {
        self.components.read().values().all(|c| c.healthy)
    }
}