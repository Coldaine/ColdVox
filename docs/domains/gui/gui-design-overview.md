---
doc_type: architecture
subsystem: gui
status: draft
freshness: stale
preservation: preserve
domain_code: gui
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# ColdVox GUI Design

## Design Realization

### Overlay Interface Design

#### Collapsed State Implementation
- Design Choice: 240×48px rounded rectangle with semi-transparent background
- Realization: Implemented in `CollapsedBar.qml` with:
	- Centered status LED indicating current state
	- Microphone icon on the left side
	- Settings gear icon on the right side with hover effects
	- Click-to-expand functionality via MouseArea
- Animation: Smooth transitions using QML Behavior properties

#### Expanded State Implementation
- Design Choice: Configurable size (up to 60% screen width × 40% screen height)
- Realization: Implemented in `ActivePanel.qml` with three main sections:
	1. Activity Indicator: Top section showing audio levels and state
	2. Transcript Area: Middle section with scrollable text display
	3. Controls Bar: Bottom section with action buttons
- Layout: ColumnLayout with proper spacing and responsive sizing

#### Window Management
- Design Choice: Always-on-top, frameless window with drag functionality
- Realization: Implemented in `AppRoot.qml` with:
	- Window flags: `Qt.Tool | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint`
	- Custom drag handlers for non-interactive areas
	- Position persistence using Qt.labs.settings
	- Smooth resize animations (300ms duration)

### Visual Feedback Design

#### Audio Level Visualization
- Design Choice: Animated bar graph with 24 bars reacting to audio levels
- Realization: Implemented in `ActivityIndicator.qml` with:
	- Repeater-based bar generation
	- Sinusoidal animation for lively appearance
	- Color coding by state (red for recording, yellow for processing, green for complete)
	- 80ms animation updates for smooth motion
- Performance: Uses QML's optimized rendering with layer.enabled and samples

#### State Indication
- Design Choice: Color-coded LED and activity indicator
- Realization:
	- Collapsed state: 8×8px LED with state-specific colors
	- Expanded state: Full activity indicator with gradient background
	- State enum mapping: Idle (gray), Recording (green), Processing (yellow), Complete (blue)

#### Transcript Display
- Design Choice: Fade-in animation for new text with auto-scroll
- Realization: Implemented in `ActivePanel.qml` transcript area with:
	- Opacity transition (0ms → 16ms → 100%) for text changes
	- Automatic scroll to bottom on new content
	- Word wrap with proper margins
	- ScrollView for handling overflow content

### User Controls Design

#### Control Buttons
- Design Choice: Simple text buttons with hover effects
- Realization: Implemented in `ControlsBar.qml` with:
	- Stop, Pause/Resume, Clear, and Settings buttons
	- Opacity transitions on hover (60% ↔ 100%)
	- Proper spacing and alignment
	- Signal propagation to parent components

#### System Tray Integration
- Design Choice: Native system tray with context menu
- Realization: Implemented in `AppRoot.qml` with:
	- Qt.labs.platform.SystemTrayIcon
	- Dynamic menu items based on current state
	- Icon changes based on visibility state
	- Full quit functionality

#### Settings Window
- Design Choice: Modal dialog with tabbed sections
- Realization: Implemented in `SettingsWindow.qml` with:
	- GroupBox-based organization for settings categories
	- Live transparency slider with immediate visual feedback
	- Placeholder inputs for future backend integration
	- Proper modal behavior with stay-on-top flag

### Configuration Management Design

#### Settings Persistence
- Design Choice: Qt.labs.settings for simple key-value storage
- Realization: Implemented in `AppRoot.qml` with:
	- Position tracking (posX, posY)
	- Transparency level (opacity)
	- Automatic save on property changes
	- Default values for first run

#### UI State Management
- Design Choice: Centralized state in GuiBridge with QML property binding
- Realization:
	- Rust-side state management in `bridge.rs`
	- QML property binding for reactive updates
	- Enum-based state machine for clear transitions
	- Signal emission for state changes

## Technical Implementation

### Rust-QML Bridge Architecture

#### CXX-Qt Integration
- Design Choice: CXX-Qt for type-safe Rust-QML interoperability
- Realization:
	- `#[cxx_qt::bridge]` macro in `bridge.rs` generates bindings
	- QObject-derived GuiBridge with properties and invokables
	- Build system integration via `build.rs`
	- Qt module linking (Gui, Qml, Quick)

