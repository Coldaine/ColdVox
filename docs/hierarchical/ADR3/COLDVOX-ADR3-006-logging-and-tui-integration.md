---
id: COLDVOX-ADR3-006-logging-and-tui-integration
type: ADR
level: 3
title: Logging Architecture and TUI Integration
status: accepted
owner: @team-core
updated: 2025-09-11
parent: COLDVOX-DOM2-006-foundation
links:
  satisfies: [COLDVOX-DOM2-006-foundation]
  depends_on: []
  supersedes: []
  related_to: [COLDVOX-DOM2-008-gui]
---

## Context
ColdVox requires comprehensive logging for debugging and monitoring while supporting both CLI and TUI modes. The TUI dashboard must display real-time status without being corrupted by stderr output, requiring separate logging strategies for different modes.

## Decision
Use the `tracing` crate for structured logging with daily file rotation. Route all logging to files in TUI mode to prevent display corruption, while allowing stderr output in CLI mode. Implement log levels controlled by `RUST_LOG` environment variable or `--log-level` flag.

## Status
Accepted

## Consequences
### Positive
- Structured logging with contextual information
- Daily log rotation prevents unbounded file growth
- TUI mode avoids display corruption from stderr output
- Flexible log level control via environment or CLI flags
- Centralized logging configuration

### Negative
- File I/O overhead for logging
- TUI mode requires separate terminal for log viewing
- Increased complexity in logging configuration
- Disk space usage for log files

## Implementation
Logging architecture:
- Primary log file: `logs/coldvox.log` with daily rotation
- TUI mode: All logging to file only (`logs/coldvox.log`)
- CLI mode: Logging to both stderr and file
- Log levels: Controlled by `RUST_LOG` environment variable or `--log-level` flag
- Structured format: JSON or human-readable based on environment

TUI integration:
- TUI dashboard binary: `cargo run --bin tui_dashboard`
- Separate runtime from main application
- Shared state through IPC or shared memory
- Real-time status updates without stderr interference

## Alternatives Considered
1. Stderr only logging - Would corrupt TUI display
2. In-memory logging only - Would lose logs on crash
3. Syslog integration - Would add platform dependencies
4. Separate logging for each component - Would increase complexity

## Related Documents
- `crates/coldvox-foundation/src/logging.rs` (if it existed)
- `crates/app/src/bin/tui_dashboard.rs`
- `CLAUDE.md` (Logging section)
- `crates/app/Cargo.toml` (tracing dependencies)

---
satisfies: COLDVOX-DOM2-006-foundation  
depends_on:   
supersedes:   
related_to: COLDVOX-DOM2-008-gui