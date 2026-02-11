---
doc_type: research
subsystem: text-injection
status: draft
freshness: historical
preservation: delete
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# ColdVox Text Injection Stack - Comprehensive Architecture Summary

## Overview

The ColdVox text injection stack is a sophisticated, multi-layered system designed to inject transcribed speech-to-text content into target applications across different Linux desktop environments. The system employs multiple injection strategies with intelligent fallback mechanisms, adaptive method selection, and comprehensive telemetry.

## Runtime Fallback Flow (visual)

Below is a precise runtime flowchart (Mermaid) that reflects the implemented fallback order and decision points in the codebase. Paste this block into any renderer that supports Mermaid (v11+) to visualize the runtime path.

```mermaid
flowchart TD
    subgraph "Speech-to-Text Pipeline"
        A1["Speech Input"] --> A2["VAD (Voice Activity Detection)"]
        A2 --> A3["STT Processing"]
        A3 --> A4["TranscriptionEvent"]
        A4 --> B1["AsyncInjectionProcessor"]
    end

    subgraph "Injection Session"
        B1 --> B2["Buffering & Silence Detection"]
        B2 --> B3["ReadyToInject?"]
        B3 -- Yes --> C1["StrategyManager::inject(text)"]
        B3 -- No --> B2
    end

    subgraph "StrategyManager Fallback Logic"
        C1 --> D1{"Is AT-SPI available & focus valid?"}
        D1 -- Yes --> D2["Try AT-SPI Injector"]
        D2 -- Success --> E1["Injection Success"]
        D2 -- Failure/Timeout --> D3{"Is Clipboard+Paste available?"}
        D1 -- No --> D3

        D3 -- Yes --> D4["Try Clipboard+Paste Injector"]
        D4 -- Success --> E1
        D4 -- Failure/Timeout --> D5{"Is ydotool/kdotool available?"}
        D3 -- No --> D5

        D5 -- Yes --> D6["Try ydotool/kdotool Injector"]
        D6 -- Success --> E1
        D6 -- Failure/Timeout --> D7{"Is Enigo available?"}
        D5 -- No --> D7

        D7 -- Yes --> D8["Try Enigo Injector"]
        D8 -- Success --> E1
        D8 -- Failure/Timeout --> E2["NoOp (Telemetry Only)"]

        D7 -- No --> E2
    end

    subgraph "Post-Injection"
        E1 --> F1["Restore Clipboard (if used)"]
        E1 --> F2["Update Metrics"]
        E2 --> F2
    end

    classDef fallback fill:#f9f,stroke:#333,stroke-width:2px;
    D2,D4,D6,D8,E2 class fallback
    classDef decision fill:#ffe,stroke:#333,stroke-width:2px;
    D1,D3,D5,D7 class decision
    classDef success fill:#dfd,stroke:#333,stroke-width:2px;
    E1 class success
```

### Key code file mapping (exact places to inspect)
- AT‑SPI check & injection: `crates/coldvox-text-injection/src/focus.rs`, `crates/coldvox-text-injection/src/atspi_injector.rs`
- Clipboard+Paste: `crates/coldvox-text-injection/src/clipboard_injector.rs`
- ydotool/kdotool: `crates/coldvox-text-injection/src/ydotool_injector.rs`, `crates/coldvox-text-injection/src/kdotool_injector.rs`
- Enigo: `crates/coldvox-text-injection/src/enigo_injector.rs`
- Strategy/ordering/cooldowns: `crates/coldvox-text-injection/src/manager.rs`
- Session buffering & timing: `crates/coldvox-text-injection/src/session.rs`


## 1. Origins and Entry Points

### 1.1 System Integration Point

The injection stack originates from the broader ColdVox speech-to-text pipeline:

```
Speech Input → VAD → STT Processing → TranscriptionEvent → Injection Stack
```

**Entry Point**: [`TranscriptionEvent`](crates/coldvox-text-injection/src/processor.rs:124) objects flowing from the STT (Speech-to-Text) system into the `AsyncInjectionProcessor`.

