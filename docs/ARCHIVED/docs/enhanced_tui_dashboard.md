# Enhanced TUI Dashboard

## Overview

The enhanced TUI dashboard provides real-time monitoring of the ColdVox audio pipeline with:

-  Live microphone level indicators (peak and RMS)
-  Pipeline stage tracking showing data flow progress
-  Buffer fill indicators
-  Frame rate monitoring
-  VAD activity detection
-  STT integration (planned): surface partial/final counts and latency

## Features

### 1. Audio Level Monitoring

-  Real-time level meter: Shows current audio level in dB (-90 to 0 dB scale)
-  Peak detection: Tracks peak sample values
-  RMS calculation: Root Mean Square for average loudness
-  History sparkline: 60-sample rolling window visualization
-  Color coding: Green (safe), Yellow (warning), Red (clipping)

### 2. Pipeline Stage Indicators

The dashboard tracks audio data flow through 4 stages:

1.  Capture Stage: Raw audio from microphone
2.  Chunker Stage: Conversion to 512-sample chunks
3.  VAD Stage: Voice Activity Detection processing
4.  Output Stage: Final VAD events

Each stage shows:

-  Activity indicator (● active, ○ inactive)
-  Frame counter
-  Visual pulse when data passes through

### 3. Metrics Tracking

-  Frame rates: FPS for each pipeline stage
-  Buffer utilization: Fill percentage for ring buffers
-  Latency monitoring: Inter-stage timing
-  Error counting: Track failures per stage

### 4. VAD Monitoring

-  Speech detection status (Speaking: YES/NO)
-  Speech segment counter
-  Last VAD event details
-  Energy levels in dB

## Architecture

### Pipeline Metrics Module (`telemetry/pipeline_metrics.rs`)

Provides thread-safe atomic metrics shared across pipeline components:

-  Audio level calculations (peak, RMS, dB)
-  Stage activity tracking
-  Buffer monitoring
-  Frame rate calculations
-  Speech activity indicators

### Data Flow Visualization

```text
Microphone → [Capture] → [Chunker] → [VAD] → [Output]
              ↓           ↓           ↓        ↓
           Metrics    Metrics    Metrics   Events
              ↓           ↓           ↓        ↓
         ╔════════════════════════════════════╗
         ║     Enhanced TUI Dashboard         ║
         ╚════════════════════════════════════╝
```

## Running the Dashboard

```bash
# Basic usage
cargo run --bin enhanced_tui_dashboard

# With specific device
cargo run --bin enhanced_tui_dashboard -- -D "USB Microphone"
```

## Controls

-  S: Start audio pipeline
-  R: Reset metrics
-  Q: Quit

## Technical Details

### Audio Processing

-  Internal format: 16kHz, 16-bit mono
-  Chunk size: 512 samples (32ms)
-  Buffer size: 100 frames (~2 seconds)
-  Update rate: 50ms (20 FPS)

### Performance Monitoring

The dashboard tracks:

-  Capture FPS: Frames captured per second
-  Chunker FPS: Chunks processed per second
-  VAD FPS: VAD frames processed per second
-  (Planned) STT metrics: partial/final rates and processing latency
-  End-to-end latency: Total pipeline delay

### Thread Architecture

-  Main thread: TUI rendering and event handling
-  Audio thread: Capture and processing pipeline
-  Monitor threads: Metrics collection from each stage
-  VAD thread: Voice activity detection processing

## How Far Does Sound Data Reach?

The dashboard confirms pipeline depth through:

1.  Stage Activity Indicators: Visual confirmation when data reaches each stage
2.  Frame Counters: Incremental counts showing data flow
3.  Buffer Fill Levels: Shows if data is queuing or flowing
4.  VAD Events: Confirms complete pipeline traversal when speech is detected

If data stops at a specific stage, you'll see:

-  The activity indicator stops pulsing at that stage
-  Frame counter stops incrementing
-  Buffer fill may increase (backpressure) or decrease (starvation)
-  Error counts may increment for that stage

## Future Enhancements

Potential additions:

-  Spectrogram visualization
-  Multi-channel support
-  Network streaming indicators
-  STT integration status
-  Waveform display
-  Recording indicators
-  Configuration hot-reload status
