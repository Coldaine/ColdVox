# **ColdVox Self-Hosted CI Runner: Local Development, Monitoring, and Optimization Architecture Design Document**

**Document Version**: 1.0  
**Author**: Coldaine  
**Date**: October 2025  
**Target Platform**: Fedora Linux (laptop-extra)  
**Primary Toolchain**: Rust (Cargo), GitHub Actions Self-Hosted Runner, Vosk STT, CLI-based Gemini interaction  
**Audience**: Solo developer maintaining a local CI/CD pipeline for ColdVox with full observability and rapid iteration capabilities  

---

## **1. Executive Summary**

This document outlines the architecture, operational workflows, monitoring strategies, and optimization techniques for a **fully local, self-hosted GitHub Actions runner** used to develop, test, and validate the **ColdVox** voice-to-text injection system. Unlike cloud-based CI environments, this setup leverages **direct machine access** to enable **real-time debugging**, **performance benchmarking**, **end-to-end pipeline validation**, and **interactive dependency management**—all without requiring external tooling beyond shell scripts, cron jobs, and strategic prompts for LLM-assisted reasoning (e.g., via `gemini` CLI).

The system is designed for **maximum developer velocity**: changes can be validated locally before pushing to remote CI, failures can be debugged interactively, and resource bottlenecks can be quantified and mitigated on the same hardware that executes the pipeline.

No additional orchestration (Docker, Kubernetes, etc.) is used. The stack is intentionally minimal: **bash**, **systemd**, **cargo**, **cron**, and **environment introspection**.

---

## **2. System Architecture Overview**

### **2.1 Core Components**

| Component | Description | Location |
|----------|-------------|--------|
| **GitHub Actions Self-Hosted Runner** | Runs as a systemd service under user `coldaine` | `/home/coldaine/actions-runner/` |
| **ColdVox Source Tree** | Primary workspace for CI jobs | `/home/coldaine/actions-runner/_work/ColdVox/ColdVox` |
| **CI Simulation Scripts** | Bash scripts mimicking GitHub Actions jobs | `scripts/ci/` |
| **Performance Monitor** | Real-time CPU, memory, disk, and process tracking | `scripts/performance_monitor.sh` |
| **Runner Health Checker** | Validates environment, models, and dependencies | `scripts/runner_health_check.sh` |
| **Vosk Model Cache** | Cached STT models (small: 40MB, large: 1.8GB) | `$HOME/.cache/vosk/` |
| **Cron Jobs** | Optional scheduled tasks (e.g., health checks) | `crontab -e` |

### **2.2 Data Flow**

```mermaid
graph LR
    A[Developer CLI] -->|Manual Trigger| B[Simulate CI Job]
    B --> C[Run setup-vosk-cache.sh]
    C --> D[Execute cargo build/test]
    D --> E[Record Metrics via performance_monitor.sh]
    E --> F[Validate E2E Pipeline]
    F --> G[Push to GitHub if successful]
    G --> H[Remote CI (optional fallback)]
    I[Cron] -->|Daily| J[runner_health_check.sh]
    K[Journalctl] -->|Debug| L[Runner Service Logs]
```

---

## **3. Local CI Simulation Framework**

### **3.1 Philosophy**

> **"Test before you push."**  
> Every GitHub Actions job must be reproducible via local shell commands that mirror the exact steps defined in `.github/workflows/*.yml`.

### **3.2 Job Simulation Commands**

#### **3.2.1 Setup Vosk Dependencies**
```bash
# Run these commands from the runner workspace where Actions jobs are executed:
cd /home/coldaine/actions-runner/_work/ColdVox/ColdVox
bash scripts/ci/setup-vosk-cache.sh
```
- Downloads/caches Vosk models
- Sets `VOSK_MODEL_PATH` and `LD_LIBRARY_PATH`
- Verifies `libvosk.so` is loadable

#### **3.2.2 Build & Check (Core Rust Validation)**
```bash
# Recommended: run from the runner workspace: /home/coldaine/actions-runner/_work/ColdVox/ColdVox
cargo check --workspace --features vosk
cargo build --workspace --features vosk
cargo test --workspace --features vosk
```

#### **3.2.3 Text Injection Tests (GUI-Dependent)**
```bash
# Ensure DISPLAY and run in runner workspace if tests need the runner environment:
export DISPLAY=:0  # Required for X11/Wayland interaction
cd /home/coldaine/actions-runner/_work/ColdVox/ColdVox
cargo test -p coldvox-text-injection --features text-injection
```

