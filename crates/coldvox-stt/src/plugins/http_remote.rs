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
use std::io::{Cursor, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

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

#[derive(Debug, Clone)]
struct HttpEndpoint {
    host: String,
    port: u16,
    base_path: String,
}

pub struct HttpRemotePlugin {
    config: HttpRemoteConfig,
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

fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn join_url_paths(base_path: &str, api_path: &str) -> String {
    let base = base_path.trim_end_matches('/');
    let api = api_path.trim_start_matches('/');
    if base.is_empty() {
        format!("/{api}")
    } else if api.is_empty() {
        base.to_string()
    } else {
        format!("{base}/{api}")
    }
}

fn parse_http_endpoint(base_url: &str) -> Result<HttpEndpoint, ColdVoxError> {
    let remainder = base_url.strip_prefix("http://").ok_or_else(|| {
        SttError::TranscriptionFailed(format!(
            "Unsupported HTTP remote base URL '{base_url}': only plain http:// endpoints are supported in this build"
        ))
    })?;

    let (authority, base_path) = match remainder.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (remainder, "/".to_string()),
    };

    if authority.is_empty() {
        return Err(SttError::TranscriptionFailed(format!(
            "Invalid HTTP remote base URL '{base_url}': missing host"
        ))
        .into());
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() && !port.is_empty() => {
            let port = port.parse::<u16>().map_err(|e| {
                SttError::TranscriptionFailed(format!(
                    "Invalid HTTP remote base URL '{base_url}': bad port ({e})"
                ))
            })?;
            (host.to_string(), port)
        }
        _ => (authority.to_string(), 80),
    };

    Ok(HttpEndpoint {
        host,
        port,
        base_path: normalize_path(&base_path),
    })
}

fn extract_http_body(response: &[u8]) -> Result<&[u8], ColdVoxError> {
    let marker = b"\r\n\r\n";
    response
        .windows(marker.len())
        .position(|window| window == marker)
        .map(|idx| &response[idx + marker.len()..])
        .ok_or_else(|| {
            SttError::TranscriptionFailed("HTTP response missing header terminator".to_string())
                .into()
        })
}

impl HttpRemotePlugin {
    pub fn new(config: HttpRemoteConfig) -> Self {
        Self {
            config,
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

        writer
            .finalize()
            .map_err(|e| SttError::TranscriptionFailed(format!("Failed to finalize WAV: {e}")))?;

        Ok(cursor.into_inner())
    }

    fn build_request_body(&self, wav_data: &[u8], boundary: &str) -> Vec<u8> {
        let mut body = Vec::new();

        let model_part = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\n{}\r\n",
            self.config.model_name
        );
        body.extend_from_slice(model_part.as_bytes());

        let response_format_part = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"response_format\"\r\n\r\njson\r\n"
        );
        body.extend_from_slice(response_format_part.as_bytes());

        let file_header = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        );
        body.extend_from_slice(file_header.as_bytes());
        body.extend_from_slice(wav_data);
        body.extend_from_slice(b"\r\n");

        let closing = format!("--{boundary}--\r\n");
        body.extend_from_slice(closing.as_bytes());

        body
    }

    fn send_transcription_request(&self, wav_data: &[u8]) -> Result<SttResponse, ColdVoxError> {
        let endpoint = parse_http_endpoint(&self.config.base_url)?;
        let request_path = join_url_paths(&endpoint.base_path, &self.config.api_path);
        let boundary = format!("coldvox-http-remote-{}", self.utterance_id);
        let body = self.build_request_body(wav_data, &boundary);

        let addr = format!("{}:{}", endpoint.host, endpoint.port);
        let socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| SttError::TranscriptionFailed(format!("Failed to resolve {addr}: {e}")))?
            .next()
            .ok_or_else(|| {
                SttError::TranscriptionFailed(format!("No socket address resolved for {addr}"))
            })?;

        let mut stream =
            TcpStream::connect_timeout(&socket_addr, Duration::from_millis(self.config.timeout_ms))
                .map_err(|e| SttError::TranscriptionFailed(format!("HTTP connect failed: {e}")))?;
        let timeout = Some(Duration::from_millis(self.config.timeout_ms));
        let _ = stream.set_read_timeout(timeout);
        let _ = stream.set_write_timeout(timeout);

        let request_headers = format!(
            concat!(
                "POST {path} HTTP/1.1\r\n",
                "Host: {host}:{port}\r\n",
                "Content-Type: multipart/form-data; boundary={boundary}\r\n",
                "Content-Length: {content_length}\r\n",
                "Accept: application/json\r\n",
                "Connection: close\r\n\r\n"
            ),
            path = request_path,
            host = endpoint.host,
            port = endpoint.port,
            boundary = boundary,
            content_length = body.len()
        );

        stream
            .write_all(request_headers.as_bytes())
            .and_then(|_| stream.write_all(&body))
            .map_err(|e| SttError::TranscriptionFailed(format!("HTTP request failed: {e}")))?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response).map_err(|e| {
            SttError::TranscriptionFailed(format!("HTTP response read failed: {e}"))
        })?;

        let response_text = String::from_utf8_lossy(&response);
        let status_line = response_text.lines().next().unwrap_or_default();
        if !status_line.contains(" 200 ") {
            return Err(SttError::TranscriptionFailed(format!(
                "Service returned error: {status_line}"
            ))
            .into());
        }

        let response_body = extract_http_body(&response)?;
        serde_json::from_slice::<SttResponse>(response_body).map_err(|e| {
            SttError::TranscriptionFailed(format!("Failed to parse STT response: {e}")).into()
        })
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
        let endpoint = match parse_http_endpoint(&self.config.base_url) {
            Ok(endpoint) => endpoint,
            Err(_) => return Ok(false),
        };

        let addr = format!("{}:{}", endpoint.host, endpoint.port);
        let socket_addr = match addr.to_socket_addrs() {
            Ok(mut addrs) => match addrs.next() {
                Some(addr) => addr,
                None => return Ok(false),
            },
            Err(_) => return Ok(false),
        };

        Ok(
            TcpStream::connect_timeout(&socket_addr, Duration::from_millis(self.config.timeout_ms))
                .is_ok(),
        )
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
        let stt_res = self.send_transcription_request(&wav_data)?;

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
    use crate::plugin::SttPlugin;
    use crate::types::{TranscriptionConfig, TranscriptionEvent};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    fn test_samples() -> Vec<i16> {
        vec![0, 1024, -1024, 16_384, -16_384, 32_767, -32_768, 256]
    }

    fn test_config(base_url: String) -> HttpRemoteConfig {
        HttpRemoteConfig {
            base_url,
            api_path: "/v1/audio/transcriptions".into(),
            model_name: "moonshine/test".into(),
            display_name: "HTTP Remote Test".into(),
            timeout_ms: 100,
            sample_rate: 16_000,
        }
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn read_http_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4096];
        let mut expected_len = None;
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .expect("set request read timeout");

        loop {
            let read = stream.read(&mut chunk).expect("read HTTP request");
            if read == 0 {
                break;
            }

            request.extend_from_slice(&chunk[..read]);

            if expected_len.is_none() {
                if let Some(header_end) = find_bytes(&request, b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..header_end + 4]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("Content-Length: ")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .expect("content length header");
                    expected_len = Some(header_end + 4 + content_length);
                }
            }

            if let Some(expected_len) = expected_len {
                if request.len() >= expected_len {
                    break;
                }
            }
        }

        request
    }

    fn spawn_stub_server<F>(handler: F) -> (String, thread::JoinHandle<()>)
    where
        F: FnOnce(std::net::TcpStream) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub server");
        let addr = listener.local_addr().expect("stub server address");
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept stub connection");
            handler(stream);
        });

        (format!("http://127.0.0.1:{}", addr.port()), handle)
    }

    fn free_local_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .expect("bind port probe")
            .local_addr()
            .expect("port probe address")
            .port()
    }

    #[test]
    fn test_wav_encoding() {
        let config = HttpRemoteConfig::default();
        let mut plugin = HttpRemotePlugin::new(config);

        let samples: Vec<i16> = (0..16000)
            .map(|i| (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 16000.0).sin() * 32767.0)
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

    #[tokio::test]
    async fn test_finalize_posts_wav_and_metadata_to_http_service() {
        let (tx, rx) = mpsc::channel();
        let (base_url, handle) = spawn_stub_server(move |mut stream| {
            let request = read_http_request(&mut stream);
            tx.send(request).expect("capture HTTP request");

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: application/json\r\n",
                "Content-Length: 27\r\n",
                "Connection: close\r\n\r\n",
                r#"{"text":"stub transcript"}"#,
            );
            stream
                .write_all(response.as_bytes())
                .expect("write HTTP response");
        });

        let mut plugin = HttpRemotePlugin::new(test_config(format!("{base_url}/service")));
        let samples = test_samples();

        plugin
            .initialize(TranscriptionConfig::default())
            .await
            .expect("initialize plugin");
        plugin
            .process_audio(&samples)
            .await
            .expect("buffer audio samples");

        let event = plugin.finalize().await.expect("finalize succeeds");
        handle.join().expect("join stub server");

        match event {
            Some(TranscriptionEvent::Final {
                utterance_id,
                text,
                words,
            }) => {
                assert_eq!(utterance_id, 0);
                assert_eq!(text, "stub transcript");
                assert!(words.is_none());
            }
            other => panic!("expected final transcription event, got {other:?}"),
        }

        let request = rx.recv().expect("captured request");
        let request_text = String::from_utf8_lossy(&request);
        assert!(request_text.starts_with("POST /service/v1/audio/transcriptions HTTP/1.1\r\n"));
        assert!(request_text.contains("Host: 127.0.0.1:"));
        assert!(request_text
            .contains("Content-Type: multipart/form-data; boundary=coldvox-http-remote-0"));
        assert!(request_text.contains("Accept: application/json"));

        let body = extract_http_body(&request).expect("extract request body");
        let body_text = String::from_utf8_lossy(body);
        assert!(body_text.contains("name=\"model\"\r\n\r\nmoonshine/test\r\n"));
        assert!(body_text.contains("name=\"response_format\"\r\n\r\njson\r\n"));
        assert!(body_text.contains("filename=\"audio.wav\""));

        let wav_start = find_bytes(body, b"Content-Type: audio/wav\r\n\r\n")
            .expect("locate WAV part start")
            + b"Content-Type: audio/wav\r\n\r\n".len();
        let wav_end =
            find_bytes(body, b"\r\n--coldvox-http-remote-0--").expect("locate WAV part end");
        let wav_data = &body[wav_start..wav_end];

        let mut reader = hound::WavReader::new(Cursor::new(wav_data)).expect("read request WAV");
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 16_000);
        assert_eq!(spec.bits_per_sample, 16);

        let roundtrip_samples: Vec<i16> = reader
            .samples::<i16>()
            .map(|sample| sample.unwrap())
            .collect();
        assert_eq!(roundtrip_samples, samples);
    }

    #[tokio::test]
    async fn test_finalize_fails_for_unreachable_endpoint() {
        let port = free_local_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let mut plugin = HttpRemotePlugin::new(test_config(base_url));

        plugin
            .initialize(TranscriptionConfig::default())
            .await
            .expect("initialize plugin");
        plugin
            .process_audio(&test_samples())
            .await
            .expect("buffer audio samples");

        let error = plugin
            .finalize()
            .await
            .expect_err("unreachable endpoint should fail");
        assert!(error.to_string().contains("HTTP connect failed"));
    }

    #[tokio::test]
    async fn test_finalize_fails_for_non_200_response() {
        let (base_url, handle) = spawn_stub_server(move |mut stream| {
            let _request = read_http_request(&mut stream);
            let response = concat!(
                "HTTP/1.1 503 Service Unavailable\r\n",
                "Content-Length: 0\r\n",
                "Connection: close\r\n\r\n"
            );
            stream
                .write_all(response.as_bytes())
                .expect("write HTTP response");
        });
        let mut plugin = HttpRemotePlugin::new(test_config(base_url));

        plugin
            .initialize(TranscriptionConfig::default())
            .await
            .expect("initialize plugin");
        plugin
            .process_audio(&test_samples())
            .await
            .expect("buffer audio samples");

        let error = plugin
            .finalize()
            .await
            .expect_err("non-200 response should fail");
        handle.join().expect("join stub server");
        assert!(error
            .to_string()
            .contains("Service returned error: HTTP/1.1 503 Service Unavailable"));
    }

    #[tokio::test]
    async fn test_finalize_fails_for_malformed_json_response() {
        let (base_url, handle) = spawn_stub_server(move |mut stream| {
            let _request = read_http_request(&mut stream);
            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: application/json\r\n",
                "Content-Length: 12\r\n",
                "Connection: close\r\n\r\n",
                "not-json!!!?"
            );
            stream
                .write_all(response.as_bytes())
                .expect("write malformed JSON response");
        });
        let mut plugin = HttpRemotePlugin::new(test_config(base_url));

        plugin
            .initialize(TranscriptionConfig::default())
            .await
            .expect("initialize plugin");
        plugin
            .process_audio(&test_samples())
            .await
            .expect("buffer audio samples");

        let error = plugin
            .finalize()
            .await
            .expect_err("malformed JSON should fail");
        handle.join().expect("join stub server");
        assert!(error.to_string().contains("Failed to parse STT response"));
    }

    #[tokio::test]
    async fn test_finalize_fails_for_malformed_http_response() {
        let (base_url, handle) = spawn_stub_server(move |mut stream| {
            let _request = read_http_request(&mut stream);
            let response = b"HTTP/1.1 200 OK\r\nContent-Length: 17\r\n{\"text\":\"broken\"}";
            stream
                .write_all(response)
                .expect("write malformed HTTP response");
        });
        let mut plugin = HttpRemotePlugin::new(test_config(base_url));

        plugin
            .initialize(TranscriptionConfig::default())
            .await
            .expect("initialize plugin");
        plugin
            .process_audio(&test_samples())
            .await
            .expect("buffer audio samples");

        let error = plugin
            .finalize()
            .await
            .expect_err("malformed HTTP should fail");
        handle.join().expect("join stub server");
        assert!(error
            .to_string()
            .contains("HTTP response missing header terminator"));
    }

    #[tokio::test]
    async fn test_finalize_fails_when_response_times_out() {
        let (base_url, handle) = spawn_stub_server(move |mut stream| {
            let _request = read_http_request(&mut stream);
            thread::sleep(Duration::from_millis(250));
        });
        let mut config = test_config(base_url);
        config.timeout_ms = 50;
        let mut plugin = HttpRemotePlugin::new(config);

        plugin
            .initialize(TranscriptionConfig::default())
            .await
            .expect("initialize plugin");
        plugin
            .process_audio(&test_samples())
            .await
            .expect("buffer audio samples");

        let error = plugin
            .finalize()
            .await
            .expect_err("timed-out response should fail");
        handle.join().expect("join stub server");
        let message = error.to_string();
        assert!(message.contains("HTTP response read failed"));
        assert!(
            message.contains("timed out")
                || message.contains("operation timed out")
                || message.contains("Resource temporarily unavailable")
                || message.contains("failed to respond")
                || message.contains("os error 10060"),
            "unexpected timeout message: {message}"
        );
    }
}
