# ColdVox DevOps Documentation

**Last Updated:** September 16, 2025  
**Scope:** This folder documents the development operations (DevOps) setup for ColdVox, including CI/CD pipelines, pre-commit hooks, runner configuration, and security practices. It is based on the current state of the repository (branch: `stt-unification-refactor`).  

## Introduction

ColdVox uses GitHub Actions for CI/CD on self-hosted runners (Fedora/Nobara Linux x64), optimized for hardware-intensive tasks like audio processing and GPU-accelerated STT (e.g., Vosk). Local development mirrors CI via `justfile` and scripts in `/scripts/`.  

The setup is robust for solo development, with persistent local caching (Vosk models via `setup-vosk-cache.sh` with SHA256 verification), automated pre-commit hooks (full `.pre-commit-config.yaml` with YAML linting, actionlint, Vosk verify, E2E WAV, GPU build), MSRV matrix testing (stable/1.75 in `ci.yml`), and real E2E tests (Vosk WAV pipeline, injection with headless Xvfb/D-Bus). Selective Rust caching (`Swatinem/rust-cache@v2` in release/text_injection jobs). Minor gaps: No universal Cargo cache, no nightly matrix, no explicit security scans (e.g., cargo-deny in CI despite `deny.toml`), no CI pre-commit enforcement. This docs guide setup, troubleshooting, and improvements.

## Quick Setup for Contributors

1. **Install Dependencies:**  
   - Rust (stable): `rustup install stable`  
   - Just: `cargo install just` (for `justfile` commands)  
   - Pre-commit: `pip install pre-commit` (see [pre-commit-hooks.md](pre-commit-hooks.md) for full hooks)  
   - Vosk model: Download to `models/vosk-model-small-en-us-0.15/` (integrity via `SHA256SUMS`)  

2. **Local CI:** Run `just ci` or `./scripts/local_ci.sh` to mirror GitHub Actions (lints, tests, builds).  

3. **Runner Access:** For self-hosted CI, ensure your personal runner is online with labels `[self-hosted, Linux, X64, fedora, nobara]`.  

4. **Common Commands:**  
   - `just lint`: Format and clippy checks.  
   - `just test-full`: Full tests (skips E2E without Vosk model).  
   - `just check`: Pre-commit equivalent (runs YAML linting, Vosk verify, E2E WAV, GPU build).  

## File Structure

- **[ci-workflows.md](ci-workflows.md)**: GitHub Actions analysis (triggers, jobs, caching, matrix, E2E).  
- **[pre-commit-hooks.md](pre-commit-hooks.md)**: Full config details, local hooks (Vosk/GPU/E2E), and integration.  
- **[runner-setup.md](runner-setup.md)**: Self-hosted runner configuration and diagnostics.  
- **[security-and-best-practices.md](security-and-best-practices.md)**: Scans, risks, and recommendations.  

## Known Issues and Roadmap

- **Minor Gaps:** Selective Rust caching (add to all jobs); no nightly matrix; enforce `deny.toml` in CI.  
- **Strengths:** Persistent Vosk caching (local script, no re-downloads); automated pre-commit (E2E on commit); real E2E tests (Vosk WAV pipeline).  
- **Future:** Universal caching, security scans, coverage reports.  

See reviews in `/docs/reviews/` for detailed critiques. Contribute via PRs to this folder.