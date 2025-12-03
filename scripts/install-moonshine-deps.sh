#!/bin/bash
# Install Moonshine Python dependencies using uv

set -euo pipefail

echo "Installing Moonshine STT dependencies..."

# Check if uv is available
if ! command -v uv &> /dev/null; then
    echo "Error: uv is not installed. Install with: curl -LsSf https://astral.sh/uv/install.sh | sh"
    exit 1
fi

echo "Using uv: $(uv --version)"

# Create/sync virtual environment from pyproject.toml
cd "$(dirname "$0")/.."
uv sync

# Verify installation
uv run python -c "
import transformers
import torch
import librosa
print('All dependencies installed successfully')
print(f'  transformers: {transformers.__version__}')
print(f'  torch: {torch.__version__}')
print(f'  librosa: {librosa.__version__}')
"

echo ""
echo "Moonshine dependencies ready!"
echo "Build ColdVox with: cargo build --features moonshine"
