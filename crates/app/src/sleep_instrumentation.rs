//! # Sleep Instrumentation Module
//!
//! This module provides utilities for observing and logging sleep calls in tests,
//! enabling waste ratio analysis and CI optimization insights.
//!
//! Enable with `TEST_SLEEP_OBSERVER=1` environment variable.

#[cfg(feature = "sleep-observer")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
#[cfg(feature = "sleep-observer")]
use tracing::debug;
use tracing::info;

/// Global flag to enable sleep observation (runtime toggle via env in tests)
static SLEEP_OBSERVER_ENABLED: AtomicBool = AtomicBool::new(false);
/// Monotonic ID counter for observed sleeps (avoids timestamp collisions)
#[cfg(feature = "sleep-observer")]
static SLEEP_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Sleep observation record
#[derive(Debug, Clone)]
pub struct SleepRecord {
    pub id: String,
    pub requested_ms: u64,
    pub actual_ms: u64,
    pub tag: String,
    pub timestamp: Instant,
}

/// Global sleep observer
pub struct SleepObserver {
    records: Mutex<Vec<SleepRecord>>,
}

impl SleepObserver {
    /// Get the global sleep observer instance
    pub fn global() -> &'static SleepObserver {
        static INSTANCE: std::sync::OnceLock<SleepObserver> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(|| SleepObserver {
            records: Mutex::new(Vec::new()),
        })
    }

    /// Record a sleep observation
    pub fn record(&self, record: SleepRecord) {
        if let Ok(mut records) = self.records.lock() {
            records.push(record);
        }
    }

    /// Get all recorded sleeps
    pub fn get_records(&self) -> Vec<SleepRecord> {
        self.records.lock().unwrap().clone()
    }

    /// Calculate waste ratio (p95 actual / requested)
    pub fn calculate_waste_ratio(&self) -> Option<f64> {
        // waste_ratio_p95 = p95(actual - requested) / mean(requested)
        let records = self.get_records();
        if records.is_empty() {
            return None;
        }
        let mut deltas: Vec<f64> = records
            .iter()
            .map(|r| (r.actual_ms as i64 - r.requested_ms as i64).max(0) as f64)
            .collect();
        deltas.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_idx = ((deltas.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
        let p95_overhead = *deltas.get(p95_idx).unwrap_or(&0.0);
        let mean_requested: f64 =
            records.iter().map(|r| r.requested_ms as f64).sum::<f64>() / records.len() as f64;
        if mean_requested == 0.0 {
            return None;
        }
        Some(p95_overhead / mean_requested)
    }

    /// Clear all records
    pub fn clear(&self) {
        if let Ok(mut records) = self.records.lock() {
            records.clear();
        }
    }
}

/// Initialize sleep observer from environment
pub fn init_sleep_observer() {
    if std::env::var("TEST_SLEEP_OBSERVER")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        SLEEP_OBSERVER_ENABLED.store(true, Ordering::SeqCst);
        info!("Sleep observer enabled - will log sleep calls for analysis");
    }
}

/// Check if sleep observer is enabled
pub fn is_sleep_observer_enabled() -> bool {
    SLEEP_OBSERVER_ENABLED.load(Ordering::SeqCst)
}

/// Instrumented sleep function that logs when observer is enabled
#[cfg(feature = "sleep-observer")]
pub async fn observed_sleep(duration: Duration, tag: &str) {
    let start = Instant::now();
    let requested_ms = duration.as_millis() as u64;

    tokio::time::sleep(duration).await;

    let actual_ms = start.elapsed().as_millis() as u64;

    if is_sleep_observer_enabled() {
        let id_raw = SLEEP_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let record = SleepRecord {
            id: format!("sleep_{}", id_raw),
            requested_ms,
            actual_ms,
            tag: tag.to_string(),
            timestamp: start,
        };

        SleepObserver::global().record(record.clone());

        debug!(
            "Sleep observed: id={}, req={}ms, act={}ms (+{}ms), tag={}",
            record.id,
            requested_ms,
            actual_ms,
            actual_ms.saturating_sub(requested_ms),
            tag
        );
    }
}