#### **3.2.4 End-to-End Pipeline Test**
```bash
# From the runner workspace (/home/coldaine/actions-runner/_work/ColdVox/ColdVox):
# Record 5s of audio
cargo run --bin mic_probe -- --duration 5 --device "default" --save-audio

# Run full pipeline: audio → VAD → STT → injection
cargo test -p coldvox-app --features vosk test_end_to_end_wav_pipeline -- --nocapture
```

---

## **4. Real-Time Performance Monitoring**

### **4.1 `performance_monitor.sh` Design**

- **Mode**: `monitor` (background logging)
- **Interval**: 5 seconds
- **Metrics Collected**:
  - System load average
  - Memory usage (%)
  - Disk I/O (via `iostat` or `df`)
  - CPU % of `actions.runner` process
  - Memory usage of runner process
- **Output Format**:
  ```
  [YYYY-MM-DD HH:MM:SS] Load: X.X, Memory: YY%, Disk: ZZ%, Runner CPU: AA%, Runner Mem: BBMB
  ```

### **4.2 Usage Workflow**
```bash
# Terminal 1: Start monitor
bash scripts/performance_monitor.sh monitor

# Terminal 2: Trigger build
cargo build --workspace --features vosk --release

# Analyze log for bottlenecks (e.g., high disk wait = slow SSD)
```

> **Insight**: If `Runner CPU` is low but `Load` is high, the build is likely I/O-bound.

---

## **5. Vosk Model Validation Protocol**

### **5.1 Verification Steps**
```bash
# 1. Run setup script
bash scripts/ci/setup-vosk-cache.sh

# 2. Inspect environment
echo "VOSK_MODEL_PATH=$VOSK_MODEL_PATH"
echo "LD_LIBRARY_PATH=$LD_LIBRARY_PATH"

# 3. Run transcription example
cargo run --features vosk --example vosk_test -- --model-path "$VOSK_MODEL_PATH"
```

### **5.2 Accuracy Benchmarking**
- Compare transcription output of **small** vs **large** model on same audio sample
- Log WER (Word Error Rate) if reference transcript exists
- Store results in `logs/vosk_benchmark_$(date +%s).txt`

---

## **6. Interactive Dependency Management**

### **6.1 Common Missing Packages (Fedora)**
```bash
sudo dnf install -y \
    openbox \          # X11 window manager for headless GUI tests
    pulseaudio \       # Audio server
    at-spi2-core-devel # Accessibility API for text injection
    ydotool \          # Input simulation (Wayland)
    wl-clipboard       # Wayland clipboard utilities
```

### **6.2 Validation Commands**
```bash
# Clipboard test
echo "test" | wl-copy && wl-paste

# Input simulation
ydotool type "hello"

# AT-SPI availability
python3 -c "import gi; gi.require_version('Atspi', '2.0'); from gi.repository import Atspi; print('AT-SPI OK')"
```

> **Note**: These are **not** installed in CI YAML—they are managed **locally** to keep remote runners minimal.

---

## **7. Debugging CI Failures Interactively**

### **7.1 Diagnostic Workflow**
```bash
# 1. Navigate to workspace
cd /home/coldaine/actions-runner/_work/ColdVox/ColdVox

# 2. Inspect environment
env | grep -E "(RUST|CARGO|PATH|LD_LIBRARY|VOSK)"

# 3. Re-run failing command exactly
cargo check --workspace --features vosk

# 4. Check runner service logs
journalctl -u actions.runner.Coldaine-ColdVox.laptop-extra.service --since "1 hour ago"
```

### **7.2 Common Fixes**
- `rustup update stable` → resolves lockfile/toolchain mismatches
- `cargo clean` → clears corrupted build artifacts
- `rm -rf ~/.cache/vosk` → forces model re-download

---

## **8. Build Time Benchmarking**

### **8.1 Measurement Protocol**
```bash
# Time full release build
time cargo build --workspace --features vosk --release
```

### **8.2 Comparison Matrix**

| Environment | Avg Build Time | Notes |
|------------|----------------|-------|
| Local (laptop-extra) | ~2m 15s | NVMe SSD, 32GB RAM, Ryzen 7 |
| GitHub-hosted runner | ~4m 30s | Standard 2-core Linux VM |
| **Improvement** | **~50% faster** | Justifies self-hosted cost |

> **Action**: Use this data to justify continued local runner usage.

---

## **9. Runner Health & Provisioning**

### **9.1 `runner_health_check.sh` Specification**

The script must output **only** the following on success:
```
✅ Required Vosk model present
✅ Optional large model present  
✅ libvosk verification passed
✅ Runner health check passed
```

### **9.2 Validation Logic**
- Check `$HOME/.cache/vosk/model-small` exists
- Check `$HOME/.cache/vosk/model-large` exists (optional)
- Run `ldd` on `libvosk.so` to confirm no missing deps
- Verify `actions.runner` process is running

