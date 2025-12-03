#!/bin/bash
set -e

echo "Verifying Moonshine STT setup..."

# Check Python version
if ! command -v python3 &> /dev/null; then
    echo "Error: python3 could not be found."
    exit 1
fi

# Check required Python packages
python3 -c "import transformers; import torch; import librosa; print('Python dependencies verified.')"

echo "Moonshine STT setup verification complete."
