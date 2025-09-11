#!/bin/bash
# Permanent libvosk installation for self-hosted runner
# This should be run ONCE on the runner to eliminate per-job extraction

set -euo pipefail

echo "=== Setting up permanent libvosk installation ==="

VOSK_VER="0.3.45"
VENDOR_DIR="/home/coldaine/Projects/ColdVox/vendor/vosk"
CACHE_DIR="/home/coldaine/ActionRunnerCache"

# Ensure we have the vendor file
if [ ! -f "$VENDOR_DIR/vosk-linux-x86_64-${VOSK_VER}.zip" ]; then
    echo "ERROR: Vendor file not found: $VENDOR_DIR/vosk-linux-x86_64-${VOSK_VER}.zip"
    exit 1
fi

# Create working directory
mkdir -p "$CACHE_DIR/libvosk-setup"
cd "$CACHE_DIR/libvosk-setup"

# Extract if not already done
if [ ! -d "vosk-linux-x86_64-${VOSK_VER}" ]; then
    echo "Extracting libvosk..."
    unzip -q "$VENDOR_DIR/vosk-linux-x86_64-${VOSK_VER}.zip"
fi

# Install permanently
echo "Installing libvosk system-wide..."
sudo cp -v "vosk-linux-x86_64-${VOSK_VER}/libvosk.so" /usr/local/lib/
sudo cp -v "vosk-linux-x86_64-${VOSK_VER}/vosk_api.h" /usr/local/include/

# Update dynamic linker cache
echo "Updating dynamic linker cache..."
sudo ldconfig

# Verify installation
echo "Verifying installation..."
if ldconfig -p | grep -q vosk; then
    echo "✅ libvosk successfully installed and cached"
    ldconfig -p | grep vosk
else
    echo "❌ libvosk not found in linker cache"
    exit 1
fi

# Test linking
echo "Testing library linking..."
if ldd /usr/local/lib/libvosk.so >/dev/null 2>&1; then
    echo "✅ libvosk dependencies resolved"
else
    echo "❌ libvosk dependency issues"
    ldd /usr/local/lib/libvosk.so
    exit 1
fi

# Create permanent ldconfig configuration
echo "Creating permanent ldconfig entry..."
echo "/usr/local/lib" | sudo tee /etc/ld.so.conf.d/vosk.conf
sudo ldconfig

echo "✅ Permanent libvosk installation complete!"
echo ""
echo "🚀 Now workflows should use validation instead of extraction:"
echo "    - Remove zip extraction from setup-coldvox action"
echo "    - Replace with simple validation check"
echo "    - Expected time savings: 5-15 seconds per job"