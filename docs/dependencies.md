---
doc_type: reference
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-12-12
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

## Mixed Rust + Python Tooling (Dec 2025)

- **uv (Python 3.13-ready)**: Use `uv` for Python env + lockfiles; leverage its global cache to keep CI downloads minimal across runners. Treat `uv lock` as the single source of truth for Python deps; prefer `uv tool install` for CLI tools (ruff, maturin).
- **maturin for packaging**: Build PyO3 wheels with `maturin build -r` and install in CI with `maturin develop` when you need editable bindings. Prefer `uv tool run maturin ...` so we do not rely on system pip. Follow PyO3 0.27 guidance for free-threaded Python 3.13 by enabling the `abi3-py313` or interpreter-specific feature in the binding crates as needed.
- **Rust/Python interface hygiene**: Avoid `pyo3` debug builds in CI; set `PYO3_CONFIG_FILE` only when linking against non-system Python. For embedded Python, ensure `python3-devel` is present on self-hosted runners and keep `extension-module` + `auto-initialize` features constrained to the crates that need them.
- **Shared caching**: Keep `target/` out of VCS; rely on Swatinem `rust-cache` in GitHub Actions plus `uv` global cache for Python wheels.

### sccache (Rust Build Cache)

sccache caches Rust compilation artifacts across builds, significantly reducing rebuild times for unchanged code.

**Installation (one-time on self-hosted runner)**:
```bash
just setup-sccache
# Or manually: cargo install sccache --locked
```

**CI Integration**: The CI workflow automatically:
1. Checks if sccache is available on the runner
2. If found, starts the sccache server and sets `RUSTC_WRAPPER`
3. If not found, builds proceed normally (no failure)

**Local Development**:
```bash
# Add to ~/.bashrc or ~/.zshrc
export RUSTC_WRAPPER=sccache
export SCCACHE_DIR=~/.cache/sccache

# Check stats
sccache --show-stats
```

**Cache Location**: `~/.cache/sccache` (configurable via `SCCACHE_DIR`)

**Expected Savings**: 30-60% reduction in incremental build times on the self-hosted runner.

## CI Gating Expectations

- **Security gates on every PR**: Run `cargo deny check` and `cargo audit` as blocking jobs (they are already configured in CI—make them non-optional locally via `just lint` or `scripts/local_ci.sh`).
- **Non-dummy end-to-end**: Hardware-backed E2E jobs (audio/VAD/STT/text injection) must execute on the self-hosted runner with real devices; remove or avoid placeholders. Use sharded CI so only that job targets the self-hosted runner while unit/integration suites run on hosted Linux.
- **Cache-aware sharding**: Reuse the cargo cache populated on the self-hosted runner across the E2E job; keep other jobs on hosted runners to reduce queue time. If possible, warm the cache with a `cargo build --locked` step before running E2E to minimize device occupancy.
- **Python/Rust lock discipline**: Keep `uv.lock` and `Cargo.lock` in sync with feature flags and PyO3 ABI choices; treat lock drift as a failing check in pre-commit/CI.
