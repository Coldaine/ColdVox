#!/usr/bin/env bash
set -euo pipefail

# Idempotent sccache setup for local dev and CI.
#
# - Installs sccache via `cargo install` if missing.
# - Starts the sccache server (best-effort).
# - Exports RUSTC_WRAPPER in GitHub Actions if $GITHUB_ENV is set.

log() {
  echo "[setup_sccache] $*"
}

find_sccache_bin() {
  if command -v sccache >/dev/null 2>&1; then
    command -v sccache
    return 0
  fi

  if [[ -x "${HOME}/.cargo/bin/sccache" ]]; then
    echo "${HOME}/.cargo/bin/sccache"
    return 0
  fi

  return 1
}

if SCCACHE_BIN="$(find_sccache_bin 2>/dev/null)"; then
  log "✓ sccache already installed: ${SCCACHE_BIN}"
  "${SCCACHE_BIN}" --version || true
else
  log "Installing sccache via cargo…"
  cargo install sccache --locked
  SCCACHE_BIN="$(find_sccache_bin)"
  log "✓ sccache installed: ${SCCACHE_BIN}"
  "${SCCACHE_BIN}" --version || true
fi

# Start server (ok if it fails; sccache can lazily start)
"${SCCACHE_BIN}" --start-server >/dev/null 2>&1 || true

if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "RUSTC_WRAPPER=${SCCACHE_BIN}" >> "${GITHUB_ENV}"
  log "RUSTC_WRAPPER set via GITHUB_ENV"
else
  log "To enable for this shell: export RUSTC_WRAPPER='${SCCACHE_BIN}'"
fi
