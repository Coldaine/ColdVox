---
doc_type: reference
subsystem: gui
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
domain_code: gui
---

# ColdVox Rust-QML Bridge Architecture

## Bridge Architecture Overview

### High-Level Architecture

```mermaid
graph TD
	A[QML UI Components] -->|Property Binding| B[CXX-Qt Generated Code]
	B -->|Method Calls| C[GuiBridge Rust]
	C -->|Service Calls| D[GUI Service Layer]
	D -->|Async Operations| E[Existing Backend Services]

	E -->|Events/Updates| D
	D -->|State Updates| C
	C -->|Signal Emission| B
	B -->|Property Updates| A

	subgraph GUI Subsystem
	A
	B
	C
	D
	end

	subgraph Backend Services
	E[coldvox-audio<br/>coldvox-stt<br/>coldvox-vad<br/>coldvox-text-injection]
	end

	F[ServiceRegistry] -->|Initializes| E
	F -->|Provides| D

	classDef guiSubsystem fill:#e1f5fe,stroke:#01579b,stroke-width:2px
	classDef backendServices fill:#f3e5f5,stroke:#4a148c,stroke-width:2px

	class A,B,C,D guiSubsystem
	class E backendServices
```

### Key Components

1. GuiBridge Rust Struct: Core bridge implementation with properties and methods
2. CXX-Qt Generated Code: Type-safe bindings between Rust and Qt
3. QML Property Bindings: Reactive UI updates based on Rust state
4. QML Signal Handlers: Event propagation from UI to Rust logic

## CXX-Qt Integration

### Build System Integration

The bridge is integrated into the build system through:

1. `build.rs`: Configures CXX-Qt build process
2. `Cargo.toml`: Declares CXX-Qt dependencies and features
3. QML Module Registration: Registers Rust types with QML engine

### Dependencies

```toml
# Core CXX-Qt dependencies
cxx = "1"
cxx-qt = "0.7"
cxx-qt-lib = { version = "0.7", features = ["qt_qml", "qt_gui"] }

# Qt modules required for bridge
qt_qml = []
qt_gui = []
```

### Build Configuration

The `build.rs` file configures:

1. Qt Module Linking: Core, Gui, Qml, Quick modules
2. CXX-Qt Code Generation: Bridge code generation
3. QML Module Registration: Type registration with QML engine

## GuiBridge API

### Properties

#### Core State Properties

```rust
// Window expansion state
#[qproperty]
expanded: bool,

// Application state (0=Idle, 1=Recording, 2=Processing, 3=Complete)
#[qproperty]
state: i32,

// Audio level (0-100)
#[qproperty]
level: i32,

// Current transcript text
#[qproperty]
transcript: QString,
```

#### Property Behavior

| Property | Type | Default | QML Binding | Description |
|----------|------|---------|-------------|-------------|
| `expanded` | bool | false | Direct | Controls UI expansion state |
| `state` | i32 | 0 (Idle) | Direct | Application state machine |
| `level` | i32 | 0 | Direct | Audio visualization level |
| `transcript` | QString | "" | Direct | Current transcript content |

### Invokable Methods

#### UI Control Methods

```rust
#[qinvokable]
pub fn toggle_expand(self: Pin<&mut Self>) {
	// Toggle expanded state
	let new_expanded = !self.expanded();
	self.set_expanded(new_expanded);
	println!("GUI: Expanded: {}", new_expanded);
}

#[qinvokable]
pub fn cmd_start(self: Pin<&mut Self>) {
	// Start recording command
	if self.state() == 0 {
		self.set_state(1);
		println!("GUI: Start recording");
	}
}

#[qinvokable]
pub fn cmd_toggle_pause(self: Pin<&mut Self>) {
	// Toggle pause state
	match self.state() {
		1 => {
			self.set_state(4); // Paused
			println!("GUI: Pause recording");
		}
		4 => {
			self.set_state(1); // Resume
			println!("GUI: Resume recording");
		}
		_ => {}
	}
}

#[qinvokable]
pub fn cmd_stop(self: Pin<&mut Self>) {
	// Stop recording command
	if self.state() == 1 || self.state() == 4 {
		self.set_state(2);
		println!("GUI: Stop recording");
	}
}

#[qinvokable]
pub fn cmd_clear(self: Pin<&mut Self>) {
	// Clear transcript command
	self.set_transcript(QString::from(""));
	self.set_state(0);
	self.set_level(0);
	println!("GUI: Clear transcript");
}

#[qinvokable]
pub fn cmd_open_settings(self: Pin<&mut Self>) {
	// Open settings window command
	println!("GUI: Open settings");
}
```