### 1.2 Core Entry Components

- **AsyncInjectionProcessor**: Main event loop that receives transcription events
- **InjectionProcessor**: Synchronous processor that handles the core injection logic
- **StrategyManager**: Orchestrates method selection and execution
- **InjectionSession**: Manages transcription buffering and timing

## 2. Architecture Components

### 2.1 Core Processing Layer

#### StrategyManager ([`manager.rs`](crates/coldvox-text-injection/src/manager.rs))

The central orchestrator that:
- **Method Selection**: Intelligently chooses injection methods based on environment, application context, and historical success rates
- **Success Tracking**: Maintains per-application success records with adaptive cooldown mechanisms
- **Environment Detection**: Identifies available backends (X11, Wayland, KDE, etc.)
- **Configuration Management**: Applies allowlists, blocklists, and injection policies

**Key Features**:
- Exponential backoff cooldown system for failed methods
- Per-application method prioritization
- Real-time success rate calculation
- Privacy-first logging with text redaction

#### InjectionSession ([`session.rs`](crates/coldvox-text-injection/src/session.rs))

State machine managing transcription buffering:

```rust
pub enum SessionState {
    Idle,                    // Waiting for first transcription
    Buffering,              // Actively receiving transcriptions
    WaitingForSilence,      // No new transcriptions, waiting for timeout
    ReadyToInject          // Silence timeout reached, ready to inject
}
```

**Buffering Logic**:
- Accumulates transcriptions with configurable join separators
- Implements silence detection with dual timeouts
- Supports punctuation-based flushing
- Enforces maximum buffer size limits

### 2.2 Injection Methods Layer

#### Available Injection Methods

| Method | Backend | Platform | Description | Status |
|--------|---------|----------|-------------|--------|
| **AtspiInsert** | AT-SPI2 | Linux | Direct accessibility API injection | Primary |
| **ClipboardPasteFallback** | wl-clipboard-rs | Wayland/X11 | Clipboard + paste simulation | Fallback |
| **KdoToolAssist** | kdotool | KDE/X11 | Window activation assistance | Optional |
| **EnigoText** | enigo | Cross-platform | Input simulation | Optional |
| **NoOp** | None | All | No-op fallback | Last resort |

#### Injector Implementations

**AT-SPI Injector** ([`atspi_injector.rs`](crates/coldvox-text-injection/src/atspi_injector.rs)):
- Uses Linux accessibility APIs to directly insert text into focused editable fields
- Queries for focused `EditableText` interface objects
- Implements retry logic for transient focus states
- Provides highest reliability for supported applications

**Clipboard Injector** ([`clipboard_injector.rs`](crates/coldvox-text-injection/src/clipboard_injector.rs)):
- Sets clipboard content using Wayland-native APIs
- Triggers paste actions through the application
- Automatically restores original clipboard content
- Works across different desktop environments

**Ydotool Injector** ([`ydotool_injector.rs`](crates/coldvox-text-injection/src/ydotool_injector.rs)):
- Uses uinput kernel interface for synthetic input events
- Requires `ydotool` daemon with proper permissions
- Provides fallback paste triggering when AT-SPI fails
- Validates uinput access and daemon availability

### 2.3 Configuration and Types

#### InjectionConfig ([`types.rs`](crates/coldvox-text-injection/src/types.rs))

Comprehensive configuration structure controlling:
- **Method Selection**: Enable/disable specific injection methods
- **Timing Controls**: Timeouts, cooldowns, and latency budgets
- **Injection Modes**: "keystroke", "paste", or "auto" selection
- **Safety Features**: Allow/block lists, focus requirements
- **Performance Tuning**: Chunk sizes, pacing rates, buffer limits

## 3. Complete Process Flow

### 3.1 High-Level Flow

```
TranscriptionEvent Received
         ↓
    Session Buffering
         ↓
   Silence Detection
         ↓
   Method Selection
         ↓
  Injector Execution
         ↓
   Success/Failure
         ↓
  Metrics & Adaptation
```

### 3.2 Detailed Execution Path

