---
doc_type: architecture
subsystem: gui
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# ColdVox GUI Design

## Overview

This document describes the design decisions and implementation details for the ColdVox GUI subsystem. It explains how the architectural requirements are realized, the tradeoffs made, potential risks, and validation approaches for the Qt Quick/QML overlay UI prototype.

## Design Realization

### Overlay Interface Design

#### Collapsed State Implementation
- **Design Choice**: 240×48px rounded rectangle with semi-transparent background
- **Realization**: Implemented in `CollapsedBar.qml` with:
  - Centered status LED indicating current state
  - Microphone icon on the left side
  - Settings gear icon on the right side with hover effects
  - Click-to-expand functionality via MouseArea
- **Animation**: Smooth transitions using QML Behavior properties

#### Expanded State Implementation
- **Design Choice**: Configurable size (up to 60% screen width × 40% screen height)
- **Realization**: Implemented in `ActivePanel.qml` with three main sections:
  1. **Activity Indicator**: Top section showing audio levels and state
  2. **Transcript Area**: Middle section with scrollable text display
  3. **Controls Bar**: Bottom section with action buttons
- **Layout**: ColumnLayout with proper spacing and responsive sizing

#### Window Management
- **Design Choice**: Always-on-top, frameless window with drag functionality
- **Realization**: Implemented in `AppRoot.qml` with:
  - Window flags: `Qt.Tool | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint`
  - Custom drag handlers for non-interactive areas
  - Position persistence using Qt.labs.settings
  - Smooth resize animations (300ms duration)

### Visual Feedback Design

#### Audio Level Visualization
- **Design Choice**: Animated bar graph with 24 bars reacting to audio levels
- **Realization**: Implemented in `ActivityIndicator.qml` with:
  - Repeater-based bar generation
  - Sinusoidal animation for lively appearance
  - Color coding by state (red for recording, yellow for processing, green for complete)
  - 80ms animation updates for smooth motion
- **Performance**: Uses QML's optimized rendering with layer.enabled and samples

#### State Indication
- **Design Choice**: Color-coded LED and activity indicator
- **Realization**:
  - Collapsed state: 8×8px LED with state-specific colors
  - Expanded state: Full activity indicator with gradient background
  - State enum mapping: Idle (gray), Recording (green), Processing (yellow), Complete (blue)

#### Transcript Display
- **Design Choice**: Fade-in animation for new text with auto-scroll
- **Realization**: Implemented in `ActivePanel.qml` transcript area with:
  - Opacity transition (0ms → 16ms → 100%) for text changes
  - Automatic scroll to bottom on new content
  - Word wrap with proper margins
  - ScrollView for handling overflow content

### User Controls Design

#### Control Buttons
- **Design Choice**: Simple text buttons with hover effects
- **Realization**: Implemented in `ControlsBar.qml` with:
  - Stop, Pause/Resume, Clear, and Settings buttons
  - Opacity transitions on hover (60% ↔ 100%)
  - Proper spacing and alignment
  - Signal propagation to parent components

#### System Tray Integration
- **Design Choice**: Native system tray with context menu
- **Realization**: Implemented in `AppRoot.qml` with:
  - Qt.labs.platform.SystemTrayIcon
  - Dynamic menu items based on current state
  - Icon changes based on visibility state
  - Full quit functionality

#### Settings Window
- **Design Choice**: Modal dialog with tabbed sections
- **Realization**: Implemented in `SettingsWindow.qml` with:
  - GroupBox-based organization for settings categories
  - Live transparency slider with immediate visual feedback
  - Placeholder inputs for future backend integration
  - Proper modal behavior with stay-on-top flag

### Configuration Management Design

#### Settings Persistence
- **Design Choice**: Qt.labs.settings for simple key-value storage
- **Realization**: Implemented in `AppRoot.qml` with:
  - Position tracking (posX, posY)
  - Transparency level (opacity)
  - Automatic save on property changes
  - Default values for first run

#### UI State Management
- **Design Choice**: Centralized state in GuiBridge with QML property binding
- **Realization**:
  - Rust-side state management in `bridge.rs`
  - QML property binding for reactive updates
  - Enum-based state machine for clear transitions
  - Signal emission for state changes

## Technical Implementation

### Rust-QML Bridge Architecture

#### CXX-Qt Integration
- **Design Choice**: CXX-Qt for type-safe Rust-QML interoperability
- **Realization**:
  - `#[cxx_qt::bridge]` macro in `bridge.rs` generates bindings
  - QObject-derived GuiBridge with properties and invokables
  - Build system integration via `build.rs`
  - Qt module linking (Gui, Qml, Quick)

#### Property System
- **Design Choice**: QML properties with Rust backend synchronization
- **Realization**:
  - `#[qproperty]` attributes for automatic property generation
  - Getter/setter methods with change notification
  - Property binding in QML for reactive UI updates
  - Type conversion between Rust and Qt types

#### Method Invocation
- **Design Choice**: QML invokables for UI-to-backend communication
- **Realization**:
  - `#[qinvokable]` attributes for exposed methods
  - Pin-based mutation for thread-safe state changes
  - Console logging for current prototype phase
  - Future integration with actual backend services

### QML Component Architecture

#### Component Hierarchy
```
AppRoot.qml (Top-level window)
├── CollapsedBar.qml (Idle state)
└── ActivePanel.qml (Active state)
    ├── ActivityIndicator.qml (Audio visualization)
    ├── ScrollView (Transcript display)
    └── ControlsBar.qml (User controls)

SettingsWindow.qml (Modal dialog)
SystemTrayIcon.qml (System tray)
```

