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
RUNNER_CACHE_DIR="/home/coldaine/ActionRunnerCache/vosk"

# --- Execution ---

mkdir -p "$MODEL_DIR"

# 1. Set up Vosk Model
echo "--- Setting up Vosk Model: $MODEL_NAME ---"
MODEL_CACHE_PATH="$RUNNER_CACHE_DIR/$MODEL_NAME"
MODEL_LINK_PATH="$MODEL_DIR/$MODEL_NAME"
if [ -d "$REPO_MODEL_DIR/graph" ]; then
    echo "✅ Found model in repo at '$REPO_MODEL_DIR'. Creating/refreshing symlink: $MODEL_LINK_PATH -> $REPO_MODEL_DIR"
    if [ -e "$MODEL_LINK_PATH" ] && [ ! -L "$MODEL_LINK_PATH" ]; then
        rm -rf "$MODEL_LINK_PATH"
    fi
    ln -sfn "$(pwd)/$REPO_MODEL_DIR" "$MODEL_LINK_PATH"
elif [ -d "$MODEL_CACHE_PATH" ]; then
    echo "✅ Found model in runner cache. Creating/refreshing symlink: $MODEL_LINK_PATH -> $MODEL_CACHE_PATH"
    # Remove any previous non-symlink directory/file at link location
    if [ -e "$MODEL_LINK_PATH" ] && [ ! -L "$MODEL_LINK_PATH" ]; then
        rm -rf "$MODEL_LINK_PATH"
    fi
    ln -sfn "$MODEL_CACHE_PATH" "$MODEL_LINK_PATH"
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