#### Phase 1: Event Reception ([`AsyncInjectionProcessor`](crates/coldvox-text-injection/src/processor.rs:295))

```rust
// Main event loop in AsyncInjectionProcessor::run()
loop {
    tokio::select! {
        // Handle transcription events
        Some(event) = self.transcription_rx.recv() => {
            let mut processor = self.processor.lock().await;
            processor.handle_transcription(event); // Phase 2
        }

        // Periodic injection checks
        _ = interval.tick() => {
            // Extract text if session ready
            let maybe_text = {
                let mut processor = self.processor.lock().await;
                processor.prepare_injection() // Phase 3
            };

            if let Some(text) = maybe_text {
                // Perform injection outside lock
                let result = self.injector.inject(&text).await; // Phase 4
                // Record results back to processor
            }
        }
    }
}
```

#### Phase 2: Transcription Processing ([`InjectionProcessor::handle_transcription`](crates/coldvox-text-injection/src/processor.rs:124))

```rust
pub fn handle_transcription(&mut self, event: TranscriptionEvent) {
    match event {
        TranscriptionEvent::Final { text, utterance_id, .. } => {
            self.session.add_transcription(text); // Buffer management
            // Record metrics
            if let Ok(mut metrics) = self.injection_metrics.lock() {
                metrics.record_buffered_chars(text_len as u64);
            }
        }
        // Handle partial transcriptions
    }
}
```

#### Phase 3: Injection Preparation ([`InjectionSession`](crates/coldvox-text-injection/src/session.rs))

The session manages state transitions:

1. **Idle → Buffering**: First transcription received
2. **Buffering → WaitingForSilence**: Buffer pause timeout reached
3. **WaitingForSilence → ReadyToInject**: Silence timeout reached
4. **ReadyToInject → Idle**: Buffer consumed for injection

**Buffering Logic**:
- Accumulates transcriptions with whitespace normalization
- Checks for punctuation-based flushing triggers
- Enforces maximum buffer size limits
- Tracks timing for silence detection

#### Phase 4: Strategy-Based Injection ([`StrategyManager::inject`](crates/coldvox-text-injection/src/manager.rs:840))

```rust
pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
    // 1. Pre-injection checks
    if text.is_empty() || self.is_paused() { return Ok(()); }

    // 2. Focus and context validation
    let focus_status = self.focus_provider.get_focus_status().await?;
    let app_id = self.get_current_app_id().await?;

    // 3. Policy enforcement (allow/block lists)
    if !self.is_app_allowed(&app_id) { /* reject */ }

    // 4. Method selection and ordering
    let method_order = self.get_method_order_cached(&app_id);

    // 5. Sequential method attempts with fallback
    for method in method_order {
        if self.is_in_cooldown(method) { continue; }

        match self.injectors.get_mut(method) {
            Some(injector) => {
                match injector.inject_text(text).await {
                    Ok(()) => {
                        self.update_success_record(&app_id, method, true);
                        return Ok(()); // Success!
                    }
                    Err(e) => {
                        self.update_success_record(&app_id, method, false);
                        // Continue to next method
                    }
                }
            }
        }
    }

    // All methods failed
    Err(InjectionError::AllMethodsFailed(...))
}
```

## 4. Component Interaction Patterns

### 4.1 Strategy Manager and Injectors

The StrategyManager maintains a registry of available injectors:

```rust
struct InjectorRegistry {
    injectors: HashMap<InjectionMethod, Box<dyn TextInjector>>,
}
```

**Injector Interface** ([`lib.rs`](crates/coldvox-text-injection/src/lib.rs:71)):
```rust
#[async_trait::async_trait]
pub trait TextInjector: Send + Sync {
    async fn inject_text(&self, text: &str) -> InjectionResult<()>;
    async fn is_available(&self) -> bool;
    fn backend_name(&self) -> &'static str;
    fn backend_info(&self) -> Vec<(&'static str, String)>;
}
```

### 4.2 Session and Processor Integration

The InjectionProcessor coordinates between session management and injection execution:

