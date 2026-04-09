#!/bin/bash
# Verify STT plugin setup

set -e

echo "Verifying ColdVox STT setup..."
echo ""

# Check Moonshine CPU
echo "=== Moonshine CPU ==="
if python3 -c "import transformers, torch, librosa" 2>/dev/null; then
    echo "✓ Python dependencies installed"
else
    echo "✗ Python dependencies missing"
    echo "  Run: ./scripts/install-moonshine-deps.sh"
fi

# Check Parakeet GPU
echo ""
echo "=== Parakeet GPU ==="
if command -v nvidia-smi &> /dev/null; then
    if nvidia-smi &> /dev/null; then
        echo "✓ CUDA GPU detected"
        nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
    else
        echo "✗ nvidia-smi failed"
    fi
else
    echo "○ No GPU detected (CPU-only mode)"
fi

# Build test
echo ""
echo "=== Build Test ==="
if cargo build --features moonshine; then
    echo "✓ Build with moonshine feature succeeded"
else
    echo "✗ Build failed"
    exit 1
fi

echo ""
echo "Setup verification complete!"
