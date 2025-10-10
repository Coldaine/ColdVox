#!/bin/bash
# Vosk Model & Library Setup Script
# This script ensures the Vosk model and libvosk.so are available locally,
# downloading them if a pre-populated cache is not available.
# It is designed to run without sudo.

set -euo pipefail

# --- Configuration ---
# Using a single vendor directory for all downloaded dependencies
VENDOR_DIR="vendor"
VOSK_DIR="$VENDOR_DIR/vosk"
MODEL_DIR="$VOSK_DIR/model"
LIB_DIR="$VOSK_DIR/lib"

# Vosk Model details
MODEL_NAME="vosk-model-small-en-us-0.15"
MODEL_URL="https://alphacephei.com/vosk/models/$MODEL_NAME.zip"
MODEL_ZIP="$MODEL_NAME.zip"
MODEL_SHA256="57919d20a3f03582a7a5b754353b3467847478b7d4b3ed2a3495b545448a44b9"
REPO_MODEL_DIR="models/$MODEL_NAME"

# Vosk Library details
LIB_VERSION="0.3.45"
LIB_ARCH="x86_64"
LIB_ZIP="vosk-linux-${LIB_ARCH}-${LIB_VERSION}.zip"
LIB_URL="https://github.com/alphacep/vosk-api/releases/download/v${LIB_VERSION}/${LIB_ZIP}"
LIB_SHA256="bbdc8ed85c43979f6443142889770ea95cbfbc56cffb5c5dcd73afa875c5fbb2"
LIB_EXTRACT_PATH="vosk-linux-${LIB_ARCH}-${LIB_VERSION}"

# Runner's cache directory (this path is specific to the self-hosted runner config)
# Try multiple possible cache locations
RUNNER_CACHE_DIR="${RUNNER_CACHE_DIR:-/home/coldaine/ActionRunnerCache/vosk}"
RUNNER_CACHE_DIR_ALT="/home/coldaine/ActionRunnerCache/vosk-models"

# --- Execution ---

mkdir -p "$MODEL_DIR"

# 1. Set up Vosk Model
echo "--- Setting up Vosk Model: $MODEL_NAME ---"

# Try primary cache location first, then fallback to alternate
MODEL_CACHE_PATH="$RUNNER_CACHE_DIR/$MODEL_NAME"
if [ ! -d "$MODEL_CACHE_PATH" ] && [ -d "$RUNNER_CACHE_DIR_ALT/$MODEL_NAME" ]; then
    echo "ℹ️  Primary cache not found, using alternate: $RUNNER_CACHE_DIR_ALT"
    MODEL_CACHE_PATH="$RUNNER_CACHE_DIR_ALT/$MODEL_NAME"
fi

# Check if model exists in repo first (for local development)
if [ -d "$REPO_MODEL_DIR/graph" ]; then
    echo "✅ Found model in repo at '$REPO_MODEL_DIR'"
    MODEL_PATH_ABS="$(pwd)/$REPO_MODEL_DIR"
elif [ -d "$MODEL_CACHE_PATH" ]; then
    echo "✅ Found model in runner cache: $MODEL_CACHE_PATH"
    echo "   Using cache path directly (no workspace symlink needed)"
    # Store the cache path for output - this persists across job boundaries
    MODEL_PATH_ABS="$MODEL_CACHE_PATH"
else
    echo "📥 Model not found in repo or cache. Downloading from $MODEL_URL..."
    # Robust download with retries
    rm -f "$MODEL_ZIP"
    if ! curl -fsSL --retry 3 --retry-delay 5 -o "$MODEL_ZIP" "$MODEL_URL"; then
        echo "❌ Failed to download model zip from primary URL: $MODEL_URL" >&2
        exit 1
    fi

    echo "Verifying checksum..."
    if ! echo "$MODEL_SHA256  $MODEL_ZIP" | sha256sum -c -; then
        echo "⚠️ Checksum mismatch on first attempt. Showing diagnostics and retrying once..." >&2
        echo "Computed sha256:" >&2
        sha256sum "$MODEL_ZIP" >&2 || true
        echo "File size (bytes):" >&2
        stat -c%s "$MODEL_ZIP" >&2 || true
        rm -f "$MODEL_ZIP"
        if ! curl -fsSL --retry 3 --retry-delay 5 -o "$MODEL_ZIP" "$MODEL_URL"; then
            echo "❌ Failed to re-download model zip." >&2
            exit 1
        fi
        echo "Re-verifying checksum..."
        echo "$MODEL_SHA256  $MODEL_ZIP" | sha256sum -c - || {
            echo "❌ Checksum mismatch persists for $MODEL_ZIP. Aborting with diagnostics." >&2
            sha256sum "$MODEL_ZIP" >&2 || true
            stat -c%s "$MODEL_ZIP" >&2 || true
            exit 1
        }
    fi

    echo "Extracting model..."
    unzip -q "$MODEL_ZIP"
    # Move to cache directory instead of workspace (persists across jobs)
    mkdir -p "$RUNNER_CACHE_DIR_ALT"
    if [ -d "$RUNNER_CACHE_DIR_ALT/$MODEL_NAME" ]; then
        echo "⚠️  Removing existing cached model"
        rm -rf "$RUNNER_CACHE_DIR_ALT/$MODEL_NAME"
    fi
    mv "$MODEL_NAME" "$RUNNER_CACHE_DIR_ALT/"
    rm "$MODEL_ZIP"
    MODEL_PATH_ABS="$RUNNER_CACHE_DIR_ALT/$MODEL_NAME"
    echo "✅ Model downloaded and cached at $MODEL_PATH_ABS"
