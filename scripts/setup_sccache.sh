#!/usr/bin/env bash
set -euo pipefail

# Idempotent sccache setup for local dev and CI.
#
# - Installs sccache via `cargo install` if missing (version pinned by default).
# - Starts the sccache server (best-effort).
# - Exports RUSTC_WRAPPER in GitHub Actions if $GITHUB_ENV is set.

SCCACHE_VERSION="${SCCACHE_VERSION:-0.8.2}"

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
  if ! "${SCCACHE_BIN}" --version >/dev/null 2>&1; then
    log "ERROR: sccache is present but --version failed"
    exit 1
  fi
else
  log "Installing sccache via cargo (version=${SCCACHE_VERSION})..."
  cargo install sccache --version "${SCCACHE_VERSION}" --locked
  SCCACHE_BIN="$(find_sccache_bin)"
  log "✓ sccache installed: ${SCCACHE_BIN}"
  if ! "${SCCACHE_BIN}" --version >/dev/null 2>&1; then
    log "ERROR: sccache version check failed after install"
    exit 1
  fi
fi

# Start server (ok if it fails; sccache can lazily start)
"${SCCACHE_BIN}" --start-server >/dev/null 2>&1 || true

# Validate the sccache path before exporting to GITHUB_ENV
if [[ -n "${GITHUB_ENV:-}" ]]; then
  if [[ -x "${SCCACHE_BIN}" && "$(basename "${SCCACHE_BIN}")" == "sccache" ]]; then
    echo "RUSTC_WRAPPER=${SCCACHE_BIN}" >> "${GITHUB_ENV}"
    log "RUSTC_WRAPPER set via GITHUB_ENV"
  else
    log "ERROR: Invalid sccache path: ${SCCACHE_BIN}"
    exit 1
  fi
else
  log "To enable for this shell: export RUSTC_WRAPPER='${SCCACHE_BIN}'"
fi
