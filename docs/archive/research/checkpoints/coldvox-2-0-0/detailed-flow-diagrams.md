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

# Detailed Flow Diagrams of ColdVox Text Injection System

## 1. Complete Text Injection Flow

```mermaid
flowchart TD
    START([STT Transcription Complete]) --> CHECK{Empty Text?}
    CHECK -->|Yes| END([End - Nothing to Inject])
    CHECK -->|No| BUFFER[Add to Session Buffer]
    
    BUFFER --> STATE_UPDATE[Update Session State]
    STATE_UPDATE --> TIMING_CHECK{Ready to Inject?}
    
    TIMING_CHECK -->|No| WAIT[Continue Waiting]
    WAIT --> TIMING_CHECK
    
    TIMING_CHECK -->|Yes| CONTEXT[Create Injection Context]
    CONTEXT --> APP_DETECTION[Detect Target Application]
    APP_DETECTION --> METHOD_SELECTION[Select Injection Method]
    
    METHOD_SELECTION --> PREWARM{Pre-warming Needed?}
    PREWARM -->|Yes| PREWARM_EXEC[Execute Pre-warming]
    PREWARM_EXEC --> INJECT_ATTEMPT[Attempt Injection]
    PREWARM -->|No| INJECT_ATTEMPT
    
    INJECT_ATTEMPT --> SUCCESS{Injection Successful?}
    SUCCESS -->|Yes| CONFIRM[Confirm Injection]
    SUCCESS -->|No| COOLDOWN[Apply Cooldown]
    
    COOLDOWN --> NEXT_METHOD{More Methods Available?}
    NEXT_METHOD -->|Yes| METHOD_SELECTION
    NEXT_METHOD -->|No| FAIL[All Methods Failed]
    
    CONFIRM --> CONFIRMED{Confirmation Successful?}
    CONFIRMED -->|Yes| METRICS[Update Success Metrics]
    CONFIRMED -->|No| UNCONFIRMED[Handle Uncertain State]
    
    METRICS --> COMPLETE([Injection Complete])
    UNCONFIRMED --> COMPLETE
    FAIL --> ERROR([Report Error])
```

## 2. Session State Management Flow

```mermaid
stateDiagram-v2
    [*] --> Idle: System Start
    
    Idle --> Buffering: First Transcription
    note right of Idle: No active session\nWaiting for input
    
    Buffering --> Buffering: Additional Transcription
    note right of Buffering: Receiving continuous\ntranscriptions
    
    Buffering --> CheckSilence: Buffer Pause Timeout
    note right of CheckSilence: Checking if speech\nhas paused
    
    CheckSilence --> WaitingForSilence: No New Transcription
    note right of CheckSilence: Speech appears\nto have paused
    
    CheckSilence --> Buffering: New Transcription
    note right of CheckSilence: Speech continues\nReset timer
    
    WaitingForSilence --> Buffering: New Transcription
    note right of WaitingForSilence: Speech resumed\nReset to buffering
    
    WaitingForSilence --> ReadyToInject: Silence Timeout
    note right of WaitingForSilence: Speech has ended\nReady to inject
    
    Buffering --> ReadyToInject: Buffer Size Limit
    note right of Buffering: Too much text\nForce injection
    
    Buffering --> ReadyToInject: Punctuation Detected
    note right of Buffering: Sentence-ending\npunctuation
    
    ReadyToInject --> Idle: Buffer Taken
    note right of ReadyToInject: Text injected\nReset session
    
    ReadyToInject --> ReadyToInject: Still Has Content
    note right of ReadyToInject: Partial injection\nContinue with remaining
```

## 3. Strategy Selection and Fallback Flow

