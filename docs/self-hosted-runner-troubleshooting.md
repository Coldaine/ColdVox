# Self-Hosted Runner Troubleshooting Guide

This guide helps you provision and validate a Linux self-hosted runner for the ColdVox CI workflows. It focuses on fixing the common failures seen in the “Setup ColdVox” step (missing system dependencies like at-spi-2.0 and pulseaudio).

If you only need a one-shot installer script, see `scripts/install_runner_deps.sh` in the repo. This document explains the dependencies and offers manual commands you can audit and adapt for your environment.

## Confirm it’s a self-hosted runner

ColdVox’s CI workflows target self-hosted runners (labels include `self-hosted`, `Linux`, `X64`, `fedora`, `nobara`). You can confirm:

- In the GitHub Actions UI for a failing run, check the Job details → Runner name/labels; or
- In the workflow YAMLs (`.github/workflows/*.yml`), look for `runs-on: [self-hosted, Linux, X64, ...]`.

If you see those labels, it’s a self-hosted runner and must be pre-provisioned.

## Symptoms

- “Setup ColdVox” step fails with messages like:
  - Required command `pulseaudio` not found
  - Required library `at-spi-2.0` not found by pkg-config
  - One or more system dependencies are missing

## Quick install recipes

Below are minimal packages to satisfy the core ColdVox CI jobs. Use the section for your distro family. These lists prioritize correctness and stability; adjust as needed.

### Fedora/Nobara (dnf)

```bash
sudo dnf update -y
# Core build toolchain and common libs
sudo dnf install -y gcc gcc-c++ make pkgconf-pkg-config git curl unzip

# Audio stack
sudo dnf install -y pulseaudio pulseaudio-utils alsa-lib alsa-lib-devel

# AT-SPI (accessibility) headers (satisfies pkg-config at-spi-2.0)
sudo dnf install -y at-spi2-core at-spi2-core-devel at-spi2-atk

# X11/desktop utilities often used by text-injection tests
sudo dnf install -y xdotool xorg-x11-server-Xvfb openbox dbus-x11 xclip wl-clipboard

# Optional/advanced
# ydotool (from distro or COPR) and uinput access
sudo dnf install -y ydotool
# Basic window tooling that can aid diagnostics
sudo dnf install -y xprop wmctrl

# If Qt-based GUI checks are enabled (optional)
sudo dnf install -y qt6-qtbase-devel
```

### Ubuntu/Debian (apt)

```bash
sudo apt-get update
# Core build toolchain and common libs
sudo apt-get install -y build-essential pkg-config git curl unzip

# Audio stack
sudo apt-get install -y pulseaudio pulseaudio-utils libasound2 libasound2-dev

# AT-SPI (accessibility) headers (satisfies pkg-config at-spi-2.0)
sudo apt-get install -y at-spi2-core libatspi2.0-dev

# X11/desktop utilities often used by text-injection tests
sudo apt-get install -y xdotool xvfb openbox dbus-x11 xclip wl-clipboard

# Optional/advanced
sudo apt-get install -y ydotool x11-utils wmctrl

# If Qt-based GUI checks are enabled (optional)
sudo apt-get install -y qtbase5-dev qt6-base-dev
```

## Validation checklist

Run these commands on the runner host to verify provisioning:

```bash
# Toolchain and package config
gcc --version
pkg-config --version

# Audio tools
pulseaudio --version

# AT-SPI headers visible to pkg-config
pkg-config --exists at-spi-2.0 && echo "at-spi-2.0 OK" || echo "at-spi-2.0 MISSING"

# Clipboard and desktop tools
xdotool -v || echo "xdotool missing"
Xvfb -version || echo "Xvfb missing"
openbox --version || echo "openbox missing"
which dbus-launch || echo "dbus-launch missing"
xclip -version || echo "xclip missing"
wl-paste --version || echo "wl-clipboard missing"

# ydotool/uinput (optional backend)
which ydotool || echo "ydotool missing"
# Check uinput access (should not error with EACCES for CI run user)
[ -e /dev/uinput ] && echo "/dev/uinput present" || echo "/dev/uinput missing"
```

If `pkg-config --exists at-spi-2.0` fails, re-check that the `-devel`/`-dev` package for AT-SPI is installed (see above).

## Vosk (context)

ColdVox sets up Vosk model and lib during the `Setup Vosk Dependencies` job. If that job passes, you generally don’t need to pre-install these manually. If you do need manual setup, see the existing CI script in `scripts/ci/setup-vosk-cache.sh` for reference.

## Headless X session for text-injection tests

Some text-injection tests expect a display server:

```bash
# Example snippet to bring up a headless session
Xvfb :99 -screen 0 1024x768x24 &
sleep 2
openbox &
sleep 1
export DISPLAY=:99
# D-Bus session (if needed by tests/tools)
eval "$(dbus-launch --sh-syntax)"
```

Note: The workflows usually manage this, but the packages must be present.

## ydotool and /dev/uinput notes (optional)

- `ydotool` requires write access to `/dev/uinput` and usually membership in the `input` group.
- To grant access (requires reboot/logout to take effect):

```bash
sudo usermod -a -G input $USER
```

- If your environment manages udev rules differently, ensure the runner user has permission to open `/dev/uinput`.

## Re-run the workflow

Once provisioning and validation pass, re-run the failed workflow in GitHub Actions. The “Setup ColdVox” step should succeed, allowing build/tests to proceed.

## Appendix: Common package name mapping

- AT-SPI dev headers:
  - Fedora/Nobara: `at-spi2-core-devel`
  - Ubuntu/Debian: `libatspi2.0-dev`
- ALSA dev headers:
  - Fedora/Nobara: `alsa-lib-devel`
  - Ubuntu/Debian: `libasound2-dev`
- Desktop tools:
  - `xdotool`, `xvfb`/`Xvfb`, `openbox`, `dbus-x11`, `xclip`, `wl-clipboard`
- Optional: `ydotool`, `xprop`, `wmctrl`

---

If you want a single-command provisioning experience, codify the packages your runner needs in your infrastructure (e.g., via Ansible) or adapt `scripts/install_runner_deps.sh` to your distro and runner image.