#### Method Behavior

| Method | Parameters | Returns | State Changes | Description |
|--------|------------|---------|---------------|-------------|
| `toggle_expand` | None | void | `expanded` | Toggle UI expansion state |
| `cmd_start` | None | void | `state` → Recording | Start audio recording |
| `cmd_toggle_pause` | None | void | `state` ↔ Paused | Toggle pause state |
| `cmd_stop` | None | void | `state` → Processing | Stop audio recording |
| `cmd_clear` | None | void | `state` → Idle, `transcript` → "" | Clear transcript and reset |
| `cmd_open_settings` | None | void | None | Open settings dialog |

### Signals

#### State Change Signals

```rust
#[qsignal]
pub fn state_changed(self: Pin<&mut Self>, new_state: i32);

#[qsignal]
pub fn transcript_delta(self: Pin<&mut Self>, delta: QString);

#[qsignal]
pub fn levels_changed(self: Pin<&mut Self>, level: i32);

#[qsignal]
pub fn error(self: Pin<&mut Self>, message: QString);
```

#### Signal Behavior

| Signal | Parameters | Emitted When | QML Handler | Description |
|--------|------------|---------------|--------------|-------------|
| `state_changed` | `new_state: i32` | State property changes | `onStateChanged` | Application state transition |
| `transcript_delta` | `delta: QString` | Transcript updates | `onTranscriptDelta` | New transcript content |
| `levels_changed` | `level: i32` | Audio level changes | `onLevelsChanged` | Audio visualization update |
| `error` | `message: QString` | Error conditions | `onError` | Error notification |

## QML Integration

### Property Binding

```qml
// In AppRoot.qml
property bool expanded: bridge.expanded
property int stateCode: bridge.state
property int level: bridge.level
property string transcript: bridge.transcript

Connections {
	target: bridge
	function onStateChanged(new_state) {
		stateCode = new_state
	}
	function onTranscriptDelta(delta) {
		transcript += delta
	}
	function onLevelsChanged(new_level) {
		level = new_level
	}
	function onError(message) {
		// Handle error
	}
}
```

### Method Invocation

```qml
Button {
	text: "Start"
	onClicked: bridge.cmd_start()
}

Button {
	text: "Pause/Resume"
	onClicked: bridge.cmd_toggle_pause()
}

MouseArea {
	anchors.fill: parent
	onClicked: bridge.toggle_expand()
}
```

## State Machine Implementation

```rust
pub enum AppState {
	Idle = 0,
	Recording = 1,
	Processing = 2,
	Complete = 3,
	Paused = 4,
}
```

### State Transitions

```mermaid
stateDiagram-v2
	[*] --> Idle
	Idle --> Recording: cmd_start
	Recording --> Paused: cmd_toggle_pause
	Paused --> Recording: cmd_toggle_pause
	Recording --> Processing: cmd_stop
	Processing --> Complete: Processing Complete
	Complete --> Idle: cmd_clear
	Recording --> Idle: cmd_clear
	Processing --> Idle: cmd_clear
	Paused --> Idle: cmd_clear
```

### State Validation

```rust
impl GuiBridge {
	fn validate_state_transition(&self, new_state: i32) -> bool {
		let current = self.state();
		match (current, new_state) {
			(0, 1) => true, // Idle → Recording
			(1, 4) => true, // Recording → Paused
			(4, 1) => true, // Paused → Recording
			(1, 2) => true, // Recording → Processing
			(2, 3) => true, // Processing → Complete
			(3, 0) => true, // Complete → Idle
			(1, 0) => true, // Recording → Idle
			(2, 0) => true, // Processing → Idle
			(4, 0) => true, // Paused → Idle
			_ => false,
		}
	}
}
```

## Data Flow Patterns

### Property Update Flow

```mermaid
sequenceDiagram
	participant Rust as Rust Backend
	participant Bridge as GuiBridge
	participant CXX as CXX-Qt
	participant QML as QML Frontend

	Rust->>Bridge: set_property(value)
	Bridge->>Bridge: validate_change(value)
	Bridge->>CXX: Property Update
	CXX->>QML: Property Binding
	QML->>QML: UI Update
```

### Method Invocation Flow

```mermaid
sequenceDiagram
	participant QML as QML Frontend
	participant CXX as CXX-Qt
	participant Bridge as GuiBridge
	participant Rust as Rust Backend

	QML->>CXX: method_call()
	CXX->>Bridge: invokable_method()
	Bridge->>Bridge: validate_action()
	Bridge->>Rust: backend_service_call()
	Rust->>Bridge: result
	Bridge->>CXX: Property Update
	CXX->>QML: Property Binding
```