```mermaid
flowchart TD
    START([Injection Request]) --> ENV_DETECT[Detect Environment]
    
    ENV_DETECT --> WAYLAND{Wayland?}
    ENV_DETECT --> X11{X11?}
    
    WAYLAND -->|Yes| BASE_ORDER[Create Base Method Order]
    X11 -->|Yes| BASE_ORDER
    WAYLAND -->|No| BASE_ORDER
    X11 -->|No| BASE_ORDER
    
    BASE_ORDER --> SUCCESS_SORT[Sort by Success Rates]
    SUCCESS_SORT --> METHOD_LOOP[Process Methods]
    
    METHOD_LOOP --> GET_METHOD[Get Next Method]
    GET_METHOD --> COOLDOWN_CHECK{Method in Cooldown?}
    
    COOLDOWN_CHECK -->|Yes| GET_METHOD
    COOLDOWN_CHECK -->|No| BUDGET_CHECK{Budget Remaining?}
    
    BUDGET_CHECK -->|No| BUDGET_FAIL[Budget Exhausted]
    BUDGET_CHECK -->|Yes| AVAILABILITY_CHECK{Method Available?}
    
    AVAILABILITY_CHECK -->|No| GET_METHOD
    AVAILABILITY_CHECK -->|Yes| ATTEMPT_INJECTION[Attempt Injection]
    
    ATTEMPT_INJECTION --> INJECTION_RESULT{Injection Successful?}
    
    INJECTION_RESULT -->|Yes| UPDATE_SUCCESS[Update Success Metrics]
    INJECTION_RESULT -->|No| UPDATE_FAILURE[Update Failure Metrics]
    
    UPDATE_FAILURE --> APPLY_COOLDOWN[Apply Exponential Cooldown]
    APPLY_COOLDOWN --> MORE_METHODS{More Methods?}
    
    MORE_METHODS -->|Yes| GET_METHOD
    MORE_METHODS -->|No| ALL_FAILED[All Methods Failed]
    
    UPDATE_SUCCESS --> INJECTION_COMPLETE[Injection Complete]
    BUDGET_FAIL --> INJECTION_FAILED[Injection Failed]
    ALL_FAILED --> INJECTION_FAILED
```

## 4. AT-SPI Injection Detailed Flow

```mermaid
flowchart TD
    START([AT-SPI Injection Request]) --> CONNECT[Connect to AT-SPI Bus]
    CONNECT --> CONNECTION_SUCCESS{Connection Successful?}
    
    CONNECTION_SUCCESS -->|No| ATSPI_FAIL[Report AT-SPI Unavailable]
    CONNECTION_SUCCESS -->|Yes| FIND_FOCUSED[Find Focused Element]
    
    FIND_FOCUSED --> COLLECTION[Get Collection Proxy]
    COLLECTION --> FOCUSED_MATCH[Find Focused Match]
    FOCUSED_MATCH --> FOCUSED_FOUND{Focused Element Found?}
    
    FOCUSED_FOUND -->|No| NO_FOCUS[Report No Editable Focus]
    FOCUSED_FOUND -->|Yes| GET_EDITABLE[Get EditableText Interface]
    
    GET_EDITABLE --> EDITABLE_PROXY[Create EditableText Proxy]
    EDITABLE_PROXY --> GET_TEXT[Get Text Interface]
    GET_TEXT --> TEXT_PROXY[Create Text Proxy]
    
    TEXT_PROXY --> GET_CARET[Get Current Caret Position]
    GET_CARET --> INSERT_TEXT[Insert Text at Caret]
    
    INSERT_TEXT --> INSERT_SUCCESS{Insertion Successful?}
    
    INSERT_SUCCESS -->|No| INSERT_FAIL[Report Insertion Failed]
    INSERT_SUCCESS -->|Yes| CONFIRM_INJECTION[Confirm Injection]
    
    CONFIRM_INJECTION --> CONFIRM_TIMEOUT{Confirmation Timeout?}
    
    CONFIRM_TIMEOUT -->|Yes| CONFIRM_FAIL[Confirmation Failed]
    CONFIRM_TIMEOUT -->|No| MONITOR_CHANGES[Monitor Text Changes]
    
    MONITOR_CHANGES --> TEXT_CHANGED{Text Changed?}
    
    TEXT_CHANGED -->|No| POLL_AGAIN{Continue Polling?}
    TEXT_CHANGED -->|Yes| CHECK_PREFIX{Prefix Matches?}
    
    CHECK_PREFIX -->|Yes| CONFIRM_SUCCESS[Injection Confirmed]
    CHECK_PREFIX -->|No| PREFIX_MISMATCH[Prefix Mismatch]
    
    POLL_AGAIN -->|Yes| MONITOR_CHANGES
    POLL_AGAIN -->|No| CONFIRM_FAIL
    
    CONFIRM_SUCCESS --> ATSPI_COMPLETE[Injection Complete]
    CONFIRM_FAIL --> ATSPI_COMPLETE
    PREFIX_MISMATCH --> ATSPI_COMPLETE
    ATSPI_FAIL --> ATSPI_ERROR[AT-SPI Error]
    NO_FOCUS --> ATSPI_ERROR
    INSERT_FAIL --> ATSPI_ERROR
```

