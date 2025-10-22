---
doc_type: research
subsystem: text-injection
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# Tracing Through the Text Injection Pipeline

## Phase 1: Event Reception to Buffering

### Entry Point: `InjectionProcessor::handle_transcription`
The flow begins in `processor.rs` when a transcription event is received:

```rust
pub fn handle_transcription(&mut self, event: TranscriptionEvent) {
    match event {
        TranscriptionEvent::Final { text, utterance_id, .. } => {
            info!("Received final transcription [{}]: {}", utterance_id, text);
            self.session.add_transcription(text);
            // Record metrics
            if let Ok(mut metrics) = self.injection_metrics.lock() {
                metrics.record_buffered_chars(text_len as u64);
            }
        }
        // ... handle other event types
    }
}
```

### Session State Management: `InjectionSession::add_transcription`
In `session.rs`, the transcription is added to the buffer and state transitions occur:

```rust
pub fn add_transcription(&mut self, text: String) {
    // Normalize and filter text
    let text = if self.normalize_whitespace {
        text.split_whitespace().collect::<Vec<&str>>().join(" ")
    } else {
        text.to_string()
    };
    
    // Add to buffer
    self.buffer.push(text);
    self.last_transcription = Some(Instant::now());
    
    // State transition logic
    match self.state {
        SessionState::Idle => {
            self.state = SessionState::Buffering;
            self.buffering_start = Some(Instant::now());
        }
        SessionState::Buffering => {
            // Continue buffering
        }
        SessionState::WaitingForSilence => {
            // Reset to buffering on new transcription
            self.state = SessionState::Buffering;
            self.buffering_start = Some(Instant::now());
        }
        // ...
    }
}
```

## Phase 2: Injection Triggering

### Periodic Checking: `InjectionProcessor::check_and_inject`
The processor periodically checks if injection should occur:

```rust
pub async fn check_and_inject(&mut self) -> anyhow::Result<()> {
    if self.session.should_inject() {
        self.perform_injection().await?;
    }
    Ok(())
}
```

### Injection Timing Logic: `InjectionSession::should_inject`
The session determines when to inject based on silence detection:

```rust
pub fn should_inject(&mut self) -> bool {
    match self.state {
        SessionState::Buffering => {
            // Check if we should transition to WaitingForSilence first
            self.check_for_silence_transition();
            false // Don't inject while still in Buffering state
        }
        SessionState::WaitingForSilence => {
            if let Some(last_time) = self.last_transcription {
                if last_time.elapsed() >= self.silence_timeout {
                    self.state = SessionState::ReadyToInject;
                    true // Ready to inject
                } else {
                    false // Still waiting for silence
                }
            } else {
                false
            }
        }
        SessionState::ReadyToInject => {
            if self.buffer.is_empty() {
                self.state = SessionState::Idle;
                false
            } else {
                true // Ready to inject
            }
        }
        SessionState::Idle => false,
    }
}
```

### Silence Detection: `InjectionSession::check_for_silence_transition`
This method checks if enough time has passed to consider speech paused:

```rust
pub fn check_for_silence_transition(&mut self) {
    if self.state == SessionState::Buffering {
        if let Some(_buffering_start) = self.buffering_start {
            let time_since_last_transcription = self.last_transcription.map(|t| t.elapsed());
            
            if let Some(time_since_last) = time_since_last_transcription {
                if time_since_last >= self.buffer_pause_timeout {
                    self.state = SessionState::WaitingForSilence;
                }
            }
        }
    }
}
```

## Phase 3: Strategy Selection

### Main Injection Logic: `StrategyManager::inject`
In `manager.rs`, the strategy manager selects and executes injection methods:

