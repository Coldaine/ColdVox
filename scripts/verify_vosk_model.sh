#!/bin/bash
set -euo pipefail

MODEL_PATH="${1:-models/vosk-model-small-en-us-0.15}"

# Check required subdirectories
for subdir in am conf graph ivector; do
    if [[ ! -d "$MODEL_PATH/$subdir" ]]; then
        echo "ERROR: Missing required subdirectory: $MODEL_PATH/$subdir"
        exit 1
    fi
done

echo "✓ Vosk model structure verified at $MODEL_PATH"
