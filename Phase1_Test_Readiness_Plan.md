# Phase 1 Test Readiness Confirmation Plan

## Executive Summary
Based on comprehensive analysis of the ColdVox codebase, **the system is ready for Phase 1 testing**. All required components are implemented and functional according to the specifications.

## Test Readiness Matrix

| Test Category | Status | Implementation Notes |
|---------------|--------|---------------------|
| **Default Capture (PipeWire)** | ✅ Ready | DeviceManager prioritizes "pipewire" device |
| **Explicit Device Capture** | ✅ Ready | Supports `--device "sysdefault:CARD=QuadCast"` |
| **Format Negotiation** | ✅ Ready | All 5 formats supported (i16, f32, u16, u8, i8) |
| **Channel Negotiation** | ✅ Ready | Stereo downmix implemented |
| **Silence Detection** | ✅ Ready | Configurable threshold, 3s warning |
| **Watchdog Timeout** | ✅ Ready | 5-second timeout with recovery |
| **Disconnect/Reconnect** | ✅ Ready | 3-attempt recovery mechanism |
| **Buffer Overflow** | ✅ Ready | Frame dropping with warnings |
| **Clean Shutdown** | ✅ Ready | Ctrl+C handling implemented |
| **Health Monitor** | ✅ Ready | Starts at boot, no registered checks yet |

## Detailed Test Procedures

### 1. Default Capture via System Mic (PipeWire)
**Command**: `cargo run --bin mic_probe -- --duration 10`

**Expected Output**:
```
Available audio devices:
  [DEFAULT] pipewire
Opening audio device: pipewire
Audio config: StreamConfig { ... }, format: I16
Stats: X frames, Y active, Z silent, 0 dropped, 0 disconnects, 0 reconnects
```

**Success Criteria**:
- Shows "pipewire" as default device
- Opens successfully with negotiated format
- Active frames > 0

### 2. Explicit Device Capture (HyperX/QuadCast)
**Command**: `cargo run --bin mic_probe -- --device "sysdefault:CARD=QuadCast" --duration 10`

**Expected Output**:
```
Opening audio device: sysdefault:CARD=QuadCast
Audio config: StreamConfig { ... }, format: I16
Stats: X frames, Y active, Z silent, 0 dropped, 0 disconnects, 0 reconnects
```

**Success Criteria**:
- Opens specified device exactly
- Stats update regularly
- Active frames > 0

### 3. Format Negotiation Coverage
**Test Matrix**:
| Format | Command | Expected |
|--------|---------|----------|
| i16 | Default | Should work |
| f32 | `--device` with f32 device | Should convert to i16 |
| u16 | `--device` with u16 device | Should convert to i16 |
| u8 | `--device` with u8 device | Should convert to i16 |
| i8 | `--device` with i8 device | Should convert to i16 |

**Verification**: Check logs for "Audio config: ... format: [Format]"

### 4. Channel Negotiation (Stereo Devices)
**Test**: Use stereo microphone
**Expected**: Automatic downmix to mono with log showing channel count

### 5. Silence Detection Behavior
**Commands**:
```bash
# Low threshold (very sensitive)
cargo run --bin mic_probe -- --silence-threshold 50 --duration 30

# High threshold (less sensitive)  
cargo run --bin mic_probe -- --silence-threshold 500 --duration 30
```

**Expected**: 
- Silent vs active frame counts vary with threshold
- >3s silence triggers "Extended silence detected" warning

### 6. Watchdog No-Data Timeout
**Manual Test**:
1. Start: `cargo run --bin mic_probe -- --duration 60`
2. After 5-10 seconds, mute/disable microphone
3. **Expected**: "Watchdog timeout! No audio data for 5s" after ~5 seconds

### 7. Disconnect/Reconnect Recovery
**Command**: `cargo run --bin mic_probe -- --expect_disconnect --duration 60`

**Manual Steps**:
1. Start with microphone connected
2. Unplug microphone (should see disconnect)
3. Plug back in (should see recovery within 3 attempts)

**Expected Output**:
```
Device disconnected, attempting recovery...
Recovery attempt 1/3
Recovery successful!
```

### 8. Buffer Overflow Behavior
**Test Setup**: Temporarily stop frame consumption
**Expected**: "Audio buffer full, dropping frame" warnings and `frames_dropped > 0`

### 9. Clean Shutdown
**Test**: Run any test and press Ctrl+C
**Expected**: Graceful shutdown with "Shutdown requested" and "Test completed successfully"

### 