```rust
pub struct InjectionProcessor {
    session: InjectionSession,        // Buffering & timing
    injector: StrategyManager,        // Method selection & execution
    config: InjectionConfig,          // Configuration
    metrics: ProcessorMetrics,        // Local metrics
    injection_metrics: Arc<Mutex<InjectionMetrics>>, // Shared telemetry
}
```

### 4.3 Metrics and Telemetry Flow

**Metrics Collection Points**:
1. **Transcription Events**: Character count, buffer size, timing
2. **Injection Attempts**: Method used, duration, success/failure
3. **Session Transitions**: State changes, buffer operations
4. **Error Conditions**: Focus issues, method failures, timeouts

## 5. Logic Flow from Start to Finish

### 5.1 Initialization Sequence

1. **System Startup**:
   ```rust
   // Create shared metrics
   let injection_metrics = Arc::new(Mutex::new(InjectionMetrics::default()));

   // Initialize processor with configuration
   let processor = InjectionProcessor::new(config, None, injection_metrics.clone()).await;

   // Create async wrapper
   let async_processor = AsyncInjectionProcessor::new(
       config,
       transcription_rx,
       shutdown_rx,
       None
   ).await;
   ```

2. **Backend Detection** ([`BackendDetector`](crates/coldvox-text-injection/src/backend.rs)):
   - Scans for available desktop environments (X11, Wayland)
   - Checks for required services (AT-SPI, ydotool daemon)
   - Validates permissions and accessibility services

### 5.2 Runtime Execution Loop

#### Event-Driven Processing

```
STT Event → AsyncInjectionProcessor → InjectionProcessor → InjectionSession
     ↓              ↓                        ↓                ↓
  Received     Transcription         Buffer Management    State Machine
               Processing             & Timing             Updates
```

#### Periodic Processing

```
Timer Tick → Check Session State → Extract Buffer → Strategy Injection
     ↓              ↓                  ↓              ↓
  100ms       Should Inject?      Take Text      Method Selection
  Interval                           ↓              ↓
                              Strategy Manager   Injector Execution
```

### 5.3 Method Selection Logic

**Environment-Based Ordering** ([`StrategyManager::_get_method_priority`](crates/coldvox-text-injection/src/manager.rs:563)):

```rust
let mut base_order: Vec<InjectionMethod> = Vec::new();

if on_wayland {
    base_order.push(InjectionMethod::AtspiInsert);  // Prefer direct insert
}

if on_x11 {
    base_order.push(InjectionMethod::AtspiInsert);   // Primary method
}

// Optional methods based on configuration
if self.config.allow_kdotool {
    base_order.push(InjectionMethod::KdoToolAssist);
}

// Clipboard paste as final fallback
base_order.push(InjectionMethod::ClipboardPasteFallback);

// Always include NoOp as last resort
base_order.push(InjectionMethod::NoOp);
```

**Success-Rate Adaptation**:
- Historical success rates influence method ordering
- Failed methods enter cooldown periods with exponential backoff
- Per-application success tracking enables context-aware selection

### 5.4 Error Handling and Fallbacks

#### Multi-Layer Error Recovery

1. **Method-Level Failures**:
   - Individual injector errors trigger fallback to next method
   - Success/failure tracking updates adaptive selection
   - Cooldown periods prevent repeated failures

2. **System-Level Failures**:
   - Focus detection failures allow continuation with warnings
   - Backend unavailability triggers graceful degradation
   - Configuration errors provide clear diagnostics

3. **Resource-Level Protections**:
   - Latency budgets prevent runaway injection attempts
   - Buffer size limits prevent memory exhaustion
   - Rate limiting protects system resources

## 6. Key Design Patterns

### 6.1 Strategy Pattern (Injection Methods)

The system uses a strategy pattern for injection methods:

```rust
enum InjectionMethod {
    AtspiInsert,
    ClipboardPasteFallback,
    KdoToolAssist,
    EnigoText,
    NoOp,
}
```

Each method implements the `TextInjector` trait, allowing runtime selection and fallback.

