#!/bin/bash
Xvfb :99 -screen 0 1280x1024x24 -ac +extension GLX +render -noreset &
timeout 30 bash -c 'until xdpyinfo >/dev/null 2>&1; do sleep 0.5; done'
openbox --sm-disable &
pulseaudio --daemonize --exit-idle-time=-1 --system=false
