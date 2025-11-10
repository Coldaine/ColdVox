//! Type definitions for the Candle Whisper engine.
//!
//! This module defines the core data structures used by the Whisper engine,
//! including configuration, transcription results, and device selection.

use std::path::PathBuf;

/// Configuration for initializing the Whisper engine.
#[derive(Debug, Clone)]
pub struct WhisperEngineInit {
    /// Path to the model weights file (safetensors or gguf format)
    pub model_path: PathBuf,
    /// Path to the tokenizer configuration directory or file
    pub tokenizer_path: PathBuf,
    /// Path to the model configuration JSON file
    pub config_path: PathBuf,
    /// Whether the model is quantized (GGUF format)
    pub quantized: bool,
    /// Device to run inference on
    pub device: WhisperDevice,
}

/// Device selection for Whisper inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhisperDevice {
    /// Use CPU for inference
    Cpu,
    /// Use CUDA GPU with the specified device index
    Cuda(usize),
    /// Use Metal (Apple Silicon)
    Metal,
}

impl WhisperDevice {
    /// Convert device to Candle device
    ///
    /// # Metal Device Index
    ///
    /// Metal (Apple Silicon) currently uses hardcoded device index 0 because:
    /// 1. Most Macs have a single GPU
    /// 2. Candle's Metal backend currently only supports the default device
    /// 3. Multi-GPU Macs are rare, and when they exist, the system scheduler
    ///    handles GPU selection automatically
    ///
    /// # CUDA Error Handling
    ///
    /// CUDA device creation can fail if:
    /// - The specified device index doesn't exist
    /// - CUDA runtime is not installed
    /// - The GPU is in use or unavailable
    /// - Driver version mismatch
    ///
    /// The error is propagated to the caller for proper handling.
    #[cfg(feature = "whisper")]
    pub(crate) fn to_candle_device(&self) -> anyhow::Result<candle_core::Device> {
        match self {
            WhisperDevice::Cpu => Ok(candle_core::Device::Cpu),
            WhisperDevice::Cuda(idx) => {
                // CUDA device creation validates the device exists and is accessible
                candle_core::Device::new_cuda(*idx)
                    .map_err(|e| anyhow::anyhow!("Failed to create CUDA device {}: {}", idx, e))
            }
            WhisperDevice::Metal => {
                // Metal device 0 is the default/primary GPU on Apple Silicon
                // Most Macs have only one GPU, and macOS handles multi-GPU scheduling
                candle_core::Device::new_metal(0)
                    .map_err(|e| anyhow::anyhow!("Failed to create Metal device: {}", e))
            }
        }
    }
}

/// Task type for Whisper inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperTask {
    /// Transcribe audio in its original language
    Transcribe,
    /// Translate audio to English
    Translate,
}

/// Options for transcription.
#[derive(Debug, Clone)]
pub struct TranscribeOptions {
    /// Language code (e.g., "en", "es"). None for auto-detection.
    pub language: Option<String>,
    /// Task to perform (transcribe or translate)
    pub task: WhisperTask,
    /// Sampling temperature (0.0 for greedy, > 0.0 for sampling)
    pub temperature: f32,
    /// Whether to generate timestamps
    pub enable_timestamps: bool,
}

impl Default for TranscribeOptions {
    fn default() -> Self {
        Self {
            language: None,
            task: WhisperTask::Transcribe,
            temperature: 0.0, // Greedy by default
            enable_timestamps: true,
        }
    }
}

/// Complete transcription result.
#[derive(Debug, Clone)]
pub struct Transcript {
    /// Transcribed text segments
    pub segments: Vec<Segment>,
    /// Detected language (if auto-detected)
    pub language: Option<String>,
}

/// A single transcription segment with timing and metadata.
#[derive(Debug, Clone)]
pub struct Segment {
    /// Start time in seconds
    pub start_seconds: f64,
    /// End time in seconds
    pub end_seconds: f64,
    /// Transcribed text
    pub text: String,
    /// Average log probability of the segment
    pub avg_logprob: f64,
    /// Probability that this segment contains no speech
    pub no_speech_prob: f64,
}