#### Data Flow
1. **User Input**: QML MouseArea/Button → Signal emission
2. **Bridge Invocation**: Signal → GuiBridge invokable method
3. **State Update**: Rust method → Property change → QML binding update
4. **Visual Feedback**: Property change → UI animation/rendering

#### Animation System
- **Design Choice**: QML built-in animations with NumberAnimation
- **Realization**:
  - Smooth transitions for window resize (300ms)
  - Opacity fades for transcript updates (16ms)
  - Continuous animation for audio levels (30ms timer)
  - Hover effects for interactive elements (120ms)

## Tradeoffs

### Technology Selection Tradeoffs

#### Qt Quick/QML vs Alternatives
- **Advantages**:
  - Mature, well-documented framework
  - Excellent performance for visual applications
  - Built-in animation and styling system
  - Good tooling support (Qt Designer, Qt Creator)
  - Native look and feel on Linux

- **Disadvantages**:
  - C++ dependency tree increases binary size
  - Learning curve for QML declarative syntax
  - Limited Rust-native ecosystem compared to alternatives
  - Build complexity with CXX-Qt integration

#### CXX-Qt vs Other Rust-QT Bindings
- **Advantages**:
  - Type-safe code generation
  - Good integration with Cargo build system
  - Active maintenance and development
  - Supports modern Qt versions

- **Disadvantages**:
  - Additional build complexity
  - Limited documentation for advanced use cases
  - Performance overhead for Rust-QML communication

### Design Tradeoffs

#### Always-on-Top Behavior
- **Advantage**: Ensures UI is always accessible during transcription
- **Disadvantage**: Can obscure content if not positioned carefully
- **Mitigation**: Semi-transparent design and small collapsed footprint

#### System Tray Dependency
- **Advantage**: Provides background operation and essential controls
- **Disadvantage**: Inconsistent behavior across desktop environments
- **Mitigation**: Fallback to window controls when system tray unavailable

#### Simulated Backend
- **Advantage**: Allows UI development without full backend implementation
- **Disadvantage**: Limited testing of actual integration scenarios
- **Mitigation**: Clear separation of interface and implementation

## Risks

### Technical Risks

#### Qt Dependency Management
- **Risk**: Qt version compatibility and distribution
- **Impact**: Build failures and runtime errors on target systems
- **Mitigation**:
  - Pin to specific Qt version in documentation
  - Provide clear installation instructions
  - Consider static linking for distribution

#### CXX-Qt Stability
- **Risk**: CXX-Qt is relatively new and may have breaking changes
- **Impact**: Required maintenance when updating dependencies
- **Mitigation**:
  - Pin CXX-Qt version in Cargo.toml
  - Monitor project for breaking changes
  - Consider alternative bindings if needed

#### Performance Bottlenecks
- **Risk**: Frequent Rust-QML communication may impact performance
- **Impact**: UI lag during high-frequency updates (audio levels)
- **Mitigation**:
  - Batch updates where possible
  - Profile and optimize critical paths
  - Consider direct QML implementation for high-frequency updates

### Platform Risks

#### Linux-Only Implementation
- **Risk**: Current implementation may not translate well to other platforms
- **Impact**: Additional work required for cross-platform support
- **Mitigation**:
  - Abstract platform-specific code
  - Design with future platform expansion in mind
  - Test on multiple Linux distributions

#### System Tray Inconsistency
- **Risk**: System tray behavior varies across desktop environments
- **Impact**: Inconsistent user experience
- **Mitigation**:
  - Test on major desktop environments (GNOME, KDE, XFCE)
  - Provide fallback UI when system tray unavailable
  - Document platform-specific requirements

### Integration Risks

#### Backend Integration Complexity
- **Risk**: Current stub implementation may not match actual backend needs
- **Impact**: Significant refactoring required during integration
- **Mitigation**:
  - Early prototyping with actual backend services
  - Design flexible bridge API
  - Maintain clear separation of concerns

#### State Synchronization
- **Risk**: Inconsistent state between Rust backend and QML frontend
- **Impact**: UI showing incorrect state or behavior
- **Mitigation**:
  - Implement robust state management patterns
  - Add comprehensive error handling
  - Create automated tests for state transitions

## Validation Plan

### Unit Testing
- **QML Components**: Qt Test framework for component behavior
- **Rust Bridge**: Unit tests for GuiBridge logic
- **State Transitions**: Verify all state changes work correctly

### Integration Testing
- **Rust-QML Communication**: Verify property binding and method invocation
- **Settings Persistence**: Test save/load of configuration
- **Window Management**: Verify positioning and resizing behavior

### User Acceptance Testing
- **Visual Design**: Validate appearance and animations
- **Interaction Flow**: Verify user workflows are intuitive
- **Performance**: Measure responsiveness and resource usage

### Platform Testing
- **Linux Distributions**: Test on Ubuntu, Fedora, Arch, etc.
- **Desktop Environments**: Verify on GNOME, KDE, XFCE, etc.
- **System Tray**: Test integration with different tray implementations

## Future Enhancements

### Backend Integration
- Connect GuiBridge invokables to actual audio processing
- Implement real-time transcript updates from STT engine
- Add error handling and recovery mechanisms

### Cross-Platform Support
- Add Windows support with native window management
- Add macOS support with proper integration
- Abstract platform-specific system tray code

### Feature Expansion
- Implement advanced transcription features
- Add accessibility support
- Create plugin system for extensibility

### Performance Optimization
- Profile and optimize rendering performance
- Reduce memory footprint
- Implement efficient update mechanisms
