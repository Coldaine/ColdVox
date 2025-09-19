#!/usr/bin/env bash
set -euo pipefail

RUST_VERSION="1.75"

note() { echo "[deps] $*"; }

# Ensure correct toolchain is available/active
if command -v rustup >/dev/null 2>&1; then
  if ! rustup toolchain list | grep -q "${RUST_VERSION}"; then
    note "Installing Rust ${RUST_VERSION}"
    rustup toolchain install "${RUST_VERSION}" --profile minimal --component rustfmt --component clippy || true
  fi
  rustup default "${RUST_VERSION}" || true
fi

# Ensure hooks installed
if command -v pre-commit >/dev/null 2>&1; then
  if [[ ! -f .git/hooks/pre-commit ]]; then
    note "Installing pre-commit hooks"
    pre-commit install --install-hooks || true
  fi
fi

# Ensure cargo-nextest
if ! command -v cargo-nextest >/dev/null 2>&1; then
  if command -v cargo >/dev/null 2>&1; then
    note "Installing cargo-nextest"
    cargo install cargo-nextest --locked || true
  fi
fi

# Optional: warn if VOSK model missing
if [[ -z "${VOSK_MODEL_PATH:-}" && ! -d models/vosk-model-small-en-us-0.15 ]]; then
  note "VOSK model not detected; E2E tests may be skipped."
fi

exit 0

