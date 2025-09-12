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
if [ -d "$RUNNER_CACHE_DIR/$MODEL_NAME" ]; then
    echo "✅ Found model in runner cache. Creating symlink."
    ln -sfn "$RUNNER_CACHE_DIR/$MODEL_NAME" "$MODEL_DIR"
else
    echo "📥 Model not found in cache. Downloading from $MODEL_URL..."
    wget -q -O "$MODEL_ZIP" "$MODEL_URL"
    
    echo "Verifying checksum..."
    echo "$MODEL_SHA256  $MODEL_ZIP" | sha256sum -c -
    
    echo "Extracting model..."
    unzip -q "$MODEL_ZIP"
    mv "$MODEL_NAME" "$MODEL_DIR/"
    rm "$MODEL_ZIP"
    echo "✅ Model downloaded and installed locally."
fi

# 2. Set up Vosk Library
echo "--- Setting up Vosk Library v$LIB_VERSION ---"
if [ -f "$RUNNER_CACHE_DIR/lib/libvosk.so" ]; then
    echo "✅ Found libvosk.so in runner cache. Creating symlink."
    ln -sfn "$RUNNER_CACHE_DIR/lib" "$LIB_DIR"
else
    mkdir -p "$LIB_DIR"
    echo "📥 Library not found in cache. Downloading from $LIB_URL..."
    wget -q -O "$LIB_ZIP" "$LIB_URL"

    echo "Verifying checksum..."
    echo "$LIB_SHA256  $LIB_ZIP" | sha256sum -c -

    echo "Extracting library..."
    unzip -q "$LIB_ZIP"
    # Move only the library file to the target lib dir
    mv "$LIB_EXTRACT_PATH/libvosk.so" "$LIB_DIR/"
    # Cleanup extracted folder and zip
    rm -r "$LIB_EXTRACT_PATH"
    rm "$LIB_ZIP"
    echo "✅ Library downloaded and installed locally."
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
