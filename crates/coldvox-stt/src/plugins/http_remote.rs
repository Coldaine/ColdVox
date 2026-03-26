//! HTTP Remote STT Plugin
//!
//! Sends audio to an OpenAI-compatible `/v1/audio/transcriptions` endpoint.
//! Buffers PCM frames during speech, encodes to WAV on finalize, and POSTs to the service.

use crate::plugin::{PluginCapabilities, PluginInfo, SttPlugin, SttPluginFactory};
use crate::types::{TranscriptionConfig, TranscriptionEvent};
use async_trait::async_trait;
use coldvox_foundation::error::{ColdVoxError, SttError};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io::Cursor;
use std::time::Duration;
use ureq::unversioned::multipart::{Form, Part};

/// Configuration for the HTTP remote plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRemoteConfig {
    /// Base URL of the STT service (e.g., "http://localhost:5096")
    pub base_url: String,
    /// API path (e.g., "/v1/audio/transcriptions")
    pub api_path: String,
    /// Model name to send in the request (e.g., "moonshine/base")
    pub model_name: String,
    /// Display name for logging/UI
    pub display_name: String,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Sample rate of the audio being sent (typically 16000)
    pub sample_rate: u32,
}

impl Default for HttpRemoteConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:5096".into(),
            api_path: "/v1/audio/transcriptions".into(),
            model_name: "moonshine/base".into(),
            display_name: "Moonshine (HTTP)".into(),
            timeout_ms: 10_000,
            sample_rate: 16_000,
        }
    }
}

/// Response from an OpenAI-compatible STT service.
#[derive(Debug, Deserialize)]
struct SttResponse {
    text: String,
}

pub struct HttpRemotePlugin {
    config: HttpRemoteConfig,
    client: ureq::Agent,
    audio_buffer: Vec<i16>,
    utterance_id: u64,
}

impl Debug for HttpRemotePlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRemotePlugin")
            .field("config", &self.config)
            .field("buffer_size", &self.audio_buffer.len())
            .field("utterance_id", &self.utterance_id)
            .finish()
    }
}

fn is_local_url(base_url: &str) -> bool {
    base_url.contains("localhost") || base_url.contains("127.0.0.1")
}

impl HttpRemotePlugin {
    pub fn new(config: HttpRemoteConfig) -> Self {
        let config_builder = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(config.timeout_ms)))
            .build();
        let client: ureq::Agent = config_builder.into();

        Self {
            config,
            client,
            audio_buffer: Vec::new(),
            utterance_id: 0,
        }
    }

    fn encode_wav(&self) -> Result<Vec<u8>, ColdVoxError> {
        let mut cursor = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.config.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::new(&mut cursor, spec).map_err(|e| {
            SttError::TranscriptionFailed(format!("Failed to create WAV writer: {e}"))
        })?;

        for &sample in &self.audio_buffer {
            writer.write_sample(sample).map_err(|e| {
                SttError::TranscriptionFailed(format!("Failed to write WAV sample: {e}"))
            })?;
        }

        writer.finalize().map_err(|e| {
            SttError::TranscriptionFailed(format!("Failed to finalize WAV: {e}"))
        })?;

        Ok(cursor.into_inner())
    }
}

