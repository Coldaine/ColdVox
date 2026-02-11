---
doc_type: reference
subsystem: gui
status: draft
freshness: stale
preservation: preserve
domain_code: gui
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# ColdVox GUI Components Architecture

## Component Hierarchy

### Top-Level Architecture

```mermaid
graph TD
	A[Qt Application] --> B[AppRoot.qml]
	B --> C[CollapsedBar.qml]
	B --> D[ActivePanel.qml]
	B --> E[SettingsWindow.qml]
	B --> F[SystemTrayIcon]

	D --> G[ActivityIndicator.qml]
	D --> H[ScrollView]
	D --> I[ControlsBar.qml]

	H --> J[Transcript Text]

	subgraph Rust Backend
	K[GuiBridge]
	L[main.rs]
	end

	B -.-> K
	K -.-> L
```

### Detailed Component Breakdown

#### AppRoot.qml (Main Window)
Purpose: Top-level window management and system integration

Key Features:
- Always-on-top frameless window
- Position persistence
- Size management (collapsed/expanded states)
- System tray integration
- Settings persistence

Properties:
- `expanded`: Boolean for UI state
- `stateCode`: Integer for application state (0=Idle, 1=Recording, 2=Processing, 3=Complete)
- `level`: Integer for audio level (0-100)
- `transcript`: String for current transcript

Methods:
- `startDrag(mouse)`: Initiates window dragging
- `doDrag(mouse)`: Handles window dragging

Child Components:
- `CollapsedBar`: Visible when not expanded
- `ActivePanel`: Visible when expanded
- `SettingsWindow`: Modal settings dialog
- `SystemTrayIcon`: Background operation interface

#### CollapsedBar.qml (Minimal Interface)
Purpose: Minimal footprint interface for idle state

Key Features:
- Status LED with state-specific colors
- Microphone icon
- Settings gear icon with hover effects
- Click-to-expand functionality

Properties:
- `stateCode`: Integer for application state

Signals:
- `openSettings()`: Emitted when settings icon is clicked

Child Components:
- Status LED (Rectangle)
- Microphone icon (Text)
- Settings gear icon (Text with MouseArea)

#### ActivePanel.qml (Full Interface)
Purpose: Full-featured interface for active transcription

Key Features:
- Three-section layout (activity, transcript, controls)
- State-based content display
- Drag handling from activity area
- Signal propagation to bridge

Properties:
- `stateCode`: Integer for application state
- `level`: Integer for audio level
- `transcript`: String for current transcript

Signals:
- `stop()`: Stop recording
- `pauseResume()`: Toggle pause state
- `clear()`: Clear transcript
- `openSettings()`: Open settings dialog

Child Components:
- `ActivityIndicator`: Audio visualization
- `ScrollView`: Transcript display
- `ControlsBar`: User controls

#### ActivityIndicator.qml (Audio Visualization)
Purpose: Real-time audio level visualization with state indication

Key Features:
- 24-bar animated waveform
- Color-coded by state
- Sinusoidal animation for lively appearance
- Gradient background hinting state

Properties:
- `stateCode`: Integer for application state
- `level`: Integer for audio level (0-100)
- `phase`: Real for animation phase

Child Components:
- Background gradient (Rectangle)
- Bar row (Row with Repeater)
- Animation timer (Timer)

#### ControlsBar.qml (User Controls)
Purpose: User control buttons with hover effects

Key Features:
- Stop, Pause/Resume, Clear buttons
- Settings button
- Hover opacity transitions
- Proper spacing and alignment

Signals:
- `stop()`: Stop recording
- `pauseResume()`: Toggle pause state
- `clear()`: Clear transcript
- `openSettings()`: Open settings dialog

Child Components:
- Background (Rectangle)
- Button row (Row)
- Individual buttons (Button with MouseArea)

#### SettingsWindow.qml (Configuration)
Purpose: Modal dialog for user preferences and configuration

Key Features:
- Grouped settings categories
- Live transparency preview
- Placeholder inputs for future backend integration
- Modal behavior with stay-on-top

Properties:
- `opacityValue`: Real for window transparency

Child Components:
- Background (Rectangle)
- ScrollView with settings groups
- Individual setting controls (GroupBox, ComboBox, Slider, etc.)

## Data Flow Architecture

### Property Binding Flow

```mermaid
graph LR
	A[GuiBridge Rust] -->|Properties| B[AppRoot.qml]
	B -->|Property Binding| C[Child Components]
	C -->|Local Properties| D[Visual Elements]

	E[User Input] -->|Mouse Events| F[QML Components]
	F -->|Signal Emission| G[GuiBridge Invokables]
	G -->|Method Calls| H[Rust Backend Logic]
	H -->|Property Updates| A
```

### State Management Flow

```mermaid
stateDiagram-v2
	[*] --> Idle
	Idle --> Recording: cmd_start
	Recording --> Processing: cmd_stop
	Processing --> Complete: Processing Complete
	Complete --> Idle: cmd_clear
	Recording --> Idle: cmd_clear
	Processing --> Idle: cmd_clear

	Recording --> Recording: cmd_toggle_pause
	Recording --> Paused: cmd_toggle_pause
	Paused --> Recording: cmd_toggle_pause
	Paused --> Idle: cmd_clear
```

