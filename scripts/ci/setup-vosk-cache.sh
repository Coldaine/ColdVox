#!/bin/bash
# Vosk Model & Library Setup Script
# This script ensures the Vosk model and libvosk.so are available locally,
# downloading them if a pre-populated cache is not available.
# It is designed to run without sudo.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

# shellcheck source=../common_utils.sh
source "${ROOT_DIR}/common_utils.sh"
# shellcheck source=../config.env
source "${ROOT_DIR}/config.env"

log_step "Starting Vosk dependency provisioning"

# --- Execution ---

mkdir -p "$VOSK_MODEL_VENDOR_DIR"

# 1. Set up Vosk Model
log_step "Setting up Vosk Model: ${VOSK_MODEL_NAME}"

# Try primary cache location first, then fallback to alternate
MODEL_CACHE_PATH="$RUNNER_CACHE_DIR/${VOSK_MODEL_NAME}"
if [ ! -d "$MODEL_CACHE_PATH" ] && [ -d "$RUNNER_CACHE_DIR_ALT/${VOSK_MODEL_NAME}" ]; then
    log_info "Primary cache not found, using alternate: ${RUNNER_CACHE_DIR_ALT}"
    MODEL_CACHE_PATH="$RUNNER_CACHE_DIR_ALT/${VOSK_MODEL_NAME}"
fi

# Check if model exists in repo first (for local development)
if [ -d "${VOSK_MODEL_REPO_DIR}/graph" ]; then
    log_success "Found model in repo at '${VOSK_MODEL_REPO_DIR}'"
    MODEL_PATH_ABS="$(pwd)/${VOSK_MODEL_REPO_DIR}"
elif [ -d "$MODEL_CACHE_PATH" ]; then
    log_success "Found model in runner cache: $MODEL_CACHE_PATH"
    log_info "Using cache path directly (no workspace symlink needed)"
    # Store the cache path for output - this persists across job boundaries
    MODEL_PATH_ABS="$MODEL_CACHE_PATH"
else
    log_warn "Model not found in repo or cache. Downloading from ${VOSK_MODEL_URL}..."
    # Robust download with retries
    rm -f "${VOSK_MODEL_ARCHIVE}"
    if ! curl -fsSL --retry 3 --retry-delay 5 -o "${VOSK_MODEL_ARCHIVE}" "${VOSK_MODEL_URL}"; then
        log_error "Failed to download model zip from primary URL: ${VOSK_MODEL_URL}"
        exit 1
    fi

    log_info "Verifying checksum..."
    if ! echo "${VOSK_MODEL_SHA256}  ${VOSK_MODEL_ARCHIVE}" | sha256sum -c -; then
        log_warn "Checksum mismatch on first attempt. Showing diagnostics and retrying once..."
        log_debug "Computed sha256: $(sha256sum "${VOSK_MODEL_ARCHIVE}" 2>/dev/null || echo 'failed')"
        log_debug "File size (bytes): $(stat -c%s "${VOSK_MODEL_ARCHIVE}" 2>/dev/null || echo 'failed')"
        rm -f "${VOSK_MODEL_ARCHIVE}"
        if ! curl -fsSL --retry 3 --retry-delay 5 -o "${VOSK_MODEL_ARCHIVE}" "${VOSK_MODEL_URL}"; then
            log_error "Failed to re-download model zip."
            exit 1
        fi
        log_info "Re-verifying checksum..."
        echo "${VOSK_MODEL_SHA256}  ${VOSK_MODEL_ARCHIVE}" | sha256sum -c - || {
            log_error "Checksum mismatch persists for ${VOSK_MODEL_ARCHIVE}. Aborting with diagnostics."
            sha256sum "${VOSK_MODEL_ARCHIVE}" >&2 || true
            stat -c%s "${VOSK_MODEL_ARCHIVE}" >&2 || true
            exit 1
        }
    fi

    log_info "Extracting model..."
    unzip -q "${VOSK_MODEL_ARCHIVE}"
    # Move to cache directory instead of workspace (persists across jobs)
    mkdir -p "$RUNNER_CACHE_DIR_ALT"
    if [ -d "$RUNNER_CACHE_DIR_ALT/${VOSK_MODEL_NAME}" ]; then
        log_warn "Removing existing cached model"
        rm -rf "$RUNNER_CACHE_DIR_ALT/${VOSK_MODEL_NAME}"
    fi
    mv "${VOSK_MODEL_NAME}" "$RUNNER_CACHE_DIR_ALT/"
    rm "${VOSK_MODEL_ARCHIVE}"
    MODEL_PATH_ABS="$RUNNER_CACHE_DIR_ALT/${VOSK_MODEL_NAME}"
    log_success "Model downloaded and cached at $MODEL_PATH_ABS"