### Signal Emission Flow

```mermaid
sequenceDiagram
	participant Rust as Rust Backend
	participant Bridge as GuiBridge
	participant CXX as CXX-Qt
	participant QML as QML Frontend

	Rust->>Bridge: event_occurred()
	Bridge->>Bridge: emit_signal(data)
	Bridge->>CXX: Signal Emission
	CXX->>QML: Signal Handler
	QML->>QML: Handle Event
```

## Error Handling Patterns

### Property Validation

```rust
impl GuiBridge {
	fn set_level(self: Pin<&mut Self>, value: i32) {
		let clamped = value.clamp(0, 100);
		if clamped != value {
			println!("GUI: Level clamped from {} to {}", value, clamped);
		}
		if self.level() != clamped {
			self.set_level(clamped);
			self.levels_changed(clamped);
		}
	}
}
```

### Method Error Handling

```rust
#[qinvokable]
pub fn cmd_start(self: Pin<&mut Self>) {
	if self.state() != 0 {
		let error_msg = QString::from("Cannot start: not in idle state");
		self.error(error_msg.clone());
		return;
	}
	if self.validate_state_transition(1) {
		self.set_state(1);
		self.state_changed(1);
		println!("GUI: Start recording");
	} else {
		let error_msg = QString::from("Invalid state transition");
		self.error(error_msg);
	}
}
```

## Performance Considerations

### Property Update Optimization
1. Change Detection: Only emit signals when values change
2. Batch Updates: Group related changes
3. Throttling: Limit high-frequency updates (audio levels)

### Memory Management
1. QString Conversion efficiency
2. Pin-based mutation safety
3. Minimize data copying in signals

### Thread Safety
1. Main thread affinity for bridge ops
2. Pin-based access patterns
3. Future async integration readiness

## Future Backend Integration

```rust
impl GuiBridge {
	#[qinvokable]
	pub fn cmd_start(self: Pin<&mut Self>) {
		if self.state() == 0 {
			self.set_state(1);
			println!("GUI: Start recording");
			// Future: audio_service.start_recording();
		}
	}

	pub fn on_audio_level(&mut self, level: i32) {
		self.set_level(level);
		self.levels_changed(level);
	}

	pub fn on_transcript_chunk(&mut self, chunk: &str) {
		let mut transcript = self.transcript().to_string();
		transcript.push_str(chunk);
		self.set_transcript(QString::from(transcript.as_str()));
		self.transcript_delta(QString::from(chunk));
	}
}
```

## Testing Strategy

### Unit Testing

```rust
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_state_validation() {
		let bridge = GuiBridge::new();
		assert!(bridge.validate_state_transition(1));
		assert!(bridge.validate_state_transition(4));
		assert!(!bridge.validate_state_transition(3));
	}
	#[test]
	fn test_level_clamping() {
		let mut bridge = GuiBridge::new();
		let bridge = Pin::new(&mut bridge);
		bridge.set_level(-10);
		assert_eq!(bridge.level(), 0);
		bridge.set_level(150);
		assert_eq!(bridge.level(), 100);
	}
}
```

### Integration Testing
1. QML property binding
2. Method invocation
3. Signal emission
4. State transitions

### Mock Backend Pattern

```rust
#[cfg(test)]
struct MockAudioService {}
#[cfg(test)]
impl MockAudioService {
	fn start(&self) -> Result<(), String> { Ok(()) }
	fn stop(&self) -> Result<(), String> { Ok(()) }
}
```

## Debugging and Logging

```rust
impl GuiBridge {
	fn log_state_change(&self, old_state: i32, new_state: i32) {
		let state_names = ["Idle", "Recording", "Processing", "Complete", "Paused"];
		let old_name = state_names.get(old_state as usize).unwrap_or(&"Unknown");
		let new_name = state_names.get(new_state as usize).unwrap_or(&"Unknown");
		println!("GUI: State change: {} → {}", old_name, new_name);
	}
}
```

### QML Debugging

```qml
onStateChanged: console.log("State changed to:", stateCode)
function debugMethodCall(method) { console.log("Method called:", method); bridge[method]() }
```

## Documentation Standards

```rust
/// Toggle the expanded state of the GUI window.
#[qinvokable]
pub fn toggle_expand(self: Pin<&mut Self>) {
	// Implementation stub
}
```

```qml
/*!
	\qmltype GuiBridge
	\brief Bridge between Rust backend and QML frontend
*/
```
