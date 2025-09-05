# [Audio] Improve audio device handling and hotplug support

**Priority:** Low

## Problem Description
The audio device handling lacks robust support for device hotplugging, error recovery, and dynamic device switching. Users experience issues when audio devices are disconnected/reconnected, and there's no graceful handling of device failures.

## Impact
- **Low**: Audio device disconnections cause system instability
- Poor user experience with device changes
- No automatic recovery from device failures
- Limited support for multiple audio devices
- Manual intervention required for device switching

## Reproduction Steps
1. Disconnect audio input device during operation
2. Reconnect different audio device
3. Test device switching without restart
4. Check error handling for device failures
5. Monitor behavior with multiple audio devices

## Expected Behavior
The system should:
- Handle audio device hotplugging gracefully
- Automatically recover from device disconnections
- Support dynamic device switching without restart
- Provide user feedback for device status changes
- Maintain audio processing continuity during device changes

## Current Behavior
The system exhibits:
- System instability when devices are disconnected
- No automatic recovery from device failures
- Requires restart for device switching
- Poor error handling for device issues
- Limited device status monitoring

## Proposed Solution
1. Implement device hotplug detection and handling
2. Add automatic device recovery mechanisms
3. Create device switching capabilities
4. Implement device status monitoring
5. Add user notifications for device events

## Implementation Steps
1. Analyze current device handling limitations
2. Implement device hotplug detection
3. Add automatic device recovery
4. Create device switching interface
5. Implement device status monitoring
6. Add user notifications for device events

## Acceptance Criteria
- [ ] Graceful handling of device disconnections
- [ ] Automatic recovery from device failures
- [ ] Dynamic device switching without restart
- [ ] Device status monitoring and feedback
- [ ] User notifications for device events
- [ ] Support for multiple audio device configurations

## Technical Details
- **Current**: Basic device enumeration without monitoring
- **Target**: Comprehensive device lifecycle management
- **Hotplug**: Real-time device connection/disconnection detection
- **Recovery**: Automatic fallback and recovery mechanisms
- **Monitoring**: Device health and status tracking

## Device Management Features
- **Hotplug Detection**: Monitor device connection/disconnection events
- **Automatic Recovery**: Seamless switching to available devices
- **Device Preferences**: User-configurable device priority lists
- **Status Monitoring**: Real-time device health monitoring
- **Error Handling**: Graceful degradation when devices fail
- **User Feedback**: Notifications for device status changes

## Recovery Strategies
- **Immediate Fallback**: Switch to next available device
- **Device Restart**: Attempt to reinitialize failed devices
- **Quality Preservation**: Maintain audio quality during transitions
- **State Preservation**: Continue processing with minimal interruption
- **User Notification**: Inform user of device changes and recovery actions

## Performance Considerations
- **Detection Latency**: < 100ms device change detection
- **Switching Time**: < 500ms device switching
- **Resource Usage**: Minimal overhead for device monitoring
- **Compatibility**: Support for various audio device types
- **Scalability**: Handle multiple device configurations

## Related Files
- `crates/coldvox-audio/src/capture.rs`
- `crates/coldvox-audio/src/device.rs`
- `crates/app/src/audio/capture.rs`
- Audio device management utilities