fi

# 2. Set up Vosk Library
log_step "Setting up Vosk Library v${VOSK_LIB_VERSION}"

# Try primary cache location first, then fallback to alternate
LIB_CACHE_FILE="$RUNNER_CACHE_DIR/lib/libvosk.so"
LIB_CACHE_FILE_ALT="$RUNNER_CACHE_DIR_ALT/lib/libvosk.so"
if [ ! -f "$LIB_CACHE_FILE" ] && [ -f "$LIB_CACHE_FILE_ALT" ]; then
    log_info "Primary cache not found, using alternate: $LIB_CACHE_FILE_ALT"
    LIB_CACHE_FILE="$LIB_CACHE_FILE_ALT"
fi

if [ -f "$LIB_CACHE_FILE" ]; then
    log_success "Found libvosk.so in runner cache: $LIB_CACHE_FILE"
    log_info "Using cache path directly (no workspace symlink needed)"
    # Store the cache directory for output - this persists across job boundaries
    LIB_PATH_ABS="$(dirname "$LIB_CACHE_FILE")"
else
    mkdir -p "$VOSK_LIB_VENDOR_DIR"
    log_warn "Library not found in cache. Downloading from ${VOSK_LIB_URL}..."
    rm -f "${VOSK_LIB_ARCHIVE}"
    if ! curl -fsSL --retry 3 --retry-delay 5 -o "${VOSK_LIB_ARCHIVE}" "${VOSK_LIB_URL}"; then
        log_error "Failed to download library zip from ${VOSK_LIB_URL}"
        exit 1
    fi

    log_info "Verifying checksum..."
    if ! echo "${VOSK_LIB_SHA256}  ${VOSK_LIB_ARCHIVE}" | sha256sum -c -; then
        log_warn "Library checksum mismatch on first attempt; retrying once..."
        rm -f "${VOSK_LIB_ARCHIVE}"
        if ! curl -fsSL --retry 3 --retry-delay 5 -o "${VOSK_LIB_ARCHIVE}" "${VOSK_LIB_URL}"; then
            log_error "Failed to re-download library zip."
            exit 1
        fi
        echo "${VOSK_LIB_SHA256}  ${VOSK_LIB_ARCHIVE}" | sha256sum -c - || {
            log_error "Library checksum mismatch persists for ${VOSK_LIB_ARCHIVE}."
            sha256sum "${VOSK_LIB_ARCHIVE}" >&2 || true
            stat -c%s "${VOSK_LIB_ARCHIVE}" >&2 || true
            exit 1
        }
    fi

    log_info "Extracting library..."
    unzip -q "${VOSK_LIB_ARCHIVE}"
    # Move to cache directory instead of workspace (persists across jobs)
    mkdir -p "$RUNNER_CACHE_DIR_ALT/lib"
    if [ -f "$RUNNER_CACHE_DIR_ALT/lib/libvosk.so" ]; then
        log_warn "Removing existing cached library"
        rm -f "$RUNNER_CACHE_DIR_ALT/lib/libvosk.so"
    fi
    mv "${VOSK_LIB_EXTRACT_DIR}/libvosk.so" "$RUNNER_CACHE_DIR_ALT/lib/"
    # Cleanup extracted folder and zip
    rm -r "${VOSK_LIB_EXTRACT_DIR}"
    rm "${VOSK_LIB_ARCHIVE}"
    LIB_PATH_ABS="$RUNNER_CACHE_DIR_ALT/lib"
    log_success "Library downloaded and cached at $LIB_PATH_ABS/libvosk.so"
fi

# --- Output for GitHub Actions ---
log_step "Outputs"

# Verify paths exist and are accessible
log_info "Verifying paths..."
if [ ! -d "$MODEL_PATH_ABS" ]; then
    log_error "Model path does not exist: $MODEL_PATH_ABS"
    exit 1
fi
if [ ! -d "$LIB_PATH_ABS" ]; then
    log_error "Library path does not exist: $LIB_PATH_ABS"
    exit 1
fi
if [ ! -f "$LIB_PATH_ABS/libvosk.so" ]; then
    log_error "libvosk.so not found in: $LIB_PATH_ABS"
    exit 1
fi

log_success "All paths verified"
log_info "Model Path: $MODEL_PATH_ABS"
log_info "Library Path: $LIB_PATH_ABS"

# Output cache paths (not workspace paths) for subsequent jobs
# These paths persist across job boundaries on self-hosted runners
echo "model_path=$MODEL_PATH_ABS" >> "$GITHUB_OUTPUT"
echo "lib_path=$LIB_PATH_ABS" >> "$GITHUB_OUTPUT"

log_success "Vosk setup complete."
