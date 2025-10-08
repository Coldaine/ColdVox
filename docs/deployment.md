# ColdVox Deployment Guide

This document provides comprehensive guidance on deploying the ColdVox application, with a focus on the new centralized configuration system using TOML files and environment variable overrides. The configuration refactoring centralizes settings in `config/default.toml`, with support for environment-specific overrides via `COLDVOX__` prefixed variables (using `__` for nested sections).

## Prerequisites

- Rust toolchain (stable channel, MSRV 1.90)
- Cargo workspace setup (see `Cargo.toml`)
- Vosk model and dependencies (handled via CI scripts like `scripts/ci/setup-vosk-cache.sh`)
- GitHub Actions for CI/CD (see `.github/workflows/ci.yml`)

## Building the Application

### Local Build
1. Clone the repository and navigate to the project root.
2. Install dependencies: `cargo build --workspace --release`
3. For production: Use `--release` flag for optimized binaries.
   - Binary location: `target/release/coldvox-app` (or equivalent per crate).

### CI/CD Build Integration
- The CI workflow (`.github/workflows/ci.yml`) automatically builds on push/PR to main.
- Release automation uses `release-plz.toml` for changelog generation and tagging (prefix: `v`).
- Vosk setup is handled in `setup-vosk-dependencies` job via `scripts/ci/setup-vosk-cache.sh`.

## Configuration Handling in Deployments

### Including config/default.toml
- **Repository Inclusion**: Commit `config/default.toml` to the repository. It contains non-sensitive default values for all components (e.g., VAD thresholds, STT preferences, injection settings).
- **Build Inclusion**: The TOML file is not embedded in the binary; it is loaded at runtime from `config/default.toml` relative to the working directory. XDG_CONFIG_HOME support is not currently implemented.
  - In deployments, ensure `config/default.toml` is copied to the deployment directory (e.g., via Dockerfile or deployment script).
  - Example deployment script snippet:
    ```
    mkdir -p /opt/coldvox/config
    cp config/default.toml /opt/coldvox/config/
    cp target/release/coldvox-app /opt/coldvox/
    ```
- **Security**: Never commit secrets to `default.toml`. Use environment variables for overrides (see below).

### Environment-Specific Configurations
- **Development/Staging/Production Overrides**:
  - Use `config/overrides.toml` for environment-specific non-secret values. This file is not loaded by default; extend the config loader in `crates/app/src/main.rs` (Settings::new()) to merge it after `default.toml`.
  - Template for `overrides.toml` (add to `.gitignore` for local/prod use):
    ```toml
    # Environment-specific overrides (e.g., staging.toml or prod.toml)
    # Load via custom logic or env var COLDVOX_CONFIG_OVERRIDE_PATH=/path/to/overrides.toml

    [stt]
    preferred = "vosk"  # Staging: Use local model
    max_mem_mb = 1024

    [injection]
    fail_fast = true
    injection_mode = "paste"  # Prod: Prefer clipboard for reliability

    [vad]
    sensitivity = 0.8  # Staging: More aggressive detection
    ```
  - For secrets (API keys, paths): Use environment variables only.
    - Prefix: `COLDVOX__` (e.g., `COLDVOX_STT__PREFERRED=cloud_whisper` for prod cloud fallback).
    - Nested: `COLDVOX_INJECTION__MAX_TOTAL_LATENCY_MS=500`.
    - Full list: See [docs/user/runflags.md](docs/user/runflags.md).

- **Deployment Strategies**:
  - **Docker**: Mount `default.toml` as volume; inject env vars via `docker run -e COLDVOX_STT__LANGUAGE=en`.
  - **Systemd Service**: Set env vars in `/etc/systemd/system/coldvox.service` under `[Service]`:
    ```
    Environment="COLDVOX_INJECTION__ALLOW_ENIGO=true"
    EnvironmentFile=/etc/coldvox/prod.env
    ```
  - **Kubernetes**: Use ConfigMaps for TOML, Secrets for env vars.
  - **Separate TOML Files**: For multi-env, use `COLDVOX_CONFIG_PATH=/path/to/staging.toml` (extend loader if needed).

## Validation Steps on Deploy

1. **Config Parsing Check**:
   - Run: `cargo run -- --help` to verify flag/env integration.
   - Custom validation script (add to `scripts/deploy-validate.sh`):
     ```
     #!/bin/bash
     set -euo pipefail
     cargo run -- --dry-run  # If implemented; else manual check
     echo "Config loaded: $(cargo run -- --print-config)"  # Hypothetical flag
     ```

2. **Runtime Validation**:
   - Start app with logging: `RUST_LOG=debug cargo run --release`.
   - Check logs for config errors (e.g., invalid TOML, missing required fields).
   - Run integration tests: `cargo test -- --test-threads=1` (focus on config-dependent tests like VAD/STT setup).

3. **CI Integration for Validation**:
   - Update `.github/workflows/ci.yml` to include config tests in `build_and_check`:
     - Add step: `cargo test --test config_validation` (implement if missing).
   - In self-hosted runner (see [docs/self-hosted-runner-complete-setup.md](docs/self-hosted-runner-complete-setup.md)), ensure env vars are set in workflows for prod-like testing.

4. **Health Checks**:
   - Post-deploy: Query app health endpoint (if TUI/GUI enabled) or log `health` metrics from `crates/coldvox-foundation/src/health.rs`.

## Integration with Existing Systems

- **CI/CD (scripts/ci/)**: Vosk setup script (`setup-vosk-cache.sh`) runs before builds; extend for config copy/validation.
- **Release Process (release-plz.toml)**: Automated tagging doesn't handle config; manually include `config/default.toml` in release artifacts.
- **Self-Hosted Runner**: See [docs/self-hosted-runner-complete-setup.md](docs/self-hosted-runner-complete-setup.md) for env setup; add config mounting in runner jobs.
- **Documentation Links**:
  - Env vars/flags: [docs/user/runflags.md](docs/user/runflags.md)
  - Config security: [config/README.md](config/README.md)

## Rollback Procedures

1. **Config Rollback**:
   - Backup: Before deploy, `cp config/default.toml config/default.toml.bak-$(date +%Y%m%d)`.
   - Revert: Restore backup and restart service: `systemctl restart coldvox`.
   - Fallback to CLI: If TOML fails, use flags/env vars directly (e.g., `./coldvox-app --stt-preferred=vosk --injection-fail-fast`).

2. **Deployment Rollback**:
   - Git: `git checkout HEAD~1 -- config/` to revert config changes.
   - Binary: Keep previous release binary; switch via symlink or service file.
   - CI: Trigger rollback workflow (add to `.github/workflows/rollback.yml` if needed).
   - Validation: Re-run `cargo test` on previous commit to confirm.

3. **Emergency Fallback**:
   - Disable new config loader temporarily (comment in `crates/app/src/main.rs`).
   - Use old CLI params only: Document legacy flags in [docs/user/runflags.md](docs/user/runflags.md) for transition.

For issues, check logs and run `RUST_BACKTRACE=1` for detailed errors. Update this doc as deployment evolves.

<!-- ... existing code ... -->