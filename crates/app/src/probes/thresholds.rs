use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MicCaptureThresholds {
    pub max_drop_rate_error: Option<f64>,
    pub max_drop_rate_warn: Option<f64>,
    pub frames_per_sec_min: Option<f64>,
    pub frames_per_sec_max: Option<f64>,
    pub watchdog_must_be_false: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Thresholds {
    #[serde(default)]
    pub mic_capture: MicCaptureThresholds,
}

pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Thresholds> {
    let s = std::fs::read_to_string(path)?;
    let t: Thresholds = toml::from_str(&s)?;
    Ok(t)
}
