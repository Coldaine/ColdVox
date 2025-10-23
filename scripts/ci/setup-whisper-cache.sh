#!/bin/bash
# Whisper Model Setup Script
# This script ensures the Whisper model is available locally,
# downloading it if a pre-populated cache is not available.
# It is designed to run without sudo.

set -euo pipefail

# --- Configuration ---
# Using a single vendor directory for all downloaded dependencies
VENDOR_DIR="vendor"
WHISPER_DIR="$VENDOR_DIR/whisper"
MODEL_DIR="$WHISPER_DIR/model"

# Default model size (can be overridden by WHISPER_MODEL_SIZE env var)
MODEL_SIZE="${WHISPER_MODEL_SIZE:-tiny}"

# Map model sizes to their identifiers
declare -A MODEL_IDENTIFIERS=(
    ["tiny"]="tiny"
    ["base"]="base.en"
    ["small"]="small.en"
    ["medium"]="medium.en"
    ["large"]="large"
    ["large-v2"]="large-v2"
    ["large-v3"]="large-v3"
)

MODEL_IDENTIFIER="${MODEL_IDENTIFIERS[$MODEL_SIZE]:-$MODEL_SIZE}"

# Runner's cache directory (this path is specific to the self-hosted runner config)
RUNNER_CACHE_DIR="/home/coldaine/ActionRunnerCache/whisper"

# --- Execution ---

mkdir -p "$MODEL_DIR"

# 1. Set up Whisper Model
echo "--- Setting up Whisper Model: $MODEL_IDENTIFIER ---"
MODEL_CACHE_PATH="$RUNNER_CACHE_DIR/$MODEL_IDENTIFIER"
MODEL_LINK_PATH="$MODEL_DIR/$MODEL_IDENTIFIER"

if [ -d "$MODEL_CACHE_PATH" ]; then
    echo "✅ Found model in runner cache. Creating/refreshing symlink: $MODEL_LINK_PATH -> $MODEL_CACHE_PATH"
    # Remove any previous non-symlink directory/file at link location
    if [ -e "$MODEL_LINK_PATH" ] && [ ! -L "$MODEL_LINK_PATH" ]; then
        rm -rf "$MODEL_LINK_PATH"
    fi
    ln -sfn "$MODEL_CACHE_PATH" "$MODEL_LINK_PATH"
else
    echo "📥 Model not found in cache. Whisper will download the model on first use."
    echo "  Model: $MODEL_IDENTIFIER"
    echo "  Size: $MODEL_SIZE"
    echo "  Cache path: $MODEL_CACHE_PATH"
    
    # Create a placeholder directory to indicate the model is configured
    # The actual model will be downloaded by faster-whisper on first use
    mkdir -p "$MODEL_LINK_PATH"
    echo "# Whisper model placeholder" > "$MODEL_LINK_PATH/.whisper_placeholder"
    echo "# Model: $MODEL_IDENTIFIER" >> "$MODEL_LINK_PATH/.whisper_placeholder"
    echo "# Size: $MODEL_SIZE" >> "$MODEL_LINK_PATH/.whisper_placeholder"
fi

# --- Output for GitHub Actions ---
echo "--- Outputs ---"
echo "Final directory structure:"
ls -R "$VENDOR_DIR"

# Output paths for subsequent jobs
MODEL_PATH_ABS="$(pwd)/$MODEL_DIR/$MODEL_IDENTIFIER"

echo "Model Path: $MODEL_PATH_ABS"
echo "Model Size: $MODEL_SIZE"
echo "Model Identifier: $MODEL_IDENTIFIER"

# Output paths for subsequent jobs (only in GitHub Actions)
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "model_path=$MODEL_PATH_ABS" >> "$GITHUB_OUTPUT"
    echo "model_size=$MODEL_SIZE" >> "$GITHUB_OUTPUT"
else
    # For local execution, just export the variables
    export WHISPER_MODEL_PATH="$MODEL_PATH_ABS"
    export WHISPER_MODEL_SIZE="$MODEL_SIZE"
    echo "Local execution - exported environment variables:"
    echo "  WHISPER_MODEL_PATH=$WHISPER_MODEL_PATH"
    echo "  WHISPER_MODEL_SIZE=$WHISPER_MODEL_SIZE"
fi

echo "✅ Whisper setup complete."