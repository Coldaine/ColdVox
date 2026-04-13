---
doc_type: reference
subsystem: foundation
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# Test Data Directory

Authoritative guidance was relocated from `crates/app/test_data/README.md` and is now maintained in this index.

This directory contains test WAV files for end-to-end pipeline testing.

## Test Files

To run the end-to-end tests, you need to provide a WAV file. You can either:

1. Set the `TEST_WAV` environment variable to point to a specific WAV file
2. Place WAV files in this directory with corresponding `.txt` transcript files

Example:
- `sample.wav` - Audio file
- `sample.txt` - Expected transcript (one word per line)

## Generating Test Data

You can generate a test WAV file using:
```bash
# Record 5 seconds of audio
arecord -f S16_LE -r 16000 -c 1 -d 5 test_data/sample.wav

# Or use text-to-speech
echo "This is a test" | espeak --stdout > test_data/sample.wav
```

## CI Testing

The CI pipeline will download or generate appropriate test files automatically.
