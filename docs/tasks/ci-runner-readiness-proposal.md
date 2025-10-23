---
doc_type: plan
subsystem: general
version: 1.0.0
status: draft
owners: Documentation Working Group
last_reviewed: 2025-10-19
---

Linked task: See [Documentation migration epic](../todo.md#epic-documentation-migration).

# CI Runner Readiness Proposal (Nobara Linux)

Owner: @Coldaine  
Date: 2025-09-19

## Summary
The repository is synced to latest `main` and GitHub Actions workflows are valid. A self-hosted runner `laptop-extra` is online for this repo with labels `self-hosted, Linux, X64, fedora, nobara`, matching all workflow `runs-on` constraints. One gap remains: the runner is missing a few system dependencies required by the composite setup action and the headless test script, which is causing failures in CI jobs.

## Current State
- Repo: up-to-date on `main` (fast-forwarded to 07c21dc).
- Workflows present: `ci.yml`, `vosk-integration.yml`, `release.yml`, `runner-test.yml`, `runner-diagnostic.yml`.
- `actionlint`: clean (exit 0) for all workflows.
- Runner status: online, labels match workflows (`self-hosted, Linux, X64, fedora, nobara`).
- Missing deps on runner:
  - Commands: `openbox`, `pulseaudio`.
  - pkg-config libs: `at-spi-2.0` (dev headers).

## Why These Matter
- `scripts/start-headless.sh` launches `openbox` and `pulseaudio` for headless GUI and audio; missing these breaks `text_injection_tests`.
- `.github/actions/setup-coldvox` asserts presence of required commands and pkg-config libs including `at-spi-2.0`.

## Remediation Steps (Nobara/Fedora)
Install the missing packages and recommended dev libs:

```bash
# Audio stack note: Nobara/Fedora default to PipeWire. Do NOT replace it.
# Ensure the PulseAudio compatibility layer or CLI is available for scripts.
# Option A (preferred on PipeWire systems): provide PulseAudio shim
sudo dnf install -y pipewire-pulseaudio
# Option B (if you truly need the classic PulseAudio CLI)
sudo dnf install -y pulseaudio

# Headless WM used by start-headless.sh
sudo dnf install -y openbox

# X utilities used by start-headless.sh (xdpyinfo)
sudo dnf install -y xorg-x11-utils

# Xvfb server (install if not already present on the runner image)
sudo dnf install -y xorg-x11-server-Xvfb

# Dev headers and libraries required by pkg-config checks/backends
sudo dnf install -y at-spi2-core-devel
sudo dnf install -y gtk3-devel libXtst-devel alsa-lib-devel
```

## Validation Checklist
- Verify commands/libs:
  ```bash
  # Verify core commands (pulseaudio may be provided by the PipeWire shim)
  for c in openbox pulseaudio xdpyinfo Xvfb; do command -v $c || echo MISSING:$c; done
  for p in at-spi-2.0 gtk+-3.0 xtst alsa; do pkg-config --exists $p || echo MISSING-PKG:$p; done
  # Optional: confirm PulseAudio on PipeWire is active
  pactl info | sed -n '1,10p' || true
  ```
- Headless environment smoke test:
  ```bash
  bash scripts/start-headless.sh
  pgrep -af "Xvfb|openbox|pulseaudio|pipewire-pulse" | cat
  ```
- GitHub runner labels and status:
  ```bash
  gh api repos/Coldaine/ColdVox/actions/runners --jq '.runners[] | {name:.name,status:.status,labels:[.labels[].name]}'
  ```
- Optional: trigger diagnostic workflow
  ```bash
  gh workflow run "Runner Diagnostic"
  gh run watch --exit-status
  ```

## Notes
- The runner currently runs via `run.sh` (no systemd service). This is acceptable, but converting to a user systemd service can improve reliability:
  - `.service` marker indicates `actions.runner.Coldaine-ColdVox.laptop-extra.service`. If desired, enable a systemd user service and configure auto-start.
- Vosk dependencies are set up per job by `setup-vosk-cache.sh`. Ensure adequate disk space (env `MIN_FREE_DISK_GB=10`).
  
- Nobara/Fedora typically ship PipeWire by default. Installing `pipewire-pulseaudio` ensures the PulseAudio compatibility layer exposes the expected CLI/bus without replacing the stock audio stack. Our script calls `pulseaudio --daemonize`; this remains compatible when the shim is present. The validation step includes `pactl info` to confirm the active server.

## Acceptance Criteria
- All remediation packages installed on the runner.
- `scripts/start-headless.sh` completes without errors.
- `ci.yml` jobs succeed on `main` for stable toolchain.
- `vosk-integration.yml` completes on PRs touching STT/Vosk.
