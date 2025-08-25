# Dual-Mode Audio Processing Implementation Plan

## Executive Summary

This document outlines the implementation of a dual-mode audio processing system for ColdVox, supporting both Voice Activity Detection (VAD) mode and Push-to-Talk (PTT) mode. Users can toggle between modes using keyboard shortcuts, with VAD mode providing automatic speech detection and PTT mode offering manual control via a held key combination.

## Overview

### Mode 1: VAD Mode (Existing)
- **Activation**: Toggle with Shift+F2
- **Behavior**: Automatic voice activity detection determines audio chunking
- **Pipeline**: Mic → Ring Buffer → VAD Processor → Smart Chunker → STT Queue
- **Use Case**: Hands-free operation, continuous conversation

### Mode 2: Push-to-Talk Mode (New)
- **Activation**: Hold Ctrl+Super (on Fedora/Wayland)
- **Behavior**: Records audio while key is held, sends to STT on release
- **Visual Feedback**: On-screen blinking indicator during recording
- **Pipeline**: Mic → Accumulator Buffer → Direct STT Submission
- **Use Case**: Precise control, noisy environments, privacy-conscious usage

## Technical Architecture

### System Components

```
┌─────────────────────────────────────────────────────────┐
│                    Main Application                      │
├─────────────────────────────────────────────────────────┤
│                    Mode Controller                       │
│                         │                                │
│         ┌───────────────┴───────────────┐               │
│         ▼                               ▼               │
│    VAD Mode                        PTT Mode             │
│         │                               │               │
├─────────┼───────────────────────────────┼───────────────┤
│         ▼                               ▼               │
│  VAD Processor                  PTT Processor           │
│         │                               │               │
├─────────┼───────────────────────────────┼───────────────┤
│         └───────────┬───────────────────┘               │
│                     ▼                                    │
│              Mic Capture Thread                          │
│                     │                                    │
│                Ring Buffer                               │
│                     │                                    │
│              STT Integration                             │
└─────────────────────────────────────────────────────────┘

Parallel Components:
- Hotkey Manager (Global shortcut monitoring)
- Overlay Indicator (Visual feedback system)
```

### State Management

```rust
enum AudioMode {
    Vad,
    PushToTalk,
}

enum PttState {
    Idle,
    Recording { 
        start_time: Instant,
        buffer: Vec<i16>,
    },
    Processing,
}

struct ModeController {
    current_mode: AudioMode,
    vad_processor: VadProcessor,
    ptt_processor: PttProcessor,
    hotkey_manager: HotkeyManager,
    overlay: OverlayIndicator,
}
```

## Implementation Phases

### Phase 1: Mode Infrastructure (Week 1)
- [ ] Add `AudioMode` enum to application state
- [ ] Create `ModeController` component
- [ ] Implement mode switching state machine
- [ ] Add configuration structure for modes
- [ ] Create mock mode transitions via CLI commands
- [ ] Unit tests for mode switching logic

### Phase 2: Push-to-Talk Audio Pipeline (Week 1-2)
- [ ] Implement `AccumulatorBuffer` for PTT recording
- [ ] Create `PttProcessor` component
- [ ] Add direct STT submission path
- [ ] Implement maximum recording time limit (60 seconds)
- [ ] Handle buffer overflow scenarios
- [ ] Integration tests with mock STT

### Phase 3: Global Hotkey Support (Week 2-3)
- [ ] Evaluate hotkey libraries:
  - `rdev`: Cross-platform, no root required
  - `evdev-rs`: Direct evdev access, requires input group
  - `device_query`: Simple API, limited Wayland support
- [ ] Implement `HotkeyManager` component
- [ ] Handle Wayland security model:
  - Add user to `input` group if needed
  - Document permission requirements
  - Provide fallback options
- [ ] Create hotkey configuration system
- [ ] Test on GNOME, KDE, and Sway compositors

### Phase 4: Visual Indicator (Week 3)
- [ ] Design indicator UI:
  - Corner dot with pulsing animation
  - Color states: Ready (green), Recording (red), Processing (yellow)
  - Semi-transparent overlay
- [ ] Technology selection:
  - **Option A**: `egui` + `winit` (simple, cross-platform)
  - **Option B**: `gtk4-layer-shell` (native Wayland)
  - **Option C**: System tray indicator (fallback)
- [ ] Implement overlay window
- [ ] Add animation system
- [ ] Handle compositor-specific quirks

### Phase 5: Integration & Polish (Week 4)
- [ ] Connect all components
- [ ] Add TOML/YAML configuration file support
- [ ] Implement comprehensive error recovery
- [ ] Performance optimization:
  - Minimize latency on key release
  - Optimize buffer allocation
  - Reduce CPU usage in idle state
- [ ] Create user documentation
- [ ] System integration tests

## Technical Considerations

### Wayland Global Hotkeys

Wayland's security model makes global hotkeys challenging. Implementation strategy:

