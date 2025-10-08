#!/bin/bash
# Quick fix for missing runner dependencies
# Run this once on the self-hosted runner to enable CI workflows

set -euo pipefail

echo "========================================"
echo "Installing Missing Runner Dependencies"
echo "========================================"
echo ""
echo "This will install:"
echo "  - openbox (window manager for headless X11)"
echo "  - pulseaudio (audio system)"
echo "  - at-spi2-core-devel (accessibility library headers)"
echo ""

# Check if running as root or with sudo
if [[ $EUID -eq 0 ]]; then
    echo "Running as root..."
    DNF_CMD="dnf"
else
    echo "Will use sudo for dnf..."
    DNF_CMD="sudo dnf"
fi

echo "Installing packages..."
$DNF_CMD install -y openbox pulseaudio at-spi2-core-devel

echo ""
echo "========================================"
echo "Verifying Installation"
echo "========================================"

# Verify installation
failed=0

if command -v openbox &> /dev/null; then
    echo "✅ openbox: $(openbox --version | head -1)"
else
    echo "❌ openbox: NOT FOUND"
    failed=1
fi

if command -v pulseaudio &> /dev/null; then
    echo "✅ pulseaudio: $(pulseaudio --version)"
else
    echo "❌ pulseaudio: NOT FOUND"
    failed=1
fi

if pkg-config --exists at-spi-2.0; then
    VERSION=$(pkg-config --modversion at-spi-2.0)
    echo "✅ at-spi-2.0: version $VERSION"
else
    echo "❌ at-spi-2.0: NOT FOUND"
    failed=1
fi

echo ""
if [[ $failed -eq 0 ]]; then
    echo "========================================"
    echo "✅ All dependencies installed successfully!"
    echo "========================================"
    echo ""
    echo "Next steps:"
    echo "  1. Re-run failed CI workflows (they should now pass)"
    echo "  2. Or push a new commit to trigger workflows automatically"
    echo ""
else
    echo "========================================"
    echo "❌ Some dependencies failed to install"
    echo "========================================"
    echo ""
    echo "Please check the errors above and retry."
    exit 1
fi
