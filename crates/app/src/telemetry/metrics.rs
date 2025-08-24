use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BasicMetrics {
    counters: Arc<RwLock<HashMap<String, AtomicU64>>>,
    gauges: Arc<RwLock<HashMap<String, AtomicU64>>>,
}

impl BasicMetrics {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn increment_counter(&self, name: &str, value: u64) {
        let counters = self.counters.read();
        if let Some(counter) = counters.get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        } else {
            drop(counters);
            let mut counters = self.counters.write();
            counters
                .entry(name.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(value, Ordering::Relaxed);
        }
    }

    pub fn set_gauge(&self, name: &str, value: u64) {
        let gauges = self.gauges.read();
        if let Some(gauge) = gauges.get(name) {
            gauge.store(value, Ordering::Relaxed);
        } else {
            drop(gauges);
            let mut gauges = self.gauges.write();
            gauges
                .entry(name.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .store(value, Ordering::Relaxed);
        }
    }

    pub fn get_counter(&self, name: &str) -> Option<u64> {
        self.counters
            .read()
            .get(name)
            .map(|c| c.load(Ordering::Relaxed))
    }

    pub fn get_gauge(&self, name: &str) -> Option<u64> {
        self.gauges
            .read()
            .get(name)
            .map(|g| g.load(Ordering::Relaxed))
    }

    pub fn get_all_metrics(&self) -> HashMap<String, u64> {
        let mut metrics = HashMap::new();

        for (name, counter) in self.counters.read().iter() {
            metrics.insert(format!("counter.{}", name), counter.load(Ordering::Relaxed));
        }

        for (name, gauge) in self.gauges.read().iter() {
            metrics.insert(format!("gauge.{}", name), gauge.load(Ordering::Relaxed));
        }

        metrics
    }
}

impl Default for BasicMetrics {
    fn default() -> Self {
        Self::new()
    }
}
