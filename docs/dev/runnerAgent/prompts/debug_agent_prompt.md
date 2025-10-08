# Runner Debugging Guide

Use this guide when a ColdVox GitHub Actions workflow fails on the self-hosted runner `laptop-extra` (Nobara Linux, x86_64). It highlights the environment, the fastest checks to run, and practical commands for reproducing and fixing issues.

## Environment Snapshot
- **Runner host**: `laptop-extra`
- **Runner install dir**: `/home/coldaine/actions-runner/`
- **Workflow workspace**: `/home/coldaine/actions-runner/_work/ColdVox/ColdVox`
- **Project type**: Rust multi-crate workspace with native dependencies (Vosk STT, text injection)
- **Key vendor asset**: `/home/coldaine/Projects/ColdVox/vendor/vosk/lib/libvosk.so`

## Triage Checklist
1. Record the failing workflow run URL and job name.
2. Download the job logs (via GitHub UI or `gh run view --log`).
3. Capture runner service health (`systemctl status`).
4. Pull the last 200 lines of runner logs (`journalctl`).
5. Re-run the failing command from the workflow locally.
6. Verify toolchain versions and vendored libraries.
7. Apply the smallest fix, re-run CI locally if possible, then push.

## Runner Health Commands
```bash
# Check service status (look for failed units or restart loops)
systemctl status actions.runner.Coldaine-ColdVox.laptop-extra.service

# Tail runner logs for recent failures
journalctl -u actions.runner.Coldaine-ColdVox.laptop-extra.service -n 200 --no-pager

# Full health check script (includes disk, CPU, and dependency checks)
bash /home/coldaine/Projects/ColdVox/scripts/runner_health_check.sh
```

## Reproduce the Workflow Locally
```bash
# Enter the workflow checkout
cd /home/coldaine/actions-runner/_work/ColdVox/ColdVox

# Match the branch and commit from the failing run
git fetch origin <branch>
git checkout <commit_sha>

# Ensure the workspace matches the CI runner clean state
git status --short

# Recreate required environment variables (copy from job log, or re-run with the same matrix)
export CI=1
export VOSK_MODEL_PATH="/home/coldaine/Projects/ColdVox/models/vosk-model-small-en-us-0.15"
# ...export any other KEY=VALUE pairs referenced in the failing step

# Run the failing job step (paste from workflow log)
just <target>            # or
cargo <subcommand> ...
```

### Tips
- Use `cargo check` before `cargo build` for quick compiler feedback.
- Add `--features vosk` or other feature flags to mirror the CI job.
- For flaky tests, run them with `-- --nocapture` to get full output.
- Keep `CARGO_TERM_COLOR=always` and `RUST_BACKTRACE=1` handy for verbose error traces.

## Dependency Verification
```bash
cargo --version
rustc --version
gh --version

# Confirm Vosk native library is available
ls -lh /home/coldaine/Projects/ColdVox/vendor/vosk/lib/libvosk.so

# Ensure expected Python tools exist when workflows call them
python3 --version
pip show maturin

# Validate pkg-config and native linkage when builds fail with linker errors
pkg-config --version
ldd target/debug/coldvox-app | grep libvosk || true
```

## Useful Diagnostics
- **List audio devices**: `cargo run --bin mic_probe -- --list-devices`
- **Validate Vosk setup**: `cargo run --features vosk,examples --example vosk_test`
- **Text injection sanity check**: `cargo run --features text-injection --example inject_demo`

## Investigating Common Failures
- **Toolchain drift**: `rustup show` and compare with `rust-toolchain.toml`.
- **Permission errors**: Check that the runner user owns `/home/coldaine/actions-runner` and the workspace (`stat -c "%U:%G"`).
- **Out-of-disk**: `df -h /home/coldaine` and prune `target/` directories as needed.
- **Port conflicts**: Use `ss -tlpn` to find lingering servers started by tests.
- **Corrupted caches**: `cargo clean -p <crate>` or remove `~/.cargo/git/db` when fetch/build artifacts are inconsistent.

## After the Fix
1. Rerun the failing command locally until it passes.
2. Optionally trigger the workflow manually: `gh workflow run <name> --ref <branch>`.
3. Archive the captured job log with your notes for future reference.
4. Monitor the runner logs for stability for at least 10 minutes after the fix.
5. Document the root cause and remediation in the PR or `docs/dev/runnerAgent/`.

## Collecting Workflow Logs
- **Latest run**: `gh run view --log --web`
- **Specific run**: `gh run view <run-id> --log > run-<id>.log`
- **Artifacts**: `gh run download <run-id>`
- **Matrix job**: `gh run view <run-id> --job <job-id> --log`

## Related References
- [RunnerAgent Architecture](../RunnerAgent.md)
- [Performance Monitoring Prompt](performance_monitor_prompt.md)
- [System Update Prompt](system_update_prompt.md)
