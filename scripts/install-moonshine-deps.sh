#!/bin/bash
set -e

# Install Moonshine dependencies
echo "Installing Moonshine dependencies..."
pip install "transformers>=4.35.0" "torch>=2.0.0" "librosa>=0.10.0"

echo "Moonshine dependencies installed successfully."
