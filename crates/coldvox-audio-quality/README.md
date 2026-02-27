# coldvox-audio-quality

Real-time audio quality monitoring and analysis for voice applications.

## Features

Automated detection of common audio quality issues:

- **Too Quiet Audio**: RMS level below -40 dBFS (user too far from microphone)
- **Clipping Audio**: Peak level above -1 dBFS (input gain too high)
- **Off-Axis Speech**: Spectral ratio < 0.3 (user not facing microphone)

## Performance

- **Latency**: 12.8µs per 512-sample frame @ 16kHz (40x under 512µs budget)
- **Allocation**: Pre-allocated buffers for real-time safety
- **Overhead**: 1.6% of frame duration (32ms frame = 512µs budget)

## Usage

```rust
use coldvox_audio_quality::{AudioQualityMonitor, QualityConfig, QualityStatus};

// Create monitor with default configuration
let config = QualityConfig::default();
let mut monitor = AudioQualityMonitor::new(config);

// Analyze audio frames (512 samples @ 16kHz)
let samples: &[i16] = /* ... audio data ... */;
let status = monitor.analyze(samples);

match status {
    QualityStatus::Good { rms_dbfs, peak_dbfs } => {
        println!("Audio quality good: RMS={:.1} dBFS, Peak={:.1} dBFS",
                 rms_dbfs, peak_dbfs);
    }
    QualityStatus::Warning(warning) => {
        println!("Quality warning: {:?}", warning);
    }
}
```

### Custom Configuration

```rust
use coldvox_audio_quality::QualityConfig;

// HyperX QuadCast preset (cardioid condenser)
let config = QualityConfig::hyperx_quadcast();

// Or custom thresholds
let config = QualityConfig::builder()
    .too_quiet_threshold(-35.0)    // More strict (default: -40.0)
    .clipping_threshold(-3.0)       // More lenient (default: -1.0)
    .off_axis_threshold(0.4)        // More lenient (default: 0.3)
    .build();
```

### Environment Variables

Override thresholds at runtime:

```bash
export COLDVOX_TOO_QUIET_THRESHOLD=-35.0
export COLDVOX_CLIPPING_THRESHOLD=-3.0
export COLDVOX_OFF_AXIS_THRESHOLD=0.4
```

## Testing

### Quick Start (No Setup Required)

Basic integration tests use committed samples (~320KB):

```bash
cargo test
```

### Comprehensive Testing (Real Audio)

1. Download external datasets:

```bash
cd ../../  # Go to project root
./scripts/download_test_audio.sh
```

This downloads:
- **LibriSpeech test-clean** (346MB) - Professional baseline recordings
- **Pyramic anechoic** (~1-2GB) - Off-axis validation at 0°, 90°, 180°
- **DAPS** (~4GB, optional) - Consumer device recordings

2. Run full test suite:

```bash
cargo test --test integration_test
```

### Test Coverage

**Unit tests** (synthetic audio):
- ✅ RMS calculation accuracy
- ✅ Peak detection with decay
- ✅ Spectral ratio calculation
- ✅ Threshold boundary conditions
- ✅ Rate limiting (2-second cooldown)

**Integration tests** (committed samples):
- ✅ Real audio baseline
- ✅ No panics on various samples
- ✅ Synthetic clipping detection
- ✅ Synthetic quiet audio detection

**Integration tests** (external datasets):
- ✅ LibriSpeech baseline (professional quality)
- ✅ Pyramic off-axis detection (0° vs 90° vs 180°)
- ✅ Spectral ratio comparison
- ✅ Custom threshold configuration

### Benchmarks

```bash
cargo bench
```

Results (AMD Ryzen/similar):
- `frame_budget_512_samples`: 12.8µs (40x under 512µs budget)

## Architecture

### Components

- **LevelMonitor**: RMS and peak detection with rolling windows
  - RMS: 500ms rolling average
  - Peak: 1-second hold with exponential decay
  - dBFS conversion: `20 * log10(amplitude)`

- **SpectralAnalyzer**: FFT-based off-axis detection
  - 2048-point FFT with Hann window
  - High-freq band: 4-8 kHz
  - Mid-freq band: 500 Hz - 2 kHz
  - Ratio < 0.3 indicates off-axis (15-20 dB high-freq rolloff)

- **AudioQualityMonitor**: Combined analysis with rate limiting
  - Processes 512-sample frames (32ms @ 16kHz)
  - 2-second warning cooldown
  - Pre-allocated buffers (no runtime allocation)

### Design Principles

✅ **Real-time safe**: Pre-allocated buffers, no allocations in hot path
✅ **Low latency**: 12.8µs processing time (1.6% overhead)
✅ **Physics-based**: Cardioid polar pattern modeling for off-axis detection
✅ **Configurable**: Environment variables and builder pattern
✅ **Well-tested**: 30+ unit tests, real audio validation

### Known Limitations

⚠️ **FrequencySpectrum allocation**: `spectrum-analyzer` crate allocates on each FFT
⚠️ **Synthetic tests**: Unit tests use synthetic sine waves (integration tests use real audio)
⚠️ **No hardware validation**: Not yet tested with HyperX QuadCast or similar hardware

See [Phase 1 Implementation Plan](../../docs/implementation-plans/phase1-audio-quality-monitoring.md) for details.

## Phase 1 Status

✅ **Level monitoring** (RMS, peak, dBFS)
✅ **Spectral analysis** (FFT, off-axis detection)
✅ **Configuration system** (builder, env vars, presets)
✅ **Unit tests** (30+ tests, all passing)
✅ **Benchmarks** (12.8µs per frame)
✅ **Integration tests** (real audio validation)
⏳ **Phase 2**: Pipeline integration, GUI feedback, hardware validation

## Contributing

When adding tests:
1. Use committed samples (test_data/*.wav) for basic tests
2. Use external datasets for comprehensive validation
3. Document expected behavior and edge cases
4. Verify performance impact with benchmarks

## License

MIT OR Apache-2.0