```rust
pub async fn inject(&mut self, text: &str) -> Result<(), InjectionError> {
    // Get current application ID
    let app_id = self.get_current_app_id().await?;
    
    // Get ordered list of methods to try
    let method_order = self.get_method_order_cached(&app_id).await;
    
    // Try each method in order
    for method in method_order {
        // Skip if in cooldown
        if self.is_in_cooldown(method) {
            continue;
        }
        
        // Check budget
        if !self.has_budget_remaining() {
            return Err(InjectionError::BudgetExhausted);
        }
        
        // Try injection with the selected injector
        if let Some(injector) = self.injectors.get(method) {
            let result = injector.inject_text(text, Some(&context)).await;
            
            match result {
                Ok(()) => {
                    // Success - update metrics and clear cooldown
                    self.update_success_record(&app_id, method, true);
                    return Ok(());
                }
                Err(e) => {
                    // Failure - update metrics and apply cooldown
                    self.update_success_record(&app_id, method, false);
                    self.apply_cooldown(&app_id, method, &e.to_string());
                    // Continue to next method
                }
            }
        }
    }
    
    // All methods failed
    Err(InjectionError::AllMethodsFailed("All injection methods failed".to_string()))
}
```

### Method Selection: `StrategyManager::_get_method_priority`
The system prioritizes methods based on environment and success rates:

```rust
pub(crate) fn _get_method_priority(&self, app_id: &str) -> Vec<InjectionMethod> {
    // Base order derived from environment
    let on_wayland = env::var("XDG_SESSION_TYPE").map(|s| s == "wayland").unwrap_or(false);
    let on_x11 = env::var("XDG_SESSION_TYPE").map(|s| s == "x11").unwrap_or(false);
    
    let mut base_order: Vec<InjectionMethod> = Vec::new();
    
    if on_wayland || on_x11 {
        base_order.push(InjectionMethod::AtspiInsert);
    }
    
    // Optional methods if enabled
    if self.config.allow_kdotool {
        base_order.push(InjectionMethod::KdoToolAssist);
    }
    if self.config.allow_enigo {
        base_order.push(InjectionMethod::EnigoText);
    }
    
    // Clipboard paste is tried last to avoid clipboard disruption
    base_order.push(InjectionMethod::ClipboardPasteFallback);
    
    // Sort by historical success rate as tiebreaker
    base_order.sort_by(|a, b| {
        // Use success rates from cache
        let rate_a = get_success_rate(app_id, *a);
        let rate_b = get_success_rate(app_id, *b);
        rate_b.partial_cmp(&rate_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    base_order.push(InjectionMethod::NoOp); // Always include as last resort
    
    base_order
}
```

## Phase 4: Injection Execution

### AT-SPI Direct Insertion: `AtspiInjector::insert_text`
The preferred method uses AT-SPI accessibility API:

```rust
pub async fn insert_text(&self, text: &str, context: &InjectionContext) -> InjectionResult<()> {
    // Connect to AT-SPI
    let conn = AccessibilityConnection::new().await?;
    let zbus_conn = conn.connection();
    
    // Find focused editable element
    let collection = CollectionProxy::builder(zbus_conn)
        .destination("org.a11y.atspi.Registry")
        .path("/org/a11y/atspi/accessible/root")
        .build().await?;
    
    let mut rule = ObjectMatchRule::default();
    rule.states = State::Focused.into();
    rule.states_mt = MatchType::All;
    rule.ifaces = Interface::EditableText.into();
    
    let matches = collection.get_matches(rule, SortOrder::Canonical, 1, false).await?;
    let obj_ref = matches.into_iter().next().ok_or(InjectionError::NoEditableFocus)?;
    
    // Get EditableText proxy
    let editable = EditableTextProxy::builder(zbus_conn)
        .destination(obj_ref.name.clone())
        .path(obj_ref.path.clone())
        .build().await?;
    
    // Get current caret position
    let text_iface = TextProxy::builder(zbus_conn)
        .destination(obj_ref.name.clone())
        .path(obj_ref.path.clone())
        .build().await?;
    let caret = text_iface.caret_offset().await?;
    
    // Insert text at caret position
    editable.insert_text(caret, text, text.chars().count() as i32).await?;
    
    // Confirm insertion if needed
    if let Some(ref target) = context.target_app {
        let window = context.window_id.as_deref().unwrap_or("unknown");
        if let Ok(result) = self.confirmation_context.confirm_injection(target, text, window).await {
            match result {
                ConfirmationResult::Success => {
                    debug!("AT-SPI insertion confirmed via text change event");
                }
                _ => {
                    debug!("AT-SPI insertion confirmation failed");
                }
            }
        }
    }
    
    Ok(())
}
```

