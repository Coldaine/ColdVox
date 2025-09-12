---
id: COLDVOX-ADR3-002-hybrid-threading-model
type: ADR
level: 3
title: Hybrid Threading Model for Audio Processing
status: accepted
owner: @team-audio
updated: 2025-09-11
parent: COLDVOX-DOM2-001-audio-capture
links:
  satisfies: [COLDVOX-DOM2-001-audio-capture]
  depends_on: []
  supersedes: []
  related_to: [COLDVOX-SYS4-001-audio-capture-thread]
---

## Context
Real-time audio applications have strict latency requirements that can be difficult to meet in an async runtime environment due to potential preemption and scheduling delays.

## Decision
Use a hybrid threading model with a dedicated real-time thread for audio capture and an async task pool for processing.

## Status
Accepted

## Consequences
### Positive
- Isolates real-time audio capture from async runtime scheduling
- Prevents blocking and priority inversion issues
- Enables automatic recovery mechanisms
- Maintains clean separation of concerns

### Negative
- Increased complexity in thread communication
- Need for lock-free communication mechanisms
- Additional overhead in data transfer between threads

## Implementation
- Dedicated OS thread owns the CPAL input stream
- Async Tokio runtime handles VAD, STT, and injection processing
- Lock-free SPSC ring buffer (rtrb) for inter-thread communication
- Broadcast channels for configuration updates

## Related Documents
- `crates/coldvox-audio/src/capture.rs`
- `crates/coldvox-audio/src/ring_buffer.rs`

---
satisfies: COLDVOX-DOM2-001-audio-capture  
depends_on:  
supersedes:  
related_to: COLDVOX-SYS4-001-audio-capture-thread