#### Property System
- Design Choice: QML properties with Rust backend synchronization
- Realization:
	- `#[qproperty]` attributes for automatic property generation
	- Getter/setter methods with change notification
	- Property binding in QML for reactive UI updates
	- Type conversion between Rust and Qt types

#### Method Invocation
- Design Choice: QML invokables for UI-to-backend communication
- Realization:
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
1. User Input: QML MouseArea/Button → Signal emission
2. Bridge Invocation: Signal → GuiBridge invokable method
3. State Update: Rust method → Property change → QML binding update
4. Visual Feedback: Property change → UI animation/rendering

#### Animation System
- Design Choice: QML built-in animations with NumberAnimation
- Realization:
	- Smooth transitions for window resize (300ms)
	- Opacity fades for transcript updates (16ms)
	- Continuous animation for audio levels (30ms timer)
	- Hover effects for interactive elements (120ms)

## Tradeoffs

### Technology Selection Tradeoffs

#### Qt Quick/QML vs Alternatives
- Advantages:
	- Mature, well-documented framework
	- Excellent performance for visual applications
	- Built-in animation and styling system
	- Good tooling support (Qt Designer, Qt Creator)
	- Native look and feel on Linux

- Disadvantages:
	- C++ dependency tree increases binary size
	- Learning curve for QML declarative syntax
	- Limited Rust-native ecosystem compared to alternatives
	- Build complexity with CXX-Qt integration

#### CXX-Qt vs Other Rust-QT Bindings
- Advantages:
	- Type-safe code generation
	- Good integration with Cargo build system
	- Active maintenance and development
	- Supports modern Qt versions

- Disadvantages:
	- Additional build complexity
	- Limited documentation for advanced use cases
	- Performance overhead for Rust-QML communication

### Design Tradeoffs

#### Always-on-Top Behavior
- Advantage: UI always accessible
- Disadvantage: Can obscure content
- Mitigation: Semi-transparent design and small collapsed footprint

#### System Tray Dependency
- Advantage: Background operation and essential controls
- Disadvantage: Inconsistent behavior across environments
- Mitigation: Fallback to window controls when system tray unavailable

#### Simulated Backend
- Advantage: UI development without full backend
- Disadvantage: Limited integration testing
- Mitigation: Clear interface separation

## Risks

### Technical Risks

#### Qt Dependency Management
- Risk: Version compatibility and distribution challenges
- Impact: Build/runtime errors
- Mitigation: Pin versions, document installation, consider static linking

#### CXX-Qt Stability
- Risk: Potential breaking changes
- Impact: Maintenance overhead
- Mitigation: Pin versions, monitor project

#### Performance Bottlenecks
- Risk: High-frequency Rust-QML communication
- Impact: UI lag
- Mitigation: Batch/throttle updates, profile critical paths

### Platform Risks

#### Linux-Only Implementation
- Risk: Portability gaps
- Impact: Extra work for cross-platform support
- Mitigation: Abstract platform code, design for expansion

#### System Tray Inconsistency
- Risk: Different behaviors across DEs
- Impact: Inconsistent UX
- Mitigation: Fallback UI, DE testing

### Integration Risks

#### Backend Integration Complexity
- Risk: Stub mismatch with real backend
- Impact: Refactoring during integration
- Mitigation: Prototype early, flexible API

#### State Synchronization
- Risk: Inconsistent state between layers
- Impact: Incorrect UI behavior
- Mitigation: Robust state patterns, error handling, tests

## Validation Plan

### Unit Testing
- QML components via Qt Test
- Rust bridge logic
- State transitions

### Integration Testing
- Rust-QML communication
- Settings persistence
- Window management

### User Acceptance Testing
- Visual design
- Interaction flow
- Performance responsiveness

### Platform Testing
- Linux distributions and desktop environments
- System tray behavior validation

## Future Enhancements

### Backend Integration
- Connect invokables to audio processing
- Real-time transcript updates
- Error recovery mechanisms

### Cross-Platform Support
- Windows/macOS window management
- System tray abstraction

### Feature Expansion
- Accessibility features
- Advanced transcription UI
- Plugin system extensibility

### Performance Optimization
- Rendering profiling
- Memory optimization
- Efficient update pipelines
