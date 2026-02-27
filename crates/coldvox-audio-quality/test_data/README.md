# Audio Quality Test Data

This directory contains small audio samples (~320KB total) committed to the repository for basic integration testing.

## Committed Samples (In Repo)

- `test_1.wav` (126KB) - Real speech sample for baseline testing
- `test_3.wav` (70KB) - Real speech sample for baseline testing
- `test_5.wav` (124KB) - Real speech sample for baseline testing

These samples are used for:
- Quick integration tests that run without external downloads
- CI/CD pipeline validation
- Basic sanity checks

## External Datasets (Downloaded On-Demand)

For comprehensive testing, download larger datasets using:

```bash
./scripts/download_test_audio.sh
```

This downloads to `test_audio/` (1.5GB-5.5GB):

### LibriSpeech test-clean (346MB)
- **Source**: https://www.openslr.org/12
- **Purpose**: Baseline professional recordings (should produce zero warnings)
- **Location**: `test_audio/baseline/LibriSpeech/`

### Pyramic Anechoic Dataset (~1-2GB)
- **Source**: https://zenodo.org/records/1209563
- **Purpose**: Off-axis speech detection (recordings at 0°, 90°, 180°)
- **Location**: `test_audio/off_axis/`

### DAPS Dataset (~4GB, optional)
- **Source**: https://zenodo.org/records/4660670
- **Purpose**: Too quiet audio (consumer device recordings)
- **Location**: `test_audio/quiet/daps/`
- **Note**: Download with `--with-daps` flag

## Test Strategy

**Unit tests**: Use synthetic audio (fast, deterministic)
**Integration tests (basic)**: Use committed samples (320KB, always available)
**Integration tests (comprehensive)**: Use external datasets (downloaded on-demand)

## Storage Strategy: Hybrid

✅ **Small samples committed** (~320KB) - Basic tests work out-of-box
✅ **Large datasets external** (1.5-5.5GB) - Opt-in comprehensive validation
✅ **Minimal repo bloat** - Only 320KB impact on git clone
