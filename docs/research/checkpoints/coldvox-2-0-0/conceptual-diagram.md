---
doc_type: research
subsystem: text-injection
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

# Conceptual Diagram of ColdVox Text Injection System

## High-Level Architecture

```mermaid
graph TB
    subgraph "Input Layer"
        STT[Speech-to-Text Engine]
        STT -->|TranscriptionEvent| IP[InjectionProcessor]
    end
    
    subgraph "Session Management"
        IP -->|handle_transcription| IS[InjectionSession]
        IS -->|State Machine| SM[Session States]
        SM -->|Idle| Idle
        SM -->|Buffering| Buffering
        SM -->|WaitingForSilence| WaitingForSilence
        SM -->|ReadyToInject| ReadyToInject
    end
    
    subgraph "Strategy Layer"
        IP -->|check_and_inject| SM2[StrategyManager]
        SM2 -->|get_method_order| BE[BackendDetector]
        SM2 -->|select_method| IM[InjectorRegistry]
        BE -->|detect_backends| ENV[Environment]
        ENV -->|Wayland| WL
        ENV -->|X11| X11
    end
    
    subgraph "Injection Backends"
        IM -->|AtspiInsert| ATSPI[AT-SPI Injector]
        IM -->|ClipboardPasteFallback| CB[Clipboard Injector]
        IM -->|EnigoText| EN[Enigo Injector]
        IM -->|KdoToolAssist| KD[Kdotool Injector]
        IM -->|NoOp| NO[NoOp Injector]
    end
    
    subgraph "Target Application"
        ATSPI -->|insert_text| TARGET[Active Application]
        CB -->|paste_text| TARGET
        EN -->|key_events| TARGET
        KD -->|window_activation| TARGET
    end
    
    subgraph "Confirmation Layer"
        TARGET -->|text_change_events| CONF[Confirmation Module]
        CONF -->|verify_injection| SM2
    end
    
    subgraph "Supporting Infrastructure"
        PRE[Prewarm Controller]
        FOCUS[Focus Tracker]
        METRICS[Injection Metrics]
        PRE -->|prewarm_resources| ATSPI
        PRE -->|prewarm_resources| CB
        FOCUS -->|focus_status| SM2
        METRICS -->|track_performance| SM2
    end
```

## Session State Machine

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Buffering: First transcription received
    Buffering --> Buffering: Additional transcription
    Buffering --> WaitingForSilence: Buffer pause timeout
    WaitingForSilence --> Buffering: New transcription
    WaitingForSilence --> ReadyToInject: Silence timeout
    ReadyToInject --> Idle: Buffer taken
    ReadyToInject --> ReadyToInject: Still has content
    Buffering --> ReadyToInject: Buffer size limit
    Buffering --> ReadyToInject: Punctuation detected
```

## Injection Strategy Selection Flow

```mermaid
flowchart TD
    START[Injection Request] --> DETECT[Detect Environment]
    DETECT --> WAYLAND{Wayland?}
    DETECT --> X11{X11?}
    
    WAYLAND -->|Yes| ATSPI_PRI[AT-SPI Priority: High]
    X11 -->|Yes| ATSPI_PRI
    
    ATSPI_PRI --> SUCCESS{Success Rate > 30%?}
    SUCCESS -->|Yes| TRY_ATSPI[Try AT-SPI First]
    SUCCESS -->|No| CHECK_COOLDOWN{Method in Cooldown?}
    
    CHECK_COOLDOWN -->|No| TRY_ATSPI
    CHECK_COOLDOWN -->|Yes| NEXT_METHOD[Try Next Method]
    
    TRY_ATSPI --> ATSPI_SUCCESS{AT-SPI Success?}
    ATSPI_SUCCESS -->|Yes| INJECT_DONE[Injection Complete]
    ATSPI_SUCCESS -->|No| UPDATE_FAILURE[Update Failure Rate]
    UPDATE_FAILURE --> APPLY_COOLDOWN[Apply Cooldown]
    APPLY_COOLDOWN --> NEXT_METHOD
    
    NEXT_METHOD --> KD_CHECK{Kdotool Enabled?}
    KD_CHECK -->|Yes| TRY_KD[Try Kdotool]
    KD_CHECK -->|No| EN_CHECK{Enigo Enabled?}
    
    EN_CHECK -->|Yes| TRY_EN[Try Enigo]
    EN_CHECK -->|No| TRY_CLIPBOARD[Try Clipboard]
    
    TRY_KD --> KD_SUCCESS{Kdotool Success?}
    KD_SUCCESS -->|Yes| INJECT_DONE
    KD_SUCCESS -->|No| EN_CHECK
    
    TRY_EN --> EN_SUCCESS{Enigo Success?}
    EN_SUCCESS -->|Yes| INJECT_DONE
    EN_SUCCESS -->|No| TRY_CLIPBOARD
    
    TRY_CLIPBOARD --> CB_SUCCESS{Clipboard Success?}
    CB_SUCCESS -->|Yes| INJECT_DONE
    CB_SUCCESS -->|No| TRY_NOOP[Try NoOp]
    
    TRY_NOOP --> ALL_FAILED[All Methods Failed]