## 5. Clipboard Injection Detailed Flow

```mermaid
flowchart TD
    START([Clipboard Injection Request]) --> BACKUP_CLIPBOARD[Read Current Clipboard]
    BACKUP_CLIPBOARD --> BACKUP_SUCCESS{Backup Successful?}
    
    BACKUP_SUCCESS -->|No| CLIPBOARD_ERROR[Clipboard Error]
    BACKUP_SUCCESS -->|Yes| SET_CLIPBOARD[Set New Clipboard Content]
    
    SET_CLIPBOARD --> SET_SUCCESS{Set Successful?}
    
    SET_SUCCESS -->|No| CLIPBOARD_ERROR
    SET_SUCCESS -->|Yes| STABILIZE[Wait for Stabilization]
    
    STABILIZE --> DETECT_METHOD{Detect Paste Method}
    
    DETECT_METHOD --> ATSPI_AVAILABLE{AT-SPI Available?}
    DETECT_METHOD --> ENIGO_AVAILABLE{Enigo Available?}
    DETECT_METHOD --> YDOTOOL_AVAILABLE{Ydotool Available?}
    
    ATSPI_AVAILABLE -->|Yes| ATSPI_PASTE[Try AT-SPI Paste]
    ATSPI_AVAILABLE -->|No| ENIGO_AVAILABLE
    
    ATSPI_PASTE --> ATSPI_PASTE_SUCCESS{AT-SPI Paste Success?}
    ATSPI_PASTE_SUCCESS -->|Yes| PASTE_COMPLETE[Paste Complete]
    ATSPI_PASTE_SUCCESS -->|No| ENIGO_AVAILABLE
    
    ENIGO_AVAILABLE -->|Yes| ENIGO_PASTE[Try Enigo Paste]
    ENIGO_AVAILABLE -->|No| YDOTOOL_AVAILABLE
    
    ENIGO_PASTE --> ENIGO_PASTE_SUCCESS{Enigo Paste Success?}
    ENIGO_PASTE_SUCCESS -->|Yes| PASTE_COMPLETE
    ENIGO_PASTE_SUCCESS -->|No| YDOTOOL_AVAILABLE
    
    YDOTOOL_AVAILABLE -->|Yes| YDOTOOL_PASTE[Try Ydotool Paste]
    YDOTOOL_AVAILABLE -->|No| PASTE_FAILED[Paste Failed]
    
    YDOTOOL_PASTE --> YDOTOOL_PASTE_SUCCESS{Ydotool Paste Success?}
    YDOTOOL_PASTE_SUCCESS -->|Yes| PASTE_COMPLETE
    YDOTOOL_PASTE_SUCCESS -->|No| PASTE_FAILED
    
    PASTE_COMPLETE --> WAIT_RESTORE[Wait for Restore Delay]
    WAIT_RESTORE --> RESTORE_CLIPBOARD[Restore Original Clipboard]
    
    RESTORE_CLIPBOARD --> RESTORE_SUCCESS{Restore Successful?}
    
    RESTORE_SUCCESS -->|Yes| CLIPBOARD_COMPLETE[Injection Complete]
    RESTORE_SUCCESS -->|No| RESTORE_WARN[Restore Warning]
    
    RESTORE_WARN --> CLIPBOARD_COMPLETE
    CLIPBOARD_ERROR --> CLIPBOARD_FAILED[Clipboard Failed]
    PASTE_FAILED --> CLIPBOARD_FAILED
```

## 6. Pre-warming Execution Flow

