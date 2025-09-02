use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveTestResult {
    pub test: String,
    pub pass: bool,
    pub metrics: HashMap<String, JsonValue>,
    pub notes: Option<String>,
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestErrorKind {
    Setup,
    Device,
    Permission,
    Timeout,
    Internal,
}

#[derive(Debug, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct TestError {
    pub kind: TestErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct TestContext {
    pub device: Option<String>,
    pub duration: Duration,
    pub thresholds: Option<crate::probes::thresholds::Thresholds>,
    pub output_dir: Option<PathBuf>,
}

impl TestContext {
    pub fn new_seconds(duration_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            ..Default::default()
        }
    }
}

pub fn ensure_results_dir(base: Option<&Path>) -> std::io::Result<PathBuf> {
    let dir = if let Some(base) = base {
        base.to_path_buf()
    } else {
        let home = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        home.join(".coldvox").join("test_runs")
    };
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn write_result_json(dir: &Path, result: &LiveTestResult) -> std::io::Result<PathBuf> {
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let file = dir.join(format!("{}_{}.json", result.test, ts));
    let data = serde_json::to_vec_pretty(result).expect("serialize result");
    std::fs::write(&file, data)?;
    Ok(file)
}
