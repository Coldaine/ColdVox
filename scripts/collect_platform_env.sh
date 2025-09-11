#!/usr/bin/env bash
set -euo pipefail

echo "== Platform Environment Snapshot =="
echo "Date: $(date -Is)"
echo
echo "-- Session --"
echo "USER: $(id -un)" || true
echo "XDG_SESSION_TYPE: ${XDG_SESSION_TYPE:-}"
echo "XDG_CURRENT_DESKTOP: ${XDG_CURRENT_DESKTOP:-}"
echo "DESKTOP_SESSION: ${DESKTOP_SESSION:-}"
echo "WAYLAND_DISPLAY: ${WAYLAND_DISPLAY:-}"
echo "DISPLAY: ${DISPLAY:-}"
echo

echo "-- OS --"
uname -a
cat /etc/os-release 2>/dev/null || true
echo

echo "-- Accessibility / AT-SPI --"
pgrep -a at-spi 2>/dev/null || echo "No at-spi processes found"
gsettings get org.gnome.desktop.interface toolkit-accessibility 2>/dev/null || true
echo

echo "-- Clipboard / Portals --"
command -v wl-copy >/dev/null && echo "wl-clipboard: present" || echo "wl-clipboard: missing"
command -v xclip >/dev/null && echo "xclip: present" || echo "xclip: missing"
command -v xsel  >/dev/null && echo "xsel: present"  || echo "xsel: missing"
command -v qdbus >/dev/null && echo "qdbus: present" || echo "qdbus: missing"
systemctl --user status xdg-desktop-portal 2>/dev/null | sed -n '1,6p' || true
echo

echo "-- Injection Helpers --"
command -v ydotool >/dev/null && echo "ydotool: present" || echo "ydotool: missing"
command -v kdotool >/dev/null && echo "kdotool: present" || echo "kdotool: missing"
getcap $(command -v ydotool 2>/dev/null || echo /usr/bin/ydotool) 2>/dev/null || true
groups | tr ' ' '\n' | grep -E 'input|uinput' || echo "No input/uinput group in current user"
echo

echo "-- Window System --"
command -v xprop >/dev/null && echo "xprop: present" || echo "xprop: missing"
command -v wmctrl >/dev/null && echo "wmctrl: present" || echo "wmctrl: missing"
echo

echo "== End Snapshot =="
