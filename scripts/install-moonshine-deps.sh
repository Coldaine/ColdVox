#!/bin/bash
# Install Moonshine Python dependencies

set -e

echo "Installing Moonshine STT dependencies..."

# Check Python version
PYTHON_VERSION=$(python3 --version | cut -d' ' -f2 | cut -d'.' -f1-2)
REQUIRED_VERSION="3.8"

if [ "$(printf '%s\n' "$REQUIRED_VERSION" "$PYTHON_VERSION" | sort -V | head -n1)" != "$REQUIRED_VERSION" ]; then
    echo "Error: Python $REQUIRED_VERSION or higher required (found $PYTHON_VERSION)"
    exit 1
fi

echo "✓ Python $PYTHON_VERSION detected"

# Install packages
pip install --upgrade pip
pip install \
    transformers>=4.35.0 \
    torch>=2.0.0 \
    librosa>=0.10.0

# Verify installation
python3 -c "
import transformers
import torch
import librosa
print('✓ All dependencies installed successfully')
print(f'  transformers: {transformers.__version__}')
print(f'  torch: {torch.__version__}')
print(f'  librosa: {librosa.__version__}')
"

echo ""
echo "Moonshine dependencies ready!"
echo "Build ColdVox with: cargo build --features moonshine"
