# Pre-Commit Hooks in ColdVox

**Last Updated:** September 16, 2025  

## Overview

Pre-commit hooks enforce code quality before commits, with full automation via `.pre-commit-config.yaml` (186 lines). Covers YAML linting, GitHub Actions validation, and local scripts (Vosk model verify, E2E STT WAV, GPU build). Installed via symlink (`.git/hooks/pre-commit` → custom script) + `pre-commit install`. `justfile` integrates for manual/local runs. This is comprehensive, running on every commit.

## Current Setup

### .pre-commit-config.yaml (Full, 186 lines)
- **Repos/Hooks:**  
  - Prettier (v3.1.0): YAML formatting (`--write`; deps: prettier@3.3.3).  
  - Yamllint (v1.35.1): YAML linting (.yamllint.yaml config; files: \.(yml|yaml)$).  
  - Pre-commit-hooks (v5.0.0): check-yaml (`--unsafe` for GH Actions), end-of-file-fixer, trailing-whitespace, check-merge-conflict, mixed-line-ending (`--fix=lf`).  
  - Actionlint (v1.7.7): GH workflow linting.  
  - Local (stages: pre-commit; pass_filenames: false):  
    - verify-vosk-model: `scripts/verify_vosk_model.sh` (model structure/SHA256).  
    - e2e-stt-wav: Vosk E2E WAV test (short transcription).  
    - gpu-build: `scripts/gpu-build-precommit.sh` (nvidia-smi >=12GB mem check; `cargo build --workspace` + `cargo test --no-run` if met; skips in CI; exits 0 skip/1 fail/2 unmet).  
    - injection-tests: Custom for text injection (via pre-commit-injection-tests script).  
- **Scope:** YAML/Rust/scripts/all files; automated on commit.  
- **Dependencies:** Node >=18 (prettier).

### justfile Integration
- **Commands:**  
  - `just lint`: `cargo fmt --all`, `cargo clippy --all-targets --locked -D warnings`, `cargo check --workspace --all-targets --locked`.  
  - `just check`: `pre-commit run --all-files` (triggers full config: YAML lint, Vosk verify, E2E, GPU).  
  - `just setup-hooks`: `pre-commit install` (enables Git hooks from config).  
  - `just commit-fast *args`: Skips Rust checks (`SKIP_RUST_CHECKS=1 git commit {{args}}`) for quick iterations.  
- **Scope:** Complements pre-commit with Cargo; manual but aligns.

### Scripts
- **setup_hooks.sh** (20 lines): Symlinks `.git/hooks/pre-commit` to `.git-hooks/pre-commit-injection-tests` (custom for injection/E2E); skippable (`COLDVOX_SKIP_HOOKS=1`).  
- **gpu-build-precommit.sh** (84 lines): GPU validation (nvidia-smi query, select first >=12GB; `cargo build/test-no-run`; skips if no NVIDIA or CI).  
- **local_ci.sh** (117 lines): Mirrors CI (colored: fmt/clippy/check/build/test; Vosk skip if no model via `--skip`).  

## Strengths
- **Automated and Comprehensive:** Full config auto-runs on commit (YAML/Actionlint, Vosk verify, E2E WAV, GPU build, injection tests).  
- **Niche Support:** Local hooks for hardware/model (Vosk SHA256, GPU mem check).  
- **Developer-Friendly:** `justfile` for manual; skips for speed; aligns with CI.  

## Weaknesses and Improvements
- **No CI Enforcement:** Local auto-runs but not in workflows (add `pre-commit/action@v5` in `ci.yml`).  
- **Coverage Gaps:** Strong YAML/Rust; add shellcheck (scripts), markdownlint (docs).  
  - **Fix:** Extend config:  
    ```yaml
    - repo: https://github.com/koalaman/shellcheck-precommit
      rev: v0.9.0
      hooks: [shellcheck]
    - repo: https://github.com/igorshubovitics/markdownlint-cli
      rev: v0.37.0
      hooks: [mdl]
    ```  
- **Portability:** Scripts assume Fedora/Fish; test Docker.  

## Usage
- **Setup:** `pre-commit install` (or `just setup-hooks`); runs on commit.  
- **Run:** `pre-commit run --all-files` (or `just check`).  
- **Bypass:** `COLDVOX_SKIP_HOOKS=1 git commit` or `just commit-fast`.  
- **CI:** Add enforcement to workflows for full pipeline.