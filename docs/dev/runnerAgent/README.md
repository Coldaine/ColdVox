# ColdVox Runner Agent Handbook

Practical documentation for running, maintaining, and debugging the ColdVox self-hosted GitHub Actions runner `laptop-extra` (Nobara Linux, x86_64).

## Contents

1. [Runner Overview](#runner-overview)
2. [CI Workflows](#ci-workflows)
3. [Daily Operations](#daily-operations)
4. [Debugging Playbook](#debugging-playbook)
5. [Diagnostics & Local Capabilities](#diagnostics--local-capabilities)
6. [Scripts & Automation](#scripts--automation)
7. [Further Reading](#further-reading)

---

## Runner Overview

- **Host name**: `laptop-extra`
- **Service**: `actions.runner.Coldaine-ColdVox.laptop-extra.service`
- **Runner install dir**: `/home/coldaine/actions-runner/`
- **Workflow workspace**: `/home/coldaine/actions-runner/_work/ColdVox/ColdVox`
- **Project**: Rust multi-crate workspace with native dependencies (Vosk STT, text injection)
- **Vendor assets**: `/home/coldaine/Projects/ColdVox/vendor/vosk/lib/libvosk.so`
- **Key scripts**: `scripts/runner_health_check.sh`, `scripts/ci/setup-vosk-cache.sh`

Keep the workspace clean (`git status` before and after experiments) and monitor disk usage under `/home/coldaine`.

## CI Workflows

All GitHub Actions definitions live in `.github/workflows/` and target the self-hosted runner. The core jobs are:

| Workflow | Purpose | Notes |
| --- | --- | --- |
| `ci.yml` | Main verification (lint, tests, build) | Runs with default features |
| `vosk-integration.yml` | Speech pipeline validation | Requires Vosk model cache |
| `ci-minimal.yml` | Fast smoke checks | Minimal features for quick validation |
| `release.yml` | Release packaging | Uses production flags |
| `runner-test.yml` | Runner self-test suite | Focuses on environment sanity |
| `runner-diagnostic.yml` | Health snapshot | Useful before big changes |

Mirror the feature flags and environment from the failing job when reproducing locally.

## Daily Operations

### Morning Checklist

```bash
# Update toolchain
rustup update stable

# Run health check (verifies deps, disk, services)
bash scripts/runner_health_check.sh

# Refresh Vosk cache before heavy runs
bash scripts/ci/setup-vosk-cache.sh

# Confirm runner service is active
systemctl status actions.runner.Coldaine-ColdVox.laptop-extra.service
```

### Ongoing Hygiene

- Keep system packages current (`sudo dnf upgrade --refresh`) during maintenance windows.
- Prune `target/` directories when disk usage exceeds ~85% (`df -h /home/coldaine`).
- Restart the runner service after major updates: `sudo systemctl restart actions.runner.Coldaine-ColdVox.laptop-extra.service`.
- Document notable incidents in `docs/dev/runnerAgent/debug_runs/`.

## Debugging Playbook

Start with the [Runner Debugging Guide](prompts/debug_agent_prompt.md) for detailed steps. Highlights:

### 1. Triage

- Record run URL, job ID, and matrix combination.
- Download job logs (`gh run view <run-id> --log > run-<id>.log`).
- Capture service status and recent journal entries.
- Note current disk usage and system load (helpful for contention issues).

### 2. Reproduce Locally

```bash
cd /home/coldaine/actions-runner/_work/ColdVox/ColdVox
git fetch origin <branch>
git checkout <commit_sha>
git status --short      # ensure clean state

# Set environment expected by the workflow
export CI=1
export VOSK_MODEL_PATH="/home/coldaine/Projects/ColdVox/models/vosk-model-small-en-us-0.15"
# add other KEY=VALUE pairs from the job log

# Re-run the failing step
just <target>            # or
cargo <subcommand> ...
```

Prefer `cargo check` for fast iteration and enable `RUST_BACKTRACE=1` when the failure is in Rust code. For flakey tests, run with `-- --nocapture` and consider `--test-threads=1` to serialize.

### 3. Diagnose

- Inspect `/tmp` artifacts, especially files left by the workflow.
- Compare toolchain versions against `rust-toolchain.toml` (`rustup show`).
- Verify vendored libraries exist and have correct permissions (`ls -lh vendor/vosk/lib/libvosk.so`).
- Use `pkg-config --version` and `ldd target/debug/coldvox-app` to track linkage issues.
- If caches look corrupted, `cargo clean -p <crate>` or prune `~/.cargo/git/db`.

### 4. Fix and Validate

- Apply the smallest change that reproduces the fix.
- Re-run the command locally until it passes.
- Trigger the workflow manually if needed: `gh workflow run <name> --ref <branch>`.
- Archive logs with notes for future reference.
- Monitor `journalctl -u actions.runner.Coldaine-ColdVox.laptop-extra.service --since "10 minutes ago" --reverse` for follow-up errors.

## Diagnostics & Local Capabilities

- **System Logs**: `journalctl`, `/var/log/` (full access on the host).
- **Hardware Access**: Audio devices listed via `cargo run --bin mic_probe -- --list-devices`.
- **Performance Profiling**: `cargo build --timings`, `perf record`, `hyperfine` for targeted benchmarks.
- **Network/Port Checks**: `ss -tlpn` to detect stray listeners from previous runs.
- **Resource Monitoring**: `htop`, `iotop`, `nvidia-smi` (if GPU workflows are added).
- **Interactive Apps**: Launch TUI dashboard locally (`cargo run --bin tui_dashboard`).

Because you control the host, you can inspect services, adjust dependencies, or install tooling immediately—take advantage of that speed compared to cloud runners.

## Scripts & Automation

| Script | Purpose |
| --- | --- |
| `scripts/runner_health_check.sh` | Aggregated health report (CPU, disk, toolchain) |
| `scripts/ci/setup-vosk-cache.sh` | Ensures Vosk models are present before CI |
| `scripts/performance_monitor.sh` | Track build/test timing trends |
| `docs/dev/runnerAgent/scripts/*` | Additional helpers maintained beside this handbook |

Run scripts from the repository root unless otherwise noted. Update or extend them when recurring issues appear.

## Further Reading

- [RunnerAgent Architecture](RunnerAgent.md) – deep dive into system design and rationale
- [Runner Debugging Guide](prompts/debug_agent_prompt.md) – step-by-step investigation playbook
- [CI Workflows](../../.github/workflows/) – authoritative job definitions
- [Debug Runs](debug_runs/) – historical notes from previous incidents

Add new findings, incident reports, and handy commands here so the handbook stays the single source of truth for runner operations.