### Event Flow

```mermaid
sequenceDiagram
	participant User
	participant QML
	participant Bridge
	participant Backend

	User->>QML: Click Start
	QML->>Bridge: cmd_start()
	Bridge->>Backend: Start Audio Capture
	Backend-->>Bridge: Audio Level Updates
	Bridge-->>QML: level Property Updates
	QML-->>User: Visual Feedback

	User->>QML: Click Stop
	QML->>Bridge: cmd_stop()
	Bridge->>Backend: Stop Audio Capture
	Backend-->>Bridge: Processing Complete
	Bridge-->>QML: state Property Updates
	QML-->>User: State Change
```

## Component Interaction Patterns

### Parent-Child Communication

#### Property Inheritance
```mermaid
graph TD
	A[AppRoot] -->|stateCode| B[CollapsedBar]
	A -->|stateCode| C[ActivePanel]
	A -->|level| C
	A -->|transcript| C

	C -->|stateCode| D[ActivityIndicator]
	C -->|level| D
```

#### Signal Propagation
```mermaid
graph TD
	A[ControlsBar] -->|stop()| B[ActivePanel]
	B -->|stop()| C[AppRoot]
	C -->|cmd_stop()| D[GuiBridge]
```

### Rust-QML Bridge Communication

#### Property Synchronization
```mermaid
graph LR
	A[Rust Backend] -->|#[qproperty]| B[GuiBridge]
	B -->|CXX-Qt Generated| C[QML Properties]
	C -->|Binding| D[Visual Elements]

	D -->|User Input| E[QML Events]
	E -->|Signal| F[QML Handlers]
	F -->|#[qinvokable]| G[GuiBridge Methods]
	G -->|Rust Logic| A
```

#### Signal Emission
```mermaid
graph LR
	A[Rust Backend] -->|State Change| B[GuiBridge]
	B -->|#[qsignal]| C[QML Signal]
	C -->|Connections| D[QML Handlers]
	D -->|Property Updates| E[Visual Elements]
```

## Component Lifecycle

### Window Management
```mermaid
sequenceDiagram
	participant Main
	participant AppRoot
	participant Settings

	Main->>AppRoot: Create Window
	AppRoot->>AppRoot: Load Settings
	AppRoot->>AppRoot: Set Position/Size

	AppRoot->>Settings: Create (Hidden)

	User->>AppRoot: Click Settings
	AppRoot->>Settings: Show Modal

	User->>Settings: Close
	Settings->>AppRoot: Hide

	Main->>AppRoot: Destroy
	AppRoot->>AppRoot: Save Settings
```

### State Transitions
```mermaid
sequenceDiagram
	participant User
	participant UI
	participant Bridge
	participant Backend

	User->>UI: Click Start
	UI->>Bridge: cmd_start()
	Bridge->>Backend: Start Recording
	Backend-->>Bridge: state = Recording
	Bridge-->>UI: stateChanged signal
	UI->>UI: Update Visual State

	loop During Recording
		Backend-->>Bridge: Audio Level Updates
		Bridge-->>UI: levelsChanged signal
		UI->>UI: Update Visualization
	end

	User->>UI: Click Stop
	UI->>Bridge: cmd_stop()
	Bridge->>Backend: Stop Recording
	Backend-->>Bridge: state = Processing
	Bridge-->>UI: stateChanged signal
	UI->>UI: Update Visual State

	Backend-->>Bridge: Processing Complete
	Bridge-->>Bridge: state = Complete
	Bridge-->>UI: stateChanged signal
	UI->>UI: Update Visual State
```

## Component Reusability and Extensibility

### Reusable Patterns
1. Property Binding: Components expose properties for external control
2. Signal Emission: Components emit signals for important events
3. State-Based Styling: Visual appearance changes based on state
4. Animation Integration: Smooth transitions for all state changes

### Extension Points
1. New Settings Categories: Add GroupBox to SettingsWindow.qml
2. New Visual Components: Add to ActivePanel.qml layout
3. New Backend Methods: Add to GuiBridge with #[qinvokable]
4. New Properties: Add to GuiBridge with #[qproperty]

### Customization Options
1. Styling: All components use centralized color scheme
2. Animation Timing: Configurable durations in Behavior properties
3. Layout: Responsive sizing with minimum/maximum constraints
4. Behavior: State machines can be extended with new states

## Performance Considerations

### Rendering Optimization
- Layer.enabled: Used for components with complex rendering
- Opacity Animations: Efficient GPU-accelerated transitions
- Property Binding: Minimized to essential connections
- Timer Frequency: Balanced between smoothness and performance

### Memory Management
- Component Reuse: Single instances of each component type
- Text Caching: Transcript text efficiently managed
- Animation Cleanup: Proper timer management
- Settings Persistence: Minimal data stored

### CPU Usage
- Animation Throttling: 30ms timer for audio visualization
- Event Debouncing: Rapid input events properly handled
- Property Update Batching: Multiple changes grouped where possible
- Idle Optimization: Reduced activity when not expanded
