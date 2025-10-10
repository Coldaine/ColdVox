#!/bin/bash
Xvfb :99 -screen 0 1280x1024x24 -ac +extension GLX +render -noreset &
timeout 30 bash -c 'until xdpyinfo >/dev/null 2>&1; do sleep 0.5; done'
openbox --sm-disable &

# Audio system check
# Modern systems (Fedora 42+, Nobara) run PipeWire system-wide by default
# Only start audio if nothing is running
if ! pgrep -x "pipewire-pulse|pulseaudio" > /dev/null 2>&1; then
  if command -v pulseaudio &> /dev/null; then
    # Start PulseAudio for headless testing
    pulseaudio --daemonize --exit-idle-time=-1 --system=false
  else
    echo "Note: No audio daemon running, but pipewire-pulse may be available via systemd" >&2
  fi
fi
