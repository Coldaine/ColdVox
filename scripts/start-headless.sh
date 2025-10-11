#!/bin/bash
# Enhanced error handling for headless environment setup (2025-10-11)
# Added proper status checks and error reporting per refactoring recommendations
set -euo pipefail

echo "Starting headless X11 environment..."

# Start Xvfb virtual display
echo "Starting Xvfb..."
Xvfb :99 -screen 0 1280x1024x24 -ac +extension GLX +render -noreset &
XVFB_PID=$!

# Wait for Xvfb to be ready with timeout
echo "Waiting for Xvfb to initialize..."
if ! timeout 30 bash -c 'until xdpyinfo -display :99 >/dev/null 2>&1; do sleep 0.5; done'; then
    echo "ERROR: Xvfb failed to start or display :99 not available" >&2
    kill "$XVFB_PID" 2>/dev/null || true
    exit 1
fi

echo "Starting Openbox window manager..."
openbox --sm-disable &
OPENBOX_PID=$!

# Give openbox a moment to start
sleep 2

# Verify openbox is running
if ! pgrep -f "openbox.*--sm-disable" >/dev/null; then
    echo "ERROR: Openbox failed to start" >&2
    kill "$XVFB_PID" 2>/dev/null || true
    exit 1
fi

echo "Headless environment ready"</search>
</search_and_replace>

# Audio system check with enhanced error handling
# Modern systems (Fedora 42+, Nobara) run PipeWire system-wide by default
# Only start audio if nothing is running
echo "Checking audio system..."
if pgrep -x "pipewire-pulse|pulseaudio" >/dev/null 2>&1; then
    echo "Audio daemon already running"
else
    if command -v pulseaudio >/dev/null 2>&1; then
        echo "Starting PulseAudio for headless testing..."
        if pulseaudio --daemonize --exit-idle-time=-1 --system=false; then
            echo "PulseAudio started successfully"
        else
            echo "WARNING: Failed to start PulseAudio, but continuing..." >&2
        fi
    else
        echo "Note: No audio daemon running, but pipewire-pulse may be available via systemd"
    fi
fi</search>
</search_and_replace>