```mermaid
flowchart TD
    START([Pre-warming Trigger]) --> CHECK_EXPIRED{Data Expired?}
    
    CHECK_EXPIRED -->|No| USE_CACHED[Use Cached Data]
    CHECK_EXPIRED -->|Yes| PARALLEL_PREWARM[Execute Parallel Pre-warming]
    
    PARALLEL_PREWARM --> ATSPI_PREWARM[Pre-warm AT-SPI]
    PARALLEL_PREWARM --> CLIPBOARD_PREWARM[Snapshot Clipboard]
    PARALLEL_PREWARM --> PORTAL_PREWARM[Prepare Portal Session]
    PARALLEL_PREWARM --> VK_PREWARM[Connect Virtual Keyboard]
    PARALLEL_PREWARM --> EVENT_PREWARM[Arm Event Listener]
    
    ATSPI_PREWARM --> ATSPI_CONNECT[Connect to AT-SPI]
    ATSPI_CONNECT --> ATSPI_FOCUS[Find Focused Element]
    ATSPI_FOCUS --> ATSPI_CACHE[Cache AT-SPI Data]
    
    CLIPBOARD_PREWARM --> CLIPBOARD_READ[Read Clipboard]
    CLIPBOARD_READ --> CLIPBOARD_CACHE[Cache Clipboard Data]
    
    PORTAL_PREWARM --> PORTAL_CHECK[Check Portal Available]
    PORTAL_CHECK --> PORTAL_CONNECT[Connect to Portal]
    PORTAL_CONNECT --> PORTAL_CACHE[Cache Portal Data]
    
    VK_PREWARM --> VK_DETECT[Detect Virtual Keyboard]
    VK_DETECT --> VK_CONNECT[Connect to VK]
    VK_CONNECT --> VK_CACHE[Cache VK Data]
    
    EVENT_PREWARM --> EVENT_ARM[Arm Event Listener]
    EVENT_ARM --> EVENT_CACHE[Cache Event State]
    
    ATSPI_CACHE --> COLLECT_RESULTS[Collect Results]
    CLIPBOARD_CACHE --> COLLECT_RESULTS
    PORTAL_CACHE --> COLLECT_RESULTS
    VK_CACHE --> COLLECT_RESULTS
    EVENT_CACHE --> COLLECT_RESULTS
    
    COLLECT_RESULTS --> UPDATE_CACHES[Update All Caches]
    UPDATE_CACHES --> PREWARM_COMPLETE[Pre-warming Complete]
    
    USE_CACHED --> PREWARM_COMPLETE
```

## 7. Metrics Collection Flow

```mermaid
flowchart TD
    START([Injection Event]) --> METRICS_START[Start Metrics Timer]
    METRICS_START --> METHOD_SELECTION[Record Method Selection]
    
    METHOD_SELECTION --> INJECTION_ATTEMPT[Attempt Injection]
    INJECTION_ATTEMPT --> INJECTION_RESULT{Injection Result}
    
    INJECTION_RESULT -->|Success| SUCCESS_METRICS[Update Success Metrics]
    INJECTION_RESULT -->|Failure| FAILURE_METRICS[Update Failure Metrics]
    
    SUCCESS_METRICS --> METHOD_SUCCESS[Record Method Success]
    FAILURE_METRICS --> METHOD_FAILURE[Record Method Failure]
    
    METHOD_SUCCESS --> APP_SUCCESS[Update App Success Rate]
    METHOD_FAILURE --> APP_FAILURE[Update App Failure Rate]
    
    APP_SUCCESS --> CONFIRMATION[Attempt Confirmation]
    APP_FAILURE --> COOLDOWN_UPDATE[Update Cooldown State]
    
    CONFIRMATION --> CONFIRMATION_RESULT{Confirmation Result}
    
    CONFIRMATION_RESULT -->|Success| CONFIRM_SUCCESS[Record Confirmation Success]
    CONFIRMATION_RESULT -->|Failure| CONFIRM_FAILURE[Record Confirmation Failure]
    CONFIRMATION_RESULT -->|Timeout| CONFIRM_TIMEOUT[Record Confirmation Timeout]
    
    CONFIRM_SUCCESS --> LATENCY[Calculate Latency]
    CONFIRM_FAILURE --> LATENCY
    CONFIRM_TIMEOUT --> LATENCY
    
    COOLDOWN_UPDATE --> LATENCY
    
    LATENCY --> UPDATE_GLOBAL[Update Global Metrics]
    UPDATE_GLOBAL --> METRICS_COMPLETE[Metrics Collection Complete]
```

These detailed flow diagrams illustrate the complete operation of the ColdVox text injection system, showing how different components interact to provide reliable text injection across various Linux desktop environments.