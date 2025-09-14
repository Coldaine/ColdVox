//! eSpeak TTS engine implementation for ColdVox

use std::collections::HashMap;
use coldvox_tts::{TtsEngine, TtsConfig, TtsResult, TtsError, VoiceInfo, VoiceGender, SynthesisEvent, SynthesisOptions, next_synthesis_id};
use async_trait::async_trait;
use tokio::process::Command;
use tracing::{debug, warn, error};
use regex::Regex;

mod tests;

pub struct EspeakEngine {
    config: TtsConfig,
    current_voice: Option<String>,
    available_voices: Vec<VoiceInfo>,
    is_initialized: bool,
}

impl EspeakEngine {
    pub fn new() -> Self {
        Self {
            config: TtsConfig::default(),
            current_voice: None,
            available_voices: Vec::new(),
            is_initialized: false,
        }
    }
    
    /// Check if espeak command is available
    async fn check_espeak_available() -> bool {
        match Command::new("espeak")
            .arg("--version")
            .output()
            .await
        {
            Ok(_) => true,
            Err(_) => {
                // Try espeak-ng as alternative
                match Command::new("espeak-ng")
                    .arg("--version")
                    .output()
                    .await
                {
                    Ok(_) => true,
                    Err(_) => false,
                }
            }
        }
    }
    
    /// Get the espeak command name (espeak or espeak-ng)
    async fn get_espeak_command() -> Option<String> {
        if Command::new("espeak").arg("--version").output().await.is_ok() {
            Some("espeak".to_string())
        } else if Command::new("espeak-ng").arg("--version").output().await.is_ok() {
            Some("espeak-ng".to_string())
        } else {
            None
        }
    }
    
    /// Parse espeak voice list output
    async fn parse_voice_list(&self, output: String) -> Vec<VoiceInfo> {
        let mut voices = Vec::new();
        
        // espeak voice list format: Pty Language Age/Gender VoiceName File Other
        // Example: 5  en             M  en                 (en 2)
        let voice_regex = Regex::new(r"^\s*(\d+)\s+([\w-]+)\s+([MF\+]?)\s+([\w\-_]+)\s+").unwrap();
        
        for line in output.lines().skip(1) { // Skip header
            if let Some(captures) = voice_regex.captures(line) {
                let language = captures.get(2).map_or("unknown", |m| m.as_str()).to_string();
                let gender_char = captures.get(3).map_or("", |m| m.as_str());
                let voice_id = captures.get(4).map_or("unknown", |m| m.as_str()).to_string();
                
                let gender = match gender_char {
                    "M" => Some(VoiceGender::Male),
                    "F" => Some(VoiceGender::Female),
                    _ => Some(VoiceGender::Unknown),
                };
                
                let voice_info = VoiceInfo {
                    id: voice_id.clone(),
                    name: format!("{} ({})", language, voice_id),
                    language,
                    gender,
                    age: None, // espeak doesn't provide age information consistently
                    properties: HashMap::new(),
                };
                
                voices.push(voice_info);
            }
        }
        
        voices
    }
    
    /// Build espeak command arguments
    fn build_espeak_args(&self, text: &str, options: Option<&SynthesisOptions>) -> Vec<String> {
        let mut args = vec!["--stdout".to_string()];
        
        // Set voice
        let voice = options.and_then(|o| o.voice.as_ref())
            .or(self.current_voice.as_ref())
            .or(self.config.default_voice.as_ref());
            
        if let Some(voice_id) = voice {
            args.push("-v".to_string());
            args.push(voice_id.clone());
        }
        
        // Set speech rate
        let rate = options.and_then(|o| o.speech_rate)
            .or(self.config.speech_rate)
            .unwrap_or(180);
        args.push("-s".to_string());
        args.push(rate.to_string());
        
        // Set pitch
        let pitch = options.and_then(|o| o.pitch)
            .or(self.config.pitch)
            .unwrap_or(1.0);
        let pitch_value = ((pitch * 50.0) as u32).max(0).min(100);
        args.push("-p".to_string());
        args.push(pitch_value.to_string());
        
        // Set volume
        let volume = options.and_then(|o| o.volume)
            .or(self.config.volume)
            .unwrap_or(0.8);
        let volume_value = ((volume * 200.0) as u32).max(0).min(200);
        args.push("-a".to_string());
        args.push(volume_value.to_string());
        
        // Add text
        args.push(text.to_string());
        
        args
    }
}

#[async_trait]
impl TtsEngine for EspeakEngine {
    fn name(&self) -> &str {
        "eSpeak"
    }
    
