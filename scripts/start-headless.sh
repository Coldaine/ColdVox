#!/bin/bash
set -euo pipefail

Xvfb :99 -screen 0 1280x1024x24 -ac +extension GLX +render -noreset &
timeout 30 bash -c 'until xdpyinfo >/dev/null 2>&1; do sleep 0.5; done'

if command -v openbox >/dev/null 2>&1; then
	openbox --sm-disable &
elif command -v fluxbox >/dev/null 2>&1; then
	fluxbox &
else
	echo "No supported window manager found (openbox/fluxbox)."
fi

if command -v pulseaudio >/dev/null 2>&1; then
	pulseaudio --daemonize --exit-idle-time=-1 --system=false || true
else
	echo "pulseaudio not found; assuming PipeWire handles audio or skipping audio daemon."
fi
