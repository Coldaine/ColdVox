use chrono::{DateTime, Local, TimeZone};
use csv::Writer;
use hound::{WavSpec, WavWriter};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::stt::TranscriptionEvent;
use coldvox_audio::chunker::AudioFrame;
use coldvox_vad::types::VadEvent;

/// Configuration for transcription persistence
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Enable persistence
    pub enabled: bool,
    /// Base directory for saving files
    pub output_dir: PathBuf,
    /// Save audio alongside transcriptions
    pub save_audio: bool,
    /// Audio format for saving
    pub audio_format: AudioFormat,
    /// Transcription format
    pub transcript_format: TranscriptFormat,
    /// Keep files for N days (0 = forever)
    pub retention_days: u32,
    /// Sample rate for audio processing
    pub sample_rate: u32,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            output_dir: PathBuf::from("transcriptions"),
            save_audio: false, // Changed from true to match CLI default
            audio_format: AudioFormat::Wav,
            transcript_format: TranscriptFormat::Json,
            retention_days: 30,
            sample_rate: 16000,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AudioFormat {
    Wav,
    // Future: Mp3, Opus, etc.
}

#[derive(Debug, Clone, Copy)]
pub enum TranscriptFormat {
    Json,
    Csv,
    Text,
    // Future: SRT (subtitles), VTT, etc.
}

/// A complete transcription session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSession {
    /// Unique session ID
    pub session_id: String,
    /// Start time of the session (ISO 8601 string)
    pub started_at: String,
    /// End time of the session (if ended)
    pub ended_at: Option<String>,
    /// List of utterances in this session
    pub utterances: Vec<UtteranceRecord>,
    /// Session metadata
    pub metadata: SessionMetadata,
}

/// Record of a single utterance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtteranceRecord {
    /// Unique utterance ID
    pub utterance_id: u64,
    /// Start timestamp
    pub started_at: String,
    /// End timestamp
    pub ended_at: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Transcribed text
    pub text: String,
    /// Confidence score (if available)
    pub confidence: Option<f32>,
    /// Path to audio file (if saved)
    pub audio_path: Option<PathBuf>,
    /// Word-level timing (if available)
    pub words: Option<Vec<WordTiming>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTiming {
    pub word: String,
    pub start_ms: u32,
    pub end_ms: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Audio device used
    pub device_name: String,
    /// Sample rate
    pub sample_rate: u32,
    /// VAD mode used
    pub vad_mode: String,
    /// STT model used
    pub stt_model: String,
    /// Application version
    pub app_version: String,
}

/// Handles persistence of transcriptions and audio
pub struct TranscriptionWriter {
    config: PersistenceConfig,
    current_session: Arc<Mutex<TranscriptionSession>>,
    current_utterance_audio: Arc<Mutex<Vec<i16>>>,
    session_dir: PathBuf,
    utterance_active: Arc<Mutex<bool>>,
    /// Track timing from VAD events
    last_speech_start_ms: Arc<Mutex<Option<u64>>>,
    last_speech_duration_ms: Arc<Mutex<Option<u64>>>,
}

impl TranscriptionWriter {
    /// Create a new transcription writer
    pub fn new(config: PersistenceConfig, metadata: SessionMetadata) -> Result<Self, String> {
        if !config.enabled {
            return Ok(Self {
                config,
                current_session: Arc::new(Mutex::new(TranscriptionSession {
                    session_id: String::new(),
                    started_at: Local::now().to_rfc3339(),
                    ended_at: None,
                    utterances: Vec::new(),
                    metadata,
                })),
                current_utterance_audio: Arc::new(Mutex::new(Vec::new())),
                session_dir: PathBuf::new(),
                utterance_active: Arc::new(Mutex::new(false)),
                last_speech_start_ms: Arc::new(Mutex::new(None)),
                last_speech_duration_ms: Arc::new(Mutex::new(None)),
            });
        }

        // Create output directory structure
        let timestamp = Local::now();
        let date_dir = config
            .output_dir
            .join(timestamp.format("%Y-%m-%d").to_string());
        let session_id = format!("{}", timestamp.format("%H%M%S"));
        let session_dir = date_dir.join(&session_id);

        fs::create_dir_all(&session_dir)
            .map_err(|e| format!("Failed to create session directory: {}", e))?;

        // Create subdirectories
        if config.save_audio {
            fs::create_dir_all(session_dir.join("audio"))
                .map_err(|e| format!("Failed to create audio directory: {}", e))?;
        }

        let session = TranscriptionSession {
            session_id: session_id.clone(),
            started_at: timestamp.to_rfc3339(),
            ended_at: None,
            utterances: Vec::new(),
            metadata,
        };

        // Save initial session manifest
        let manifest_path = session_dir.join("session.json");
        let manifest_json = serde_json::to_string_pretty(&session)
            .map_err(|e| format!("Failed to serialize session: {}", e))?;
        fs::write(&manifest_path, manifest_json)
            .map_err(|e| format!("Failed to write session manifest: {}", e))?;

        Ok(Self {
            config,
            current_session: Arc::new(Mutex::new(session)),
            current_utterance_audio: Arc::new(Mutex::new(Vec::with_capacity(16000 * 10))), // 10 second buffer
            session_dir,
            utterance_active: Arc::new(Mutex::new(false)),
            last_speech_start_ms: Arc::new(Mutex::new(None)),
            last_speech_duration_ms: Arc::new(Mutex::new(None)),
        })
    }

