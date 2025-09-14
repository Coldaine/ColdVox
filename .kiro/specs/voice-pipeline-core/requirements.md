# Requirements Document

## Introduction

ColdVox is a modular Rust-based voice-to-text application that provides real-time audio capture, voice activity detection (VAD), speech-to-text (STT) transcription, and cross-platform text injection. The system implements a complete voice pipeline that captures audio from microphones, detects when speech is occurring, transcribes the speech to text, and can automatically inject the transcribed text into active applications.

## Requirements

### Requirement 1

**User Story:** As a user, I want the system to capture audio from my microphone in real-time, so that I can provide voice input for transcription.

#### Acceptance Criteria

1. WHEN the application starts THEN the system SHALL initialize audio capture from the default or specified microphone device
2. WHEN audio capture is active THEN the system SHALL continuously capture audio at 16 kHz, 16-bit mono format
3. WHEN multiple audio devices are available THEN the system SHALL allow device selection via command-line arguments or environment variables
4. WHEN the specified audio device is unavailable THEN the system SHALL fall back to the default device and log the fallback
5. WHEN audio capture encounters errors THEN the system SHALL attempt automatic recovery with watchdog monitoring
6. WHEN no audio data is received for 5 seconds THEN the system SHALL trigger watchdog recovery and restart the audio stream

### Requirement 2

**User Story:** As a user, I want the system to detect when I'm speaking versus when there's silence, so that transcription only occurs during actual speech.

#### Acceptance Criteria

1. WHEN audio frames are received THEN the system SHALL process them through voice activity detection (VAD)
2. WHEN speech is detected THEN the system SHALL emit a SpeechStart event
3. WHEN speech ends THEN the system SHALL emit a SpeechEnd event after minimum silence duration
4. WHEN using Silero VAD THEN the system SHALL use a default threshold of 0.3 for speech detection
5. WHEN using energy-based VAD THEN the system SHALL fall back to Level3 energy detection as a backup option
6. WHEN speech duration is less than 250ms THEN the system SHALL ignore the speech event as too short
7. WHEN silence duration is less than 100ms THEN the system SHALL not trigger speech end event

### Requirement 3

**User Story:** As a user, I want my speech to be transcribed to text accurately, so that I can convert voice input to written text.

#### Acceptance Criteria

1. WHEN a SpeechStart event occurs THEN the system SHALL begin audio buffering for transcription
2. WHEN a SpeechEnd event occurs THEN the system SHALL send the buffered audio to the STT engine
3. WHEN using Vosk STT THEN the system SHALL load the specified model from the configured path
4. WHEN transcription is in progress THEN the system SHALL emit partial transcription events for real-time feedback
5. WHEN transcription is complete THEN the system SHALL emit a final transcription event with the complete text
6. WHEN STT processing fails THEN the system SHALL emit an error event and attempt fallback to alternative STT plugins
7. WHEN multiple STT plugins are configured THEN the system SHALL support failover between plugins based on error thresholds

### Requirement 4

**User Story:** As a user, I want the transcribed text to be automatically injected into my active application, so that I can use voice input seamlessly in any text field.

#### Acceptance Criteria

1. WHEN a final transcription event is received THEN the system SHALL inject the text into the currently focused application
2. WHEN running on Linux with Wayland THEN the system SHALL use AT-SPI or clipboard-based injection methods
3. WHEN running on Linux with X11 THEN the system SHALL support kdotool or ydotool injection methods
4. WHEN primary injection method fails THEN the system SHALL attempt fallback injection strategies
5. WHEN text injection is disabled THEN the system SHALL only log transcription results without injection
6. WHEN injection encounters permission errors THEN the system SHALL log the error and continue operation
7. WHEN clipboard restoration is enabled THEN the system SHALL restore the original clipboard contents after injection

### Requirement 5

**User Story:** As a user, I want to control when the voice pipeline is active, so that I can choose between continuous listening and manual activation.

#### Acceptance Criteria

1. WHEN activation mode is set to "vad" THEN the system SHALL continuously listen and process speech based on VAD events
2. WHEN activation mode is set to "hotkey" THEN the system SHALL only process speech when the hotkey is pressed
3. WHEN hotkey activation is used THEN the system SHALL support global hotkey registration across desktop environments
4. WHEN running on KDE THEN the system SHALL use KGlobalAccel for hotkey handling
5. WHEN hotkey is pressed THEN the system SHALL activate the voice pipeline and provide visual feedback
6. WHEN hotkey is released THEN the system SHALL deactivate the voice pipeline and process any captured speech
7. WHEN activation mode changes THEN the system SHALL reconfigure the pipeline accordingly without restart

### Requirement 6

**User Story:** As a user, I want to monitor the system's performance and health, so that I can ensure optimal operation and troubleshoot issues.

#### Acceptance Criteria

1. WHEN the application is running THEN the system SHALL log pipeline metrics every 30 seconds
2. WHEN metrics are logged THEN the system SHALL include capture FPS, chunker FPS, VAD FPS, and buffer fill percentages
3. WHEN health monitoring is active THEN the system SHALL check system health every 10 seconds
4. WHEN errors occur THEN the system SHALL log detailed error information with appropriate severity levels
5. WHEN TUI mode is enabled THEN the system SHALL provide a real-time dashboard with pipeline status
6. WHEN file logging is active THEN the system SHALL write logs to daily-rotated files in the logs directory
7. WHEN debug mode is enabled THEN the system SHALL provide detailed event dumping for troubleshooting

### Requirement 7

**User Story:** As a user, I want the system to handle configuration flexibly, so that I can customize behavior for my specific environment and needs.

#### Acceptance Criteria

1. WHEN the application starts THEN the system SHALL accept configuration via command-line arguments and environment variables
2. WHEN device selection is needed THEN the system SHALL support exact device name matching or substring matching
3. WHEN resampler quality is specified THEN the system SHALL support Fast, Balanced, and Quality modes
4. WHEN STT plugin configuration is provided THEN the system SHALL support plugin selection, fallbacks, and memory limits
5. WHEN text injection options are specified THEN the system SHALL configure injection backends and timeouts accordingly
6. WHEN logging configuration is provided THEN the system SHALL respect RUST_LOG environment variable settings
7. WHEN configuration validation fails THEN the system SHALL provide clear error messages and exit gracefully

### Requirement 8

**User Story:** As a user, I want the system to gracefully handle shutdown and cleanup, so that resources are properly released and no data is lost.

#### Acceptance Criteria

1. WHEN a shutdown signal is received THEN the system SHALL begin graceful shutdown process
2. WHEN shutdown begins THEN the system SHALL transition through defined application states (Running -> Stopping -> Stopped)
3. WHEN audio capture is active during shutdown THEN the system SHALL properly close audio streams and release device handles
4. WHEN STT processing is active during shutdown THEN the system SHALL complete or cancel pending transcriptions
5. WHEN file handles are open during shutdown THEN the system SHALL flush and close all log files
6. WHEN background tasks are running during shutdown THEN the system SHALL wait for task completion or timeout
7. WHEN shutdown is complete THEN the system SHALL log successful shutdown and exit with appropriate status code
