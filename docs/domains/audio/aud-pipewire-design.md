---
doc_type: architecture
subsystem: audio
status: draft
freshness: stale
preservation: preserve
summary: PipeWire detection and ALSA fallback strategy
signals: ['pipewire', 'alsa', 'audio-routing']
domain_code: aud
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# PipeWire Audio System Integration Design

ColdVox integrates with Linux audio systems through a priority-based device selection strategy optimized for modern PipeWire setups while maintaining compatibility with legacy ALSA and PulseAudio configurations.

## PipeWire Detection and Compatibility

### Detection Strategy

The system performs runtime checks to determine the audio environment:

1. **PulseAudio/PipeWire Server Detection**
	- Executes `pactl info` to check for PulseAudio-compatible server
	- Searches for "pulseaudio" in output (case-insensitive)
	- PipeWire typically identifies as "PulseAudio (on PipeWire X.Y.Z)"

2. **ALSA Routing Verification**
	- Executes `aplay -L` to enumerate ALSA devices
	- Checks for "pulse" or "pipewire" routing in ALSA output
	- Verifies ALSA default device routes to PipeWire/Pulse

### Warning System

Non-fatal warnings are issued for suboptimal configurations:

- **No Pulse Server**: Install `pulseaudio` or `pipewire-pulse` for compatibility
- **ALSA Routing Missing**: Install `pipewire-alsa` for proper ALSA integration

## Device Selection Priority

### Priority Order

1. **ALSA "default"** - Desktop Environment aware shim (highest priority)
2. **"pipewire"** - Direct PipeWire device
3. **OS Default Input** - System-determined default
4. **Preferred Hardware** - Microphone-specific patterns
5. **Fallback** - Any available input device

### ALSA "default" Device

The ALSA "default" device is prioritized because:
- Respects desktop environment audio routing
- Automatically handles PipeWire session management
- Provides consistent behavior across DE configurations
- Handles permission and routing complexities transparently

### Hardware Preference Patterns

When no virtual devices are available, the system scores hardware devices:

- **"front:" prefix**: +3 points (ALSA hardware reference)
- **Microphone keywords**: +2 points ("HyperX", "QuadCast", "Microphone")
- **Blacklisted devices**: Excluded ("default", "sysdefault", "pipewire")

## Error Handling

### Graceful Degradation

- **Command failures**: Empty output treated as unavailable, warnings issued
- **Missing dependencies**: System continues with reduced functionality
- **Hardware detection**: Falls back to OS default if hardware scoring fails

### User-Specified Devices

When users specify a device name:
- **Exact match**: Preferred if available
- **Substring match**: Case-insensitive fallback with warning
- **No fallback**: Error returned rather than silent substitution

## Performance Considerations

### Setup Timing Budget

- Target: <100ms for audio setup check
- Warning issued if budget exceeded
- Helps identify performance regressions in audio stack

### Mock Testing

Environment variables enable deterministic testing:
- `MOCK_PACTL_OUTPUT`: Simulates `pactl info` output
- `MOCK_APLAY_OUTPUT`: Simulates `aplay -L` output

## Integration Points

### Platform Detection

Build-time detection (`build.rs`) identifies:
- Wayland vs X11 desktop sessions
- KDE/Plasma environments
- Available audio backends

### Audio Pipeline Integration

- Device changes trigger pipeline restart
- Resampling handles sample rate differences
- Ring buffer maintains lock-free audio flow

## Known Limitations

1. **Command Dependencies**: Requires `pactl` and `aplay` utilities
2. **Root Permissions**: Some ALSA devices may require elevated access
3. **Hot-Plug**: Device changes require manual restart
4. **Latency**: Priority on compatibility over minimal latency

## Future Considerations

- **Native PipeWire API**: Direct integration without ALSA/Pulse shims
- **Device Hot-Plug**: Automatic device change detection
- **Profile Management**: Multiple audio configuration profiles
- **Latency Optimization**: Real-time audio configuration options