    /// Handle audio frame for potential saving
    pub fn handle_audio_frame(&self, frame: &AudioFrame) {
        if !self.config.enabled || !self.config.save_audio {
            return;
        }

        let is_active = *self.utterance_active.lock();
        if is_active {
            // Convert f32 samples back to i16
            let i16_samples: Vec<i16> = frame
                .samples
                .iter()
                .map(|&s| (s * i16::MAX as f32) as i16)
                .collect();

            // Accumulate audio for current utterance
            let mut audio = self.current_utterance_audio.lock();
            audio.extend_from_slice(&i16_samples);
        }
    }

    /// Handle VAD event
    pub fn handle_vad_event(&self, event: &VadEvent) {
        if !self.config.enabled {
            return;
        }

        match event {
            VadEvent::SpeechStart { timestamp_ms, .. } => {
                *self.utterance_active.lock() = true;
                *self.last_speech_start_ms.lock() = Some(*timestamp_ms);
                if self.config.save_audio {
                    // Clear buffer for new utterance
                    self.current_utterance_audio.lock().clear();
                }
            }
            VadEvent::SpeechEnd { duration_ms, .. } => {
                *self.utterance_active.lock() = false;
                *self.last_speech_duration_ms.lock() = Some(*duration_ms);
            }
        }
    }
    /// Handle transcription event
    pub async fn handle_transcription(&self, event: &TranscriptionEvent) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        match event {
            TranscriptionEvent::Final {
                utterance_id,
                text,
                words,
            } => {
                // Get timing information from VAD events
                let (start_ms, duration_ms) = {
                    let start_lock = self.last_speech_start_ms.lock();
                    let duration_lock = self.last_speech_duration_ms.lock();
                    (*start_lock, *duration_lock)
                };

                // Calculate actual timestamps using VAD timing relative to session start
                let (started_at, ended_at) =
                    if let (Some(start_ms), Some(duration_ms)) = (start_ms, duration_ms) {
                        // Parse session start time
                        let session_start =
                            DateTime::parse_from_rfc3339(&self.current_session.lock().started_at)
                                .map_err(|e| format!("Failed to parse session start time: {}", e))
                                .ok()
                                .and_then(|dt| Some(dt.with_timezone(&Local)));

                        if let Some(session_start) = session_start {
                            // VAD timestamps are relative to session start
                            let start_time =
                                session_start + chrono::Duration::milliseconds(start_ms as i64);
                            let end_time =
                                start_time + chrono::Duration::milliseconds(duration_ms as i64);
                            (start_time.to_rfc3339(), end_time.to_rfc3339())
                        } else {
                            // Fallback if session start parsing fails
                            let now = Local::now();
                            (now.to_rfc3339(), now.to_rfc3339())
                        }
                    } else {
                        // Fallback to current time if VAD timing not available
                        let now = Local::now();
                        (now.to_rfc3339(), now.to_rfc3339())
                    };

                let audio_path = if self.config.save_audio {
                    let audio_data = std::mem::take(&mut *self.current_utterance_audio.lock());
                    if !audio_data.is_empty() {
                        let filename = format!("utterance_{:06}.wav", utterance_id);
                        let path = self.session_dir.join("audio").join(&filename);

                        // Use spawn_blocking for WAV writing to avoid blocking the async runtime
                        let path_clone = path.clone();
                        let audio_data_move = audio_data;
                        let sample_rate = self.config.sample_rate;
                        match tokio::task::spawn_blocking(move || {
                            Self::save_wav_file(&path_clone, &audio_data_move, sample_rate)
                        })
                        .await
                        {
                            Ok(Ok(())) => Some(PathBuf::from(format!("audio/{}", filename))),
                            Ok(Err(e)) => {
                                tracing::error!("Failed to save audio: {}", e);
                                None
                            }
                            Err(e) => {
                                tracing::error!("WAV writing task panicked: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Create utterance record with actual timing
                let utterance = UtteranceRecord {
                    utterance_id: *utterance_id,
                    started_at,
                    ended_at,
                    duration_ms: duration_ms.unwrap_or(0),
                    text: text.clone(),
                    confidence: None,
                    audio_path,
                    words: words.as_ref().map(|w| {
                        w.iter()
                            .map(|word| WordTiming {
                                word: word.text.clone(),
                                start_ms: (word.start * 1000.0) as u32,
                                end_ms: (word.end * 1000.0) as u32,
                                confidence: word.conf,
                            })
                            .collect()
                    }),
                };

                // Add to session
                {
                    let mut session = self.current_session.lock();
                    session.utterances.push(utterance.clone());
                }

                // Save individual utterance file
                self.save_utterance(&utterance).await?;

                // Update session manifest
                self.update_session_manifest().await?;
            }
            _ => {
                // Handle partial results if needed
            }
        }

        Ok(())
    }

    /// Save individual utterance based on format
    async fn save_utterance(&self, utterance: &UtteranceRecord) -> Result<(), String> {
        let filename = format!("utterance_{:06}", utterance.utterance_id);

        match self.config.transcript_format {
            TranscriptFormat::Json => {
                let path = self.session_dir.join(format!("{}.json", filename));
                let json = serde_json::to_string_pretty(utterance)
                    .map_err(|e| format!("Failed to serialize utterance: {}", e))?;
                tokio::fs::write(&path, json)
                    .await
                    .map_err(|e| format!("Failed to write utterance file: {}", e))?;
            }
            TranscriptFormat::Text => {
                let path = self.session_dir.join(format!("{}.txt", filename));
                let content = format!("[{}] {}\n", utterance.started_at, utterance.text);
                tokio::fs::write(&path, content)
                    .await
                    .map_err(|e| format!("Failed to write text file: {}", e))?;
            }
            TranscriptFormat::Csv => {
                // Append to CSV file using proper CSV writer
                let path = self.session_dir.join("transcriptions.csv");

                // Check if file exists and is empty to determine if we need headers
                let needs_header = tokio::fs::metadata(&path)
                    .await
                    .map(|m| m.len() == 0)
                    .unwrap_or(true);

                // Use spawn_blocking for CSV writing to avoid blocking
                let path_clone = path.clone();
                let utterance_clone = utterance.clone();

                let csv_join = tokio::task::spawn_blocking(move || {
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&path_clone)
                        .map_err(|e| format!("Failed to open CSV file: {}", e))?;

                    let mut wtr = Writer::from_writer(file);

                    // Write header if file is new
                    if needs_header {
                        wtr.write_record(&[
                            "utterance_id",
                            "timestamp",
                            "duration_ms",
                            "text",
                            "audio_path",
                        ])
                        .map_err(|e| format!("Failed to write CSV header: {}", e))?;
                    }

                    // Write the record (CSV writer handles proper escaping and quoting)
                    wtr.write_record(&[
                        utterance_clone.utterance_id.to_string(),
                        utterance_clone.started_at,
                        utterance_clone.duration_ms.to_string(),
                        utterance_clone.text,
                        utterance_clone
                            .audio_path
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    ])
                    .map_err(|e| format!("Failed to write CSV record: {}", e))?;

                    wtr.flush()
                        .map_err(|e| format!("Failed to flush CSV writer: {}", e))?;

                    Ok::<(), String>(())
                })
                .await
                .map_err(|e| format!("CSV writing task panicked: {}", e))?;
                csv_join.map_err(|e| e)?;
            }
        }

        Ok(())
    }

    /// Update the session manifest file
    async fn update_session_manifest(&self) -> Result<(), String> {
        let session = self.current_session.lock().clone();
        let manifest_path = self.session_dir.join("session.json");
        let json = serde_json::to_string_pretty(&session)
            .map_err(|e| format!("Failed to serialize session: {}", e))?;
        tokio::fs::write(&manifest_path, json)
            .await
            .map_err(|e| format!("Failed to update session manifest: {}", e))?;
        Ok(())
    }

    /// Save audio data as WAV file
    fn save_wav_file(path: &Path, samples: &[i16], sample_rate: u32) -> Result<(), String> {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(path, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;

        for sample in samples {
            writer
                .write_sample(*sample)
                .map_err(|e| format!("Failed to write WAV sample: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

        Ok(())
    }

    /// Finalize the session
    pub async fn finalize(&self) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        {
            let mut session = self.current_session.lock();
            session.ended_at = Some(Local::now().to_rfc3339());
        }

        self.update_session_manifest().await?;

        // Create summary file
        let summary = self.generate_summary();
        let summary_path = self.session_dir.join("summary.txt");
        tokio::fs::write(&summary_path, summary)
            .await
            .map_err(|e| format!("Failed to write summary: {}", e))?;

        Ok(())
    }

    /// Generate session summary
    fn generate_summary(&self) -> String {
        let session = self.current_session.lock();

        // Parse the timestamps using proper conversion
        let start_time = DateTime::parse_from_rfc3339(&session.started_at)
            .map(|dt| dt.with_timezone(&Local))
            .unwrap_or_else(|_| Local::now());

        let end_time = session
            .ended_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Local))
            .unwrap_or_else(|| Local::now());

        let duration = end_time.signed_duration_since(start_time);

        let total_words: usize = session
            .utterances
            .iter()
            .map(|u| u.text.split_whitespace().count())
            .sum();

        format!(
            "Session Summary\n\
             ===============\n\
             Session ID: {}\n\
             Started: {}\n\
             Duration: {} minutes\n\
             Utterances: {}\n\
             Total Words: {}\n\
             Device: {}\n\
             Model: {}\n",
            session.session_id,
            session.started_at,
            duration.num_minutes(),
            session.utterances.len(),
            total_words,
            session.metadata.device_name,
            session.metadata.stt_model,
        )
    }

    /// Clean up old files based on retention policy
    pub async fn cleanup_old_files(&self) -> Result<(), String> {
        if self.config.retention_days == 0 {
            return Ok(());
        }

        let cutoff = Local::now() - chrono::Duration::days(self.config.retention_days as i64);
        let output_dir = self.config.output_dir.clone();

        // Use spawn_blocking for file system operations
        tokio::task::spawn_blocking(move || {
            let entries = std::fs::read_dir(&output_dir)
                .map_err(|e| format!("Failed to read output directory: {}", e))?;

            for entry in entries {
                let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
                let path = entry.path();

                if path.is_dir() {
                    // Parse date from directory name (YYYY-MM-DD format)
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if let Ok(date) = chrono::NaiveDate::parse_from_str(dir_name, "%Y-%m-%d") {
                            let datetime = date
                                .and_hms_opt(0, 0, 0)
                                .and_then(|dt| Local.from_local_datetime(&dt).single());

                            if let Some(dt) = datetime {
                                if dt < cutoff {
                                    tracing::info!(
                                        "Removing old transcription directory: {:?}",
                                        path
                                    );
                                    std::fs::remove_dir_all(&path).map_err(|e| {
                                        format!("Failed to remove old directory: {}", e)
                                    })?;
                                }
                            }
                        }
                    }
                }
            }

            Ok(())
        })
        .await
        .map_err(|e| format!("Cleanup task panicked: {}", e))?
    }
}

/// Spawn the persistence handler task
pub fn spawn_persistence_handler(
    config: PersistenceConfig,
    metadata: SessionMetadata,
    mut audio_rx: tokio::sync::broadcast::Receiver<AudioFrame>,
    mut vad_rx: tokio::sync::mpsc::Receiver<VadEvent>,
    mut transcript_rx: tokio::sync::mpsc::Receiver<TranscriptionEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let writer = match TranscriptionWriter::new(config, metadata) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Failed to create transcription writer: {}", e);
                return;
            }
        };

        tracing::info!("Transcription persistence handler started");
        // Apply retention policy at startup (best-effort)
        if let Err(e) = writer.cleanup_old_files().await {
            tracing::warn!("Retention cleanup failed: {}", e);
        }

        loop {
            tokio::select! {
                Ok(frame) = audio_rx.recv() => {
                    writer.handle_audio_frame(&frame);
                }
                Some(event) = vad_rx.recv() => {
                    writer.handle_vad_event(&event);
                }
                Some(event) = transcript_rx.recv() => {
                    if let Err(e) = writer.handle_transcription(&event).await {
                        tracing::error!("Failed to persist transcription: {}", e);
                    }
                }
                else => {
                    tracing::info!("Persistence handler shutting down");
                    break;
                }
            }
        }

        // Finalize session
        if let Err(e) = writer.finalize().await {
            tracing::error!("Failed to finalize session: {}", e);
        }
    })
}
