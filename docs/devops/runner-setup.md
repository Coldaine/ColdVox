# Self-Hosted Runner Setup in ColdVox

**Last Updated:** September 16, 2025  

## Overview

ColdVox CI runs on a personal self-hosted GitHub runner (labels: `[self-hosted, Linux, X64, fedora, nobara]`), ideal for GPU/audio tasks (e.g., Vosk STT). No cloud runners; all workflows depend on it. Setup via GitHub UI; diagnostics ensure reliability. Persistent cache dir: `/home/coldaine/ActionRunnerCache/` (for Vosk/models; survives runs).

## Runner Configuration

### Labels and Environment
- **OS/Base:** Fedora/Nobara (RPM-based; uses `dnf` for pkgs like `xdotool`).  
- **Arch:** x64 (Linux).  
- **Env Vars (Workflows):** `RUSTFLAGS="-D warnings"`, `VOSK_MODEL_BASENAME`, `MIN_FREE_DISK_GB=10`, `MAX_LOAD_AVERAGE=5`.  
- **Working Dir:** `/home/runner/work/ColdVox/ColdVox` (standard GH runner).  
- **Sudo:** Passwordless required (tested in diagnostics); enables dep installs.  
- **Cache Persistence:** /home/coldaine/ActionRunnerCache/vosk/ (Vosk models/libs via setup-vosk-cache.sh; SHA256 verify, symlink to vendor/vosk/; survives runs).  

### Setup Steps
1. **GitHub Side:**  
   - Settings > Actions > Runners > New self-hosted runner.  
   - Download/install on Fedora machine (follow GH docs).  
   - Add labels: `self-hosted`, `Linux`, `X64`, `fedora`, `nobara`.  
   - Start: `./run.sh` (persistent).  

2. **Local Machine Prep:**  
   - Install Rust: `rustup toolchain install stable`.  
   - GPU (if needed): NVIDIA drivers/CUDA; use `detect-target-gpu.sh` to verify.  
   - Vosk Libs: `sudo dnf install vosk-api` (or via `verify_libvosk.sh`).  
   - Disk: Ensure >10GB free (env check).  
   - Network: Test DNS/HTTPS (e.g., `dig codeload.github.com`).  
   - Cache Dir: /home/coldaine/ActionRunnerCache/vosk/ auto-populated by script (first run downloads; subsequent symlinks).  

3. **Security:**  
   - Run as non-root user (`runner`).  
   - Firewall: Allow GH IPs (docs: GitHub meta API).  
   - Tokens: Use fine-grained PAT for runner registration.  

## Diagnostics and Troubleshooting

### Workflow: runner-diagnostic.yml
- **Trigger:** Manual.  
- **Checks:** Env (user/PATH/OS), network (DNS/HTTPS), sudo/pkgs (install `xdotool`).  
- **Run:** GitHub > Actions > Runner Diagnostic > Run.  

### Workflow: runner-test.yml
- **Trigger:** Manual or push to `fedora-runner-test`.  
- **Checks:** Echo basics, OS/CPU/memory (`free -h`), Rust setup, Cargo.lock clone/test.  
- **Timeout:** 5min.  

### Common Issues
- **Runner Offline:** Jobs queue; add fallback to `ubuntu-latest` in workflows.  
- **Sudo Fail:** Configure `/etc/sudoers` for nopasswd.  
- **Disk Full:** Monitor via `df -h`; clean `target/` periodically.  
- **GPU Detection:** Run `scripts/detect-target-gpu.sh`; ensure CUDA in PATH.  
- **Vosk Cache Miss:** First run downloads/verifies (SHA256); subsequent symlinks from /home/coldaine/ActionRunnerCache/vosk/. Pre-populate: bash scripts/ci/setup-vosk-cache.sh.  

## Maintenance
- **Updates:** Restart runner; re-run diagnostics.  
- **Monitoring:** Use `performance_monitor.sh` for load/disk.  
- **Scaling:** For multiple machines, add more runners with same labels.  
- **Fallback Plan:** Hybrid: Self-hosted for GPU jobs, cloud for basics (update `runs-on`).  

See [ci-workflows.md](../ci-workflows.md) for workflow integration.