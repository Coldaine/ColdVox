# Audio Device Selection on Linux (PipeWire/ALSA) - UPDATED

## Problem we're solving

On Linux (Nobara/PipeWire), the app sometimes starts capture on an ALSA "default" device that isn't the actual active microphone, leading to 0 FPS (no callbacks) and an empty TUI. We need reliable selection of the correct input device across runs and distros, and clear fallbacks when the default route is wrong.

## What we observed (live tests)

- Default device run: mic_capture reported 0 FPS (no audio frames).
- Targeted device run: `front:CARD=QuadCast,DEV=0` passed with ~35.8 FPS at 16 kHz (573 frames in ~16s).
- Watchdog panic was due to using Tokio timers in a std::thread; fixed by switching to a std::thread loop.

## Root cause

- CPAL integrates with PipeWire through ALSA/JACK compatibility layers (no native PipeWire backend exists in mainline CPAL).
- ALSA's "default" PCM may not track PipeWire's selected "Default source"; it can point to another card or a non-capturing route.
- Current implementation lacks CLI/env device selection and robust fallback mechanisms.

## Solution design (robust and portable)

### Selection order (strongest to weakest):
1. **User override**: CLI `-D/--device <STR>` or ENV `COLDVOX_DEVICE`
2. **WirePlumber default**: Query current default via `wpctl status`
3. **ALSA PipeWire devices**: `default`, `sysdefault`, `pipewire` (verified via runtime enumeration)
4. **Hardware pattern matching**: Names containing `front:`, `QuadCast`, `Microphone`
5. **OS default input** as last resort

### Startup preflight and auto-fallback:
- After starting capture, verify frames > 0 within 3–5 seconds
- If zero frames detected, stop and retry with next device in priority order
- On final failure, emit diagnostic error with `wpctl status` output and troubleshooting guidance

### App wiring:
- Add `-D/--device` and `COLDVOX_DEVICE` support to main binary
- Pass chosen device to `AudioCaptureThread::spawn(..., Some(device))`
- Log requested vs actual device, CPAL host backend, and final `StreamConfig`

## WirePlumber Integration

### Device Priority Management:
```bash
# Configure priorities via WirePlumber rules (NOT wpctl set-priority)
~/.config/wireplumber/main.lua.d/50-device-priorities.lua
rule = {
  matches = {
    {
      { "device.name", "matches", "alsa:*" }
    }
  },
  apply_properties = {
    ["priority.session"] = 1000,  -- Higher = preferred
    ["priority.driver"] = 1000,
  }
}

# Apply changes and verify
systemctl --user restart wireplumber
wpctl status  # Confirm priorities
```

### Runtime Device Switching:
```bash
# Set default device
wpctl set-default <device_id>

# Set device profile
wpctl set-profile <device_id> <profile>

# Persist settings
wpctl settings --save

# Verify current settings
pw-metadata -n sm-settings 0
```

### Diagnostics Commands:
```bash
# Essential troubleshooting commands
wpctl status                    # Show all devices/nodes
wpctl inspect <id>             # Show device properties
pw-metadata -n sm-settings 0   # Show WirePlumber settings
aplay -L                       # List ALSA devices
arecord -L                     # List ALSA input devices
```

## Verification checklist

- [ ] `mic_probe list-devices -v` shows devices including `pipewire` and `front:CARD=QuadCast,DEV=0`
- [ ] Running `mic_probe mic-capture -d 10 -v` without `-D` succeeds (when system default is correctly configured)
- [ ] Running with `-D 'front:CARD=QuadCast,DEV=0'` succeeds consistently
- [ ] Main app logs the requested device and the device actually opened
- [ ] `wpctl status` shows correct device priorities and defaults
- [ ] Device switching works via `wpctl set-default`

## Action items

### Phase 1: Fix Immediate Issues (High Priority)
- [ ] Add CLI `-D/--device` and ENV `COLDVOX_DEVICE` support to `src/main.rs`
- [ ] Implement device enumeration diagnostics with `wpctl status` output
- [ ] Add graceful fallback chain: user override → WirePlumber default → ALSA devices → hardware patterns → OS default
- [ ] Implement 3-5s frame verification with auto-retry logic
- [ ] Improve error messages with diagnostic commands and troubleshooting steps

### Phase 2: WirePlumber Integration (Medium Priority)
- [ ] Add WirePlumber priority configuration via config files
- [ ] Implement runtime device switching via `wpctl set-default`
- [ ] Add device profile management via `wpctl set-profile`
- [ ] Integrate `wpctl settings --save` for persistence
- [ ] Add hotplug event handling via WirePlumber policy

### Phase 3: Enhanced Diagnostics (Low Priority)
- [ ] Add comprehensive device health monitoring
- [ ] Implement smart device scoring respecting WirePlumber priorities
- [ ] Add performance metrics for device selection
- [ ] Create user-friendly device selection interface

## Technical Notes

### CPAL Backend Reality:
- **No native PipeWire backend** exists in mainline CPAL
- Integration through **ALSA/JACK compatibility layers** provided by PipeWire
- Current implementation correctly uses ALSA layer - no changes needed
- Backend selection should assume ALSA/JACK on Linux environments

### WirePlumber vs wpctl Commands:
- **Priority management**: Via WirePlumber config files, NOT `wpctl set-priority` (command doesn't exist)
- **Device switching**: `wpctl set-default` and `wpctl set-profile` (these exist)
- **Persistence**: `wpctl settings --save` saves to `sm-settings` metadata
- **Diagnostics**: `wpctl status` and `wpctl inspect` for device information

### Device Name Handling:
- **Conservative approach**: Rely on runtime enumeration (`wpctl status`, `aplay -L`)
- **Standard names only**: `default`, `sysdefault`, `pipewire`
- **Avoid assumptions**: Don't assume non-standard names like "pipewire-input"
- **Pattern matching**: Use for hardware-specific names when standard devices fail

## Implementation Status

- ✅ Watchdog uses std::thread (already implemented)
- ✅ Basic device selection logic exists in `device.rs`
- ❌ CLI/ENV device selection (needs implementation)
- ❌ WirePlumber integration (needs implementation)
- ❌ Runtime diagnostics (needs implementation)
- ❌ Auto-fallback logic (needs implementation)

## Migration Path

### From Current Implementation:
1. **Keep existing device.rs logic** as foundation
2. **Add CLI/env parsing** to main.rs
3. **Enhance error handling** with diagnostic output
4. **Layer WirePlumber features** on top

### Backward Compatibility:
- Existing automatic device selection continues to work
- New CLI/env options are additive
- No breaking changes to current API

## Success Metrics

- [ ] Zero device detection failures on Nobara and other Fedora derivatives
- [ ] Clear error messages with actionable troubleshooting steps
- [ ] Successful device switching without application restart
- [ ] Consistent behavior across different PipeWire/WirePlumber configurations