impl Segment {
    /// Create a new segment with default probabilities
    ///
    /// # Default Probability Values
    ///
    /// Both `avg_logprob` and `no_speech_prob` default to 0.0 because:
    /// 1. The current decoder implementation doesn't calculate these values (TODO in decode.rs)
    /// 2. 0.0 is a safe neutral value that won't bias downstream processing
    /// 3. Once implemented, these will be overridden with actual computed values
    pub fn new(start: f64, end: f64, text: String) -> Self {
        Self {
            start_seconds: start,
            end_seconds: end,
            text,
            avg_logprob: 0.0,
            no_speech_prob: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_device_equality() {
        assert_eq!(WhisperDevice::Cpu, WhisperDevice::Cpu);
        assert_eq!(WhisperDevice::Cuda(0), WhisperDevice::Cuda(0));
        assert_eq!(WhisperDevice::Metal, WhisperDevice::Metal);

        assert_ne!(WhisperDevice::Cpu, WhisperDevice::Metal);
        assert_ne!(WhisperDevice::Cuda(0), WhisperDevice::Cuda(1));
    }

    #[test]
    #[cfg(feature = "whisper")]
    fn test_cpu_device_creation() {
        // CPU device should always succeed
        let device = WhisperDevice::Cpu.to_candle_device();
        assert!(device.is_ok());
    }

    #[test]
    #[cfg(feature = "whisper")]
    fn test_cuda_device_error_message() {
        // Test that CUDA errors include the device index
        // This will fail unless you have CUDA device 999 (you probably don't)
        let device = WhisperDevice::Cuda(999).to_candle_device();
        if let Err(e) = device {
            let err_msg = e.to_string();
            assert!(err_msg.contains("999"), "Error message should include device index");
        }
        // Note: If CUDA device 999 exists, this test will pass anyway
    }

    #[test]
    fn test_transcribe_options_default() {
        let opts = TranscribeOptions::default();
        assert_eq!(opts.temperature, 0.0, "Default should use greedy decoding");
        assert_eq!(opts.task, WhisperTask::Transcribe);
        assert!(opts.enable_timestamps, "Timestamps should be enabled by default");
        assert_eq!(opts.language, None, "Should auto-detect language by default");
    }

    #[test]
    fn test_transcribe_options_custom() {
        let opts = TranscribeOptions {
            language: Some("es".to_string()),
            task: WhisperTask::Translate,
            temperature: 0.5,
            enable_timestamps: false,
        };
        assert_eq!(opts.language, Some("es".to_string()));
        assert_eq!(opts.task, WhisperTask::Translate);
        assert_eq!(opts.temperature, 0.5);
        assert!(!opts.enable_timestamps);
    }

    #[test]
    fn test_segment_creation() {
        let seg = Segment::new(1.5, 3.7, "Hello world".to_string());
        assert_eq!(seg.start_seconds, 1.5);
        assert_eq!(seg.end_seconds, 3.7);
        assert_eq!(seg.text, "Hello world");
        assert_eq!(seg.avg_logprob, 0.0);
        assert_eq!(seg.no_speech_prob, 0.0);
    }

    #[test]
    fn test_segment_zero_duration() {
        // Edge case: segment with zero duration (start == end)
        let seg = Segment::new(2.0, 2.0, "".to_string());
        assert_eq!(seg.start_seconds, seg.end_seconds);
        assert!(seg.text.is_empty());
    }

    #[test]
    fn test_segment_negative_time() {
        // Edge case: negative timestamps (shouldn't happen but API allows it)
        let seg = Segment::new(-1.0, 0.5, "Test".to_string());
        assert_eq!(seg.start_seconds, -1.0);
        assert_eq!(seg.end_seconds, 0.5);
    }

    #[test]
    fn test_transcript_empty_segments() {
        let transcript = Transcript {
            segments: vec![],
            language: None,
        };
        assert!(transcript.segments.is_empty());
        assert_eq!(transcript.language, None);
    }

    #[test]
    fn test_transcript_with_language() {
        let transcript = Transcript {
            segments: vec![Segment::new(0.0, 1.0, "Hello".to_string())],
            language: Some("en".to_string()),
        };
        assert_eq!(transcript.segments.len(), 1);
        assert_eq!(transcript.language, Some("en".to_string()));
    }

    #[test]
    fn test_whisper_task_equality() {
        assert_eq!(WhisperTask::Transcribe, WhisperTask::Transcribe);
        assert_eq!(WhisperTask::Translate, WhisperTask::Translate);
        assert_ne!(WhisperTask::Transcribe, WhisperTask::Translate);
    }

    #[test]
    fn test_engine_init_paths() {
        use std::path::PathBuf;

        let init = WhisperEngineInit {
            model_path: PathBuf::from("/models/whisper.safetensors"),
            tokenizer_path: PathBuf::from("/models/tokenizer.json"),
            config_path: PathBuf::from("/models/config.json"),
            quantized: false,
            device: WhisperDevice::Cpu,
        };

        assert_eq!(init.model_path, PathBuf::from("/models/whisper.safetensors"));
        assert!(!init.quantized);
        assert_eq!(init.device, WhisperDevice::Cpu);
    }

    #[test]
    fn test_engine_init_quantized() {
        use std::path::PathBuf;

        let init = WhisperEngineInit {
            model_path: PathBuf::from("/models/whisper.gguf"),
            tokenizer_path: PathBuf::from("/models/tokenizer.json"),
            config_path: PathBuf::from("/models/config.json"),
            quantized: true,
            device: WhisperDevice::Cuda(0),
        };

        assert!(init.quantized);
        assert_eq!(init.device, WhisperDevice::Cuda(0));
    }
}
