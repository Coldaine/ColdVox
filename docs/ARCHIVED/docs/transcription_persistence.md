# Transcription Persistence Feature

## Overview

The transcription persistence feature saves speech transcriptions and optionally the corresponding audio to disk for later review and analysis. This is useful for:

- Reviewing what was transcribed during a session
- Training data collection for improving STT models
- Analytics on speech patterns and word usage
- Debugging VAD and STT performance
- Compliance and audit trails

## Usage

### Basic Usage

Enable transcription persistence with the `--save-transcriptions` flag (requires
the `vosk` feature and a valid model on disk):

```bash
cargo run --features vosk -- --save-transcriptions
```

### Save Audio with Transcriptions

To save the audio chunks alongside transcriptions:

```bash
cargo run --features vosk -- --save-transcriptions --save-audio
```

### Specify Output Directory

By default, files are saved to `transcriptions/`. To use a different directory:

```bash
cargo run --features vosk -- --save-transcriptions --output-dir /path/to/output
```

### Choose Output Format

Available formats: `json` (default), `csv`, `text`

```bash
# JSON format (default) - structured data with metadata
cargo run --features vosk -- --save-transcriptions --transcript-format json

# CSV format - tabular data for spreadsheets
cargo run --features vosk -- --save-transcriptions --transcript-format csv

# Text format - simple timestamped text
cargo run --features vosk -- --save-transcriptions --transcript-format text
```

## Directory Structure

When persistence is enabled, the following directory structure is created:

```text
transcriptions/
├── 2025-08-29/                    # Date directory
│   ├── 143025/                    # Session directory (HHMMSS)
│   │   ├── session.json           # Session metadata and all utterances
│   │   ├── summary.txt            # Human-readable session summary
│   │   ├── utterance_000001.json  # Individual utterance (JSON format)
│   │   ├── utterance_000002.json
│   │   ├── transcriptions.csv     # All utterances (CSV format)
│   │   └── audio/                 # Audio files (if --save-audio enabled)
│   │       ├── utterance_000001.wav
│   │       └── utterance_000002.wav
│   └── 151230/                    # Another session
└── 2025-08-30/                    # Next day's sessions
```

## File Formats

### Session Manifest (session.json)

Contains complete session information:

```json
{
  "session_id": "143025",
  "started_at": "2025-08-29T14:30:25.123456+00:00",
  "ended_at": "2025-08-29T14:45:30.789012+00:00",
  "utterances": [
    {
      "utterance_id": 1,
      "started_at": "2025-08-29T14:30:30.123456+00:00",
      "ended_at": "2025-08-29T14:30:35.789012+00:00",
      "duration_ms": 5665,
      "text": "Hello, this is a test transcription",
      "confidence": null,
      "audio_path": "audio/utterance_000001.wav",
      "words": null
    }
  ],
  "metadata": {
    "device_name": "USB Microphone",
    "sample_rate": 16000,
    "vad_mode": "Silero",
    "stt_model": "models/vosk-model-small-en-us-0.15",
    "app_version": "0.1.0"
  }
}
```

### CSV Format (transcriptions.csv)

```csv
utterance_id,timestamp,duration_ms,text,audio_path
1,2025-08-29T14:30:30+00:00,5665,"Hello, this is a test transcription",audio/utterance_000001.wav
2,2025-08-29T14:31:45+00:00,3250,"Another transcription example",audio/utterance_000002.wav
```

### Text Format (utterance_NNNNNN.txt)

```text
[2025-08-29T14:30:30+00:00] Hello, this is a test transcription
```

### Session Summary (summary.txt)

```
Session Summary
===============
Session ID: 143025
Started: 2025-08-29T14:30:25+00:00
Duration: 15 minutes
Utterances: 12
Total Words: 245
Device: USB Microphone
Model: models/vosk-model-small-en-us-0.15
```

## Configuration

The persistence system can be configured through the `PersistenceConfig` struct:

- `enabled`: Enable/disable persistence
- `output_dir`: Base directory for saving files
- `save_audio`: Save audio chunks alongside transcriptions
- `audio_format`: Format for audio files (currently WAV)
- `transcript_format`: Format for transcription files (Json/Csv/Text)
- `max_file_size_mb`: Maximum file size before rotation (future feature)
-  `retention_days`: Keep files for N days (0 = forever)

## Performance Considerations

-  Storage: Each session creates multiple files. With audio enabled, expect
  about 1 MB per minute of speech
- **I/O Operations**: Files are written asynchronously to avoid blocking the audio pipeline
- **Memory**: Audio buffers are cleared after each utterance to minimize memory usage
-  CPU: Minimal impact - file operations run in separate async tasks

## Privacy and Security

⚠️ **Important**: Transcriptions and audio files may contain sensitive information. Ensure:

1.  Output directory has appropriate permissions
2.  Implement retention policies for compliance
3.  Consider encryption for sensitive deployments
4.  Review transcriptions before sharing
5.  Audio files are only saved when explicitly enabled

## Examples

### Development/Testing

```bash
# Save everything for debugging
cargo run --features vosk -- \
  --save-transcriptions \
  --save-audio \
  --output-dir debug_sessions \
  --transcript-format json
```

### Production Use

```bash
# Save only transcriptions in CSV format
cargo run --features vosk -- \
  --save-transcriptions \
  --output-dir /var/log/coldvox \
  --transcript-format csv
```

### With Specific Device

```bash
# Use specific microphone and save transcriptions
cargo run --features vosk -- \
  -D "USB Microphone" \
  --save-transcriptions \
  --save-audio
```

## Troubleshooting

### No Files Created

-  Check that STT is enabled (Vosk model must be present)
-  Verify VAD is detecting speech
-  Check write permissions for output directory
-  Look for errors in logs

### Audio Files Missing

-  Ensure `--save-audio` flag is set
-  Check disk space availability
-  Verify audio is being captured correctly

### Incomplete Sessions

-  Sessions are finalized on shutdown
-  Use Ctrl+C for graceful shutdown
-  Check `session.json` for partial data

## Future Enhancements

-  [ ] Automatic file rotation based on size
-  [ ] Compression for audio files (MP3/Opus)
-  [ ] Subtitle formats (SRT/VTT)
-  [ ] Real-time streaming to external services
-  [ ] Database backend option
-  [ ] Encryption at rest
-  [ ] Web UI for browsing transcriptions
