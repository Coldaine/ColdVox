#!/bin/bash
# Vosk Model Setup Script
# Extracted from .github/workflows/ci.yml for maintainability

set -euo pipefail

# Export LD_LIBRARY_PATH for Vosk
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
sudo ldconfig

# Use pre-cached models from permanent cache location (only on self-hosted)
CACHE_DIR="/home/coldaine/ActionRunnerCache/vosk-models"
MODEL_DIR="models"

mkdir -p $MODEL_DIR

# Check if we're on self-hosted runner with cache
if [ -d "$CACHE_DIR/vosk-model-small-en-us-0.15" ]; then
    # Self-hosted runner: use cached models
    rm -rf "$MODEL_DIR/vosk-model-small-en-us-0.15"
    ln -sf "$CACHE_DIR/vosk-model-small-en-us-0.15" "$MODEL_DIR/"
    echo "✅ Linked cached vosk-model-small-en-us-0.15"
    
    if [ -d "$CACHE_DIR/vosk-model-en-us-0.22" ]; then
        rm -rf "$MODEL_DIR/vosk-model-en-us-0.22"
        ln -sf "$CACHE_DIR/vosk-model-en-us-0.22" "$MODEL_DIR/"
        echo "✅ Linked cached vosk-model-en-us-0.22"
    fi
else
    # GitHub-hosted runner: download model
    echo "📥 Downloading Vosk model (cache not available)..."
    wget -q -O vosk-model-small-en-us-0.15.zip "https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip"
    unzip -q vosk-model-small-en-us-0.15.zip
    # If target already exists and is non-empty, remove it first (stale partial)
    if [ -d "$MODEL_DIR/vosk-model-small-en-us-0.15" ]; then
        if [ "$(ls -A $MODEL_DIR/vosk-model-small-en-us-0.15 2>/dev/null)" ]; then
            echo "⚠️  Removing existing non-empty stale directory: $MODEL_DIR/vosk-model-small-en-us-0.15"
            rm -rf "$MODEL_DIR/vosk-model-small-en-us-0.15"
        else
            rm -rf "$MODEL_DIR/vosk-model-small-en-us-0.15"
        fi
    fi
    # Move extracted directory into place
    if [ -d "vosk-model-small-en-us-0.15" ]; then
        mv "vosk-model-small-en-us-0.15" "$MODEL_DIR/"
    else
        echo "❌ Extracted model directory not found after unzip" >&2
        exit 1
    fi
    rm vosk-model-small-en-us-0.15.zip
    echo "✅ Downloaded vosk-model-small-en-us-0.15"
fi

echo ""
echo "Model directory contents:"
ls -la $MODEL_DIR/
echo ""
echo "✅ Model setup complete"

# Output model path for GitHub Actions
echo "MODEL_PATH=$(pwd)/models/vosk-model-small-en-us-0.15"