    fn version(&self) -> &str {
        "1.0.0" // This implementation version
    }
    
    async fn initialize(&mut self, config: TtsConfig) -> TtsResult<()> {
        if !Self::check_espeak_available().await {
            return Err(TtsError::EngineNotAvailable(
                "eSpeak not found. Please install espeak or espeak-ng.".to_string()
            ));
        }
        
        self.config = config;
        
        // Load available voices
        if let Some(cmd) = Self::get_espeak_command().await {
            match Command::new(&cmd)
                .arg("--voices")
                .output()
                .await
            {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    self.available_voices = self.parse_voice_list(output_str.to_string()).await;
                    debug!("Loaded {} espeak voices", self.available_voices.len());
                }
                Err(e) => {
                    warn!("Failed to load espeak voices: {}", e);
                    return Err(TtsError::InitializationError(format!("Failed to load voices: {}", e)));
                }
            }
        }
        
        self.is_initialized = true;
        Ok(())
    }
    
    async fn is_available(&self) -> bool {
        Self::check_espeak_available().await
    }
    
    async fn synthesize(&mut self, text: &str, options: Option<SynthesisOptions>) -> TtsResult<SynthesisEvent> {
        if !self.is_initialized {
            return Err(TtsError::InitializationError("Engine not initialized".to_string()));
        }
        
        if text.trim().is_empty() {
            return Err(TtsError::InvalidInput("Empty text input".to_string()));
        }
        
        let synthesis_id = next_synthesis_id();
        let _voice_id = options.as_ref()
            .and_then(|o| o.voice.as_ref())
            .or(self.current_voice.as_ref())
            .or(self.config.default_voice.as_ref())
            .map(|s| s.clone())
            .unwrap_or_else(|| "default".to_string());
        
        // Get espeak command
        let cmd = Self::get_espeak_command().await
            .ok_or_else(|| TtsError::EngineNotAvailable("eSpeak command not found".to_string()))?;
        
        // Build arguments
        let args = self.build_espeak_args(text, options.as_ref());
        
        debug!("Running espeak synthesis: {} {:?}", cmd, args);
        
        // Execute espeak
        match Command::new(&cmd)
            .args(&args)
            .output()
            .await
        {
            Ok(output) => {
                if output.status.success() {
                    // espeak outputs WAV data to stdout
                    let audio_data = output.stdout;
                    
                    if audio_data.is_empty() {
                        return Ok(SynthesisEvent::Failed {
                            synthesis_id,
                            error: "No audio data generated".to_string(),
                        });
                    }
                    
                    // Return audio data event
                    // espeak typically outputs 22050 Hz, 16-bit mono WAV
                    Ok(SynthesisEvent::AudioData {
                        synthesis_id,
                        data: audio_data,
                        sample_rate: 22050,
                        channels: 1,
                    })
                } else {
                    let error_msg = String::from_utf8_lossy(&output.stderr);
                    error!("eSpeak synthesis failed: {}", error_msg);
                    Ok(SynthesisEvent::Failed {
                        synthesis_id,
                        error: format!("eSpeak error: {}", error_msg),
                    })
                }
            }
            Err(e) => {
                error!("Failed to execute espeak: {}", e);
                Ok(SynthesisEvent::Failed {
                    synthesis_id,
                    error: format!("Process execution failed: {}", e),
                })
            }
        }
    }
    
    async fn list_voices(&self) -> TtsResult<Vec<VoiceInfo>> {
        if !self.is_initialized {
            return Err(TtsError::InitializationError("Engine not initialized".to_string()));
        }
        
        Ok(self.available_voices.clone())
    }
    
    async fn set_voice(&mut self, voice_id: &str) -> TtsResult<()> {
        // Verify voice exists
        if !self.available_voices.iter().any(|v| v.id == voice_id) {
            return Err(TtsError::VoiceNotFound(voice_id.to_string()));
        }
        
        self.current_voice = Some(voice_id.to_string());
        Ok(())
    }
    
    async fn stop_synthesis(&mut self) -> TtsResult<()> {
        // espeak doesn't support stopping mid-synthesis in this implementation
        // This would require more complex process management
        debug!("Stop synthesis requested (not implemented for espeak)");
        Ok(())
    }
    
    fn config(&self) -> &TtsConfig {
        &self.config
    }
    
    async fn shutdown(&mut self) -> TtsResult<()> {
        self.is_initialized = false;
        self.current_voice = None;
        self.available_voices.clear();
        debug!("eSpeak engine shutdown");
        Ok(())
    }
}