//! Moonshine CPU STT plugin using PyO3/HuggingFace Transformers.
//!
//! This is the PRIMARY Moonshine backend for CPU-based transcription:
//! - Uses HuggingFace Transformers via PyO3 Python bindings
//! - Automatic model downloading from HuggingFace Hub
//! - Production-quality inference without custom DSP/tokenizer work
//! - 5x faster than Whisper on CPU
//! - English-only, optimized for 16kHz audio

use crate::constants::SAMPLE_RATE_HZ;
use crate::plugin::*;
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use async_trait::async_trait;
use coldvox_foundation::error::{ColdVoxError, SttError};
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

#[cfg(feature = "moonshine")]
use pyo3::{
    ffi::c_str,
    types::{PyAnyMethods, PyDict, PyDictMethods, PyModule},
    Py, PyAny, Python,
};
#[cfg(feature = "moonshine")]
use tempfile::NamedTempFile;

/// Moonshine model variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoonshineModelSize {
    Tiny,
    Base,
}

impl MoonshineModelSize {
    pub fn model_identifier(&self) -> &'static str {
        match self {
            Self::Tiny => "UsefulSensors/moonshine-tiny",
            Self::Base => "UsefulSensors/moonshine-base",
        }
    }

    pub fn memory_usage_mb(&self) -> u32 {
        match self {
            Self::Tiny => 300,
            Self::Base => 500,
        }
    }
}

impl Default for MoonshineModelSize {
    fn default() -> Self {
        Self::Base
    }
}

/// Maximum audio buffer size (10 minutes at 16kHz = 9.6M samples)
#[cfg(feature = "moonshine")]
const MAX_AUDIO_BUFFER_SAMPLES: usize = 16000 * 60 * 10;

/// Moonshine CPU plugin using PyO3/HuggingFace
pub struct MoonshinePlugin {
    model_size: MoonshineModelSize,
    model_path: Option<PathBuf>,
    initialized: bool,
    #[cfg(feature = "moonshine")]
    audio_buffer: Vec<i16>,
    /// Cached Python model (loaded once in initialize, reused across transcriptions)
    ///
    /// SAFETY: `Py<PyAny>` is `Send` but requires the Python GIL for all access.
    /// All methods that access `cached_model` must use `Python::with_gil()`.
    #[cfg(feature = "moonshine")]
    cached_model: Option<Py<PyAny>>,
    /// Cached Python processor (loaded once in initialize, reused across transcriptions)
    ///
    /// SAFETY: `Py<PyAny>` is `Send` but requires the Python GIL for all access.
    /// All methods that access `cached_processor` must use `Python::with_gil()`.
    #[cfg(feature = "moonshine")]
    cached_processor: Option<Py<PyAny>>,
}

// Manual Debug impl because Py<PyAny> doesn't implement Debug
impl std::fmt::Debug for MoonshinePlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoonshinePlugin")
            .field("model_size", &self.model_size)
            .field("model_path", &self.model_path)
            .field("initialized", &self.initialized)
            .finish_non_exhaustive()
    }
}

impl MoonshinePlugin {
    pub fn new() -> Self {
        Self {
            model_size: MoonshineModelSize::default(),
            model_path: None,
            initialized: false,
            #[cfg(feature = "moonshine")]
            audio_buffer: Vec::new(),
            #[cfg(feature = "moonshine")]
            cached_model: None,
            #[cfg(feature = "moonshine")]
            cached_processor: None,
        }
    }

    pub fn with_model_size(mut self, size: MoonshineModelSize) -> Self {
        self.model_size = size;
        self
    }

    pub fn with_model_path(mut self, path: PathBuf) -> Self {
        self.model_path = Some(path);
        self
    }

    #[cfg(feature = "moonshine")]
    fn verify_sample_rate(&self) -> Result<(), ColdVoxError> {
        const REQUIRED_SAMPLE_RATE: u32 = 16000;

        if SAMPLE_RATE_HZ != REQUIRED_SAMPLE_RATE {
            return Err(SttError::LoadFailed(format!(
                "Moonshine requires {}Hz audio, but SAMPLE_RATE_HZ is {}Hz",
                REQUIRED_SAMPLE_RATE, SAMPLE_RATE_HZ
            ))
            .into());
        }
        Ok(())
    }

    #[cfg(feature = "moonshine")]
    fn verify_python_environment() -> Result<(), ColdVoxError> {
        Python::attach(|py| {
            PyModule::import(py, "transformers").map_err(|_| {
                SttError::LoadFailed(
                    "transformers not installed. Run: pip install transformers>=4.35.0".to_string(),
                )
            })?;

            PyModule::import(py, "torch").map_err(|_| {
                SttError::LoadFailed(
                    "torch not installed. Run: pip install torch>=2.0.0".to_string(),
                )
            })?;

            PyModule::import(py, "librosa").map_err(|_| {
                SttError::LoadFailed(
                    "librosa not installed. Run: pip install librosa>=0.10.0".to_string(),
                )
            })?;

            info!(target: "coldvox::stt::moonshine", "Python environment verified");
            Ok(())
        })
    }

