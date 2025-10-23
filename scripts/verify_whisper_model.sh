#!/bin/bash
# Verify Whisper Model Structure
# This script verifies that the Whisper model directory structure is correct

set -euo pipefail

# Default model path (can be overridden by WHISPER_MODEL_PATH env var)
MODEL_PATH="${WHISPER_MODEL_PATH:-vendor/whisper/model/tiny}"

echo "Verifying Whisper model structure at: $MODEL_PATH"

# Check if model directory exists
if [[ ! -d "$MODEL_PATH" ]]; then
    echo "❌ Model directory not found: $MODEL_PATH"
    exit 1
fi

# Check if it's a symlink to the cache
if [[ -L "$MODEL_PATH" ]]; then
    echo "✅ Model is a symlink to cache"
    # Resolve the symlink
    REAL_PATH=$(readlink -f "$MODEL_PATH")
    echo "   Resolves to: $REAL_PATH"
    
    # Check if the target exists
    if [[ ! -d "$REAL_PATH" ]]; then
        echo "❌ Symlink target does not exist: $REAL_PATH"
        exit 1
    fi
else
    echo "⚠️  Model is not a symlink (might be a placeholder)"
fi

# Check for placeholder file (indicates model is configured but not downloaded)
if [[ -f "$MODEL_PATH/.whisper_placeholder" ]]; then
    echo "ℹ️  Model placeholder found - model will be downloaded on first use"
    cat "$MODEL_PATH/.whisper_placeholder"
else
    echo "ℹ️  No placeholder found - model might already be downloaded"
fi

echo "✅ Whisper model structure verification complete"