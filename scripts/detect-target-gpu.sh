#!/bin/bash
# GPU Detection Script for Pre-commit Hooks
# Returns exit code 0 if any NVIDIA GPU detected, 1 otherwise

set -euo pipefail

# Check if nvidia-smi is available
if ! command -v nvidia-smi &> /dev/null; then
    echo "nvidia-smi not found - no NVIDIA GPU detected" >&2
    exit 1
fi

# Get GPU names
gpu_names=$(nvidia-smi --query-gpu=name --format=csv,noheader,nounits 2>/dev/null || echo "")

if [[ -z "$gpu_names" ]]; then
    echo "No NVIDIA GPUs detected" >&2
    exit 1
fi

# Check for any NVIDIA GPU
if echo "$gpu_names" | grep -qi "nvidia"; then
    echo "NVIDIA GPU detected: $(echo "$gpu_names" | head -1)"
    exit 0
else
    echo "No NVIDIA GPU detected. Found: $gpu_names" >&2
    exit 1
fi