    /// Load model and processor into Python, caching them for reuse.
    /// This is called once during initialize() to avoid the 5-10 second
    /// model loading delay on every transcription.
    #[cfg(feature = "moonshine")]
    fn load_model_and_processor(&mut self) -> Result<(), ColdVoxError> {
        // Use custom model path if provided, otherwise use HuggingFace model identifier
        let model_id = self
            .model_path
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or_else(|| self.model_size.model_identifier());

        Python::attach(|py| {
            let locals = PyDict::new(py);
            locals
                .set_item("model_id", model_id)
                .map_err(|e| SttError::LoadFailed(format!("Failed to set model_id: {}", e)))?;

            // Load model and processor using safe variable passing
            // NOTE: Must use run (not eval) because this contains statements
            py.run(
                c_str!(
                    r#"
import torch
from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor

device = "cpu"
torch_dtype = torch.float32

_model = AutoModelForSpeechSeq2Seq.from_pretrained(
    model_id,
    torch_dtype=torch_dtype,
    low_cpu_mem_usage=True,
)
_model.to(device)
_model.eval()  # Set to evaluation mode for inference

_processor = AutoProcessor.from_pretrained(model_id)
"#
                ),
                None,
                Some(&locals),
            )
                .map_err(|e| SttError::LoadFailed(format!("Failed to load model: {}", e)))?;

            // Extract model and processor from locals dict
            let model = locals
                .get_item("_model")
                .map_err(|e| SttError::LoadFailed(format!("Failed to get model: {}", e)))?
                .ok_or_else(|| SttError::LoadFailed("Model not found in locals".to_string()))?;
            let processor = locals
                .get_item("_processor")
                .map_err(|e| SttError::LoadFailed(format!("Failed to get processor: {}", e)))?
                .ok_or_else(|| SttError::LoadFailed("Processor not found in locals".to_string()))?;

            // Store as Py<PyAny> for later use (increments reference count)
            self.cached_model = Some(model.unbind());
            self.cached_processor = Some(processor.unbind());

            info!(
                target: "coldvox::stt::moonshine",
                model = %self.model_size.model_identifier(),
                "Model and processor cached successfully"
            );

            Ok(())
        })
    }

    /// Transcribe audio using the cached model and processor.
    /// SECURITY: Uses PyO3's locals dict to pass the audio path safely,
    /// preventing code injection attacks via malicious file paths.
    #[cfg(feature = "moonshine")]
    fn transcribe_via_python(&self, audio_path: &Path) -> Result<String, ColdVoxError> {
        let model = self
            .cached_model
            .as_ref()
            .ok_or_else(|| SttError::TranscriptionFailed("Model not loaded".to_string()))?;
        let processor = self
            .cached_processor
            .as_ref()
            .ok_or_else(|| SttError::TranscriptionFailed("Processor not loaded".to_string()))?;

        Python::attach(|py| {
            let locals = PyDict::new(py);

            // SECURITY: Pass variables via locals dict, not string interpolation
            // This prevents code injection via malicious file paths
            locals.set_item("model", model.bind(py)).map_err(|e| {
                SttError::TranscriptionFailed(format!("Failed to set model: {}", e))
            })?;
            locals
                .set_item("processor", processor.bind(py))
                .map_err(|e| {
                    SttError::TranscriptionFailed(format!("Failed to set processor: {}", e))
                })?;

            // Convert path to string safely (use forward slashes on all platforms)
            let path_str = audio_path.to_string_lossy().replace('\\', "/");
            locals.set_item("audio_path", path_str).map_err(|e| {
                SttError::TranscriptionFailed(format!("Failed to set audio_path: {}", e))
            })?;

            // NOTE: Must use run (not eval) because this contains statements
            py.run(
                c_str!(
                    r#"
import torch
import librosa

# Load audio using the safely-passed path variable
audio_array, sampling_rate = librosa.load(audio_path, sr=16000, mono=True)

# Process with cached model and processor
inputs = processor(audio_array, sampling_rate=16000, return_tensors="pt")
inputs = {k: v.to("cpu") for k, v in inputs.items()}

with torch.no_grad():
    generated_ids = model.generate(**inputs)

_transcription = processor.batch_decode(generated_ids, skip_special_tokens=True)[0]
"#
                ),
                None,
                Some(&locals),
            )
                .map_err(|e| SttError::TranscriptionFailed(format!("Python error: {}", e)))?;

            let result = locals
                .get_item("_transcription")
                .map_err(|e| SttError::TranscriptionFailed(format!("Failed to get result: {}", e)))?
                .ok_or_else(|| {
                    SttError::TranscriptionFailed("Transcription not found in locals".to_string())
                })?;

            let text: String = result.extract().map_err(|e| {
                SttError::TranscriptionFailed(format!("Failed to extract text: {}", e))
            })?;

            Ok(text)
        })
    }