1. **Primary Approach**: Use compositor-specific APIs
   - GNOME: Use GSettings shortcuts API
   - KDE: Use KGlobalAccel
   - Sway/wlroots: Use custom protocol

2. **Fallback Approach**: Direct input device access
   - Requires adding user to `input` group
   - Use `evdev-rs` or `rdev` library
   - Monitor `/dev/input/event*` devices

3. **Alternative**: Desktop Environment Integration
   - Register shortcuts through system settings
   - Use D-Bus for communication
   - More user-friendly but less portable

### Audio Buffer Management

```rust
struct PttAccumulator {
    buffer: Vec<i16>,
    max_samples: usize,  // e.g., 60 seconds at 16kHz = 960,000
    overflow_policy: OverflowPolicy,
}

enum OverflowPolicy {
    StopRecording,
    DropOldest,
    AutoSubmit,
}
```

### Performance Targets

- Mode switch latency: < 50ms
- Key press to recording start: < 20ms
- Key release to STT submission: < 100ms
- Overlay update rate: 30 FPS
- Memory usage (PTT buffer): < 10MB for 60s recording

## Dependencies

### New Crate Dependencies

```toml
# Global hotkeys
rdev = "0.5"  # or evdev-rs = "0.6"

# Overlay UI
egui = "0.24"
winit = "0.29"
# OR
gtk4 = "0.7"
gtk4-layer-shell = "0.3"

# Configuration
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"

# Async runtime for hotkeys
tokio = { version = "1.44", features = ["rt", "sync"] }
```

### System Requirements

- Linux with Wayland compositor (GNOME/KDE/Sway)
- User in `input` group (for evdev access)
- Graphics driver supporting transparency
- D-Bus for desktop integration (optional)

## Testing Strategy

### Unit Tests
- Mode switching state machine
- Buffer accumulation logic
- Hotkey parsing and validation
- Overlay animation states

### Integration Tests
- Mock hotkey events → mode changes
- Audio pipeline switching
- STT submission from both modes
- Configuration loading and validation

### Manual Testing Matrix
| Compositor | Hotkeys | Overlay | Notes |
|------------|---------|---------|-------|
| GNOME | Test | Test | Primary target |
| KDE Plasma | Test | Test | Secondary target |
| Sway | Test | Test | Minimal compositor |
| Hyprland | Test | Test | Modern Wayland |

### Performance Tests
- Rapid mode switching
- Maximum recording duration
- Memory usage over time
- CPU usage in each mode

## Risk Mitigation

### High-Risk Areas

1. **Wayland Hotkey Access**
   - Risk: May not work on all compositors
   - Mitigation: Multiple implementation strategies, clear documentation

2. **Overlay Compatibility**
   - Risk: Rendering issues on some compositors
   - Mitigation: Fallback to system tray indicator

3. **Audio Buffer Overflow**
   - Risk: Memory exhaustion on long recordings
   - Mitigation: Hard limits, automatic submission

4. **Permission Issues**
   - Risk: Users unable to configure input group
   - Mitigation: Alternative activation methods (GUI button, CLI command)

## Future Enhancements

1. **Voice Activation for PTT**
   - Combine modes: Hold key + speak to activate
   - Release key or silence to stop

2. **Configurable Indicators**
   - Multiple indicator styles
   - Position customization
   - Accessibility options

3. **Recording History**
   - Buffer recent PTT recordings
   - Allow replay/re-submission
   - Export capabilities

4. **Multi-Key Support**
   - Different keys for different actions
   - Mouse button support
   - Gamepad integration

## Configuration Schema

```toml
[modes]
default = "vad"  # or "ptt"

[modes.vad]
enabled = true
toggle_key = "Shift+F2"
energy_threshold = 0.3
vad_threshold = 0.5

[modes.ptt]
enabled = true
record_key = "Ctrl+Super"
max_duration_seconds = 60
min_duration_ms = 500
overflow_policy = "auto_submit"  # or "stop", "drop_oldest"

[indicator]
enabled = true
position = "top-right"  # or "top-left", "bottom-right", "bottom-left"
size = 20  # pixels
opacity = 0.8
colors = { ready = "#00ff00", recording = "#ff0000", processing = "#ffff00" }

[hotkeys]
backend = "evdev"  # or "rdev", "compositor"
permission_check = true
```

## Success Criteria

- [ ] Seamless switching between VAD and PTT modes
- [ ] Global hotkeys work on major Wayland compositors
- [ ] Visual indicator provides clear recording status
- [ ] PTT audio quality matches VAD mode
- [ ] No memory leaks during extended usage
- [ ] Documentation covers all setup requirements
- [ ] User feedback confirms improved usability

## Timeline

- **Week 1**: Mode infrastructure + PTT pipeline foundation
- **Week 2**: Complete PTT pipeline + start hotkey implementation
- **Week 3**: Finish hotkeys + implement visual indicator
- **Week 4**: Integration, testing, and polish
- **Week 5**: Documentation and release preparation

Total estimated time: 4-5 weeks for full implementation