fi

# 2. Set up Vosk Library
echo "--- Setting up Vosk Library v$LIB_VERSION ---"

# Try primary cache location first, then fallback to alternate
LIB_CACHE_FILE="$RUNNER_CACHE_DIR/lib/libvosk.so"
LIB_CACHE_FILE_ALT="$RUNNER_CACHE_DIR_ALT/lib/libvosk.so"
if [ ! -f "$LIB_CACHE_FILE" ] && [ -f "$LIB_CACHE_FILE_ALT" ]; then
    echo "ℹ️  Primary cache not found, using alternate: $LIB_CACHE_FILE_ALT"
    LIB_CACHE_FILE="$LIB_CACHE_FILE_ALT"
fi

if [ -f "$LIB_CACHE_FILE" ]; then
    echo "✅ Found libvosk.so in runner cache: $LIB_CACHE_FILE"
    echo "   Using cache path directly (no workspace symlink needed)"
    # Store the cache directory for output - this persists across job boundaries
    LIB_PATH_ABS="$(dirname "$LIB_CACHE_FILE")"
else
    mkdir -p "$LIB_DIR"
    echo "📥 Library not found in cache. Downloading from $LIB_URL..."
    rm -f "$LIB_ZIP"
    if ! curl -fsSL --retry 3 --retry-delay 5 -o "$LIB_ZIP" "$LIB_URL"; then
        echo "❌ Failed to download library zip from $LIB_URL" >&2
        exit 1
    fi

    echo "Verifying checksum..."
    if ! echo "$LIB_SHA256  $LIB_ZIP" | sha256sum -c -; then
        echo "⚠️ Library checksum mismatch on first attempt; retrying once..." >&2
        rm -f "$LIB_ZIP"
        if ! curl -fsSL --retry 3 --retry-delay 5 -o "$LIB_ZIP" "$LIB_URL"; then
            echo "❌ Failed to re-download library zip." >&2
            exit 1
        fi
        echo "$LIB_SHA256  $LIB_ZIP" | sha256sum -c - || {
            echo "❌ Library checksum mismatch persists for $LIB_ZIP." >&2
            sha256sum "$LIB_ZIP" >&2 || true
            stat -c%s "$LIB_ZIP" >&2 || true
            exit 1
        }
    fi

    echo "Extracting library..."
    unzip -q "$LIB_ZIP"
    # Move to cache directory instead of workspace (persists across jobs)
    mkdir -p "$RUNNER_CACHE_DIR_ALT/lib"
    if [ -f "$RUNNER_CACHE_DIR_ALT/lib/libvosk.so" ]; then
        echo "⚠️  Removing existing cached library"
        rm -f "$RUNNER_CACHE_DIR_ALT/lib/libvosk.so"
    fi
    mv "$LIB_EXTRACT_PATH/libvosk.so" "$RUNNER_CACHE_DIR_ALT/lib/"
    # Cleanup extracted folder and zip
    rm -r "$LIB_EXTRACT_PATH"
    rm "$LIB_ZIP"
    LIB_PATH_ABS="$RUNNER_CACHE_DIR_ALT/lib"
    echo "✅ Library downloaded and cached at $LIB_PATH_ABS/libvosk.so"
fi

# --- Output for GitHub Actions ---
echo "--- Outputs ---"

# Verify paths exist and are accessible
echo "Verifying paths..."
if [ ! -d "$MODEL_PATH_ABS" ]; then
    echo "❌ ERROR: Model path does not exist: $MODEL_PATH_ABS" >&2
    exit 1
fi
if [ ! -d "$LIB_PATH_ABS" ]; then
    echo "❌ ERROR: Library path does not exist: $LIB_PATH_ABS" >&2
    exit 1
fi
if [ ! -f "$LIB_PATH_ABS/libvosk.so" ]; then
    echo "❌ ERROR: libvosk.so not found in: $LIB_PATH_ABS" >&2
    exit 1
fi

echo "✅ All paths verified"
echo "Model Path: $MODEL_PATH_ABS"
echo "Library Path: $LIB_PATH_ABS"

# Output cache paths (not workspace paths) for subsequent jobs
# These paths persist across job boundaries on self-hosted runners
echo "model_path=$MODEL_PATH_ABS" >> "$GITHUB_OUTPUT"
echo "lib_path=$LIB_PATH_ABS" >> "$GITHUB_OUTPUT"

echo "✅ Vosk setup complete."