### 6.2 State Machine Pattern (Session Management)

The `InjectionSession` implements a clear state machine:

```
Idle → Buffering → WaitingForSilence → ReadyToInject → Idle
  ↑         ↓              ↓                ↓           ↑
  │    New TX    Buffer Pause    Silence Timeout   Inject   │
  │    Received    Timeout        Reached         Complete  │
  └─────────────────────────────────────────────────────────┘
```

### 6.3 Observer Pattern (Metrics Collection)

Metrics are collected through shared mutable state:

```rust
pub struct InjectionMetrics {
    pub attempts: u64,
    pub successes: u64,
    pub method_metrics: HashMap<InjectionMethod, MethodMetrics>,
    // ... comprehensive telemetry
}
```

## 7. Configuration-Driven Behavior

### 7.1 Injection Modes

**Auto Mode** (Default):
```rust
"auto" => text.len() > self.config.paste_chunk_chars as usize
```
- Uses paste for large texts (>500 chars by default)
- Uses keystroke simulation for shorter texts

**Paste Mode**:
- Always uses clipboard-based injection
- Best for large text blocks
- Requires paste permissions

**Keystroke Mode**:
- Always uses direct input simulation
- Best for real-time typing scenarios
- Works in more applications but slower

### 7.2 Safety and Privacy Features

**Allow/Block Lists**:
- Regex-based application filtering
- Prevents injection into sensitive applications
- Supports both inclusive and exclusive policies

**Privacy Controls**:
- Text content redaction in logs
- Configurable log verbosity levels
- Secure handling of clipboard content

## 8. Performance Characteristics

### 8.1 Latency Management

**Total Latency Budget** (Default: 800ms):
- Enforces end-to-end injection time limits
- Prevents system resource exhaustion
- Configurable per deployment needs

**Per-Method Timeouts** (Default: 250ms):
- Individual method attempt limits
- Prevents hanging on unresponsive backends
- Allows quick fallback to working methods

### 8.2 Memory Management

**Buffer Size Limits** (Default: 5000 characters):
- Prevents unbounded memory growth
- Forces injection when limits reached
- Configurable based on use case

**Efficient Text Processing**:
- Iterator-based chunking without allocation
- Streaming text processing
- Minimal memory copying

## 9. Integration Points

### 9.1 STT System Integration

Receives `TranscriptionEvent` objects:
```rust
pub enum TranscriptionEvent {
    Partial { text: String, utterance_id: u32, .. },
    Final { text: String, utterance_id: u32, .. },
    Error { code: i32, message: String },
}
```

### 9.2 Desktop Environment Integration

**AT-SPI Integration**:
- Connects to accessibility bus
- Queries focused application state
- Uses `EditableText` interface for direct insertion

**Window Management**:
- Detects active window class/context
- Supports both X11 and Wayland environments
- Handles focus tracking and validation

## 10. Monitoring and Debugging

### 10.1 Comprehensive Metrics

**InjectionMetrics** tracks:
- Success/failure rates by method and application
- Latency histograms and timing data
- Buffer utilization and throughput
- Error categorization and frequency

**ProcessorMetrics** provides:
- Session state and buffer information
- Real-time injection status
- Performance characteristics

### 10.2 Debug Capabilities

**StrategyManager::print_stats()**:
```rust
Injection Statistics:
  Total attempts: 150
  Successes: 142
  Failures: 8
  Success rate: 94.7%
  Method AtspiInsert: 120 attempts, 118 successes, 2 failures
```

**Session Inspection**:
- Buffer preview for debugging
- State machine visualization
- Timing and performance data

## Conclusion

The ColdVox injection stack represents a robust, adaptive text injection system that gracefully handles the complexities of Linux desktop environments. Through its multi-layered architecture, comprehensive fallback mechanisms, and intelligent method selection, it provides reliable text injection across diverse applications and desktop environments while maintaining strong safety, privacy, and performance characteristics.

The system's design emphasizes reliability through redundancy, adaptability through learning from experience, and maintainability through clear separation of concerns and comprehensive configuration options.