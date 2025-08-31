use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct SilenceDetector {
    threshold: i16,
    silence_start: Option<Instant>,
    last_check: Instant,
}

impl SilenceDetector {
    pub fn new(threshold: i16) -> Self {
        Self {
            threshold,
            silence_start: None,
            last_check: Instant::now(),
        }
    }

    pub fn is_silence(&mut self, samples: &[i16]) -> bool {
        self.last_check = Instant::now();

        // Calculate RMS
        let sum: i64 = samples.iter().map(|&s| s as i64 * s as i64).sum();
        let rms = ((sum / samples.len() as i64) as f64).sqrt() as i16;

        if rms < self.threshold {
            if self.silence_start.is_none() {
                self.silence_start = Some(Instant::now());
            }
            true
        } else {
            self.silence_start = None;
            false
        }
    }

    pub fn silence_duration(&self) -> Duration {
        self.silence_start
            .map(|start| Instant::now().duration_since(start))
            .unwrap_or(Duration::ZERO)
    }

    pub fn reset(&mut self) {
        self.silence_start = None;
    }
}
