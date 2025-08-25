use super::{LiveTest, TestContext, LiveTestResult, TestError};
use super::common::AudioChunker;
use super::common::VadProcessor;
use super::common::VadAdapter;

#[derive(Debug, PartialEq)]
pub struct VadFromMicCheck {
    duration: u64,
}

impl VadFromMicCheck {
    pub fn new(duration: u64) -> Self {
        VadFromMicCheck { duration }
    }
}

impl LiveTest for VadFromMicCheck {
    fn name() -> &'static str {
        "vad_mic"
    }

    fn run(ctx: &mut TestContext) -> Result<LiveTestResult, TestError> {
        // Initialize audio capture with device selection from context
        let mut chunker = match AudioChunker::new(&ctx.device_selection, 512, 16000) {
            Ok(c) => c,
            Err(e) => return Err(TestError::Device),
        };

        // Initialize VAD processor with Silero adapter
        let mut vad_processor = match VadAdapter::new("silero") {
            Ok(v) => v,
            Err(e) => return Err(TestError::Internal),
        };

        // Start capture for N seconds
        let start_time = std::time::Instant::now();
        let mut event_counts = 0;
        let mut avg_probability = 0.0;
        let mut latency_samples = 0;

        while start_time.elapsed().as_secs() < ctx.timeouts {
            // Process audio chunks
            let chunk = chunker.read_chunk()?;
            let vad_result = vad_processor.process_chunk(chunk);
            
            // Count events and measure latency
            if vad_result.is_speech() {
                event_counts += 1;
                latency_samples += vad_result.latency_samples;
            }
            
            // Calculate metrics
            avg_probability = vad_result.probability;
        }

        // Calculate metrics
        let latency_avg_ms = (latency_samples as f64 / event_counts as f64) * 1000.0;
        
        // Evaluate thresholds
        let pass = !ctx.thresholds
            .vad_mic
            .check(&[
                ("event_counts", event_counts),
                ("avg_probability", avg_probability),
                ("latency_avg_ms", latency_avg_ms),
            ]);

        Ok(LiveTestResult {
            metrics: {
                let mut metrics = std::collections::HashMap::new();
                metrics.insert("event_counts".to_string(), event_counts.to_string());
                metrics.insert("avg_probability".to_string(), avg_probability.to_string());
                metrics.insert("latency_avg_ms".to_string(), latency_avg_ms.to_string());
                metrics
            },
            pass,
            notes: if pass {
                "OK".to_string()
            } else {
                "Thresholds violated".to_string()
            },
            artifacts: vec![],
        })
    }
}