```

## AT-SPI Injection Flow

```mermaid
sequenceDiagram
    participant IP as InjectionProcessor
    participant SM as StrategyManager
    participant ATSPI as AT-SPI Injector
    participant ATSPI_BUS as AT-SPI Bus
    participant TARGET as Target Application
    participant CONFIRM as Confirmation Module
    
    IP->>SM: inject(text)
    SM->>ATSPI: inject_text(text, context)
    
    ATSPI->>ATSPI_BUS: Connect to AT-SPI
    ATSPI_BUS-->>ATSPI: Connection established
    
    ATSPI->>ATSPI_BUS: Find focused element
    ATSPI_BUS-->>ATSPI: Element reference
    
    ATSPI->>ATSPI_BUS: Get EditableText interface
    ATSPI_BUS-->>ATSPI: Interface proxy
    
    ATSPI->>ATSPI_BUS: Get current caret position
    ATSPI_BUS-->>ATSPI: Caret offset
    
    ATSPI->>ATSPI_BUS: Insert text at caret
    ATSPI_BUS->>TARGET: Text inserted
    ATSPI_BUS-->>ATSPI: Success
    
    ATSPI->>CONFIRM: confirm_injection(target, text, window)
    CONFIRM->>ATSPI_BUS: Monitor text changes
    ATSPI_BUS->>TARGET: Text content
    ATSPI_BUS-->>CONFIRM: Text content
    
    CONFIRM-->>ATSPI: Confirmation result
    ATSPI-->>SM: Injection result
    SM-->>IP: Injection status
```

## Clipboard Injection Flow

```mermaid
sequenceDiagram
    participant IP as InjectionProcessor
    participant SM as StrategyManager
    participant CB as ClipboardInjector
    participant CLIP as System Clipboard
    participant TARGET as Target Application
    
    IP->>SM: inject(text)
    SM->>CB: inject_text(text, context)
    
    CB->>CLIP: Read current clipboard
    CLIP-->>CB: Backup content
    
    CB->>CLIP: Set new content (text)
    CLIP-->>CB: Success
    
    Note over CB: Wait for stabilization (20ms)
    
    CB->>TARGET: Trigger paste action
    TARGET->>CLIP: Request clipboard content
    CLIP-->>TARGET: Text content
    TARGET->>TARGET: Paste text
    
    Note over CB: Wait for paste completion (500ms)
    
    CB->>CLIP: Restore backup content
    CLIP-->>CB: Success
    
    CB-->>SM: Injection result
    SM-->>IP: Injection status
```

## Key Components Interactions

```mermaid
graph LR
    subgraph "Core Components"
        IP[InjectionProcessor]
        IS[InjectionSession]
        SM[StrategyManager]
    end
    
    subgraph "Injectors"
        ATSPI[AT-SPI Injector]
        CB[Clipboard Injector]
        EN[Enigo Injector]
        YD[Ydotool Injector]
    end
    
    subgraph "Supporting Systems"
        PRE[Prewarm Controller]
        CONF[Confirmation Module]
        METRICS[Metrics Collection]
        FOCUS[Focus Tracker]
    end
    
    IP --> IS
    IP --> SM
    SM --> ATSPI
    SM --> CB
    SM --> EN
    SM --> YD
    
    ATSPI --> CONF
    CB --> CONF
    
    PRE --> ATSPI
    PRE --> CB
    
    SM --> METRICS
    SM --> FOCUS
    
    IS --> METRICS
```

## Data Flow

```mermaid
flowchart LR
    STT[STT Event] --> BUFFER[Transcription Buffer]
    BUFFER --> STATE[Session State]
    STATE --> TRIGGER[Injection Trigger]
    TRIGGER --> STRATEGY[Strategy Selection]
    STRATEGY --> INJECT[Injection Method]
    INJECT --> TARGET[Target Application]
    TARGET --> CONFIRM[Confirmation]
    CONFIRM --> METRICS[Metrics Update]
    METRICS --> STRATEGY
```

This conceptual diagram illustrates the high-level architecture of the ColdVox text injection system, showing how different components interact to reliably inject transcribed text into active applications across various Linux desktop environments.