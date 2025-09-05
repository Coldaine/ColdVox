#!/usr/bin/env bash
set -euo pipefail

export DISPLAY=${DISPLAY:-:99}

Xvfb "$DISPLAY" -screen 0 1024x768x24 &
sleep 1
fluxbox -display "$DISPLAY" &
sleep 1

# Start a D-Bus session so AT-SPI and other services can talk
eval "$(dbus-launch --sh-syntax)"
export DBUS_SESSION_BUS_ADDRESS

echo "DISPLAY=$DISPLAY"; echo "DBUS_SESSION_BUS_ADDRESS set"

if [[ -f Cargo.toml ]]; then
  dbus-run-session -- bash -lc "cargo test --workspace"
else
  echo "Mount the repository at /work, then run the container"
  exit 2
fi
