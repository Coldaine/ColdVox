#!/usr/bin/env bash
set -euo pipefail

# Pinned versions (keep in sync with .tool-versions and rust-toolchain.toml)
RUST_VERSION="1.90.0"
JUST_VERSION="1.15.0"
PRECOMMIT_VERSION="3.5.0"
NEXTEST_VERSION="0.9.67"

log() { echo "[setup $(date +'%H:%M:%S')] $*"; }
err() { echo "[setup] ERROR: $*" >&2; }

# 1) Ensure rustup + toolchain
if command -v rustup >/dev/null 2>&1; then
  log "Installing Rust toolchain ${RUST_VERSION} (if needed)"
  rustup toolchain install "${RUST_VERSION}" --profile minimal --component rustfmt --component clippy || true
  rustup default "${RUST_VERSION}" || true
else
  err "rustup not found. Install Rust: https://rustup.rs/"
fi

# 2) Ensure just
if ! command -v just >/dev/null 2>&1; then
  if command -v cargo >/dev/null 2>&1; then
    log "Installing just ${JUST_VERSION}"
    cargo install just --version "${JUST_VERSION}" || true
  else
    err "cargo not found; cannot install 'just' automatically"
  fi
else
  log "just present: $(just --version | awk '{print $2}')"
fi

# 3) Ensure pre-commit
if ! command -v pre-commit >/dev/null 2>&1; then
  if command -v pip >/dev/null 2>&1; then
    log "Installing pre-commit ${PRECOMMIT_VERSION}"
    pip install --user "pre-commit==${PRECOMMIT_VERSION}" || true
  else
    err "pip not found; cannot install pre-commit automatically"
  fi
else
  log "pre-commit present: $(pre-commit --version | awk '{print $3}')"
fi

# 4) Install hooks
if command -v pre-commit >/dev/null 2>&1; then
  log "Installing git hooks via pre-commit"
  pre-commit install --install-hooks || true
  pre-commit install --hook-type commit-msg || true
  pre-commit install --hook-type pre-push || true
fi

# 5) Ensure cargo-nextest
if ! command -v cargo-nextest >/dev/null 2>&1; then
  if command -v cargo >/dev/null 2>&1; then
    log "Installing cargo-nextest ${NEXTEST_VERSION}"
    cargo install cargo-nextest --locked --version "${NEXTEST_VERSION}" || true
  fi
fi

# 6) Pre-fetch dependencies for speed
if command -v cargo >/dev/null 2>&1; then
  log "Fetching Cargo dependencies"
  cargo fetch || true
fi

# 7) Vosk model hint
if [[ ! -d "models/vosk-model-small-en-us-0.15" ]]; then
  log "Vosk model not found. Set VOSK_MODEL_PATH or download a model into models/"
fi

log "Setup complete. For text-injection (requires sudo), run: scripts/setup_text_injection.sh"

