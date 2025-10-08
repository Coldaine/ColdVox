#!/bin/bash
# Test script to verify Vosk setup works (mimics CI workflow steps)
set -euo pipefail

echo "========================================"
echo "Vosk Setup Verification Test"
echo "========================================"

cd "$(dirname "$0")"
PROJECT_ROOT="$(pwd)"

# Step 1: Run setup script
echo ""
echo "Step 1: Running setup-vosk-cache.sh..."
bash scripts/ci/setup-vosk-cache.sh

# Step 2: Verify vendor directory structure
echo ""
echo "Step 2: Verifying vendor directory structure..."
if [ ! -L "$PROJECT_ROOT/vendor/vosk/model/vosk-model-en-us-0.22" ]; then
    echo "❌ Model symlink missing"
    exit 1
fi
if [ ! -L "$PROJECT_ROOT/vendor/vosk/lib/libvosk.so" ]; then
    echo "❌ Library symlink missing"
    exit 1
fi
echo "✅ Vendor structure OK"

# Step 3: Set environment like CI does
export VOSK_MODEL_PATH="$PROJECT_ROOT/vendor/vosk/model/vosk-model-en-us-0.22"
export LD_LIBRARY_PATH="$PROJECT_ROOT/vendor/vosk/lib:${LD_LIBRARY_PATH:-}"

echo ""
echo "Step 3: Environment variables set:"
echo "  VOSK_MODEL_PATH=$VOSK_MODEL_PATH"
echo "  LD_LIBRARY_PATH=$LD_LIBRARY_PATH"

# Step 4: Verify model directory is accessible
echo ""
echo "Step 4: Verifying model accessibility..."
if [ ! -d "$VOSK_MODEL_PATH" ]; then
    echo "❌ Model directory not accessible"
    exit 1
fi
echo "Model directory contents:"
ls -lh "$VOSK_MODEL_PATH" | head -10
echo "✅ Model accessible"

# Step 5: Build Vosk components (like CI does)
echo ""
echo "Step 5: Building Vosk components..."
echo "Building coldvox-stt-vosk..."
cargo build --locked -p coldvox-stt-vosk --features vosk --quiet
echo "✅ coldvox-stt-vosk builds successfully"

# Step 6: Run Vosk unit tests
echo ""
echo "Step 6: Running Vosk unit tests..."
cargo test --locked -p coldvox-stt-vosk --features vosk --lib -- --test-threads=1 --quiet
echo "✅ Vosk unit tests pass"

# Step 7: Quick model validation
echo ""
echo "Step 7: Model structure validation..."
for subdir in am conf graph ivector; do
    if [ ! -d "$VOSK_MODEL_PATH/$subdir" ]; then
        echo "❌ Missing required model subdirectory: $subdir"
        exit 1
    fi
done
echo "✅ Model structure complete"

echo ""
echo "========================================"
echo "✅ All Vosk setup verification tests passed!"
echo "========================================"
echo ""
echo "Summary:"
echo "  Model: vosk-model-en-us-0.22 (large, production)"
echo "  Library: libvosk.so v0.3.45"
echo "  Cache source: /home/coldaine/ActionRunnerCache/"
echo "  Status: Ready for CI workflows"