    #[cfg(feature = "moonshine")]
    fn save_audio_to_wav(&self, samples: &[i16]) -> Result<NamedTempFile, ColdVoxError> {
        let temp_file = NamedTempFile::new().map_err(|e| {
            SttError::TranscriptionFailed(format!("Failed to create temp file: {}", e))
        })?;

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE_HZ,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::new(temp_file.reopen()?, spec).map_err(|e| {
            SttError::TranscriptionFailed(format!("Failed to create WAV writer: {}", e))
        })?;

        for &sample in samples {
            writer.write_sample(sample).map_err(|e| {
                SttError::TranscriptionFailed(format!("Failed to write sample: {}", e))
            })?;
        }

        writer
            .finalize()
            .map_err(|e| SttError::TranscriptionFailed(format!("Failed to finalize WAV: {}", e)))?;

        Ok(temp_file)
    }
}

impl Default for MoonshinePlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SttPlugin for MoonshinePlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "moonshine".to_string(),
            name: format!(
                "Moonshine {} CPU",
                match self.model_size {
                    MoonshineModelSize::Tiny => "Tiny",
                    MoonshineModelSize::Base => "Base",
                }
            ),
            description:
                "CPU-optimized local transcription (English-only, 16kHz, 5x faster than Whisper)"
                    .to_string(),
            requires_network: false,
            is_local: true,
            is_available: check_moonshine_available(),
            supported_languages: vec!["en".to_string()],
            memory_usage_mb: Some(self.model_size.memory_usage_mb()),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            streaming: false,
            batch: true,
            word_timestamps: false,
            confidence_scores: false,
            speaker_diarization: false,
            auto_punctuation: true,
            custom_vocabulary: false,
        }
    }

    async fn is_available(&self) -> Result<bool, ColdVoxError> {
        Ok(check_moonshine_available())
    }

    async fn initialize(&mut self, config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            self.verify_sample_rate()?;
            Self::verify_python_environment()?;

            info!(
                target: "coldvox::stt::moonshine",
                model = %self.model_size.model_identifier(),
                "Initializing Moonshine CPU model via PyO3/HuggingFace"
            );

            // Load and cache model + processor (this takes 5-10 seconds on first run)
            // Subsequent transcriptions will reuse the cached model
            self.load_model_and_processor()?;

            self.audio_buffer.clear();
            self.initialized = true;
            let _ = config; // Config used during load_model_and_processor if needed

            info!(target: "coldvox::stt::moonshine", "Moonshine CPU initialized successfully");
            Ok(())
        }

        #[cfg(not(feature = "moonshine"))]
        {
            let _ = config;
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled. Build with --features moonshine"
                    .to_string(),
            }
            .into())
        }
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            if !self.initialized {
                return Err(SttError::NotAvailable {
                    plugin: "moonshine".to_string(),
                    reason: "Plugin not initialized".to_string(),
                }
                .into());
            }

            // Check buffer size limit to prevent memory exhaustion
            let new_size = self.audio_buffer.len() + samples.len();
            if new_size > MAX_AUDIO_BUFFER_SAMPLES {
                warn!(
                    target: "coldvox::stt::moonshine",
                    current = self.audio_buffer.len(),
                    incoming = samples.len(),
                    max = MAX_AUDIO_BUFFER_SAMPLES,
                    "Audio buffer would exceed maximum size, truncating"
                );
                // Only take as many samples as we can fit
                let available = MAX_AUDIO_BUFFER_SAMPLES.saturating_sub(self.audio_buffer.len());
                if available > 0 {
                    self.audio_buffer
                        .extend_from_slice(&samples[..available.min(samples.len())]);
                }
            } else {
                self.audio_buffer.extend_from_slice(samples);
            }
            Ok(None)
        }

        #[cfg(not(feature = "moonshine"))]
        {
            let _ = samples;
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            if !self.initialized || self.audio_buffer.is_empty() {
                return Ok(None);
            }

            let buffer_size = self.audio_buffer.len();
            info!(
                target: "coldvox::stt::moonshine",
                samples = buffer_size,
                duration_secs = %format!("{:.2}", buffer_size as f32 / SAMPLE_RATE_HZ as f32),
                "Transcribing via PyO3/HuggingFace"
            );

            let temp_file = self.save_audio_to_wav(&self.audio_buffer)?;
            let audio_path = temp_file.path();

            let text = self.transcribe_via_python(audio_path)?;

            self.audio_buffer.clear();

            debug!(target: "coldvox::stt::moonshine", text = %text, "Transcription complete");

            Ok(Some(TranscriptionEvent::Final {
                utterance_id: 0,
                text,
                words: None,
            }))
        }

        #[cfg(not(feature = "moonshine"))]
        {
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            self.audio_buffer.clear();
            Ok(())
        }

        #[cfg(not(feature = "moonshine"))]
        {
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled".to_string(),
            }
            .into())
        }
    }

    async fn load_model(&mut self, _model_path: Option<&Path>) -> Result<(), ColdVoxError> {
        // Moonshine loads models during initialize() and caches them,
        // so this method is intentionally a no-op.
        Ok(())
    }

    async fn unload(&mut self) -> Result<(), ColdVoxError> {
        #[cfg(feature = "moonshine")]
        {
            self.audio_buffer.clear();
            self.cached_model = None;
            self.cached_processor = None;
            self.initialized = false;
            info!(target: "coldvox::stt::moonshine", "Moonshine model unloaded");
            Ok(())
        }

        #[cfg(not(feature = "moonshine"))]
        {
            Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Moonshine feature not compiled".to_string(),
            }
            .into())
        }
    }
}

