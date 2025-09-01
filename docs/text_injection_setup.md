# Text Injection Setup for ColdVox

## Overview

ColdVox includes text injection capabilities to automatically type recognized speech into any focused application on KDE Plasma Wayland. This uses the most reliable methods available in 2024-2025.

## Quick Setup

Run the automated setup script:

```bash
./scripts/setup_text_injection.sh
```

This will:
1. Install required tools (wl-clipboard, ydotool)
2. Configure uinput permissions
3. Add your user to the input group
4. Enable the ydotool service
5. Test the setup

**Important:** After running the script, log out and log back in for group changes to take effect.

## Manual Setup

### Required Tools

```bash
# Fedora/Nobara
sudo dnf install -y wl-clipboard ydotool

# Arch/EndeavourOS
sudo pacman -S wl-clipboard ydotool

# Ubuntu/Debian
sudo apt install -y wl-clipboard ydotool
```

### Configure Permissions

1. Create udev rule for uinput access:
```bash
echo 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"' | \
  sudo tee /etc/udev/rules.d/99-uinput.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

2. Add your user to the input group:
```bash
sudo usermod -a -G input $USER
# Log out and log back in after this
```

3. Enable ydotool service:
```bash
sudo systemctl enable --now ydotool
```

## How It Works

The text injection system uses a multi-tier approach:

1. **Primary Method:** `wl-clipboard` + `ydotool` paste
   - Sets clipboard with recognized text
   - Simulates Ctrl+V to paste
   - Most reliable method

2. **Fallback:** Direct typing with `ydotool`
   - Types text character by character
   - Slower but works when paste fails

3. **Last Resort:** Clipboard only
   - Sets clipboard and notifies user
   - User manually pastes with Ctrl+V

## Testing

Test the injection manually:

```bash
# Test clipboard
echo "Test text" | wl-copy
wl-paste  # Should output "Test text"

# Test ydotool
echo "Hello World" | wl-copy && ydotool key ctrl+v

# Test direct typing
ydotool type "Hello from ydotool"
```

## Troubleshooting

### ydotool not working

1. Check if you're in the input group:
```bash
groups | grep input
```

2. Check if the service is running:
```bash
systemctl status ydotool
```

3. Check uinput permissions:
```bash
ls -l /dev/uinput
```

### Clipboard not working

Ensure you're running under Wayland:
```bash
echo $WAYLAND_DISPLAY
```

### Text not appearing

1. Some applications may block automated input
2. Try clicking in the text field first
3. Check if the application is running under XWayland

## Security Notes

- Text injection requires access to `/dev/uinput`
- Being in the `input` group allows keyboard/mouse simulation
- Only grant these permissions to trusted users
- The system respects Wayland's security model

## Optional Enhancements

### Install kdotool (improves focus detection)

kdotool helps ensure text is injected into the correct window:

```bash
# From source
git clone https://github.com/jinliu/kdotool
cd kdotool
make && sudo make install

# On Arch (AUR)
yay -S kdotool
```

## Architecture

The implementation is in the `crates/coldvox-text-injection/` crate and follows 2024-2025 best practices for KDE Plasma Wayland:

- Automatic capability detection
- Graceful fallbacks
- Production-ready error handling
- Minimal dependencies (no complex crates)
- Based on proven tools (ydotool, wl-clipboard)

## Known Limitations

- Some sandboxed applications (Flatpak) may not accept input
- Portal-based permission systems add UX friction
- XWayland applications may have different behavior
- Virtual keyboard protocols are not fully supported in KWin