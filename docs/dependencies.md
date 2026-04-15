---
doc_type: reference
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-12-03
---

# Dependency Overview

This document tracks runtime and tooling dependencies for ColdVox.

## Runtime

See `Cargo.toml` in each crate for Rust dependencies. Key runtime dependencies:

- **Audio**: `cpal` (cross-platform audio), `rtrb` (ring buffer)
- **VAD**: `voice_activity_detector` (Silero ONNX VAD)
- **STT**: `parakeet-rs` (NVIDIA Parakeet), `pyo3` (Python bindings for Moonshine)
- **Text Injection**: `enigo`, `atspi`, `wl-clipboard` bindings
- **TUI**: `ratatui`, `crossterm`

## Tooling

### Security Scanning

ColdVox uses two security tools that run in CI:

#### cargo-audit

Scans `Cargo.lock` for crates with known security vulnerabilities from the [RustSec Advisory Database](https://rustsec.org/).

```bash
# Install
cargo install cargo-audit

# Run
cargo audit
```

#### cargo-deny

Comprehensive dependency linting: licenses, bans, advisories, and sources. Configuration in `deny.toml`.

```bash
# Install
cargo install cargo-deny

# Run all checks
cargo deny check

# Run specific check
cargo deny check licenses
cargo deny check advisories
cargo deny check bans
```

### deny.toml Configuration

The `deny.toml` file in the repository root configures cargo-deny:

- **[licenses]**: Allowed licenses (MIT, Apache-2.0, BSD-3-Clause, etc.)
- **[licenses.private]**: Ignores unpublished workspace crates
- **[bans]**: Crate banning rules (currently empty, logs duplicates as warnings)
- **[advisories]**: Security advisory handling (ignores unmaintained crates with no security impact)
- **[sources]**: Warns on unknown registries or git sources

### Other Tooling

- **rustfmt**: Code formatting (`cargo fmt`)
- **clippy**: Linting (`cargo clippy`)
- **uv**: Python dependency management for STT plugins