/// Factory for Moonshine plugin
pub struct MoonshinePluginFactory {
    model_size: MoonshineModelSize,
    model_path: Option<PathBuf>,
}

impl MoonshinePluginFactory {
    pub fn new() -> Self {
        let model_size = env::var("MOONSHINE_MODEL")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "tiny" => Some(MoonshineModelSize::Tiny),
                "base" => Some(MoonshineModelSize::Base),
                _ => {
                    warn!(target: "coldvox::stt::moonshine", "Invalid MOONSHINE_MODEL: {}", v);
                    None
                }
            })
            .unwrap_or_default();

        Self {
            model_size,
            model_path: env::var("MOONSHINE_MODEL_PATH").ok().map(PathBuf::from),
        }
    }
}

impl Default for MoonshinePluginFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl SttPluginFactory for MoonshinePluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        let mut plugin = MoonshinePlugin::new().with_model_size(self.model_size);

        if let Some(ref path) = self.model_path {
            plugin = plugin.with_model_path(path.clone());
        }

        Ok(Box::new(plugin))
    }

    fn plugin_info(&self) -> PluginInfo {
        MoonshinePlugin::new()
            .with_model_size(self.model_size)
            .info()
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        if !check_moonshine_available() {
            return Err(SttError::NotAvailable {
                plugin: "moonshine".to_string(),
                reason: "Python 3.8+ with transformers, torch, and librosa packages required. Run: ./scripts/install-moonshine-deps.sh".to_string(),
            }
            .into());
        }

        #[cfg(feature = "moonshine")]
        MoonshinePlugin::verify_python_environment()?;

        Ok(())
    }
}

#[cfg(feature = "moonshine")]
fn check_moonshine_available() -> bool {
    Python::attach(|py| {
        PyModule::import(py, "transformers").is_ok()
            && PyModule::import(py, "torch").is_ok()
            && PyModule::import(py, "librosa").is_ok()
    })
}

#[cfg(not(feature = "moonshine"))]
fn check_moonshine_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_is_base() {
        assert_eq!(MoonshineModelSize::default(), MoonshineModelSize::Base);
    }

    #[test]
    fn model_identifiers_correct() {
        assert_eq!(
            MoonshineModelSize::Tiny.model_identifier(),
            "UsefulSensors/moonshine-tiny"
        );
        assert_eq!(
            MoonshineModelSize::Base.model_identifier(),
            "UsefulSensors/moonshine-base"
        );
    }

    #[test]
    fn memory_usage_reasonable() {
        assert_eq!(MoonshineModelSize::Tiny.memory_usage_mb(), 300);
        assert_eq!(MoonshineModelSize::Base.memory_usage_mb(), 500);
        assert!(MoonshineModelSize::Base.memory_usage_mb() < 1000);
    }

    #[test]
    fn plugin_info_correct() {
        let plugin = MoonshinePlugin::new();
        let info = plugin.info();

        assert_eq!(info.id, "moonshine");
        assert!(info.supported_languages.contains(&"en".to_string()));
        assert!(info.is_local);
    }

    #[test]
    fn capabilities_correct() {
        let plugin = MoonshinePlugin::new();
        let caps = plugin.capabilities();

        assert!(!caps.streaming);
        assert!(caps.batch);
        assert!(caps.auto_punctuation);
    }
}