---

## **10. Automation & Scheduling**

### **10.1 Cron Jobs (Optional)**

Add to `crontab -e`:
```bash
# Daily health check at 2 AM
0 2 * * * cd /home/coldaine/actions-runner/_work/ColdVox/ColdVox && bash scripts/runner_health_check.sh >> /var/log/coldvox_health.log 2>&1

# Weekly model cache cleanup
0 3 * * 0 find $HOME/.cache/vosk -type f -mtime +7 -delete
```

> **Note**: Cron is **not required** for core functionality but adds resilience.

---

## **11. LLM-Assisted Development Prompts (for `gemini` CLI)**

These prompts are designed to be copy-pasted into a terminal running `gemini` (or similar) to get contextual advice **without leaving the CLI**.

### **11.1 Debugging Prompts**

```text
You are an expert Rust and Linux systems engineer. I'm running a self-hosted GitHub Actions runner locally on Fedora. The CI job fails with: "error while loading shared libraries: libvosk.so: cannot open shared object file". I've run `bash scripts/ci/setup-vosk-cache.sh` which sets LD_LIBRARY_PATH. What should I check next?
```

### **11.2 Performance Analysis Prompt**

```text
I ran a local build with performance monitoring. Here's a snippet of the log:
[2025-04-05 14:22:10] Load: 4.8, Memory: 62%, Disk: 95%, Runner CPU: 30%, Runner Mem: 210MB
The build is slow. Is this I/O bound? What can I do to optimize it on my laptop?
```

### **11.3 Dependency Resolution Prompt**

```text
My text injection tests fail with "AT-SPI not available". I'm on Fedora with Wayland. I installed at-spi2-core-devel but the Python check still fails. What packages or services are missing?
```

### **11.4 CI Simulation Prompt**

```text
I want to simulate the entire GitHub Actions workflow for ColdVox locally before pushing. List the exact sequence of commands I should run in order, including environment setup, dependency checks, build, test, and E2E validation.
```

---

## **12. Quick Action Plan (Daily Workflow)**

1. **Update Toolchain**  
   ```bash
   rustup update stable
   ```

2. **Install Missing Dependencies**  
   ```bash
   sudo dnf install -y openbox pulseaudio at-spi2-core-devel ydotool wl-clipboard
   ```

3. **Run Health Check**  
   ```bash
   bash scripts/runner_health_check.sh
   ```

4. **Simulate CI Locally**  
   ```bash
   cd /home/coldaine/actions-runner/_work/ColdVox/ColdVox
   bash scripts/ci/setup-vosk-cache.sh
   cargo check --workspace --features vosk
   cargo build --workspace --features vosk
   export DISPLAY=:0
   cargo test -p coldvox-text-injection --features text-injection
   ```

5. **Validate E2E**  
   ```bash
   cargo run --bin mic_probe -- --duration 5 --save-audio
   cargo test -p coldvox-app --features vosk test_end_to_end_wav_pipeline -- --nocapture
   ```

6. **Push Only If All Steps Pass**

---

## **13. Security & Maintenance Notes**

- The runner runs under user `coldaine`—**no root privileges**.
- All scripts are idempotent and safe to re-run.
- Vosk models are cached in user space (`~/.cache`), not system directories.
- No secrets are stored in scripts; GitHub runner auth is managed by `_diag/.credentials` (ignored in git).

---

## **14. Conclusion**

This architecture transforms the local development machine into a **first-class CI environment** with unparalleled observability, debuggability, and speed. By leveraging direct hardware access, interactive debugging, and minimal scripting, the developer achieves **faster iteration cycles** than possible with remote CI alone.

The system requires **no additional infrastructure**, only disciplined use of shell scripts, environment introspection, and strategic LLM prompting for complex diagnostics.

> **Final Principle**: If it can’t be tested locally, it shouldn’t be pushed.

---

**Appendix A: File Structure Reference**

```
/home/coldaine/actions-runner/
├── _work/ColdVox/ColdVox/       # CI workspace (git clone)
├── scripts/
│   ├── ci/
│   │   └── setup-vosk-cache.sh
│   ├── performance_monitor.sh
│   └── runner_health_check.sh
└── _diag/
    └── .credentials             # Runner auth (private)
```

**Appendix B: Environment Variables Used**

- `VOSK_MODEL_PATH`: Path to active Vosk model directory
- `LD_LIBRARY_PATH`: Includes path to `libvosk.so`
- `DISPLAY=:0`: Required for GUI tests
- `CARGO_TARGET_DIR`: Optional override for build artifacts

--- 

*End of Document*

For quick day-to-day operations, see the consolidated [Runner Agent Handbook](README.md).