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

# Vosk Library details
LIB_VERSION="0.3.45"
LIB_ARCH="x86_64"
LIB_ZIP="vosk-linux-${LIB_ARCH}-${LIB_VERSION}.zip"
LIB_URL="https://github.com/alphacep/vosk-api/releases/download/v${LIB_VERSION}/${LIB_ZIP}"
LIB_SHA256="25c3c27c63b505a682833f44a1bde99a48b1088f682b3325789a454990a13b46"
LIB_EXTRACT_PATH="vosk-linux-${LIB_ARCH}-${LIB_VERSION}"

# Runner's cache directory (this path is specific to the self-hosted runner config)
RUNNER_CACHE_DIR="/home/coldaine/ActionRunnerCache/vosk"

# --- Execution ---

mkdir -p "$MODEL_DIR"

# 1. Set up Vosk Model
echo "--- Setting up Vosk Model: $MODEL_NAME ---"
MODEL_CACHE_PATH="$RUNNER_CACHE_DIR/$MODEL_NAME"
MODEL_LINK_PATH="$MODEL_DIR/$MODEL_NAME"
if [ -d "$MODEL_CACHE_PATH" ]; then
    echo "✅ Found model in runner cache. Creating/refreshing symlink: $MODEL_LINK_PATH -> $MODEL_CACHE_PATH"
    # Remove any previous non-symlink directory/file at link location
    if [ -e "$MODEL_LINK_PATH" ] && [ ! -L "$MODEL_LINK_PATH" ]; then
        rm -rf "$MODEL_LINK_PATH"
    fi
    ln -sfn "$MODEL_CACHE_PATH" "$MODEL_LINK_PATH"
else
    echo "📥 Model not found in cache. Downloading from $MODEL_URL..."

    # Retry download up to 3 times with 5s delay between attempts
    MAX_RETRIES=3
    RETRY_DELAY=5
    for attempt in $(seq 1 $MAX_RETRIES); do
        echo "Download attempt $attempt/$MAX_RETRIES..."
        if wget -q -O "$MODEL_ZIP" "$MODEL_URL"; then
            echo "Download successful."
            break
        else
            echo "Download failed."
            if [ $attempt -lt $MAX_RETRIES ]; then
                echo "Retrying in ${RETRY_DELAY}s..."
                sleep $RETRY_DELAY
            else
                echo "❌ Download failed after $MAX_RETRIES attempts."
                exit 1
            fi
        fi
    done

    echo "Verifying checksum..."
    COMPUTED_SHA256=$(sha256sum "$MODEL_ZIP" | awk '{print $1}')
    echo "Expected checksum: $MODEL_SHA256"
    echo "Computed checksum: $COMPUTED_SHA256"

    if [ "$COMPUTED_SHA256" != "$MODEL_SHA256" ]; then
        echo "❌ Checksum mismatch! Expected: $MODEL_SHA256, Got: $COMPUTED_SHA256"
        rm -f "$MODEL_ZIP"
        exit 1
    fi
    echo "✅ Checksum verified successfully."

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
LIB_CACHE_FILE="$RUNNER_CACHE_DIR/lib/libvosk.so"
LIB_TARGET_FILE="$LIB_DIR/libvosk.so"
if [ -f "$LIB_CACHE_FILE" ]; then
    echo "✅ Found libvosk.so in runner cache. Creating/refreshing symlink: $LIB_TARGET_FILE -> $LIB_CACHE_FILE"
    mkdir -p "$LIB_DIR"
    if [ -e "$LIB_TARGET_FILE" ] && [ ! -L "$LIB_TARGET_FILE" ]; then
        rm -f "$LIB_TARGET_FILE"
    fi
    ln -sfn "$LIB_CACHE_FILE" "$LIB_TARGET_FILE"
else
    mkdir -p "$LIB_DIR"
    echo "📥 Library not found in cache. Downloading from $LIB_URL..."

    # Retry download up to 3 times with 5s delay between attempts
    MAX_RETRIES=3
    RETRY_DELAY=5
    for attempt in $(seq 1 $MAX_RETRIES); do
        echo "Download attempt $attempt/$MAX_RETRIES..."
        if wget -q -O "$LIB_ZIP" "$LIB_URL"; then
            echo "Download successful."
            break
        else
            echo "Download failed."
            if [ $attempt -lt $MAX_RETRIES ]; then
                echo "Retrying in ${RETRY_DELAY}s..."
                sleep $RETRY_DELAY
            else
                echo "❌ Download failed after $MAX_RETRIES attempts."
                exit 1
            fi
        fi
    done

    echo "Verifying checksum..."
    COMPUTED_SHA256=$(sha256sum "$LIB_ZIP" | awk '{print $1}')
    echo "Expected checksum: $LIB_SHA256"
    echo "Computed checksum: $COMPUTED_SHA256"

    if [ "$COMPUTED_SHA256" != "$LIB_SHA256" ]; then
        echo "❌ Checksum mismatch! Expected: $LIB_SHA256, Got: $COMPUTED_SHA256"
        rm -f "$LIB_ZIP"
        exit 1
    fi
    echo "✅ Checksum verified successfully."

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

echo "model_path=$MODEL_PATH_ABS" >> "$GITHUB_OUTPUT"
echo "lib_path=$LIB_PATH_ABS" >> "$GITHUB_OUTPUT"

echo "✅ Vosk setup complete."
