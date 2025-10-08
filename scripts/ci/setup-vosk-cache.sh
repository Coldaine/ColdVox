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

# Vosk Model details - using large production-quality model
MODEL_NAME="vosk-model-en-us-0.22"
MODEL_URL="https://alphacephei.com/vosk/models/$MODEL_NAME.zip"
MODEL_ZIP="$MODEL_NAME.zip"
# Large model (1.8GB) - production quality, better accuracy than small model
MODEL_SHA256="47f9a81ebb039dbb0bd319175c36ac393c0893b796c2b6303e64cf58c27b69f6"

# Vosk Library details
LIB_VERSION="0.3.45"
LIB_ARCH="x86_64"
LIB_ZIP="vosk-linux-${LIB_ARCH}-${LIB_VERSION}.zip"
LIB_URL="https://github.com/alphacep/vosk-api/releases/download/v${LIB_VERSION}/${LIB_ZIP}"
LIB_SHA256="25c3c27c63b505a682833f44a1bde99a48b1088f682b3325789a454990a13b46"
LIB_EXTRACT_PATH="vosk-linux-${LIB_ARCH}-${LIB_VERSION}"

# Runner's cache directory (this path is specific to the self-hosted runner config)
# Try multiple possible cache locations (vosk-models is the actual location on this runner)
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

MODEL_LINK_PATH="$MODEL_DIR/$MODEL_NAME"
if [ -d "$MODEL_CACHE_PATH" ]; then
    echo "✅ Found model in runner cache: $MODEL_CACHE_PATH"
    echo "   Creating/refreshing symlink: $MODEL_LINK_PATH -> $MODEL_CACHE_PATH"
    # Remove any previous non-symlink directory/file at link location
    if [ -e "$MODEL_LINK_PATH" ] && [ ! -L "$MODEL_LINK_PATH" ]; then
        rm -rf "$MODEL_LINK_PATH"
    fi
    ln -sfn "$MODEL_CACHE_PATH" "$MODEL_LINK_PATH"
else
    echo "📥 Model not found in cache. Downloading from $MODEL_URL..."
    
    # Use wget if available, otherwise fallback to curl
    if command -v wget >/dev/null 2>&1; then
        wget -q --show-progress -O "$MODEL_ZIP" "$MODEL_URL" || wget -O "$MODEL_ZIP" "$MODEL_URL"
    elif command -v curl >/dev/null 2>&1; then
        curl -L --progress-bar -o "$MODEL_ZIP" "$MODEL_URL"
    else
        echo "ERROR: Neither wget nor curl found. Cannot download model." >&2
        exit 1
    fi

    echo "Verifying checksum..."
    if ! echo "$MODEL_SHA256  $MODEL_ZIP" | sha256sum -c -; then
        echo "ERROR: Checksum verification failed!" >&2
        echo "Expected: $MODEL_SHA256" >&2
        echo "Got:      $(sha256sum "$MODEL_ZIP" | cut -d' ' -f1)" >&2
        echo "This may indicate a corrupted download or upstream model change." >&2
        exit 1
    fi

    echo "Extracting model..."
    unzip -q "$MODEL_ZIP"
    # Ensure a clean target if something stale is present
    if [ -e "$MODEL_LINK_PATH" ]; then
        rm -rf "$MODEL_LINK_PATH"
    fi
    mv "$MODEL_NAME" "$MODEL_DIR/"
    rm "$MODEL_ZIP"
    echo "✅ Model downloaded and installed locally at $MODEL_LINK_PATH."
fi

# 2. Set up Vosk Library
echo "--- Setting up Vosk Library v$LIB_VERSION ---"

# Try multiple cache locations for libvosk
LIB_CACHE_FILE="$RUNNER_CACHE_DIR/lib/libvosk.so"
if [ ! -f "$LIB_CACHE_FILE" ]; then
    # Try alternate cache structure (libvosk-setup/vosk-linux-*/libvosk.so)
    if [ -f "/home/coldaine/ActionRunnerCache/libvosk-setup/$LIB_EXTRACT_PATH/libvosk.so" ]; then
        LIB_CACHE_FILE="/home/coldaine/ActionRunnerCache/libvosk-setup/$LIB_EXTRACT_PATH/libvosk.so"
        echo "ℹ️  Using libvosk from alternate cache: $LIB_CACHE_FILE"
    fi
fi

# Also check system-wide installation
if [ ! -f "$LIB_CACHE_FILE" ] && [ -f "/usr/local/lib/libvosk.so" ]; then
    echo "ℹ️  Using system-installed libvosk at /usr/local/lib/libvosk.so"
    LIB_CACHE_FILE="/usr/local/lib/libvosk.so"
fi

LIB_TARGET_FILE="$LIB_DIR/libvosk.so"
if [ -f "$LIB_CACHE_FILE" ]; then
    echo "✅ Found libvosk.so: $LIB_CACHE_FILE"
    echo "   Creating/refreshing symlink: $LIB_TARGET_FILE -> $LIB_CACHE_FILE"
    mkdir -p "$LIB_DIR"
    if [ -e "$LIB_TARGET_FILE" ] && [ ! -L "$LIB_TARGET_FILE" ]; then
        rm -f "$LIB_TARGET_FILE"
    fi
    ln -sfn "$LIB_CACHE_FILE" "$LIB_TARGET_FILE"
else
    mkdir -p "$LIB_DIR"
    echo "📥 Library not found in cache or system. Downloading from $LIB_URL..."
    
    # Use wget if available, otherwise fallback to curl
    if command -v wget >/dev/null 2>&1; then
        wget -q --show-progress -O "$LIB_ZIP" "$LIB_URL" || wget -O "$LIB_ZIP" "$LIB_URL"
    elif command -v curl >/dev/null 2>&1; then
        curl -L --progress-bar -o "$LIB_ZIP" "$LIB_URL"
    else
        echo "ERROR: Neither wget nor curl found. Cannot download library." >&2
        exit 1
    fi

    echo "Verifying checksum..."
    if ! echo "$LIB_SHA256  $LIB_ZIP" | sha256sum -c -; then
        echo "ERROR: Checksum verification failed!" >&2
        echo "Expected: $LIB_SHA256" >&2
        echo "Got:      $(sha256sum "$LIB_ZIP" | cut -d' ' -f1)" >&2
        exit 1
    fi

    echo "Extracting library..."
    unzip -q "$LIB_ZIP"
    # Ensure no stale file
    if [ -e "$LIB_TARGET_FILE" ]; then
        rm -f "$LIB_TARGET_FILE"
    fi
    mv "$LIB_EXTRACT_PATH/libvosk.so" "$LIB_DIR/"
    # Cleanup extracted folder and zip
    rm -r "$LIB_EXTRACT_PATH"
    rm "$LIB_ZIP"
    echo "✅ Library downloaded and installed locally at $LIB_TARGET_FILE."
fi

# --- Output for GitHub Actions ---
echo "--- Outputs ---"
echo "Final directory structure:"
ls -R "$VENDOR_DIR"

# Output paths for subsequent jobs
MODEL_PATH_ABS="$(pwd)/$MODEL_DIR/$MODEL_NAME"
LIB_PATH_ABS="$(pwd)/$LIB_DIR"

echo "Model Path: $MODEL_PATH_ABS"
echo "Library Path: $LIB_PATH_ABS"

# Output for GitHub Actions (only if running in CI)
if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "model_path=$MODEL_PATH_ABS" >> "$GITHUB_OUTPUT"
    echo "lib_path=$LIB_PATH_ABS" >> "$GITHUB_OUTPUT"
fi

echo "✅ Vosk setup complete."
