#!/bin/bash
# KDE Wayland Text Injection Setup for ColdVox
# Based on 2024-2025 best practices

set -e

echo "======================================"
echo "ColdVox Text Injection Setup"
echo "For KDE Plasma Wayland"
echo "======================================"
echo

# Detect distro
if command -v dnf &> /dev/null; then
    PKG_MANAGER="dnf"
    PKG_INSTALL="sudo dnf install -y"
elif command -v pacman &> /dev/null; then
    PKG_MANAGER="pacman"
    PKG_INSTALL="sudo pacman -S --noconfirm"
elif command -v apt &> /dev/null; then
    PKG_MANAGER="apt"
    PKG_INSTALL="sudo apt install -y"
else
    echo "Unsupported package manager. Please install manually."
    exit 1
fi

echo "Detected package manager: $PKG_MANAGER"
echo

# Function to check if a command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# 1. Install required tools
echo "Step 1: Installing required tools..."

# wl-clipboard (required)
if ! command_exists wl-copy; then
    echo "Installing wl-clipboard..."
    $PKG_INSTALL wl-clipboard
else
    echo "✓ wl-clipboard already installed"
fi

# ydotool (recommended)
if ! command_exists ydotool; then
    echo "Installing ydotool..."
    $PKG_INSTALL ydotool
else
    echo "✓ ydotool already installed"
fi

# kdotool (optional but helpful)
if ! command_exists kdotool; then
    echo "kdotool not found. It's optional but improves focus detection."
    echo "Install it from: https://github.com/jinliu/kdotool"
    echo "Or via AUR on Arch: yay -S kdotool"
else
    echo "✓ kdotool already installed"
fi

echo

# 2. Configure uinput permissions
echo "Step 2: Configuring uinput permissions..."

UDEV_RULE="/etc/udev/rules.d/99-uinput.rules"
if [ ! -f "$UDEV_RULE" ]; then
    echo "Creating udev rule for uinput access..."
    echo 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"' | sudo tee "$UDEV_RULE" > /dev/null

    # Reload udev rules
    sudo udevadm control --reload-rules
    sudo udevadm trigger
    echo "✓ udev rule created"
else
    echo "✓ udev rule already exists"
fi

echo

# 3. Add user to input group
echo "Step 3: Adding user to input group..."

if ! groups | grep -q "input"; then
    echo "Adding $USER to input group..."
    sudo usermod -a -G input "$USER"
    echo "✓ User added to input group"
    echo "⚠ You need to log out and log back in for this to take effect!"
    GROUP_CHANGE_NEEDED=true
else
    echo "✓ User already in input group"
    GROUP_CHANGE_NEEDED=false
fi

echo

# 4. Configure ydotool service
echo "Step 4: Configuring ydotool service..."

# Check if systemd service exists
if systemctl list-unit-files | grep -q "ydotool.service"; then
    # Try user service first
    if systemctl --user is-enabled ydotool.service &> /dev/null; then
        echo "✓ ydotool user service already enabled"
    elif systemctl is-enabled ydotool.service &> /dev/null; then
        echo "✓ ydotool system service already enabled"
    else
        echo "Enabling ydotool service..."
        # Try system service (Fedora/Nobara style)
        if sudo systemctl enable --now ydotool.service &> /dev/null; then
            echo "✓ ydotool system service enabled"
        else
            # Try user service
            systemctl --user enable --now ydotool.service
            echo "✓ ydotool user service enabled"
        fi
    fi
else
    echo "⚠ ydotool service not found. You may need to run ydotool manually."
fi

echo

# 5. Test the setup
echo "Step 5: Testing the setup..."
echo

# Test wl-clipboard
if echo "test" | wl-copy && wl-paste | grep -q "test"; then
    echo "✓ wl-clipboard working"
else
    echo "✗ wl-clipboard test failed"
fi

# Test ydotool (if in input group)
if groups | grep -q "input"; then
    if timeout 1 ydotool --help &> /dev/null; then
        echo "✓ ydotool accessible"
    else
        echo "⚠ ydotool not accessible (service may need to be started)"
    fi
else
    echo "⚠ ydotool test skipped (not in input group yet)"
fi

# Test kdotool
if command_exists kdotool; then
    if kdotool getactivewindow &> /dev/null; then
        echo "✓ kdotool working"
    else
        echo "⚠ kdotool installed but not working (is KDE running?)"
    fi
fi

echo
echo "======================================"
echo "Setup Summary"
echo "======================================"

# Check final status
ISSUES=()

if ! command_exists wl-copy; then
    ISSUES+=("wl-clipboard not installed (REQUIRED)")
fi

if ! command_exists ydotool; then
    ISSUES+=("ydotool not installed (recommended)")
fi

if ! groups | grep -q "input"; then
    ISSUES+=("User not in input group (needed for ydotool)")
fi

if [ ${#ISSUES[@]} -eq 0 ]; then
    echo "✓ All checks passed! ColdVox text injection is ready."

    if [ "$GROUP_CHANGE_NEEDED" = true ]; then
        echo
        echo "⚠ IMPORTANT: Log out and log back in for group changes to take effect!"
    fi
else
    echo "Issues found:"
    for issue in "${ISSUES[@]}"; do
        echo "  - $issue"
    done
    echo
    echo "Please resolve these issues and run the script again."
fi

echo
echo "To test text injection manually:"
echo "  echo 'Hello from ColdVox' | wl-copy && ydotool key ctrl+v"
echo