#[cfg(not(feature = "sleep-observer"))]
pub async fn observed_sleep(duration: Duration, _tag: &str) {
    tokio::time::sleep(duration).await;
}

/// Macro for easy sleep observation
#[macro_export]
macro_rules! observed_sleep {
    ($duration:expr, $tag:expr) => {
        $crate::sleep_instrumentation::observed_sleep($duration, $tag).await
    };
}

/// Readiness polling utility
pub async fn poll_until<F, Fut>(
    mut predicate: F,
    max_duration: Duration,
    poll_interval: Duration,
) -> Result<(), ()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = Instant::now();

    while start.elapsed() < max_duration {
        if predicate().await {
            return Ok(());
        }
        observed_sleep(poll_interval, "poll_until").await;
    }

    Err(())
}

/// Wait for AT-SPI bus to be ready
pub async fn wait_for_atspi(max_wait: Duration) -> Result<(), ()> {
    poll_until(
        || async {
            // Check if AT-SPI bus is available
            tokio::process::Command::new("dbus-send")
                .args([
                    "--session",
                    "--dest=org.a11y.atspi.Registry",
                    "--type=method_call",
                    "--print-reply",
                    "/org/a11y/atspi/accessible/root",
                    "org.freedesktop.DBus.Introspectable.Introspect",
                ])
                .output()
                .await
                .map(|output| output.status.success())
                .unwrap_or(false)
        },
        max_wait,
        Duration::from_millis(100),
    )
    .await
}

/// Wait for clipboard to be stable
pub async fn wait_for_clipboard_stable(max_wait: Duration) -> Result<(), ()> {
    let initial_content = get_clipboard_content().await;

    poll_until(
        || async {
            let current = get_clipboard_content().await;
            current == initial_content
        },
        max_wait,
        Duration::from_millis(50),
    )
    .await
}

/// Wait for terminal to be ready (file-based check)
pub async fn wait_for_terminal_ready(
    capture_file: &std::path::Path,
    max_wait: Duration,
) -> Result<(), ()> {
    poll_until(
        || async { capture_file.exists() },
        max_wait,
        Duration::from_millis(100),
    )
    .await
}

/// Helper to get clipboard content (simplified)
async fn get_clipboard_content() -> Option<String> {
    // Try wl-paste first (Wayland)
    if let Ok(output) = tokio::process::Command::new("wl-paste")
        .arg("--no-newline")
        .output()
        .await
    {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    // Try xclip (X11)
    if let Ok(output) = tokio::process::Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .arg("-o")
        .output()
        .await
    {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    None
}

/// Export recorded sleep data to JSON file (CI artifact). No-op if feature disabled or not enabled at runtime.
pub fn export_sleep_observer_json<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<()> {
    if !is_sleep_observer_enabled() {
        return Ok(());
    }
    #[cfg(feature = "sleep-observer")]
    {
        use std::fs::File;
        use std::io::Write;
        let records = SleepObserver::global().get_records();
        let waste = SleepObserver::global().calculate_waste_ratio();
        let json_records: Vec<serde_json::Value> = records
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "requested_ms": r.requested_ms,
                    "actual_ms": r.actual_ms,
                    "overhead_ms": r.actual_ms.saturating_sub(r.requested_ms),
                    "tag": r.tag,
                })
            })
            .collect();
        let doc = serde_json::json!({
            "total_sleeps": records.len(),
            "waste_ratio_p95": waste,
            "records": json_records,
        });
        let mut f = File::create(path)?;
        f.write_all(doc.to_string().as_bytes())?;
        f.flush()?;
    }
    let _ = path; // silence unused when feature off
    Ok(())
}