#[async_trait]
impl SttPlugin for HttpRemotePlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "http-remote".to_string(),
            name: self.config.display_name.clone(),
            description: format!("Transcribe via HTTP API ({})", self.config.base_url),
            requires_network: true,
            is_local: is_local_url(&self.config.base_url),
            is_available: true,
            supported_languages: vec!["en".to_string()],
            memory_usage_mb: Some(10),
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
        match self.client.get(&self.config.base_url).call() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn initialize(&mut self, _config: TranscriptionConfig) -> Result<(), ColdVoxError> {
        self.audio_buffer.clear();
        Ok(())
    }

    async fn process_audio(
        &mut self,
        samples: &[i16],
    ) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        self.audio_buffer.extend_from_slice(samples);
        Ok(None)
    }

    async fn finalize(&mut self) -> Result<Option<TranscriptionEvent>, ColdVoxError> {
        if self.audio_buffer.is_empty() {
            return Ok(None);
        }

        let wav_data = self.encode_wav()?;
        let url = format!("{}{}", self.config.base_url, self.config.api_path);

        let part = Part::bytes(&wav_data)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| {
                SttError::TranscriptionFailed(format!("Failed to build multipart: {e}"))
            })?;

        let form = Form::new()
            .part("file", part)
            .text("model", &self.config.model_name)
            .text("response_format", "json");

        let mut response = self
            .client
            .post(&url)
            .send(form)
            .map_err(|e| SttError::TranscriptionFailed(format!("HTTP request failed: {e}")))?;

        let stt_res: SttResponse = response.body_mut().read_json().map_err(|e| {
            SttError::TranscriptionFailed(format!("Failed to parse STT response: {e}"))
        })?;

        let event = TranscriptionEvent::Final {
            utterance_id: self.utterance_id,
            text: stt_res.text,
            words: None,
        };

        self.audio_buffer.clear();
        self.utterance_id += 1;

        Ok(Some(event))
    }

    async fn reset(&mut self) -> Result<(), ColdVoxError> {
        self.audio_buffer.clear();
        self.utterance_id += 1;
        Ok(())
    }

    async fn unload(&mut self) -> Result<(), ColdVoxError> {
        self.audio_buffer.clear();
        Ok(())
    }
}

pub struct HttpRemotePluginFactory {
    config: HttpRemoteConfig,
}

impl HttpRemotePluginFactory {
    pub fn new(config: HttpRemoteConfig) -> Self {
        Self { config }
    }

    pub fn moonshine_base() -> Self {
        Self::new(HttpRemoteConfig::default())
    }

    pub fn parakeet_gpu() -> Self {
        Self::new(HttpRemoteConfig {
            base_url: "http://localhost:8200".into(),
            api_path: "/audio/transcriptions".into(),
            model_name: "parakeet".into(),
            display_name: "Parakeet GPU (Docker)".into(),
            timeout_ms: 10_000,
            sample_rate: 16_000,
        })
    }
}

impl SttPluginFactory for HttpRemotePluginFactory {
    fn create(&self) -> Result<Box<dyn SttPlugin>, ColdVoxError> {
        Ok(Box::new(HttpRemotePlugin::new(self.config.clone())))
    }

    fn plugin_info(&self) -> PluginInfo {
        PluginInfo {
            id: "http-remote".to_string(),
            name: self.config.display_name.clone(),
            description: format!("Transcribe via HTTP API ({})", self.config.base_url),
            requires_network: true,
            is_local: is_local_url(&self.config.base_url),
            is_available: true,
            supported_languages: vec!["en".to_string()],
            memory_usage_mb: Some(10),
        }
    }

    fn check_requirements(&self) -> Result<(), ColdVoxError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wav_encoding() {
        let config = HttpRemoteConfig::default();
        let mut plugin = HttpRemotePlugin::new(config);

        let samples: Vec<i16> = (0..16000)
            .map(|i| {
                (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 16000.0).sin() * 32767.0
            })
            .map(|s| s as i16)
            .collect();

        plugin.audio_buffer.extend_from_slice(&samples);

        let wav_data = plugin.encode_wav().expect("Should encode WAV");
        assert!(wav_data.len() > 32000);

        let mut reader = hound::WavReader::new(Cursor::new(wav_data)).expect("Should read WAV");
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 16000);
        assert_eq!(spec.bits_per_sample, 16);

        let read_samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(read_samples.len(), 16000);
        assert_eq!(read_samples[0], samples[0]);
    }

    #[test]
    fn test_info_is_correct() {
        let config = HttpRemoteConfig {
            base_url: "http://localhost:1234".into(),
            display_name: "Test Plugin".into(),
            ..Default::default()
        };
        let plugin = HttpRemotePlugin::new(config);
        let info = plugin.info();

        assert_eq!(info.id, "http-remote");
        assert!(info.is_local);
    }
}
