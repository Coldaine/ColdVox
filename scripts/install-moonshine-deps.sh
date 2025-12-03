#!/bin/bash
# Install Moonshine Python dependencies

set -e

echo "Installing Moonshine STT dependencies..."

# Check Python version (using Python itself for portability)
if ! python3 -c "import sys; exit(0 if sys.version_info >= (3, 8) else 1)"; then
    echo "Error: Python 3.8 or higher required"
    exit 1
fi

PYTHON_VERSION=$(python3 --version | cut -d' ' -f2 | cut -d'.' -f1-2)
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