### Clipboard-based Injection: `ClipboardInjector::inject`
The fallback method uses clipboard seed/restore:

```rust
pub async fn inject(&self, text: &str, context: &InjectionContext) -> InjectionResult<()> {
    // Backup current clipboard
    let backup = self.read_clipboard().await?;
    
    // Seed clipboard with payload
    self.write_clipboard(text.as_bytes(), "text/plain").await?;
    
    // Stabilize clipboard
    tokio::time::sleep(Duration::from_millis(20)).await;
    
    // Perform paste action
    self.perform_paste().await?;
    
    // Wait for paste to complete
    let restore_delay = self.config.clipboard_restore_delay_ms.unwrap_or(500);
    tokio::time::sleep(Duration::from_millis(restore_delay)).await;
    
    // Restore clipboard backup
    self.restore_clipboard(&backup).await?;
    
    Ok(())
}
```

## Phase 5: Confirmation and Fallback

### Injection Confirmation: `confirm::text_changed`
The system attempts to confirm successful injection:

```rust
pub async fn text_changed(target: &str, prefix: &str, window: &str) -> InjectionResult<ConfirmationResult> {
    // Connect to AT-SPI
    let conn = AccessibilityConnection::new().await?;
    let zbus_conn = conn.connection();
    
    // Get initial text content
    let collection = CollectionProxy::builder(zbus_conn)
        .destination("org.a11y.atspi.Registry")
        .path("/org/a11y.atspi/accessible/root")
        .build().await?;
    
    let mut last_text = String::new();
    // ... get initial text
    
    // Poll for text changes
    let poll_interval = Duration::from_millis(10);
    let timeout_duration = Duration::from_millis(75);
    
    let start_time = Instant::now();
    while start_time.elapsed() < timeout_duration {
        // Get current text and check for changes
        if let Some(obj_ref) = get_focused_element().await? {
            let current_text = get_text_content(&obj_ref).await?;
            
            if current_text != last_text {
                // Check if the change matches our expected prefix
                if current_text.len() > last_text.len() {
                    let new_chars = &current_text[last_text.len()..];
                    if matches_prefix(new_chars, prefix) {
                        return Ok(ConfirmationResult::Success);
                    }
                }
                last_text = current_text;
            }
        }
        
        tokio::time::sleep(poll_interval).await;
    }
    
    Ok(ConfirmationResult::Timeout)
}
```

## Key Insights from Tracing

### 1. Timing Mechanisms
- The system uses a two-stage timeout: `buffer_pause_timeout` (short) and `silence_timeout` (longer)
- Default configuration has `buffer_pause_timeout_ms = 0` and `silence_timeout_ms = 0`, meaning injection happens immediately
- This suggests the STT system handles audio buffering internally

### 2. Strategy Selection
- AT-SPI direct insertion is preferred on both Wayland and X11
- Clipboard paste is used as a fallback to avoid disrupting user's clipboard
- Historical success rates influence method ordering

### 3. Error Handling
- Each method has per-method timeouts and cooldown periods
- Failed methods enter exponential backoff cooldown
- The system tracks success/failure rates per application-method combination

### 4. Confirmation Mechanism
- Uses AT-SPI text change events with prefix matching
- Has a 75ms timeout with 10ms polling intervals
- Extracts 3-6 characters from the injected text for matching

### 5. Pre-warming
- Pre-warms AT-SPI connections, clipboard snapshots, and portal sessions
- Uses TTL caching (3 seconds) for pre-warmed data
- Can target pre-warming based on the injection method to be used

This trace reveals a sophisticated system with multiple fallback mechanisms, careful timing control, and robust error handling designed for reliable text injection across different Linux desktop environments.